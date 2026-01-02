//! Code parsing via tree-sitter
//!
//! Parses source code into language-agnostic AST.
//! Supports: Rust, TypeScript, Python, Go, C#, Java

use crate::ast::*;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

// Re-export language enum
pub use crate::ast::Language;

/// Parse Rust source code to AST
pub fn parse_rust(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_rust::language())
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
    let last_expr_idx = children.iter().rposition(|c| c.kind() != "{" && c.kind() != "}");

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
    let mut guard = None;
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
                    let end = Box::new(parse_rust_expr(range_children[range_children.len() - 1], source));
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

fn node_span(node: Node) -> Span {
    Span {
        start_line: node.start_position().row + 1,
        start_col: node.start_position().column,
        end_line: node.end_position().row + 1,
        end_col: node.end_position().column,
    }
}

// ============================================================================
// TypeScript Parser
// ============================================================================

/// Parse TypeScript source code to AST
pub fn parse_typescript(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_typescript::language_typescript())
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
                return_type = Some(child.utf8_text(source.as_bytes()).unwrap_or("").trim_start_matches(": ").to_string());
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
                        name = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "type_annotation" => {
                        typ = param_child.utf8_text(source.as_bytes()).unwrap_or("").trim_start_matches(": ").to_string();
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
    let last_expr_idx = children.iter().rposition(|c| c.kind() != "{" && c.kind() != "}");

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
                        name = decl_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    _ if value.is_none() && decl_child.kind() != "=" && decl_child.kind() != "type_annotation" => {
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
            let value = text.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string();
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
            AstNode::Unknown { kind: "empty_parens".into(), span: node_span(node) }
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
                            if arg_child.kind() != "(" && arg_child.kind() != ")" && arg_child.kind() != "," {
                                args.push(parse_ts_expr(arg_child, source));
                            }
                        }
                    }
                    _ => {}
                }
            }
            AstNode::Call { function, args, span: node_span(node) }
        }
        "member_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let object = Box::new(parse_ts_expr(children[0], source));
                let field = children.last().and_then(|n| n.utf8_text(source.as_bytes()).ok()).unwrap_or("").to_string();
                AstNode::Field { object, field, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "member_expression".into(), span: node_span(node) }
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
                                    counter = id.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                                }
                            }
                        }
                    }
                    "statement_block" => {
                        body = Some(Box::new(parse_ts_block(child, source)));
                    }
                    _ if collection.is_none() && !counter.is_empty() && child.kind() != "for" && child.kind() != "(" && child.kind() != ")" && child.kind() != "of" && child.kind() != "in" => {
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
                    body: body.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown { kind: "for_statement".into(), span: node_span(node) }
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
                condition: condition.unwrap_or(Box::new(AstNode::Literal { value: LiteralValue::Bool(true), span: node_span(node) })),
                body: body.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
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
                                    catch_var = Some(catch_child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                                }
                                "statement_block" => {
                                    catch_block = Some(Box::new(parse_ts_block(catch_child, source)));
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
                try_block: try_block.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
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
                AstNode::Assign { target, value, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "assignment_incomplete".into(), span: node_span(node) }
            }
        }
        "await_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "await" {
                    return AstNode::Await { expr: Box::new(parse_ts_expr(child, source)), span: node_span(node) };
                }
            }
            AstNode::Unknown { kind: "await_empty".into(), span: node_span(node) }
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
                                params.push(param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
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
                body: body.unwrap_or(Box::new(AstNode::Literal { value: LiteralValue::Unit, span: node_span(node) })),
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
        return AstNode::Unknown { kind: "binary_incomplete".into(), span: node_span(node) };
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
        _ => return AstNode::Unknown { kind: format!("unknown_op:{}", op_text), span: node_span(node) },
    };

    AstNode::Binary { op, left, right, span: node_span(node) }
}

fn parse_ts_unary(node: Node, source: &str) -> AstNode {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if children.len() < 2 {
        return AstNode::Unknown { kind: "unary_incomplete".into(), span: node_span(node) };
    }

    let op_text = children[0].utf8_text(source.as_bytes()).unwrap_or("");
    let operand = Box::new(parse_ts_expr(children[1], source));

    let op = match op_text {
        "-" => UnaryOp::Neg,
        "!" => UnaryOp::Not,
        _ => return AstNode::Unknown { kind: format!("unknown_unary:{}", op_text), span: node_span(node) },
    };

    AstNode::Unary { op, operand, span: node_span(node) }
}

// ============================================================================
// Python Parser
// ============================================================================

