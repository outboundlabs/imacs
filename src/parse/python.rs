//! Python parser - parses Python source code to AST

use crate::ast::*;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

use super::node_span;

pub fn parse_python(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    let root = tree.root_node();
    let mut functions = Vec::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(func) = parse_py_function(child, source) {
                functions.push(func);
            }
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let source_hash = format!("sha256:{}", hex::encode(&hasher.finalize()[..8]));

    Ok(CodeAst {
        language: Language::Python,
        functions,
        source_hash,
    })
}

fn parse_py_function(node: Node, source: &str) -> Option<Function> {
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
            "parameters" => {
                params = parse_py_parameters(child, source);
            }
            "type" => {
                return_type = Some(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            }
            "block" => {
                body = Some(parse_py_block(child, source));
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

fn parse_py_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "typed_parameter" || child.kind() == "identifier" {
            let mut name = String::new();
            let mut typ = String::new();

            if child.kind() == "identifier" {
                name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            } else {
                let mut param_cursor = child.walk();
                for param_child in child.children(&mut param_cursor) {
                    match param_child.kind() {
                        "identifier" => {
                            name = param_child
                                .utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .to_string();
                        }
                        "type" => {
                            typ = param_child
                                .utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .to_string();
                        }
                        _ => {}
                    }
                }
            }

            if !name.is_empty() {
                params.push(Parameter { name, typ });
            }
        }
    }

    params
}

fn parse_py_block(node: Node, source: &str) -> AstNode {
    let mut statements = Vec::new();
    let mut result = None;
    let mut cursor = node.walk();

    let children: Vec<_> = node.children(&mut cursor).collect();
    let last_stmt_idx = children
        .iter()
        .rposition(|c| c.kind() != ":" && !c.kind().is_empty());

    for (i, child) in children.iter().enumerate() {
        let is_last = Some(i) == last_stmt_idx;

        match child.kind() {
            "return_statement" => {
                let mut ret_cursor = child.walk();
                for ret_child in child.children(&mut ret_cursor) {
                    if ret_child.kind() != "return" {
                        let expr = parse_py_expr(ret_child, source);
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
                let expr = parse_py_if(*child, source);
                if is_last {
                    result = Some(Box::new(expr));
                } else {
                    statements.push(expr);
                }
            }
            "expression_statement" => {
                if let Some(expr_child) = child.child(0) {
                    statements.push(parse_py_expr(expr_child, source));
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

fn parse_py_if(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut then_branch = None;
    let mut else_branch = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" | "elif" => {}
            "block" if then_branch.is_none() => {
                then_branch = Some(Box::new(parse_py_block(child, source)));
            }
            "elif_clause" | "else_clause" => {
                let mut else_cursor = child.walk();
                for else_child in child.children(&mut else_cursor) {
                    match else_child.kind() {
                        "else" | "elif" => {}
                        "block" => {
                            else_branch = Some(Box::new(parse_py_block(else_child, source)));
                        }
                        _ if condition.is_none() => {
                            // elif condition
                        }
                        _ => {}
                    }
                }
            }
            _ if condition.is_none() => {
                condition = Some(Box::new(parse_py_expr(child, source)));
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

fn parse_py_expr(node: Node, source: &str) -> AstNode {
    match node.kind() {
        "integer" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0");
            AstNode::Literal {
                value: LiteralValue::Int(text.parse().unwrap_or(0)),
                span: node_span(node),
            }
        }
        "float" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0.0");
            AstNode::Literal {
                value: LiteralValue::Float(text.parse().unwrap_or(0.0)),
                span: node_span(node),
            }
        }
        "string" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("\"\"");
            let value = text.trim_matches(|c| c == '"' || c == '\'').to_string();
            AstNode::Literal {
                value: LiteralValue::String(value),
                span: node_span(node),
            }
        }
        "true" | "True" => AstNode::Literal {
            value: LiteralValue::Bool(true),
            span: node_span(node),
        },
        "false" | "False" => AstNode::Literal {
            value: LiteralValue::Bool(false),
            span: node_span(node),
        },
        "identifier" => AstNode::Var {
            name: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            span: node_span(node),
        },
        "binary_operator" | "comparison_operator" | "boolean_operator" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let left = Box::new(parse_py_expr(children[0], source));
                let op_text = children[1].utf8_text(source.as_bytes()).unwrap_or("");
                let right = Box::new(parse_py_expr(children[2], source));
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
                    "and" => BinaryOp::And,
                    "or" => BinaryOp::Or,
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
                    return parse_py_expr(child, source);
                }
            }
            AstNode::Unknown {
                kind: "empty_parens".into(),
                span: node_span(node),
            }
        }
        "call" => {
            let mut function = String::new();
            let mut args = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" | "attribute" if function.is_empty() => {
                        function = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "argument_list" => {
                        let mut arg_cursor = child.walk();
                        for arg_child in child.children(&mut arg_cursor) {
                            if arg_child.kind() != "("
                                && arg_child.kind() != ")"
                                && arg_child.kind() != ","
                            {
                                args.push(parse_py_expr(arg_child, source));
                            }
                        }
                    }
                    _ => {}
                }
            }
            AstNode::Call {
                function,
                args,
                span: node_span(node),
            }
        }
        "for_statement" => {
            // Python: for item in collection:
            let mut item = String::new();
            let mut collection = None;
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" if item.is_empty() => {
                        item = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "block" => {
                        body = Some(Box::new(parse_py_block(child, source)));
                    }
                    _ if collection.is_none()
                        && !item.is_empty()
                        && child.kind() != "for"
                        && child.kind() != "in"
                        && child.kind() != ":" =>
                    {
                        collection = Some(Box::new(parse_py_expr(child, source)));
                    }
                    _ => {}
                }
            }
            AstNode::ForEach {
                item,
                index: None,
                collection: collection.unwrap_or(Box::new(AstNode::Unknown {
                    kind: "no_collection".into(),
                    span: node_span(node),
                })),
                body: body.unwrap_or(Box::new(AstNode::Block {
                    statements: vec![],
                    result: None,
                    span: node_span(node),
                })),
                span: node_span(node),
            }
        }
        "while_statement" => {
            let mut condition = None;
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "block" => {
                        body = Some(Box::new(parse_py_block(child, source)));
                    }
                    _ if condition.is_none() && child.kind() != "while" && child.kind() != ":" => {
                        condition = Some(Box::new(parse_py_expr(child, source)));
                    }
                    _ => {}
                }
            }
            AstNode::While {
                condition: condition.unwrap_or(Box::new(AstNode::Literal {
                    value: LiteralValue::Bool(true),
                    span: node_span(node),
                })),
                body: body.unwrap_or(Box::new(AstNode::Block {
                    statements: vec![],
                    result: None,
                    span: node_span(node),
                })),
                span: node_span(node),
            }
        }
        "try_statement" => {
            let mut try_block = None;
            let mut catch_var = None;
            let mut catch_block = None;
            let mut finally_block = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "block" if try_block.is_none() => {
                        try_block = Some(Box::new(parse_py_block(child, source)));
                    }
                    "except_clause" => {
                        let mut except_cursor = child.walk();
                        for except_child in child.children(&mut except_cursor) {
                            match except_child.kind() {
                                "identifier" | "as_pattern" => {
                                    catch_var = Some(
                                        except_child
                                            .utf8_text(source.as_bytes())
                                            .unwrap_or("")
                                            .to_string(),
                                    );
                                }
                                "block" => {
                                    catch_block =
                                        Some(Box::new(parse_py_block(except_child, source)));
                                }
                                _ => {}
                            }
                        }
                    }
                    "finally_clause" => {
                        let mut fin_cursor = child.walk();
                        for fin_child in child.children(&mut fin_cursor) {
                            if fin_child.kind() == "block" {
                                finally_block = Some(Box::new(parse_py_block(fin_child, source)));
                            }
                        }
                    }
                    _ => {}
                }
            }
            AstNode::Try {
                try_block: try_block.unwrap_or(Box::new(AstNode::Block {
                    statements: vec![],
                    result: None,
                    span: node_span(node),
                })),
                catch_var,
                catch_block,
                finally_block,
                span: node_span(node),
            }
        }
        "assignment" | "augmented_assignment" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let target = Box::new(parse_py_expr(children[0], source));
                let value = Box::new(parse_py_expr(children[children.len() - 1], source));
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
        "await" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "await" {
                    return AstNode::Await {
                        expr: Box::new(parse_py_expr(child, source)),
                        span: node_span(node),
                    };
                }
            }
            AstNode::Unknown {
                kind: "await_empty".into(),
                span: node_span(node),
            }
        }
        "lambda" => {
            let mut params = Vec::new();
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "lambda_parameters" => {
                        let mut param_cursor = child.walk();
                        for param_child in child.children(&mut param_cursor) {
                            if param_child.kind() == "identifier" {
                                params.push(
                                    param_child
                                        .utf8_text(source.as_bytes())
                                        .unwrap_or("")
                                        .to_string(),
                                );
                            }
                        }
                    }
                    _ if body.is_none() && child.kind() != "lambda" && child.kind() != ":" => {
                        body = Some(Box::new(parse_py_expr(child, source)));
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
