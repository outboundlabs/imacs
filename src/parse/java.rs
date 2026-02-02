//! Java parser - parses Java source code to AST

use crate::ast::*;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

use super::node_span;

pub fn parse_java(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    let root = tree.root_node();
    let mut functions = Vec::new();

    // Recursively find method declarations
    fn find_methods(node: Node, source: &str, functions: &mut Vec<Function>) {
        if node.kind() == "method_declaration" {
            if let Some(func) = parse_java_method(node, source) {
                functions.push(func);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            find_methods(child, source, functions);
        }
    }

    find_methods(root, source, &mut functions);

    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let source_hash = format!("sha256:{}", hex::encode(&hasher.finalize()[..8]));

    Ok(CodeAst {
        language: Language::Java,
        functions,
        source_hash,
    })
}

fn parse_java_method(node: Node, source: &str) -> Option<Function> {
    let mut name = String::new();
    let mut params = Vec::new();
    let mut return_type = None;
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" if name.is_empty() => {
                name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            "formal_parameters" => {
                params = parse_java_parameters(child, source);
            }
            "void_type"
            | "integral_type"
            | "floating_point_type"
            | "boolean_type"
            | "type_identifier" => {
                return_type = Some(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            }
            "block" => {
                body = Some(parse_java_block(child, source));
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(Function {
        name,
        params,
        return_type,
        body: body.unwrap_or(AstNode::Block {
            statements: vec![],
            result: None,
            span: node_span(node),
        }),
        span: node_span(node),
    })
}

fn parse_java_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "formal_parameter" {
            let mut name = String::new();
            let mut typ = String::new();

            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                match param_child.kind() {
                    "identifier" => {
                        name = param_child
                            .utf8_text(source.as_bytes())
                            .unwrap_or("")
                            .to_string();
                    }
                    "integral_type"
                    | "floating_point_type"
                    | "boolean_type"
                    | "type_identifier"
                    | "generic_type" => {
                        typ = param_child
                            .utf8_text(source.as_bytes())
                            .unwrap_or("")
                            .to_string();
                    }
                    _ => {}
                }
            }

            if !name.is_empty() {
                params.push(Parameter { name, typ });
            }
        }
    }

    params
}

fn parse_java_block(node: Node, source: &str) -> AstNode {
    let mut statements = Vec::new();
    let mut result = None;
    let mut cursor = node.walk();

    let children: Vec<_> = node.children(&mut cursor).collect();
    let last_stmt_idx = children
        .iter()
        .rposition(|c| c.kind() != "{" && c.kind() != "}");

    for (i, child) in children.iter().enumerate() {
        let is_last = Some(i) == last_stmt_idx;

        match child.kind() {
            "{" | "}" => continue,
            "return_statement" => {
                let mut ret_cursor = child.walk();
                for ret_child in child.children(&mut ret_cursor) {
                    if ret_child.kind() != "return" && ret_child.kind() != ";" {
                        let expr = parse_java_expr(ret_child, source);
                        if is_last {
                            result = Some(Box::new(expr));
                        } else {
                            statements.push(AstNode::Return {
                                value: Some(Box::new(expr)),
                                span: node_span(*child),
                            });
                        }
                        break;
                    }
                }
            }
            "if_statement" => {
                let expr = parse_java_if(*child, source);
                if is_last {
                    result = Some(Box::new(expr));
                } else {
                    statements.push(expr);
                }
            }
            "local_variable_declaration" => {
                statements.push(parse_java_let(*child, source));
            }
            _ => {}
        }
    }

    AstNode::Block {
        statements,
        result,
        span: node_span(node),
    }
}

fn parse_java_let(node: Node, source: &str) -> AstNode {
    let mut name = String::new();
    let mut value = None;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let mut decl_cursor = child.walk();
            for decl_child in child.children(&mut decl_cursor) {
                match decl_child.kind() {
                    "identifier" => {
                        name = decl_child
                            .utf8_text(source.as_bytes())
                            .unwrap_or("")
                            .to_string();
                    }
                    _ if value.is_none()
                        && decl_child.kind() != "="
                        && decl_child.kind() != "dimensions" =>
                    {
                        value = Some(Box::new(parse_java_expr(decl_child, source)));
                    }
                    _ => {}
                }
            }
        }
    }

    AstNode::Let {
        name,
        value: value.unwrap_or(Box::new(AstNode::Literal {
            value: LiteralValue::Unit,
            span: node_span(node),
        })),
        span: node_span(node),
    }
}

