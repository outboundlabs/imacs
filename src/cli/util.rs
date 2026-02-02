//! CLI utility helpers

use imacs::{Error, Result, Target};
use std::fs;
use std::path::PathBuf;

/// Parse --lang argument to determine target language
pub fn parse_target_arg(args: &[String]) -> Target {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--lang" || arg == "-l" {
            if let Some(lang) = args.get(i + 1) {
                return match lang.to_lowercase().as_str() {
                    "rust" | "rs" => Target::Rust,
                    "typescript" | "ts" => Target::TypeScript,
                    "python" | "py" => Target::Python,
                    "csharp" | "cs" | "c#" => Target::CSharp,
                    "java" => Target::Java,
                    "go" | "golang" => Target::Go,
                    _ => Target::Rust,
                };
            }
        }
    }
    Target::Rust
}

/// Parse --output argument to determine output file path
pub fn parse_output_arg(args: &[String]) -> Option<PathBuf> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--output" || arg == "-o" {
            if let Some(path) = args.get(i + 1) {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

/// Write content to file or stdout
pub fn write_output(path: &Option<PathBuf>, content: &str) -> Result<()> {
    match path {
        Some(p) => {
            fs::write(p, content).map_err(Error::Io)?;
            eprintln!("Written to: {}", p.display());
        }
        None => {
            println!("{}", content);
        }
    }
    Ok(())
}
