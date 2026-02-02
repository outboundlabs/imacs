//! Go parser - parses Go source code to AST

use crate::ast::*;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

use super::node_span;

pub fn parse_go(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    let root = tree.root_node();
    let mut functions = Vec::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_declaration" {
            if let Some(func) = parse_go_function(child, source) {
                functions.push(func);
            }
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let source_hash = format!("sha256:{}", hex::encode(&hasher.finalize()[..8]));

    Ok(CodeAst {
        language: Language::Go,
        functions,
        source_hash,
    })
}

fn parse_go_function(node: Node, source: &str) -> Option<Function> {
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
            "parameter_list" => {
                params = parse_go_parameters(child, source);
            }
            "type_identifier" | "qualified_type" => {
                return_type = Some(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            }
            "block" => {
                body = Some(parse_go_block(child, source));
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

fn parse_go_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
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
                    "type_identifier" | "qualified_type" | "pointer_type" | "slice_type" => {
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

fn parse_go_block(node: Node, source: &str) -> AstNode {
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
                    if ret_child.kind() != "return" {
                        let expr = parse_go_expr(ret_child, source);
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
                let expr = parse_go_if(*child, source);
                if is_last {
                    result = Some(Box::new(expr));
                } else {
                    statements.push(expr);
                }
            }
            "expression_statement" => {
                if let Some(expr_child) = child.child(0) {
                    statements.push(parse_go_expr(expr_child, source));
                }
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

fn parse_go_if(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut then_branch = None;
    let mut else_branch = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" => {}
            "block" if then_branch.is_none() => {
                then_branch = Some(Box::new(parse_go_block(child, source)));
            }
            "block" if then_branch.is_some() => {
                else_branch = Some(Box::new(parse_go_block(child, source)));
            }
            "if_statement" => {
                else_branch = Some(Box::new(parse_go_if(child, source)));
            }
            "else" => {}
            _ if condition.is_none() => {
                condition = Some(Box::new(parse_go_expr(child, source)));
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

fn parse_go_expr(node: Node, source: &str) -> AstNode {
    match node.kind() {
        "int_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0");
            AstNode::Literal {
                value: LiteralValue::Int(text.parse().unwrap_or(0)),
                span: node_span(node),
            }
        }
        "float_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0.0");
            AstNode::Literal {
                value: LiteralValue::Float(text.parse().unwrap_or(0.0)),
                span: node_span(node),
            }
        }
        "interpreted_string_literal" | "raw_string_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("\"\"");
            let value = text.trim_matches(|c| c == '"' || c == '`').to_string();
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
                let left = Box::new(parse_go_expr(children[0], source));
                let op_text = children[1].utf8_text(source.as_bytes()).unwrap_or("");
                let right = Box::new(parse_go_expr(children[2], source));
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
                    return parse_go_expr(child, source);
                }
            }
            AstNode::Unknown {
                kind: "empty_parens".into(),
                span: node_span(node),
            }
        }
        "selector_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_go_expr(children[0], source));
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
                    kind: "selector_expression".into(),
                    span: node_span(node),
                }
            }
        }
        "for_statement" => {
            // Go: for i := 0; i < n; i++ { } or for range collection { }
            let mut counter = String::new();
            let mut collection = None;
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "for_clause" => {
                        // Traditional for loop: for i := 0; i < n; i++
                        let mut clause_cursor = child.walk();
                        for clause_child in child.children(&mut clause_cursor) {
                            if clause_child.kind() == "short_var_declaration" {
                                let mut decl_cursor = clause_child.walk();
                                for decl_child in clause_child.children(&mut decl_cursor) {
                                    if decl_child.kind() == "expression_list" {
                                        if let Some(id) = decl_child.child(0) {
                                            counter = id
                                                .utf8_text(source.as_bytes())
                                                .unwrap_or("")
                                                .to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "range_clause" => {
                        // Range loop: for i, v := range collection
                        let mut range_cursor = child.walk();
                        for range_child in child.children(&mut range_cursor) {
                            if range_child.kind() == "expression_list" {
                                if let Some(id) = range_child.child(0) {
                                    counter =
                                        id.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                                }
                            } else if range_child.kind() != "range"
                                && range_child.kind() != ":="
                                && range_child.kind() != ","
                            {
                                collection = Some(Box::new(parse_go_expr(range_child, source)));
                            }
                        }
                    }
                    "block" => {
                        body = Some(Box::new(parse_go_block(child, source)));
                    }
                    _ => {}
                }
            }
            if let Some(coll) = collection {
                AstNode::ForEach {
                    item: counter,
                    index: None,
                    collection: coll,
                    body: body.unwrap_or(Box::new(AstNode::Block {
                        statements: vec![],
                        result: None,
                        span: node_span(node),
                    })),
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "for_statement".into(),
                    span: node_span(node),
                }
            }
        }
        "assignment_statement" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let target = Box::new(parse_go_expr(children[0], source));
                let value = Box::new(parse_go_expr(children[children.len() - 1], source));
                AstNode::Assign {
                    target,
                    value,
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "assignment_incomplete".into(),
                    span: node_span(node),
                }
            }
        }
        "func_literal" => {
            let mut params = Vec::new();
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "parameter_list" => {
                        let mut param_cursor = child.walk();
                        for param_child in child.children(&mut param_cursor) {
                            if param_child.kind() == "parameter_declaration" {
                                let mut decl_cursor = param_child.walk();
                                for decl_child in param_child.children(&mut decl_cursor) {
                                    if decl_child.kind() == "identifier" {
                                        params.push(
                                            decl_child
                                                .utf8_text(source.as_bytes())
                                                .unwrap_or("")
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    "block" => {
                        body = Some(Box::new(parse_go_block(child, source)));
                    }
                    _ => {}
                }
            }
            AstNode::Closure {
                params,
                body: body.unwrap_or(Box::new(AstNode::Literal {
                    value: LiteralValue::Unit,
                    span: node_span(node),
                })),
                span: node_span(node),
            }
        }
        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}
