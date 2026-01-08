//! Code parsing via tree-sitter
//!
//! Parses source code into language-agnostic AST.
//! Supports: Rust, TypeScript, Python, Go, C#, Java

mod csharp;
mod go;
mod java;
mod python;
mod rust;
mod typescript;

use crate::ast::*;
use crate::error::{Error, Result};
use tree_sitter::{Node, Parser};

// Re-export language-specific parsers
pub use csharp::parse_csharp;
pub use go::parse_go;
pub use java::parse_java;
pub use python::parse_python;
pub use rust::parse_rust;
pub use typescript::parse_typescript;

/// Context for collecting diagnostics during parsing
struct ParseContext {
    diagnostics: ParseDiagnostics,
    source: String,
}

impl ParseContext {
    fn new(source: &str) -> Self {
        Self {
            diagnostics: ParseDiagnostics::new(),
            source: source.to_string(),
        }
    }

    fn record_unknown(&mut self, kind: &str, span: Span) {
        let source_text = self.extract_source_text(span);
        self.diagnostics.unknown_nodes.push(UnknownNodeInfo {
            kind: kind.to_string(),
            span,
            source_text,
        });
    }

    #[allow(dead_code)]
    fn record_syntax_error(&mut self, message: &str, span: Span) {
        let source_text = self.extract_source_text(span);
        self.diagnostics.syntax_errors.push(SyntaxErrorInfo {
            message: message.to_string(),
            span,
            source_text,
        });
    }

    fn extract_source_text(&self, span: Span) -> String {
        let lines: Vec<&str> = self.source.lines().collect();
        if span.start_line > 0 && span.start_line <= lines.len() {
            let line = lines[span.start_line - 1];
            // Extract the relevant portion, limit to 100 chars
            let start = span.start_col.saturating_sub(1).min(line.len());
            let end = (start + 100).min(line.len());
            line[start..end].to_string()
        } else {
            String::new()
        }
    }
}

// Re-export language enum
pub use crate::ast::Language;

/// Get raw tree-sitter S-expression for source code
///
/// This is useful for debugging what tree-sitter sees vs what IMACS extracts.
/// The S-expression format is a Lisp-like representation of the AST.
pub fn to_sexp(source: &str, lang: Language) -> Result<String> {
    let mut parser = Parser::new();

    let language = match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Unknown => {
            return Err(Error::CodeParse("Cannot parse unknown language".into()));
        }
    };

    parser
        .set_language(&language)
        .map_err(|e| Error::CodeParse(format!("Failed to set language: {}", e)))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| Error::CodeParse("Failed to parse source".into()))?;

    Ok(tree.root_node().to_sexp())
}

/// Detect language from file extension
pub fn detect_language(path: &str) -> Language {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => Language::Rust,
        "ts" | "tsx" | "js" | "jsx" => Language::TypeScript,
        "py" => Language::Python,
        "go" => Language::Go,
        "cs" => Language::CSharp,
        "java" => Language::Java,
        _ => Language::Unknown,
    }
}

/// Parse Rust source code to AST with diagnostics
///
/// Returns the AST along with diagnostics about unknown nodes and syntax errors.
pub fn parse_rust_with_diagnostics(source: &str) -> Result<ParseResult> {
    let ast = parse_rust(source)?;
    let diagnostics = collect_diagnostics_from_ast(&ast, source);
    Ok(ParseResult { ast, diagnostics })
}

/// Collect diagnostics by walking the AST for unknown nodes
fn collect_diagnostics_from_ast(ast: &CodeAst, source: &str) -> ParseDiagnostics {
    let mut ctx = ParseContext::new(source);

    for func in &ast.functions {
        collect_unknowns_from_node(&func.body, &mut ctx);
    }

    ctx.diagnostics
}

