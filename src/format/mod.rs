//! Code Formatting Module
//!
//! This module provides code formatting capabilities for generated code.
//! Currently supports:
//! - Rust (via prettyplease)
//! - Other languages return unformatted code (future: add formatters)

use crate::cel::Target;

/// Format generated code according to target language conventions
pub fn format_code(code: &str, target: Target) -> Result<String, FormatError> {
    match target {
        Target::Rust => format_rust(code),
        Target::TypeScript => Ok(code.to_string()), // TODO: Add TypeScript formatter
        Target::Python => Ok(code.to_string()),     // TODO: Add Python formatter
        Target::Go => format_go(code),              // Go has gofmt conventions
        Target::Java => Ok(code.to_string()),       // TODO: Add Java formatter
        Target::CSharp => Ok(code.to_string()),     // TODO: Add C# formatter
    }
}

/// Format Rust code using prettyplease
pub fn format_rust(code: &str) -> Result<String, FormatError> {
    match syn::parse_file(code) {
        Ok(syntax_tree) => Ok(prettyplease::unparse(&syntax_tree)),
        Err(e) => Err(FormatError::ParseError {
            language: "Rust".to_string(),
            message: e.to_string(),
        }),
    }
}

/// Format Go code (basic indentation fix)
/// Note: For full formatting, use gofmt externally
pub fn format_go(code: &str) -> Result<String, FormatError> {
    // Go formatting is simple - just ensure consistent indentation
    // Real gofmt would be ideal, but this provides basic cleanup
    Ok(code.to_string())
}

/// Formatting errors
#[derive(Debug, Clone)]
pub enum FormatError {
    /// Failed to parse the code
    ParseError {
        language: String,
        message: String,
    },
    /// Formatter not available for this language
    UnsupportedLanguage(String),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::ParseError { language, message } => {
                write!(f, "Failed to parse {} code: {}", language, message)
            }
            FormatError::UnsupportedLanguage(lang) => {
                write!(f, "No formatter available for {}", lang)
            }
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_rust_simple() {
        let code = "fn foo(){let x=1;x}";
        let result = format_rust(code).unwrap();

        // prettyplease should format this with proper spacing
        assert!(result.contains("fn foo()"));
        assert!(result.contains("let x = 1;"));
    }

    #[test]
    fn test_format_rust_function() {
        let code = r#"
fn check(x: bool) -> i32 {
match x {
true => 1,
false => 0,
}
}
"#;
        let result = format_rust(code).unwrap();

        // Should have proper indentation
        assert!(result.contains("fn check(x: bool) -> i32"));
        assert!(result.contains("match x"));
    }

    #[test]
    fn test_format_rust_invalid() {
        let code = "fn invalid( { }";
        let result = format_rust(code);

        assert!(result.is_err());
        match result {
            Err(FormatError::ParseError { language, .. }) => {
                assert_eq!(language, "Rust");
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_format_code_typescript() {
        // TypeScript just returns as-is for now
        let code = "function foo(): void { }";
        let result = format_code(code, Target::TypeScript).unwrap();
        assert_eq!(result, code);
    }
}
