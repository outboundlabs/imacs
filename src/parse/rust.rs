//! Rust parser - parses Rust source code to AST

use crate::ast::*;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

use super::node_span;

pub fn parse_rust(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    let root = tree.root_node();
    let mut functions = Vec::new();

    // Walk top-level items looking for functions
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_item" {
            if let Some(func) = parse_rust_function(child, source) {
                functions.push(func);
            }
        }
    }

    // Compute source hash
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let source_hash = format!("sha256:{}", hex::encode(&hasher.finalize()[..8]));

    Ok(CodeAst {
        language: Language::Rust,
        functions,
        source_hash,
    })
}

fn parse_rust_function(node: Node, source: &str) -> Option<Function> {
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
            "parameters" => {
                params = parse_rust_parameters(child, source);
            }
            "type_identifier" | "generic_type" | "primitive_type" => {
                // Return type
                return_type = Some(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            }
            "block" => {
                body = Some(parse_rust_block(child, source));
            }
            _ => {}
        }
    }

    // Also check for return type in a different position
    if return_type.is_none() {
        if let Some(ret_type_node) = node.child_by_field_name("return_type") {
            return_type = Some(
                ret_type_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .trim_start_matches("-> ")
                    .trim()
                    .to_string(),
            );
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

fn parse_rust_parameters(node: Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "parameter" {
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
                    "type_identifier" | "primitive_type" | "generic_type" | "reference_type" => {
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

fn parse_rust_block(node: Node, source: &str) -> AstNode {
    let mut statements = Vec::new();
    let mut result = None;
    let mut cursor = node.walk();

    let children: Vec<_> = node.children(&mut cursor).collect();

    // Find the index of the last non-brace child
    let last_expr_idx = children
        .iter()
        .rposition(|c| c.kind() != "{" && c.kind() != "}");

    for (i, child) in children.iter().enumerate() {
        let is_last = Some(i) == last_expr_idx;

        match child.kind() {
            "{" | "}" => continue,
            "expression_statement" => {
                if let Some(expr) = child.child(0) {
                    let parsed = parse_rust_expr(expr, source);
                    // If this is the last item, it's the block's result value
                    if is_last {
                        result = Some(Box::new(parsed));
                    } else {
                        statements.push(parsed);
                    }
                }
            }
            "let_declaration" => {
                statements.push(parse_rust_let(*child, source));
            }
            "return_expression" => {
                let value = child.child(1).map(|c| Box::new(parse_rust_expr(c, source)));
                if is_last {
                    result = value;
                } else {
                    statements.push(AstNode::Return {
                        value,
                        span: node_span(*child),
                    });
                }
            }
            _ => {
                let expr = parse_rust_expr(*child, source);
                // Last expression without semicolon is the result
                if is_last && !child.kind().ends_with("_statement") {
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

fn parse_rust_let(node: Node, source: &str) -> AstNode {
    let mut name = String::new();
    let mut value = None;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            "=" => {}
            _ if value.is_none() && child.kind() != "let" && child.kind() != "mut" => {
                // The value expression
                if child.kind() != "type_identifier" && child.kind() != ":" && child.kind() != ";" {
                    value = Some(Box::new(parse_rust_expr(child, source)));
                }
            }
            _ => {}
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

fn parse_rust_expr(node: Node, source: &str) -> AstNode {
    match node.kind() {
        "integer_literal" => {
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

        "float_literal" => {
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

        "boolean_literal" | "true" | "false" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("false");
            AstNode::Literal {
                value: LiteralValue::Bool(text == "true"),
                span: node_span(node),
            }
        }

        "identifier" => AstNode::Var {
            name: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            span: node_span(node),
        },

        "binary_expression" => parse_rust_binary(node, source),

        "unary_expression" => parse_rust_unary(node, source),

        "if_expression" => parse_rust_if(node, source),

        "match_expression" => parse_rust_match(node, source),

        "block" => parse_rust_block(node, source),

        "return_expression" => {
            let value = node.child(1).map(|c| Box::new(parse_rust_expr(c, source)));
            AstNode::Return {
                value,
                span: node_span(node),
            }
        }

        "call_expression" => parse_rust_call(node, source),

        "field_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_rust_expr(children[0], source));
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
                    kind: "field_expression".into(),
                    span: node_span(node),
                }
            }
        }

        "index_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_rust_expr(children[0], source));
                let index = Box::new(parse_rust_expr(children[1], source));
                AstNode::Index {
                    object,
                    index,
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "index_expression".into(),
                    span: node_span(node),
                }
            }
        }

        "tuple_expression" => {
            let mut elements = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    elements.push(parse_rust_expr(child, source));
                }
            }
            AstNode::Tuple {
                elements,
                span: node_span(node),
            }
        }

        "array_expression" => {
            let mut elements = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "[" && child.kind() != "]" && child.kind() != "," {
                    elements.push(parse_rust_expr(child, source));
                }
            }
            AstNode::Array {
                elements,
                span: node_span(node),
            }
        }

        "parenthesized_expression" => {
            // Unwrap parentheses
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_rust_expr(child, source);
                }
            }
            AstNode::Unknown {
                kind: "empty_parens".into(),
                span: node_span(node),
            }
        }

        "unit_expression" | "()" => AstNode::Literal {
            value: LiteralValue::Unit,
            span: node_span(node),
        },

        "for_expression" => parse_rust_for(node, source),

        "while_expression" => parse_rust_while(node, source),

        "loop_expression" => parse_rust_loop(node, source),

        "assignment_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let target = Box::new(parse_rust_expr(children[0], source));
                let value = Box::new(parse_rust_expr(children[2], source));
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
                if child.kind() != "." && child.kind() != "await" {
                    return AstNode::Await {
                        expr: Box::new(parse_rust_expr(child, source)),
                        span: node_span(node),
                    };
                }
            }
            AstNode::Unknown {
                kind: "await_empty".into(),
                span: node_span(node),
            }
        }

        "closure_expression" => parse_rust_closure(node, source),

        // Macro invocation (e.g., println!(...))
        "macro_invocation" => {
            let mut name = String::new();
            let mut args = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" | "scoped_identifier" if name.is_empty() => {
                        name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "token_tree" => {
                        args = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    _ => {}
                }
            }
            AstNode::MacroCall {
                name,
                args,
                span: node_span(node),
            }
        }

        // Reference expression (e.g., &x, &mut x)
        "reference_expression" => {
            let mut mutable = false;
            let mut expr = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "&" => {}
                    "mutable_specifier" | "mut" => mutable = true,
                    _ => {
                        expr = Some(Box::new(parse_rust_expr(child, source)));
                    }
                }
            }
            if let Some(inner) = expr {
                AstNode::Ref {
                    mutable,
                    expr: inner,
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "reference_empty".into(),
                    span: node_span(node),
                }
            }
        }

        // Type cast expression (e.g., x as i32)
        "type_cast_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let expr = Box::new(parse_rust_expr(children[0], source));
                let target_type = children
                    .last()
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .unwrap_or("")
                    .to_string();
                AstNode::Cast {
                    expr,
                    target_type,
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown {
                    kind: "cast_incomplete".into(),
                    span: node_span(node),
                }
            }
        }

        // Syntax error node from tree-sitter
        "ERROR" => AstNode::SyntaxError {
            message: "Syntax error in source".to_string(),
            source_text: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            span: node_span(node),
        },

        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}