/// Recursively collect unknown nodes from an AST node
fn collect_unknowns_from_node(node: &AstNode, ctx: &mut ParseContext) {
    match node {
        AstNode::Unknown { kind, span } => {
            ctx.record_unknown(kind, *span);
        }
        AstNode::Binary { left, right, .. } => {
            collect_unknowns_from_node(left, ctx);
            collect_unknowns_from_node(right, ctx);
        }
        AstNode::Unary { operand, .. } => {
            collect_unknowns_from_node(operand, ctx);
        }
        AstNode::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_unknowns_from_node(condition, ctx);
            collect_unknowns_from_node(then_branch, ctx);
            if let Some(else_b) = else_branch {
                collect_unknowns_from_node(else_b, ctx);
            }
        }
        AstNode::Match {
            scrutinee, arms, ..
        } => {
            collect_unknowns_from_node(scrutinee, ctx);
            for arm in arms {
                collect_unknowns_from_node(&arm.body, ctx);
                if let Some(guard) = &arm.guard {
                    collect_unknowns_from_node(guard, ctx);
                }
            }
        }
        AstNode::Block {
            statements, result, ..
        } => {
            for stmt in statements {
                collect_unknowns_from_node(stmt, ctx);
            }
            if let Some(res) = result {
                collect_unknowns_from_node(res, ctx);
            }
        }
        AstNode::Return { value, .. } => {
            if let Some(val) = value {
                collect_unknowns_from_node(val, ctx);
            }
        }
        AstNode::Let { value, .. } => {
            collect_unknowns_from_node(value, ctx);
        }
        AstNode::Call { args, .. } => {
            // function is a String, not a node, so just check args
            for arg in args {
                collect_unknowns_from_node(arg, ctx);
            }
        }
        AstNode::Field { object, .. } => {
            collect_unknowns_from_node(object, ctx);
        }
        AstNode::Index { object, index, .. } => {
            collect_unknowns_from_node(object, ctx);
            collect_unknowns_from_node(index, ctx);
        }
        AstNode::Tuple { elements, .. } => {
            for elem in elements {
                collect_unknowns_from_node(elem, ctx);
            }
        }
        AstNode::Array { elements, .. } => {
            for elem in elements {
                collect_unknowns_from_node(elem, ctx);
            }
        }
        AstNode::For {
            start, end, body, ..
        } => {
            collect_unknowns_from_node(start, ctx);
            collect_unknowns_from_node(end, ctx);
            collect_unknowns_from_node(body, ctx);
        }
        AstNode::ForEach {
            collection, body, ..
        } => {
            collect_unknowns_from_node(collection, ctx);
            collect_unknowns_from_node(body, ctx);
        }
        AstNode::While {
            condition, body, ..
        } => {
            collect_unknowns_from_node(condition, ctx);
            collect_unknowns_from_node(body, ctx);
        }
        AstNode::Try {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            collect_unknowns_from_node(try_block, ctx);
            if let Some(catch) = catch_block {
                collect_unknowns_from_node(catch, ctx);
            }
            if let Some(finally) = finally_block {
                collect_unknowns_from_node(finally, ctx);
            }
        }
        AstNode::Assign { target, value, .. } => {
            collect_unknowns_from_node(target, ctx);
            collect_unknowns_from_node(value, ctx);
        }
        AstNode::Await { expr, .. } => {
            collect_unknowns_from_node(expr, ctx);
        }
        AstNode::Closure { body, .. } => {
            collect_unknowns_from_node(body, ctx);
        }
        AstNode::MacroCall { .. } => {
            // Macro calls are leaf nodes (args is raw string)
        }
        AstNode::Ref { expr, .. } => {
            collect_unknowns_from_node(expr, ctx);
        }
        AstNode::Cast { expr, .. } => {
            collect_unknowns_from_node(expr, ctx);
        }
        AstNode::SyntaxError { span, .. } => {
            // Record syntax errors as unknown nodes
            ctx.record_unknown("SyntaxError", *span);
        }
        // Leaf nodes - no children to check
        AstNode::Literal { .. } | AstNode::Var { .. } => {}
    }
}

/// Convert tree-sitter Node position to Span
pub(crate) fn node_span(node: Node) -> Span {
    Span {
        start_line: node.start_position().row + 1,
        start_col: node.start_position().column,
        end_line: node.end_position().row + 1,
        end_col: node.end_position().column,
    }
}

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
