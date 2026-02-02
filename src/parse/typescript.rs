//! TypeScript parser - parses TypeScript/JavaScript source code to AST

use crate::ast::*;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

use super::node_span;

pub fn parse_typescript(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    let root = tree.root_node();
    let mut functions = Vec::new();

    // Walk top-level items looking for functions
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "export_statement" => {
                if let Some(func) = parse_ts_function(child, source) {
                    functions.push(func);
                }
            }
            _ => {}
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let source_hash = format!("sha256:{}", hex::encode(&hasher.finalize()[..8]));

    Ok(CodeAst {
        language: Language::TypeScript,
        functions,
        source_hash,
    })
}

fn parse_ts_function(node: Node, source: &str) -> Option<Function> {
    // Handle export_statement wrapper
    if node.kind() == "export_statement" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_declaration" {
                return parse_ts_function(child, source);
            }
        }
        return None;
    }

    let mut name = String::new();
    let mut params = Vec::new();
    let mut return_type = None;
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            "formal_parameters" => {
                params = parse_ts_parameters(child, source);
            }
            "type_annotation" => {
                return_type = Some(
                    child
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .trim_start_matches(": ")
                        .to_string(),
                );
            }
            "statement_block" => {
                body = Some(parse_ts_block(child, source));
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

fn parse_ts_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
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
                    "type_annotation" => {
                        typ = param_child
                            .utf8_text(source.as_bytes())
                            .unwrap_or("")
                            .trim_start_matches(": ")
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

fn parse_ts_block(node: Node, source: &str) -> AstNode {
    let mut statements = Vec::new();
    let mut result = None;
    let mut cursor = node.walk();

    let children: Vec<_> = node.children(&mut cursor).collect();
    let last_expr_idx = children
        .iter()
        .rposition(|c| c.kind() != "{" && c.kind() != "}");

    for (i, child) in children.iter().enumerate() {
        let is_last = Some(i) == last_expr_idx;

        match child.kind() {
            "{" | "}" => continue,
            "return_statement" => {
                let mut ret_cursor = child.walk();
                for ret_child in child.children(&mut ret_cursor) {
                    if ret_child.kind() != "return" && ret_child.kind() != ";" {
                        let expr = parse_ts_expr(ret_child, source);
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
                let expr = parse_ts_if(*child, source);
                if is_last {
                    result = Some(Box::new(expr));
                } else {
                    statements.push(expr);
                }
            }
            "expression_statement" => {
                if let Some(expr_child) = child.child(0) {
                    let expr = parse_ts_expr(expr_child, source);
                    if is_last {
                        result = Some(Box::new(expr));
                    } else {
                        statements.push(expr);
                    }
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                statements.push(parse_ts_let(*child, source));
            }
            _ => {
                let expr = parse_ts_expr(*child, source);
                if is_last {
                    result = Some(Box::new(expr));
                } else {
                    statements.push(expr);
                }
            }
        }
    }

    AstNode::Block {
        statements,
        result,
        span: node_span(node),
    }
}

fn parse_ts_let(node: Node, source: &str) -> AstNode {
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
                        && decl_child.kind() != "type_annotation" =>
                    {
                        value = Some(Box::new(parse_ts_expr(decl_child, source)));
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

fn parse_ts_if(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut then_branch = None;
    let mut else_branch = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" => {}
            "parenthesized_expression" => {
                condition = Some(Box::new(parse_ts_expr(child, source)));
            }
            "statement_block" if then_branch.is_none() => {
                then_branch = Some(Box::new(parse_ts_block(child, source)));
            }
            "else_clause" => {
                let mut else_cursor = child.walk();
                for else_child in child.children(&mut else_cursor) {
                    match else_child.kind() {
                        "else" => {}
                        "statement_block" => {
                            else_branch = Some(Box::new(parse_ts_block(else_child, source)));
                        }
                        "if_statement" => {
                            else_branch = Some(Box::new(parse_ts_if(else_child, source)));
                        }
                        _ => {}
                    }
                }
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

fn parse_ts_expr(node: Node, source: &str) -> AstNode {
    match node.kind() {
        "number" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0");
            if text.contains('.') {
                AstNode::Literal {
                    value: LiteralValue::Float(text.parse().unwrap_or(0.0)),
                    span: node_span(node),
                }
            } else {
                AstNode::Literal {
                    value: LiteralValue::Int(text.parse().unwrap_or(0)),
                    span: node_span(node),
                }
            }
        }
        "string" | "template_string" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("\"\"");
            let value = text
                .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                .to_string();
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
        "binary_expression" => parse_ts_binary(node, source),
        "unary_expression" => parse_ts_unary(node, source),
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_ts_expr(child, source);
                }
            }
            AstNode::Unknown {
                kind: "empty_parens".into(),
                span: node_span(node),
            }
        }
        "call_expression" => {
            let mut function = String::new();
            let mut args = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" | "member_expression" if function.is_empty() => {
                        function = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "arguments" => {
                        let mut arg_cursor = child.walk();
                        for arg_child in child.children(&mut arg_cursor) {
                            if arg_child.kind() != "("
                                && arg_child.kind() != ")"
                                && arg_child.kind() != ","
                            {
                                args.push(parse_ts_expr(arg_child, source));
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
        "member_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let object = Box::new(parse_ts_expr(children[0], source));
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
                    kind: "member_expression".into(),
                    span: node_span(node),
                }
            }
        }
        "for_statement" | "for_in_statement" => {
            // for (let i = 0; i < n; i++) or for (item of collection)
            let mut counter = String::new();
            let mut collection = None;
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" if counter.is_empty() => {
                        counter = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "variable_declaration" => {
                        let mut decl_cursor = child.walk();
                        for decl_child in child.children(&mut decl_cursor) {
                            if decl_child.kind() == "variable_declarator" {
                                if let Some(id) = decl_child.child_by_field_name("name") {
                                    counter =
                                        id.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                                }
                            }
                        }
                    }
                    "statement_block" => {
                        body = Some(Box::new(parse_ts_block(child, source)));
                    }
                    _ if collection.is_none()
                        && !counter.is_empty()
                        && child.kind() != "for"
                        && child.kind() != "("
                        && child.kind() != ")"
                        && child.kind() != "of"
                        && child.kind() != "in" =>
                    {
                        collection = Some(Box::new(parse_ts_expr(child, source)));
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
        "while_statement" => {
            let mut condition = None;
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "parenthesized_expression" if condition.is_none() => {
                        condition = Some(Box::new(parse_ts_expr(child, source)));
                    }
                    "statement_block" => {
                        body = Some(Box::new(parse_ts_block(child, source)));
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
                    "statement_block" if try_block.is_none() => {
                        try_block = Some(Box::new(parse_ts_block(child, source)));
                    }
                    "catch_clause" => {
                        let mut catch_cursor = child.walk();
                        for catch_child in child.children(&mut catch_cursor) {
                            match catch_child.kind() {
                                "identifier" => {
                                    catch_var = Some(
                                        catch_child
                                            .utf8_text(source.as_bytes())
                                            .unwrap_or("")
                                            .to_string(),
                                    );
                                }
                                "statement_block" => {
                                    catch_block =
                                        Some(Box::new(parse_ts_block(catch_child, source)));
                                }
                                _ => {}
                            }
                        }
                    }
                    "finally_clause" => {
                        let mut fin_cursor = child.walk();
                        for fin_child in child.children(&mut fin_cursor) {
                            if fin_child.kind() == "statement_block" {
                                finally_block = Some(Box::new(parse_ts_block(fin_child, source)));
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
        "assignment_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let target = Box::new(parse_ts_expr(children[0], source));
                let value = Box::new(parse_ts_expr(children[2], source));
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
        "await_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "await" {
                    return AstNode::Await {
                        expr: Box::new(parse_ts_expr(child, source)),
                        span: node_span(node),
                    };
                }
            }
            AstNode::Unknown {
                kind: "await_empty".into(),
                span: node_span(node),
            }
        }
        "arrow_function" | "function_expression" => {
            let mut params = Vec::new();
            let mut body = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "formal_parameters" => {
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
                    "identifier" if params.is_empty() => {
                        // Single param arrow function: x => ...
                        params.push(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    }
                    "statement_block" => {
                        body = Some(Box::new(parse_ts_block(child, source)));
                    }
                    _ if body.is_none() && child.kind() != "=>" => {
                        body = Some(Box::new(parse_ts_expr(child, source)));
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

fn parse_ts_binary(node: Node, source: &str) -> AstNode {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() < 3 {
        return AstNode::Unknown {
            kind: "binary_incomplete".into(),
            span: node_span(node),
        };
    }

    let left = Box::new(parse_ts_expr(children[0], source));
    let op_text = children[1].utf8_text(source.as_bytes()).unwrap_or("");
    let right = Box::new(parse_ts_expr(children[2], source));

    let op = match op_text {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        "%" => BinaryOp::Mod,
        "==" | "===" => BinaryOp::Eq,
        "!=" | "!==" => BinaryOp::Ne,
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
}

fn parse_ts_unary(node: Node, source: &str) -> AstNode {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() < 2 {
        return AstNode::Unknown {
            kind: "unary_incomplete".into(),
            span: node_span(node),
        };
    }

    let op_text = children[0].utf8_text(source.as_bytes()).unwrap_or("");
    let operand = Box::new(parse_ts_expr(children[1], source));

    let op = match op_text {
        "-" => UnaryOp::Neg,
        "!" => UnaryOp::Not,
        _ => {
            return AstNode::Unknown {
                kind: format!("unknown_unary:{}", op_text),
                span: node_span(node),
            }
        }
    };

    AstNode::Unary {
        op,
        operand,
        span: node_span(node),
    }
}