fn parse_rust_binary(node: Node, source: &str) -> AstNode {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() < 3 {
        return AstNode::Unknown {
            kind: "binary_incomplete".into(),
            span: node_span(node),
        };
    }

    let left = Box::new(parse_rust_expr(children[0], source));
    let op_text = children[1].utf8_text(source.as_bytes()).unwrap_or("");
    let right = Box::new(parse_rust_expr(children[2], source));

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
        "&" => BinaryOp::BitAnd,
        "|" => BinaryOp::BitOr,
        "^" => BinaryOp::BitXor,
        "<<" => BinaryOp::Shl,
        ">>" => BinaryOp::Shr,
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

fn parse_rust_unary(node: Node, source: &str) -> AstNode {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() < 2 {
        return AstNode::Unknown {
            kind: "unary_incomplete".into(),
            span: node_span(node),
        };
    }

    let op_text = children[0].utf8_text(source.as_bytes()).unwrap_or("");
    let operand = Box::new(parse_rust_expr(children[1], source));

    let op = match op_text {
        "-" => UnaryOp::Neg,
        "!" => UnaryOp::Not,
        "~" => UnaryOp::BitNot,
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

fn parse_rust_if(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut then_branch = None;
    let mut else_branch = None;

    let mut cursor = node.walk();
    let mut seen_if = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" => seen_if = true,
            "block" if then_branch.is_none() => {
                // First block is the then branch
                then_branch = Some(Box::new(parse_rust_block(child, source)));
            }
            "else_clause" => {
                // Handle else clause - contains either a block or another if_expression
                let mut else_cursor = child.walk();
                for else_child in child.children(&mut else_cursor) {
                    match else_child.kind() {
                        "else" => {} // Skip the keyword
                        "block" => {
                            else_branch = Some(Box::new(parse_rust_block(else_child, source)));
                        }
                        "if_expression" => {
                            // else if
                            else_branch = Some(Box::new(parse_rust_if(else_child, source)));
                        }
                        _ => {}
                    }
                }
            }
            // Legacy handling for older tree-sitter versions
            "else" => {}
            "block" if then_branch.is_some() && else_branch.is_none() => {
                else_branch = Some(Box::new(parse_rust_block(child, source)));
            }
            "if_expression" => {
                // else if (legacy)
                if then_branch.is_some() {
                    else_branch = Some(Box::new(parse_rust_if(child, source)));
                }
            }
            _ if seen_if && condition.is_none() && child.kind() != "let" => {
                condition = Some(Box::new(parse_rust_expr(child, source)));
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

fn parse_rust_match(node: Node, source: &str) -> AstNode {
    let mut scrutinee = None;
    let mut arms = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "match" => {}
            "match_block" => {
                let mut arm_cursor = child.walk();
                for arm_node in child.children(&mut arm_cursor) {
                    if arm_node.kind() == "match_arm" {
                        if let Some(arm) = parse_rust_match_arm(arm_node, source) {
                            arms.push(arm);
                        }
                    }
                }
            }
            _ if scrutinee.is_none() => {
                scrutinee = Some(Box::new(parse_rust_expr(child, source)));
            }
            _ => {}
        }
    }

    AstNode::Match {
        scrutinee: scrutinee.unwrap_or(Box::new(AstNode::Unknown {
            kind: "missing_scrutinee".into(),
            span: node_span(node),
        })),
        arms,
        span: node_span(node),
    }
}

fn parse_rust_match_arm(node: Node, source: &str) -> Option<MatchArm> {
    let mut pattern = None;
    let guard = None;
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "match_pattern" => {
                // Get the actual pattern inside
                let mut pat_cursor = child.walk();
                for pat_child in child.children(&mut pat_cursor) {
                    if pat_child.kind() != "," {
                        pattern = Some(parse_rust_pattern(pat_child, source));
                        break;
                    }
                }
            }
            "if" => {
                // Guard clause follows
            }
            "=>" => {}
            _ if pattern.is_some() && body.is_none() => {
                body = Some(parse_rust_expr(child, source));
            }
            _ if pattern.is_none() => {
                pattern = Some(parse_rust_pattern(child, source));
            }
            _ => {}
        }
    }

    Some(MatchArm {
        pattern: pattern.unwrap_or(Pattern::Wildcard),
        guard,
        body: body.unwrap_or(AstNode::Unknown {
            kind: "missing_arm_body".into(),
            span: node_span(node),
        }),
        span: node_span(node),
    })
}

fn parse_rust_pattern(node: Node, source: &str) -> Pattern {
    match node.kind() {
        "_" => Pattern::Wildcard,

        "identifier" => {
            let name = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            if name == "_" {
                Pattern::Wildcard
            } else {
                Pattern::Binding(name)
            }
        }

        "integer_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0");
            let value = text.parse().unwrap_or(0);
            Pattern::Literal(LiteralValue::Int(value))
        }

        "boolean_literal" | "true" | "false" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("false");
            Pattern::Literal(LiteralValue::Bool(text == "true"))
        }

        "string_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("\"\"");
            let value = text.trim_matches('"').to_string();
            Pattern::Literal(LiteralValue::String(value))
        }

        "tuple_pattern" => {
            let mut elements = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    elements.push(parse_rust_pattern(child, source));
                }
            }
            Pattern::Tuple(elements)
        }

        "or_pattern" => {
            let mut patterns = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "|" {
                    patterns.push(parse_rust_pattern(child, source));
                }
            }
            Pattern::Or(patterns)
        }

        "tuple_struct_pattern" | "struct_pattern" => {
            let mut name = String::new();
            let mut fields = Vec::new();
            let mut cursor = node.walk();

            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" | "scoped_identifier" if name.is_empty() => {
                        name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "(" | ")" | "{" | "}" | "," => {}
                    _ => {
                        fields.push(parse_rust_pattern(child, source));
                    }
                }
            }

            Pattern::Constructor { name, fields }
        }

        "rest_pattern" | ".." => Pattern::Rest,

        _ => {
            // Try to get the text and see if it's a simple case
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "_" {
                Pattern::Wildcard
            } else if text == "true" {
                Pattern::Literal(LiteralValue::Bool(true))
            } else if text == "false" {
                Pattern::Literal(LiteralValue::Bool(false))
            } else if let Ok(i) = text.parse::<i64>() {
                Pattern::Literal(LiteralValue::Int(i))
            } else {
                Pattern::Binding(text.to_string())
            }
        }
    }
}