fn parse_java_if(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut then_branch = None;
    let mut else_branch = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" | "(" | ")" => {}
            "parenthesized_expression" => {
                condition = Some(Box::new(parse_java_expr(child, source)));
            }
            "block" if then_branch.is_none() => {
                then_branch = Some(Box::new(parse_java_block(child, source)));
            }
            "block" if then_branch.is_some() && else_branch.is_none() => {
                else_branch = Some(Box::new(parse_java_block(child, source)));
            }
            "if_statement" => {
                else_branch = Some(Box::new(parse_java_if(child, source)));
            }
            "else" => {}
            _ if condition.is_none() => {
                condition = Some(Box::new(parse_java_expr(child, source)));
            }
            _ => {}
        }
    }

    AstNode::If {
        condition: condition.unwrap_or(Box::new(AstNode::Literal {
            value: LiteralValue::Bool(true),
            span: node_span(node),
        })),
        then_branch: then_branch.unwrap_or(Box::new(AstNode::Block {
            statements: vec![],
            result: None,
            span: node_span(node),
        })),
        else_branch,
        span: node_span(node),
    }
}

fn parse_java_expr(node: Node, source: &str) -> AstNode {
    match node.kind() {
        "decimal_integer_literal" | "hex_integer_literal" | "octal_integer_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0");
            let value = text
                .trim_end_matches(|c: char| c.is_alphabetic())
                .parse()
                .unwrap_or(0);
            AstNode::Literal {
                value: LiteralValue::Int(value),
                span: node_span(node),
            }
        }
        "decimal_floating_point_literal" | "hex_floating_point_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0.0");
            let value = text
                .trim_end_matches(|c: char| c.is_alphabetic())
                .parse()
                .unwrap_or(0.0);
            AstNode::Literal {
                value: LiteralValue::Float(value),
                span: node_span(node),
            }
        }
        "string_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("\"\"");
            let value = text.trim_matches('"').to_string();
            AstNode::Literal {
                value: LiteralValue::String(value),
                span: node_span(node),
            }
        }
        "true" => AstNode::Literal {
            value: LiteralValue::Bool(true),
            span: node_span(node),
        },
        "false" => AstNode::Literal {
            value: LiteralValue::Bool(false),
            span: node_span(node),
        },
        "identifier" => AstNode::Var {
            name: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            span: node_span(node),
        },
        "binary_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let left = Box::new(parse_java_expr(children[0], source));
                let op_text = children[1].utf8_text(source.as_bytes()).unwrap_or("");
                let right = Box::new(parse_java_expr(children[2], source));
                let op = match op_text {
                    "+" => BinaryOp::Add,
                    "-" => BinaryOp::Sub,
                    "*" => BinaryOp::Mul,
                    "/" => BinaryOp::Div,
                    "%" => BinaryOp::Mod,
                    "==" => BinaryOp::Eq,
                    "!=" => BinaryOp::Ne,
                    "<" => BinaryOp::Lt,
                    "<=" => BinaryOp::Le,
                    ">" => BinaryOp::Gt,
                    ">=" => BinaryOp::Ge,
                    "&&" => BinaryOp::And,
                    "||" => BinaryOp::Or,
                    _ => {
                        return AstNode::Unknown {
                            kind: format!("unknown_op:{}", op_text),
                            span: node_span(node),
                        }
                    }
                };
                AstNode::Binary {
                    op,
                    left,
                    right,
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "binary_incomplete".into(),
                    span: node_span(node),
                }
            }
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_java_expr(child, source);
                }
            }
            AstNode::Unknown {
                kind: "empty_parens".into(),
                span: node_span(node),
            }
        }
        "field_access" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_java_expr(children[0], source));
                let field = children
                    .last()
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .unwrap_or("")
                    .to_string();
                AstNode::Field {
                    object,
                    field,
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "field_access".into(),
                    span: node_span(node),
                }
            }
        }
        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}
