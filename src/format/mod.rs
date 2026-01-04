//! Code Formatting Module
//!
//! This module provides code formatting capabilities for generated code.
//! Supports:
//! - Rust (via prettyplease - built-in)
//! - TypeScript (via prettier - external tool)
//! - Python (via black or ruff - external tool)
//! - Go (via gofmt - external tool)
//! - Java, C# (passthrough - no formatter yet)

use crate::cel::Target;
use std::io::Write;
use std::process::{Command, Stdio};

/// Format generated code according to target language conventions
pub fn format_code(code: &str, target: Target) -> Result<String, FormatError> {
    match target {
        Target::Rust => format_rust(code),
        Target::TypeScript => format_typescript(code),
        Target::Python => format_python(code),
        Target::Go => format_go(code),
        Target::Java => Ok(code.to_string()), // TODO: Add Java formatter (google-java-format)
        Target::CSharp => Ok(code.to_string()), // TODO: Add C# formatter (dotnet format)
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

/// Format Go code using gofmt
/// Falls back to original code if gofmt is not available
pub fn format_go(code: &str) -> Result<String, FormatError> {
    match run_external_formatter(code, "gofmt", &[], "Go") {
        Ok(formatted) => Ok(formatted),
        Err(FormatError::FormatterNotFound { .. }) => Ok(code.to_string()),
        Err(e) => Err(e),
    }
}

/// Format TypeScript code using prettier
/// Tries: prettier, npx prettier
/// Falls back to original code if prettier is not available
pub fn format_typescript(code: &str) -> Result<String, FormatError> {
    // Try direct prettier first
    if let Ok(formatted) = run_external_formatter(
        code,
        "prettier",
        &["--parser", "typescript", "--stdin-filepath", "input.ts"],
        "TypeScript",
    ) {
        return Ok(formatted);
    }

    // Try npx prettier as fallback
    if let Ok(formatted) = run_external_formatter(
        code,
        "npx",
        &[
            "prettier",
            "--parser",
            "typescript",
            "--stdin-filepath",
            "input.ts",
        ],
        "TypeScript",
    ) {
        return Ok(formatted);
    }

    // No prettier available, return as-is
    Ok(code.to_string())
}

/// Format Python code using black or ruff
/// Tries: black, ruff format
/// Falls back to original code if no formatter is available
pub fn format_python(code: &str) -> Result<String, FormatError> {
    // Try black first (most common)
    if let Ok(formatted) = run_external_formatter(code, "black", &["-", "-q"], "Python") {
        return Ok(formatted);
    }

    // Try ruff format as fallback (faster, gaining popularity)
    if let Ok(formatted) = run_external_formatter(code, "ruff", &["format", "-"], "Python") {
        return Ok(formatted);
    }

    // No formatter available, return as-is
    Ok(code.to_string())
}

/// Run an external formatter that reads from stdin and writes to stdout
fn run_external_formatter(
    code: &str,
    command: &str,
    args: &[&str],
    language: &str,
) -> Result<String, FormatError> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| FormatError::FormatterNotFound {
            formatter: command.to_string(),
            message: e.to_string(),
        })?;

    // Write code to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(code.as_bytes())
            .map_err(|e| FormatError::ParseError {
                language: language.to_string(),
                message: format!("Failed to write to {}: {}", command, e),
            })?;
    }

    // Wait for output
    let output = child
        .wait_with_output()
        .map_err(|e| FormatError::ParseError {
            language: language.to_string(),
            message: format!("{} failed: {}", command, e),
        })?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| FormatError::ParseError {
            language: language.to_string(),
            message: format!("Invalid UTF-8 from {}: {}", command, e),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(FormatError::ParseError {
            language: language.to_string(),
            message: format!("{} error: {}", command, stderr),
        })
    }
}

/// Formatting errors
#[derive(Debug, Clone)]
pub enum FormatError {
    /// Failed to parse the code
    ParseError { language: String, message: String },
    /// Formatter not available for this language
    UnsupportedLanguage(String),
    /// External formatter not found
    FormatterNotFound { formatter: String, message: String },
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
            FormatError::FormatterNotFound { formatter, message } => {
                write!(f, "Formatter '{}' not found: {}", formatter, message)
            }
        }
    }
}

impl std::error::Error for FormatError {}

/// Check if a formatter is available on the system
pub fn is_formatter_available(formatter: &str) -> bool {
    Command::new(formatter)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Get available formatters for each language
pub fn available_formatters() -> Vec<(&'static str, &'static str)> {
    let mut available = vec![("Rust", "prettyplease (built-in)")];

    if is_formatter_available("prettier") {
        available.push(("TypeScript", "prettier"));
    } else if is_formatter_available("npx") {
        available.push(("TypeScript", "npx prettier"));
    }

    if is_formatter_available("black") {
        available.push(("Python", "black"));
    } else if is_formatter_available("ruff") {
        available.push(("Python", "ruff"));
    }

    if is_formatter_available("gofmt") {
        available.push(("Go", "gofmt"));
    }

    available
}

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
        // TypeScript returns code (formatted if prettier available, otherwise as-is)
        let code = "function foo(): void { }";
        let result = format_code(code, Target::TypeScript).unwrap();
        // Either formatted or original - both are valid
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_code_python() {
        // Python returns code (formatted if black/ruff available, otherwise as-is)
        let code = "def foo():\n    pass";
        let result = format_code(code, Target::Python).unwrap();
        // Either formatted or original - both are valid
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_code_go() {
        // Go returns code (formatted if gofmt available, otherwise as-is)
        let code = "package main\n\nfunc foo() {}";
        let result = format_code(code, Target::Go).unwrap();
        // Either formatted or original - both are valid
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_code_java_passthrough() {
        // Java just returns as-is (no formatter yet)
        let code = "public class Foo { }";
        let result = format_code(code, Target::Java).unwrap();
        assert_eq!(result, code);
    }

    #[test]
    fn test_format_code_csharp_passthrough() {
        // C# just returns as-is (no formatter yet)
        let code = "public class Foo { }";
        let result = format_code(code, Target::CSharp).unwrap();
        assert_eq!(result, code);
    }
}