/// Parse Python source code to AST
pub fn parse_python(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_python::language())
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
                            name = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                        }
                        "type" => {
                            typ = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
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
    let last_stmt_idx = children.iter().rposition(|c| c.kind() != ":" && !c.kind().is_empty());

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
                    _ => return AstNode::Unknown { kind: format!("unknown_op:{}", op_text), span: node_span(node) },
                };
                AstNode::Binary { op, left, right, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "binary_incomplete".into(), span: node_span(node) }
            }
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_py_expr(child, source);
                }
            }
            AstNode::Unknown { kind: "empty_parens".into(), span: node_span(node) }
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
                            if arg_child.kind() != "(" && arg_child.kind() != ")" && arg_child.kind() != "," {
                                args.push(parse_py_expr(arg_child, source));
                            }
                        }
                    }
                    _ => {}
                }
            }
            AstNode::Call { function, args, span: node_span(node) }
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
                    _ if collection.is_none() && !item.is_empty() && child.kind() != "for" && child.kind() != "in" && child.kind() != ":" => {
                        collection = Some(Box::new(parse_py_expr(child, source)));
                    }
                    _ => {}
                }
            }
            AstNode::ForEach {
                item,
                index: None,
                collection: collection.unwrap_or(Box::new(AstNode::Unknown { kind: "no_collection".into(), span: node_span(node) })),
                body: body.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
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
                condition: condition.unwrap_or(Box::new(AstNode::Literal { value: LiteralValue::Bool(true), span: node_span(node) })),
                body: body.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
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
                                    catch_var = Some(except_child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                                }
                                "block" => {
                                    catch_block = Some(Box::new(parse_py_block(except_child, source)));
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
                try_block: try_block.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
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
                AstNode::Assign { target, value, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "assignment_incomplete".into(), span: node_span(node) }
            }
        }
        "await" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "await" {
                    return AstNode::Await { expr: Box::new(parse_py_expr(child, source)), span: node_span(node) };
                }
            }
            AstNode::Unknown { kind: "await_empty".into(), span: node_span(node) }
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
                                params.push(param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
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
                body: body.unwrap_or(Box::new(AstNode::Literal { value: LiteralValue::Unit, span: node_span(node) })),
                span: node_span(node),
            }
        }
        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}

// ============================================================================
// Go Parser
// ============================================================================

/// Parse Go source code to AST
pub fn parse_go(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_go::language())
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
                        name = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "type_identifier" | "qualified_type" | "pointer_type" | "slice_type" => {
                        typ = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
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
    let last_stmt_idx = children.iter().rposition(|c| c.kind() != "{" && c.kind() != "}");

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
                    _ => return AstNode::Unknown { kind: format!("unknown_op:{}", op_text), span: node_span(node) },
                };
                AstNode::Binary { op, left, right, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "binary_incomplete".into(), span: node_span(node) }
            }
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_go_expr(child, source);
                }
            }
            AstNode::Unknown { kind: "empty_parens".into(), span: node_span(node) }
        }
        "selector_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_go_expr(children[0], source));
                let field = children.last().and_then(|n| n.utf8_text(source.as_bytes()).ok()).unwrap_or("").to_string();
                AstNode::Field { object, field, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "selector_expression".into(), span: node_span(node) }
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
                                            counter = id.utf8_text(source.as_bytes()).unwrap_or("").to_string();
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
                                    counter = id.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                                }
                            } else if range_child.kind() != "range" && range_child.kind() != ":=" && range_child.kind() != "," {
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
                    body: body.unwrap_or(Box::new(AstNode::Block { statements: vec![], result: None, span: node_span(node) })),
                    span: node_span(node),
                }
            } else {
                AstNode::Unknown { kind: "for_statement".into(), span: node_span(node) }
            }
        }
        "assignment_statement" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let target = Box::new(parse_go_expr(children[0], source));
                let value = Box::new(parse_go_expr(children[children.len() - 1], source));
                AstNode::Assign { target, value, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "assignment_incomplete".into(), span: node_span(node) }
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
                                        params.push(decl_child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
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
                body: body.unwrap_or(Box::new(AstNode::Literal { value: LiteralValue::Unit, span: node_span(node) })),
                span: node_span(node),
            }
        }
        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}

// ============================================================================
// C# Parser
// ============================================================================

/// Parse C# source code to AST
pub fn parse_csharp(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_c_sharp::language())
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    let root = tree.root_node();
    let mut functions = Vec::new();

    // Recursively find method declarations
    fn find_methods(node: Node, source: &str, functions: &mut Vec<Function>) {
        if node.kind() == "method_declaration" {
            if let Some(func) = parse_cs_method(node, source) {
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
        language: Language::CSharp,
        functions,
        source_hash,
    })
}