fn parse_rust_call(node: Node, source: &str) -> AstNode {
    let mut function = String::new();
    let mut args = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "scoped_identifier" | "field_expression" if function.is_empty() => {
                function = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            "arguments" => {
                let mut arg_cursor = child.walk();
                for arg_child in child.children(&mut arg_cursor) {
                    if arg_child.kind() != "(" && arg_child.kind() != ")" && arg_child.kind() != ","
                    {
                        args.push(parse_rust_expr(arg_child, source));
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

fn parse_rust_for(node: Node, source: &str) -> AstNode {
    // Rust for loops: for <pattern> in <expr> { body }
    let mut counter = String::new();
    let mut index = None;
    let mut collection = None;
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" if counter.is_empty() => {
                counter = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            "tuple_pattern" => {
                // for (idx, item) in ...
                let mut tuple_cursor = child.walk();
                let mut parts = Vec::new();
                for tuple_child in child.children(&mut tuple_cursor) {
                    if tuple_child.kind() == "identifier" {
                        parts.push(
                            tuple_child
                                .utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .to_string(),
                        );
                    }
                }
                if parts.len() >= 2 {
                    index = Some(parts[0].clone());
                    counter = parts[1].clone();
                } else if !parts.is_empty() {
                    counter = parts[0].clone();
                }
            }
            "range_expression" => {
                // for i in 0..n
                let mut range_cursor = child.walk();
                let range_children: Vec<_> = child.children(&mut range_cursor).collect();
                if range_children.len() >= 2 {
                    let start = Box::new(parse_rust_expr(range_children[0], source));
                    let end = Box::new(parse_rust_expr(
                        range_children[range_children.len() - 1],
                        source,
                    ));
                    return AstNode::For {
                        counter,
                        start,
                        end,
                        body: body.unwrap_or(Box::new(AstNode::Block {
                            statements: vec![],
                            result: None,
                            span: node_span(node),
                        })),
                        span: node_span(node),
                    };
                }
            }
            "call_expression" | "method_call_expression" | "field_expression" | "identifier" => {
                if collection.is_none() && !counter.is_empty() {
                    collection = Some(Box::new(parse_rust_expr(child, source)));
                }
            }
            "block" => {
                body = Some(Box::new(parse_rust_block(child, source)));
            }
            _ => {}
        }
    }

    // If we have a collection, it's a ForEach
    if let Some(coll) = collection {
        AstNode::ForEach {
            item: counter,
            index,
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
            kind: "for_expression".into(),
            span: node_span(node),
        }
    }
}

fn parse_rust_while(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "while" => {}
            "block" => {
                body = Some(Box::new(parse_rust_block(child, source)));
            }
            _ if condition.is_none() => {
                condition = Some(Box::new(parse_rust_expr(child, source)));
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

fn parse_rust_loop(node: Node, source: &str) -> AstNode {
    // Infinite loop: loop { body }
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            body = Some(Box::new(parse_rust_block(child, source)));
        }
    }

    // Convert to While(true)
    AstNode::While {
        condition: Box::new(AstNode::Literal {
            value: LiteralValue::Bool(true),
            span: node_span(node),
        }),
        body: body.unwrap_or(Box::new(AstNode::Block {
            statements: vec![],
            result: None,
            span: node_span(node),
        })),
        span: node_span(node),
    }
}

fn parse_rust_closure(node: Node, source: &str) -> AstNode {
    let mut params = Vec::new();
    let mut body = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "closure_parameters" => {
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
            "block" => {
                body = Some(Box::new(parse_rust_block(child, source)));
            }
            _ if body.is_none() && child.kind() != "|" && child.kind() != "move" => {
                // Single expression closure body
                body = Some(Box::new(parse_rust_expr(child, source)));
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
