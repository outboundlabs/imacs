//! Validation CLI command

use imacs::*;
use std::fs;

pub fn cmd_validate(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(
            "Usage: imacs validate <spec.yaml> [--strict] [--json] [--fix] [--dry-run] [--all]"
                .into(),
        );
    }

    let spec_path = &args[0];
    let strict = args.contains(&"--strict".to_string());
    let json_output = args.contains(&"--json".to_string());
    let apply_fixes = args.contains(&"--fix".to_string());
    let dry_run = args.contains(&"--dry-run".to_string());
    let apply_all = args.contains(&"--all".to_string());

    let spec_content = fs::read_to_string(spec_path).map_err(Error::Io)?;
    let mut spec = Spec::from_yaml(&spec_content)?;
    let report = imacs::completeness::validate_spec(&spec, strict);

    // Apply fixes if requested
    if apply_fixes && !report.fixes.is_empty() {
        use imacs::completeness::{apply_fixes, apply_fixes_to_yaml};

        if dry_run {
            // Show what would change without modifying
            let (new_yaml, result) = apply_fixes_to_yaml(&spec_content, &report.fixes, apply_all)?;

            println!("Would apply {} fix(es):", result.applied.len());
            for code in &result.applied {
                if let Some(fix) = report.fixes.iter().find(|f| f.issue_code == *code) {
                    println!("  [{}] {}", fix.issue_code, fix.description);
                }
            }

            if !result.skipped.is_empty() {
                println!(
                    "\nWould skip {} fix(es) (low confidence):",
                    result.skipped.len()
                );
                for code in &result.skipped {
                    println!("  {}", code);
                }
            }

            if !result.errors.is_empty() {
                println!("\nErrors:");
                for err in &result.errors {
                    println!("  {}", err);
                }
            }

            println!("\n--- Proposed changes ---");
            println!("{}", new_yaml);
        } else {
            // Actually apply fixes
            let result = apply_fixes(&mut spec, &report.fixes, apply_all);

            println!("Applied {} fix(es):", result.applied.len());
            for code in &result.applied {
                if let Some(fix) = report.fixes.iter().find(|f| f.issue_code == *code) {
                    println!("  [{}] {}", fix.issue_code, fix.description);
                }
            }

            if !result.skipped.is_empty() {
                println!(
                    "\nSkipped {} fix(es) (low confidence):",
                    result.skipped.len()
                );
                for code in &result.skipped {
                    println!("  {}", code);
                }
            }

            if !result.errors.is_empty() {
                eprintln!("\nErrors applying fixes:");
                for err in &result.errors {
                    eprintln!("  {}", err);
                }
            }

            // Write back to file
            let new_yaml = spec.to_yaml()?;
            fs::write(spec_path, new_yaml).map_err(Error::Io)?;
            println!("\n✓ Updated {}", spec_path);
        }
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if !apply_fixes {
        // Only print report if not applying fixes (fixes already printed their own output)
        print_validation_report(&report, spec_path);
    }

    // Exit code: 0 = valid, 1 = invalid
    if report.is_valid {
        Ok(())
    } else {
        Err("Validation failed".into())
    }
}

fn print_validation_report(report: &imacs::completeness::ValidationReport, spec_path: &str) {
    if report.is_valid {
        println!("✓ {}: valid (no issues found)", spec_path);
        return;
    }

    println!(
        "✗ {}: {} error(s), {} warning(s)\n",
        spec_path, report.error_count, report.warning_count
    );

    // Group by severity
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, imacs::completeness::Severity::Error))
        .collect();
    let warnings: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, imacs::completeness::Severity::Warning))
        .collect();

    if !errors.is_empty() {
        println!("ERRORS:");
        for issue in errors {
            println!("  [{}] {}", issue.code, issue.message);
            if !issue.affected_rules.is_empty() {
                println!(
                    "         Affected rules: {}",
                    issue.affected_rules.join(", ")
                );
            }
            if let Some(suggestion) = &issue.suggestion {
                println!("         Suggestion: {}", suggestion);
            }
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("WARNINGS:");
        for issue in warnings {
            println!("  [{}] {}", issue.code, issue.message);
            if !issue.affected_rules.is_empty() {
                println!(
                    "         Affected rules: {}",
                    issue.affected_rules.join(", ")
                );
            }
            if let Some(suggestion) = &issue.suggestion {
                println!("         Suggestion: {}", suggestion);
            }
        }
        println!();
    }

    println!(
        "Summary: {} error(s), {} warning(s)",
        report.error_count, report.warning_count
    );
}