fn parse_cs_method(node: Node, source: &str) -> Option<Function> {
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
                params = parse_cs_parameters(child, source);
            }
            "predefined_type" | "identifier" | "generic_name" if return_type.is_none() && !name.is_empty() => {
                // Already have name, this might be return type (handled separately)
            }
            "block" => {
                body = Some(parse_cs_block(child, source));
            }
            _ => {}
        }
    }

    // Get return type from first type-like child
    let mut cursor2 = node.walk();
    for child in node.children(&mut cursor2) {
        match child.kind() {
            "predefined_type" | "nullable_type" | "generic_name" => {
                return_type = Some(child.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                break;
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

fn parse_cs_parameters(node: Node, source: &str) -> Vec<Parameter> {
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
                        name = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "predefined_type" | "nullable_type" | "generic_name" | "identifier" if typ.is_empty() => {
                        typ = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
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

fn parse_cs_block(node: Node, source: &str) -> AstNode {
    let mut statements = Vec::new();
    let mut result = None;
    let mut cursor = node.walk();

    let children: Vec<_> = node.children(&mut cursor).collect();
    let last_stmt_idx = children.iter().rposition(|c| c.kind() != "{" && c.kind() != "}");

    for (i, child) in children.iter().enumerate() {
        let is_last = Some(i) == last_stmt_idx;

        match child.kind() {
            "{" | "}" => continue,
            "return_statement" => {
                let mut ret_cursor = child.walk();
                for ret_child in child.children(&mut ret_cursor) {
                    if ret_child.kind() != "return" && ret_child.kind() != ";" {
                        let expr = parse_cs_expr(ret_child, source);
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
                let expr = parse_cs_if(*child, source);
                if is_last {
                    result = Some(Box::new(expr));
                } else {
                    statements.push(expr);
                }
            }
            "local_declaration_statement" => {
                statements.push(parse_cs_let(*child, source));
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

fn parse_cs_let(node: Node, source: &str) -> AstNode {
    let mut name = String::new();
    let mut value = None;

    fn find_declarator(node: Node, source: &str, name: &mut String, value: &mut Option<Box<AstNode>>) {
        if node.kind() == "variable_declarator" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        *name = child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "equals_value_clause" => {
                        let mut eq_cursor = child.walk();
                        for eq_child in child.children(&mut eq_cursor) {
                            if eq_child.kind() != "=" {
                                *value = Some(Box::new(parse_cs_expr(eq_child, source)));
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            find_declarator(child, source, name, value);
        }
    }

    find_declarator(node, source, &mut name, &mut value);

    AstNode::Let {
        name,
        value: value.unwrap_or(Box::new(AstNode::Literal {
            value: LiteralValue::Unit,
            span: node_span(node),
        })),
        span: node_span(node),
    }
}

fn parse_cs_if(node: Node, source: &str) -> AstNode {
    let mut condition = None;
    let mut then_branch = None;
    let mut else_branch = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if" | "(" | ")" => {}
            "block" if then_branch.is_none() => {
                then_branch = Some(Box::new(parse_cs_block(child, source)));
            }
            "else_clause" => {
                let mut else_cursor = child.walk();
                for else_child in child.children(&mut else_cursor) {
                    match else_child.kind() {
                        "else" => {}
                        "block" => {
                            else_branch = Some(Box::new(parse_cs_block(else_child, source)));
                        }
                        "if_statement" => {
                            else_branch = Some(Box::new(parse_cs_if(else_child, source)));
                        }
                        _ => {}
                    }
                }
            }
            _ if condition.is_none() => {
                condition = Some(Box::new(parse_cs_expr(child, source)));
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

fn parse_cs_expr(node: Node, source: &str) -> AstNode {
    match node.kind() {
        "integer_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0");
            let value = text.trim_end_matches(|c: char| c.is_alphabetic()).parse().unwrap_or(0);
            AstNode::Literal {
                value: LiteralValue::Int(value),
                span: node_span(node),
            }
        }
        "real_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0.0");
            let value = text.trim_end_matches(|c: char| c.is_alphabetic()).parse().unwrap_or(0.0);
            AstNode::Literal {
                value: LiteralValue::Float(value),
                span: node_span(node),
            }
        }
        "string_literal" | "verbatim_string_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("\"\"");
            let value = text.trim_matches('"').trim_start_matches('@').to_string();
            AstNode::Literal {
                value: LiteralValue::String(value),
                span: node_span(node),
            }
        }
        "boolean_literal" => {
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
        "binary_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 3 {
                let left = Box::new(parse_cs_expr(children[0], source));
                let op_text = children[1].utf8_text(source.as_bytes()).unwrap_or("");
                let right = Box::new(parse_cs_expr(children[2], source));
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
                    _ => return AstNode::Unknown { kind: format!("unknown_op:{}", op_text), span: node_span(node) },
                };
                AstNode::Binary { op, left, right, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "binary_incomplete".into(), span: node_span(node) }
            }
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_cs_expr(child, source);
                }
            }
            AstNode::Unknown { kind: "empty_parens".into(), span: node_span(node) }
        }
        "member_access_expression" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_cs_expr(children[0], source));
                let field = children.last().and_then(|n| n.utf8_text(source.as_bytes()).ok()).unwrap_or("").to_string();
                AstNode::Field { object, field, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "member_access".into(), span: node_span(node) }
            }
        }
        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}

// ============================================================================
// Java Parser
// ============================================================================

/// Parse Java source code to AST
pub fn parse_java(source: &str) -> Result<CodeAst> {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_java::language())
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
            "void_type" | "integral_type" | "floating_point_type" | "boolean_type" | "type_identifier" => {
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
                        name = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    "integral_type" | "floating_point_type" | "boolean_type" | "type_identifier" | "generic_type" => {
                        typ = param_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
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
    let last_stmt_idx = children.iter().rposition(|c| c.kind() != "{" && c.kind() != "}");

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
                        name = decl_child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    }
                    _ if value.is_none() && decl_child.kind() != "=" && decl_child.kind() != "dimensions" => {
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
            let value = text.trim_end_matches(|c: char| c.is_alphabetic()).parse().unwrap_or(0);
            AstNode::Literal {
                value: LiteralValue::Int(value),
                span: node_span(node),
            }
        }
        "decimal_floating_point_literal" | "hex_floating_point_literal" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("0.0");
            let value = text.trim_end_matches(|c: char| c.is_alphabetic()).parse().unwrap_or(0.0);
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
                    _ => return AstNode::Unknown { kind: format!("unknown_op:{}", op_text), span: node_span(node) },
                };
                AstNode::Binary { op, left, right, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "binary_incomplete".into(), span: node_span(node) }
            }
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" {
                    return parse_java_expr(child, source);
                }
            }
            AstNode::Unknown { kind: "empty_parens".into(), span: node_span(node) }
        }
        "field_access" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            if children.len() >= 2 {
                let object = Box::new(parse_java_expr(children[0], source));
                let field = children.last().and_then(|n| n.utf8_text(source.as_bytes()).ok()).unwrap_or("").to_string();
                AstNode::Field { object, field, span: node_span(node) }
            } else {
                AstNode::Unknown { kind: "field_access".into(), span: node_span(node) }
            }
        }
        _ => AstNode::Unknown {
            kind: node.kind().to_string(),
            span: node_span(node),
        },
    }
}

