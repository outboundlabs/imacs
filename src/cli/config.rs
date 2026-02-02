//! Config and schema CLI commands

use imacs::*;

pub fn cmd_config(args: &[String]) -> Result<()> {
    use config_validate::{validate_project, Severity};

    if args.is_empty() {
        return Err("Usage: imacs config <check|schema> [options]".into());
    }

    match args[0].as_str() {
        "check" => {
            let json_output = args.contains(&"--json".to_string());

            // Find project root
            let current_dir = std::env::current_dir().map_err(Error::Io)?;
            let result = validate_project(&current_dir);

            if json_output {
                // JSON output
                let issues_json: Vec<_> = result.issues.iter().map(|i| {
                    serde_json::json!({
                        "severity": match i.severity { Severity::Error => "error", Severity::Warning => "warning" },
                        "code": i.code,
                        "message": i.message,
                        "file": i.file,
                    })
                }).collect();

                let output = serde_json::json!({
                    "valid": !result.has_errors(),
                    "errors": result.error_count(),
                    "warnings": result.warning_count(),
                    "issues": issues_json,
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                // Human-readable output
                if result.issues.is_empty() && result.root_valid {
                    println!("✓ Configuration is valid");
                } else {
                    for issue in &result.issues {
                        let prefix = match issue.severity {
                            Severity::Error => "✗",
                            Severity::Warning => "⚠",
                        };
                        let level = match issue.severity {
                            Severity::Error => "ERROR",
                            Severity::Warning => "WARN",
                        };
                        println!("{} [{}] {}: {}", prefix, issue.code, level, issue.message);
                        println!("  File: {}", issue.file);
                    }

                    println!();
                    if result.has_errors() {
                        println!(
                            "✗ {} error(s), {} warning(s)",
                            result.error_count(),
                            result.warning_count()
                        );
                    } else {
                        println!("✓ {} warning(s) (no errors)", result.warning_count());
                    }
                }
            }

            if result.has_errors() {
                return Err("Configuration validation failed".into());
            }
            Ok(())
        }
        "schema" => {
            let schema_name = args.get(1).map(|s| s.as_str()).unwrap_or("imacs_root");

            match schema_name {
                "imacs_root" | "root" => {
                    let schema = schemars::schema_for!(config::ImacRoot);
                    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
                }
                "local" | "config" => {
                    let schema = schemars::schema_for!(config::LocalConfig);
                    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
                }
                _ => {
                    return Err(format!(
                        "Unknown schema: {}. Use 'imacs_root' or 'local'.",
                        schema_name
                    )
                    .into());
                }
            }
            Ok(())
        }
        cmd => Err(format!(
            "Unknown config subcommand: {}. Use 'check' or 'schema'.",
            cmd
        )
        .into()),
    }
}

pub fn cmd_schema(args: &[String]) -> Result<()> {
    let schema_name = args.first().map(|s| s.as_str()).unwrap_or("list");

    match schema_name {
        "list" => {
            println!(
                "Available schemas: spec, verify, analyze, extract, drift, completeness, validate"
            );
            Ok(())
        }
        "spec" => print_schema::<Spec>(),
        "verify" => print_schema::<VerificationResult>(),
        "analyze" => print_schema::<AnalysisReport>(),
        "extract" => print_schema::<ExtractedSpec>(),
        "drift" => print_schema::<DriftReport>(),
        "completeness" => print_schema::<IncompletenessReport>(),
        "validate" => print_schema::<imacs::completeness::ValidationReport>(),
        _ => Err(format!("Unknown schema: {}", schema_name).into()),
    }
}

fn print_schema<T: schemars::JsonSchema>() -> Result<()> {
    let schema = schemars::schema_for!(T);
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