// ============================================================================
// Auto-detection
// ============================================================================

/// Auto-detect language and parse
pub fn parse_auto(source: &str) -> Result<CodeAst> {
    // Simple heuristics based on syntax
    if source.contains("fn ") && (source.contains("->") || source.contains("let ")) {
        parse_rust(source)
    } else if source.contains("def ") && source.contains(":") {
        parse_python(source)
    } else if source.contains("func ") && source.contains("package ") {
        parse_go(source)
    } else if source.contains("function ") || source.contains("export ") {
        parse_typescript(source)
    } else if source.contains("public class") && source.contains("void") {
        parse_java(source)
    } else if source.contains("public static") && source.contains("namespace") {
        parse_csharp(source)
    } else {
        // Default to Rust
        parse_rust(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let ast = parse_rust(code).unwrap();
        assert_eq!(ast.functions.len(), 1);
        assert_eq!(ast.functions[0].name, "add");
        assert_eq!(ast.functions[0].params.len(), 2);
    }

    #[test]
    fn test_parse_match() {
        let code = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        false => 0,
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        assert_eq!(ast.functions.len(), 1);

        // Check that body contains a match
        if let AstNode::Block { result, .. } = &ast.functions[0].body {
            assert!(matches!(result.as_deref(), Some(AstNode::Match { .. })));
        }
    }

    #[test]
    fn test_parse_tuple_match() {
        let code = r#"
fn check(a: bool, b: bool) -> i32 {
    match (a, b) {
        (true, true) => 1,
        (true, false) => 2,
        (false, _) => 3,
    }
}
"#;
        let ast = parse_rust(code).unwrap();

        if let AstNode::Block { result, .. } = &ast.functions[0].body {
            if let Some(AstNode::Match { arms, .. }) = result.as_deref() {
                assert_eq!(arms.len(), 3);

                // First arm should be (true, true)
                if let Pattern::Tuple(elements) = &arms[0].pattern {
                    assert_eq!(elements.len(), 2);
                }
            }
        }
    }

    #[test]
    fn test_parse_if_else() {
        let code = r#"
fn check(x: i32) -> bool {
    if x > 0 {
        true
    } else {
        false
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        assert_eq!(ast.functions.len(), 1);

        if let AstNode::Block { result, .. } = &ast.functions[0].body {
            assert!(matches!(result.as_deref(), Some(AstNode::If { .. })));
        }
    }
}
