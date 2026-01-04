//! IMACS CLI - Command-line interface
//!
//! Commands:
//!   verify   - Check code against spec
//!   render   - Generate code from spec
//!   test     - Generate tests from spec
//!   analyze  - Analyze code complexity
//!   extract  - Extract spec from code
//!   drift    - Compare implementations
//!   update   - Update to latest version

mod update;

use imacs::*;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Non-blocking update check in background thread
    update::check_for_updates_background();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    let result = match args[1].as_str() {
        "verify" => cmd_verify(&args[2..]),
        "render" => cmd_render(&args[2..]),
        "test" => cmd_test(&args[2..]),
        "analyze" => cmd_analyze(&args[2..]),
        "extract" => cmd_extract(&args[2..]),
        "drift" => cmd_drift(&args[2..]),
        "completeness" => cmd_completeness(&args[2..]),
        "validate" => cmd_validate(&args[2..]),
        "schema" => cmd_schema(&args[2..]),
        "init" => cmd_init(&args[2..]),
        "regen" => cmd_regen(),
        "status" => cmd_status(&args[2..]),
        "selfcheck" => cmd_selfcheck(),
        "update" => cmd_update(),
        "version" | "--version" | "-v" => {
            println!("imacs {}", VERSION);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
            print_usage();
            Err("Unknown command".into())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    println!(
        r#"
IMACS - Intelligent Model-Assisted Code Synthesis

USAGE:
    imacs <COMMAND> [OPTIONS]

COMMANDS:
    verify <spec.yaml> <code.rs>     Check code implements spec
    render <spec.yaml> [--lang]      Generate code from spec
    test <spec.yaml> [--lang]        Generate tests from spec
    analyze <code.rs>                Analyze code complexity
    extract <code.rs>                 Extract spec from code
    drift <code_a.rs> <code_b.rs>    Compare implementations
    completeness <spec.yaml|dir>     Analyze spec(s) for missing cases
                                      Use directory for suite analysis
    validate <spec.yaml> [--strict]  Validate spec for impossible situations
    schema [name]                     Print JSON schema for output type
    init [--root]                    Initialize imacs/ folder (--root for project root)
    regen [--all] [--force]          Regenerate code from specs
    status [--json]                  Show project status and stale specs
    selfcheck                        Verify generated code matches specs
    update                           Update to latest version

OPTIONS:
    --lang <rust|typescript|python|csharp|java|go>   Target language (default: rust)
    --output <file>                   Output file (default: stdout)
    --json                            JSON output format (verify, analyze, extract, drift, completeness, validate)
    --full                            Full exhaustive analysis for completeness suite mode
    --strict                          Strict mode: treat warnings as errors (validate command)

EXAMPLES:
    imacs verify login.yaml src/login.rs
    imacs render checkout.yaml --lang typescript
    imacs test auth.yaml --lang python > test_auth.py
    imacs analyze src/complex.rs
    imacs extract src/legacy.rs > extracted.yaml
    imacs drift src/backend.rs src/frontend.ts
"#
    );
}

fn cmd_verify(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err("Usage: imacs verify <spec.yaml> <code.rs>".into());
    }

    let spec_path = &args[0];
    let code_path = &args[1];
    let json_output = args.contains(&"--json".to_string());

    let spec_content = fs::read_to_string(spec_path).map_err(Error::Io)?;
    let code_content = fs::read_to_string(code_path).map_err(Error::Io)?;

    let spec = Spec::from_yaml(&spec_content)?;
    let code = parse_rust(&code_content)?;

    let result = verify(&spec, &code);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.to_report());
    }

    if result.passed {
        Ok(())
    } else {
        Err("Verification failed".into())
    }
}

fn cmd_render(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err("Usage: imacs render <spec.yaml> [--lang rust|typescript|python]".into());
    }

    let spec_path = &args[0];
    let target = parse_target_arg(args);
    let output = parse_output_arg(args);

    let spec_content = fs::read_to_string(spec_path).map_err(Error::Io)?;

    // Check if this is an orchestrator (has 'chain:' key) or a regular spec
    let code = if spec_content.contains("\nchain:") || spec_content.contains("\nuses:") {
        // It's an orchestrator
        let orch = orchestrate::Orchestrator::from_yaml(&spec_content)?;
        let specs = std::collections::HashMap::new(); // TODO: load referenced specs
        orchestrate::render_orchestrator(&orch, &specs, target)
    } else {
        // It's a regular decision table spec
        let spec = Spec::from_yaml(&spec_content)?;
        render(&spec, target)
    };

    write_output(&output, &code)?;
    Ok(())
}

fn cmd_test(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err("Usage: imacs test <spec.yaml> [--lang rust|typescript|python]".into());
    }

    let spec_path = &args[0];
    let target = parse_target_arg(args);
    let output = parse_output_arg(args);

    let spec_content = fs::read_to_string(spec_path).map_err(Error::Io)?;
    let spec = Spec::from_yaml(&spec_content)?;

    let tests = generate_tests(&spec, target);

    write_output(&output, &tests)?;
    Ok(())
}

fn cmd_analyze(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err("Usage: imacs analyze <code.rs>".into());
    }

    let code_path = &args[0];
    let json_output = args.contains(&"--json".to_string());

    let code_content = fs::read_to_string(code_path).map_err(Error::Io)?;
    let code = parse_rust(&code_content)?;

    let report = analyze(&code);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.to_report());
    }

    Ok(())
}

fn cmd_extract(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err("Usage: imacs extract <code.rs>".into());
    }

    let code_path = &args[0];
    let json_output = args.contains(&"--json".to_string());
    let output = parse_output_arg(args);

    let code_content = fs::read_to_string(code_path).map_err(Error::Io)?;
    let code = parse_rust(&code_content)?;

    let extracted = extract(&code);

    if json_output {
        let json_str = serde_json::to_string_pretty(&extracted)?;
        write_output(&output, &json_str)?;
    } else {
        write_output(&output, &extracted.to_yaml())?;
    }
    Ok(())
}

fn cmd_drift(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err("Usage: imacs drift <code_a.rs> <code_b.rs>".into());
    }

    let path_a = &args[0];
    let path_b = &args[1];
    let json_output = args.contains(&"--json".to_string());

    let content_a = fs::read_to_string(path_a).map_err(Error::Io)?;
    let content_b = fs::read_to_string(path_b).map_err(Error::Io)?;

    let code_a = parse_rust(&content_a)?;
    let code_b = parse_rust(&content_b)?;

    let report = compare(&code_a, &code_b);

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.to_report());
    }

    match report.status {
        DriftStatus::Synced => Ok(()),
        DriftStatus::MinorDrift => Ok(()),
        _ => Err("Drift detected".into()),
    }
}

fn cmd_completeness(args: &[String]) -> Result<()> {
    // Find the first non-flag argument as the path
    let path = args
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .ok_or("Usage: imacs completeness <spec.yaml> [--json] [--full]")?;

    let json_output = args.contains(&"--json".to_string());
    let full_mode = args.contains(&"--full".to_string());

    let path_buf = PathBuf::from(path);

    // Check if it's a directory (suite mode) or file (single spec)
    if path_buf.is_dir() {
        // Suite mode: analyze all YAML files in directory
        cmd_completeness_suite(path, json_output, full_mode)
    } else {
        // Single spec mode
        let spec_content = fs::read_to_string(path).map_err(Error::Io)?;
        let spec = Spec::from_yaml(&spec_content)?;
        let report = imacs::completeness::analyze_completeness(&spec);

        if json_output {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!("{}", report.to_report());
        }

        // Exit code: 0 = complete, 1 = incomplete
        if report.is_complete {
            Ok(())
        } else {
            Err("Spec is incomplete".into())
        }
    }
}

fn cmd_completeness_suite(dir_path: &str, json_output: bool, full_mode: bool) -> Result<()> {
    // Check if directory contains orchestrators
    let dir_result = imacs::completeness::analyze_directory_with_orchestrators(dir_path, full_mode);

    match dir_result {
        Ok(dir_result) if dir_result.orchestrators_found > 0 => {
            // Orchestrator-aware analysis
            if json_output {
                println!("{}", serde_json::to_string_pretty(&dir_result)?);
            } else {
                print_orchestrator_suite_report(&dir_result, dir_path);
            }
        }
        Ok(dir_result) => {
            // Regular suite analysis (no orchestrators found)
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&dir_result.overall_suite_result)?
                );
            } else {
                print_suite_report(&dir_result.overall_suite_result, dir_path);
            }
        }
        Err(e) => {
            // Fallback to regular suite analysis
            eprintln!("Warning: {}", e);
            eprintln!("Falling back to regular suite analysis...");

            let mut specs: Vec<(String, Spec)> = Vec::new();
            let dir = fs::read_dir(dir_path).map_err(Error::Io)?;

            for entry in dir {
                let entry = entry.map_err(Error::Io)?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                    || path.extension().and_then(|s| s.to_str()) == Some("yml")
                {
                    let spec_content = fs::read_to_string(&path).map_err(Error::Io)?;
                    // Skip orchestrators
                    if spec_content.contains("\nchain:") || spec_content.contains("\nuses:") {
                        continue;
                    }
                    match Spec::from_yaml(&spec_content) {
                        Ok(spec) => {
                            let spec_id = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            specs.push((spec_id, spec));
                        }
                        Err(_) => {}
                    }
                }
            }

            if specs.is_empty() {
                return Err(format!("No YAML specs found in {}", dir_path).into());
            }

            let suite_result = imacs::completeness::analyze_suite(&specs, full_mode);

            if json_output {
                println!("{}", serde_json::to_string_pretty(&suite_result)?);
            } else {
                print_suite_report(&suite_result, dir_path);
            }
        }
    }

    Ok(())
}

fn print_orchestrator_suite_report(
    result: &imacs::completeness::DirectorySuiteResult,
    dir_path: &str,
) {
    println!("Analyzing directory: {}\n", dir_path);
    println!(
        "Found {} spec(s) and {} orchestrator(s)\n",
        result.specs_found, result.orchestrators_found
    );

    // Print orchestrator-specific results
    for orch_result in &result.orchestrator_results {
        println!("ORCHESTRATOR: {}\n", orch_result.orchestrator_id);
        println!(
            "  Referenced specs: {}",
            orch_result.referenced_spec_ids.join(", ")
        );
        println!("  Found: {}", orch_result.found_specs.join(", "));

        if !orch_result.missing_specs.is_empty() {
            println!("  ⚠ Missing: {}", orch_result.missing_specs.join(", "));
        }

        if !orch_result.mapping_issues.is_empty() {
            println!("\n  MAPPING ISSUES:");
            for issue in &orch_result.mapping_issues {
                println!(
                    "    [{}] {}:{} - {}",
                    issue.step_id,
                    issue.spec_id,
                    format!("{:?}", issue.issue_type),
                    issue.details
                );
            }
        }

        // Print suite analysis for this orchestrator's specs
        if !orch_result.found_specs.is_empty() {
            println!("\n  Suite analysis for referenced specs:");
            print_suite_report(&orch_result.suite_result, "");
        }

        println!("\n{}", "=".repeat(60));
    }

    // Print overall suite analysis
    println!("\nOVERALL SUITE ANALYSIS (all specs):\n");
    print_suite_report(&result.overall_suite_result, "");
}

fn print_suite_report(result: &imacs::completeness::SuiteAnalysisResult, _dir_path: &str) {
    println!("Analyzing {} specs...\n", result.individual_results.len());

    // Individual results
    println!("INDIVIDUAL RESULTS:");
    for spec_result in &result.individual_results {
        if spec_result.passed {
            println!("  ✓ {}: passed", spec_result.spec_id);
        } else {
            let missing = spec_result.report.missing_cases.len();
            let overlaps = spec_result.report.overlaps.len();
            println!(
                "  ✗ {}: {} missing cases, {} overlaps",
                spec_result.spec_id, missing, overlaps
            );
        }
    }

    // Cross-spec analysis
    if !result.collisions.is_empty()
        || !result.duplicates.is_empty()
        || !result.relationships.is_empty()
        || !result.suite_gaps.is_empty()
    {
        println!("\nCROSS-SPEC ANALYSIS:\n");

        // Collisions
        if !result.collisions.is_empty() {
            println!("COLLISIONS (same variable names, different meanings):");
            for (idx, collision) in result.collisions.iter().enumerate() {
                println!(
                    "  [C{:03}] Variable '{}' used in {} specs with different definitions:",
                    idx + 1,
                    collision.variable_name,
                    collision.occurrences.len()
                );
                for occ in &collision.occurrences {
                    if let Some(values) = &occ.variable.values {
                        println!("         • {}: values {:?}", occ.spec_id, values);
                    } else {
                        println!("         • {}: type {:?}", occ.spec_id, occ.variable.typ);
                    }
                }
            }
            println!();
        }

        // Duplicates
        if !result.duplicates.is_empty() {
            println!("DUPLICATES (same logic in multiple specs):");
            for (idx, dup) in result.duplicates.iter().enumerate() {
                println!("  [D{:03}] Rules cover identical input space:", idx + 1);
                println!(
                    "         • {}:{} ({})",
                    dup.rule_a.spec_id,
                    dup.rule_a.rule_id,
                    dup.rule_a.cel_condition.as_deref().unwrap_or("N/A")
                );
                println!(
                    "         • {}:{} ({})",
                    dup.rule_b.spec_id,
                    dup.rule_b.rule_id,
                    dup.rule_b.cel_condition.as_deref().unwrap_or("N/A")
                );
                println!("         Overlap: {}", dup.overlap_cel);
            }
            println!();
        }

        // Relationships
        if !result.relationships.is_empty() {
            println!("RELATIONSHIPS:");
            for (idx, rel) in result.relationships.iter().enumerate() {
                match &rel.relationship_type {
                    imacs::completeness::RelationshipType::Chain => {
                        println!(
                            "  [R{:03}] Output from {} matches input in {}",
                            idx + 1,
                            rel.spec_a,
                            rel.spec_b
                        );
                        for mapping in &rel.details.output_to_input_mapping {
                            println!("         {} → {}", mapping.output_name, mapping.input_name);
                        }
                    }
                    imacs::completeness::RelationshipType::MergeOpportunity => {
                        println!(
                            "  [R{:03}] Potential merge: {} and {} ({}% variable overlap)",
                            idx + 1,
                            rel.spec_a,
                            rel.spec_b,
                            (rel.details.overlap_ratio * 100.0) as u32
                        );
                    }
                }
            }
            println!();
        }

        // Suite gaps
        if !result.suite_gaps.is_empty() {
            println!("GAPS (missing across entire suite):");
            for (idx, gap) in result.suite_gaps.iter().enumerate() {
                println!("  [G{:03}] No spec handles: {}", idx + 1, gap.cel_condition);
            }
            println!();
        }
    }

    // Complexity report
    println!("COMPLEXITY REPORT:");
    println!(
        "  Total unique predicates across suite: {}",
        result.complexity.total_unique_predicates
    );
    if result.complexity.combined_input_space < u64::MAX {
        println!(
            "  Combined input space: 2^{} = {} combinations",
            result.complexity.total_unique_predicates, result.complexity.combined_input_space
        );
    } else {
        println!("  Combined input space: too large to compute");
    }
    match result.complexity.analysis_mode {
        imacs::completeness::AnalysisMode::Incremental => {
            println!("  ⚠ Analysis used incremental pairwise mode");
            if let Some(warning) = &result.complexity.warning {
                println!("  {}", warning);
            }
            println!("  Run with --full for exhaustive analysis (may be slow)");
        }
        imacs::completeness::AnalysisMode::Full => {
            println!("  ✓ Full exhaustive analysis completed");
        }
    }

    // Suggestions
    if !result.suggestions.is_empty() {
        println!("\nSUGGESTIONS:");
        for suggestion in &result.suggestions {
            println!("  [{}] {}", suggestion.code, suggestion.description);
            match &suggestion.fix {
                imacs::completeness::SuggestedFix::Rename { from, to, in_spec } => {
                    println!("         Rename '{}' to '{}' in {}", from, to, in_spec);
                }
                imacs::completeness::SuggestedFix::Namespace { prefix, variables } => {
                    println!(
                        "         Add prefix '{}' to variables: {:?}",
                        prefix, variables
                    );
                }
                imacs::completeness::SuggestedFix::Merge { specs, into } => {
                    println!("         Merge {:?} into {}", specs, into);
                }
                imacs::completeness::SuggestedFix::Extract { rules, into } => {
                    println!("         Extract rules {:?} into {}", rules, into);
                }
                imacs::completeness::SuggestedFix::DefineChain {
                    specs,
                    as_orchestrator,
                } => {
                    println!(
                        "         Define orchestrator '{}' chaining {:?}",
                        as_orchestrator, specs
                    );
                }
            }
        }
    }

    // Summary
    let failed_count = result
        .individual_results
        .iter()
        .filter(|r| !r.passed)
        .count();
    println!("\nSummary: {} spec(s) failed, {} collision(s), {} duplicate(s), {} relationship(s), {} gap(s)",
        failed_count,
        result.collisions.len(),
        result.duplicates.len(),
        result.relationships.len(),
        result.suite_gaps.len());
}

fn cmd_validate(args: &[String]) -> Result<()> {
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

fn cmd_schema(args: &[String]) -> Result<()> {
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

fn parse_target_arg(args: &[String]) -> Target {
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

fn parse_output_arg(args: &[String]) -> Option<PathBuf> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--output" || arg == "-o" {
            if let Some(path) = args.get(i + 1) {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn write_output(path: &Option<PathBuf>, content: &str) -> Result<()> {
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

fn cmd_init(args: &[String]) -> Result<()> {
    let is_root = args.contains(&"--root".to_string());
    let current_dir = std::env::current_dir().map_err(Error::Io)?;

    let imacs_dir = if is_root {
        current_dir.join("imacs")
    } else {
        // Find nearest imacs folder or create one
        match imacs::find_root(&current_dir)? {
            Some(root_path) => {
                println!("Found existing root at: {}", root_path.display());
                return Err("Root already exists. Use 'imacs init' without --root to create a local folder.".into());
            }
            None => {
                // Check if we're in a project with a root
                let mut check_dir = current_dir.clone();
                let mut found_root = false;
                loop {
                    let check_imacs = check_dir.join("imacs").join(".imacs_root");
                    let check_hidden = check_dir.join(".imacs").join(".imacs_root");
                    if check_imacs.exists() || check_hidden.exists() {
                        found_root = true;
                        break;
                    }
                    match check_dir.parent() {
                        Some(parent) => check_dir = parent.to_path_buf(),
                        None => break,
                    }
                }
                current_dir.join("imacs")
            }
        }
    };

    // Check if already exists
    if imacs_dir.exists() {
        return Err(format!("Directory {} already exists", imacs_dir.display()).into());
    }

    // Create directory
    fs::create_dir_all(&imacs_dir).map_err(Error::Io)?;
    println!("✓ Created directory: {}", imacs_dir.display());

    if is_root {
        // Create .imacs_root file
        let root_config = r#"# IMACS Project Root - v1
version: 1
imacs_version: ">=0.0.1"

project:
  name: my-project
  spec_id_prefix: ""

defaults:
  targets:
    - rust
  auto_format: true
  naming:
    code: "{spec_id}.{ext}"
    tests: "{spec_id}_test.{ext}"

validation:
  require_unique_ids: true
  require_descriptions: false
  max_rules_per_spec: 50
"#;
        let root_file = imacs_dir.join(".imacs_root");
        fs::write(&root_file, root_config).map_err(Error::Io)?;
        println!("✓ Created: {}", root_file.display());
    } else {
        // Create optional config.yaml
        let config = r#"# Local configuration (optional)
# Merges with root .imacs_root defaults

# targets:
#   - rust
#   - typescript

# auto_format: true
"#;
        let config_file = imacs_dir.join("config.yaml");
        fs::write(&config_file, config).map_err(Error::Io)?;
        println!("✓ Created: {}", config_file.display());
    }

    // Create sample spec
    let sample_spec = r#"id: example
name: "Example Spec"

inputs:
  - name: input
    type: bool

outputs:
  - name: result
    type: int

rules:
  - id: R1
    when: "input"
    then: 1
    description: "Input is true"
  - id: R2
    when: "!input"
    then: 0
    description: "Input is false"
"#;
    let sample_file = imacs_dir.join("example.yaml");
    fs::write(&sample_file, sample_spec).map_err(Error::Io)?;
    println!("✓ Created sample spec: {}", sample_file.display());

    println!("\n✓ Initialized IMACS folder at: {}", imacs_dir.display());
    if is_root {
        println!("  This is the project root. Child folders can inherit these settings.");
    } else {
        println!("  Add specs to this folder and run 'imacs regen' to generate code.");
    }

    Ok(())
}

fn cmd_status(args: &[String]) -> Result<()> {
    let json_output = args.contains(&"--json".to_string());
    let current_dir = std::env::current_dir().map_err(Error::Io)?;

    let structure = imacs::load_project_structure(&current_dir)?;

    if structure.root.is_none() {
        if json_output {
            println!("{{\"root\": null, \"folders\": []}}");
        } else {
            println!("No IMACS project root found.");
            println!("Run 'imacs init --root' to create a project root.");
        }
        return Ok(());
    }

    let root = structure.root.as_ref().unwrap();

    if json_output {
        // JSON output
        let output = serde_json::json!({
            "root": {
                "path": root.path.display().to_string(),
                "is_root": true
            },
            "folders": structure.folders.iter().map(|f| {
                serde_json::json!({
                    "path": f.path.display().to_string(),
                    "is_root": false
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Human-readable output
        println!("IMACS Project Status\n");
        println!("Root: {}", root.path.display());
        println!("Folders: {}", structure.folders.len() + 1);

        // Check for stale specs
        let mut total_stale = 0;
        for folder in &structure.folders {
            let generated_dir = imacs::get_generated_dir(&folder.path);
            if let Ok(stale) = imacs::find_stale_specs(&folder.path, &generated_dir) {
                total_stale += stale.len();
            }
        }

        if let Ok(stale) = imacs::find_stale_specs(&root.path, &imacs::get_generated_dir(&root.path)) {
            total_stale += stale.len();
        }

        if total_stale > 0 {
            println!("\n⚠ {} stale spec(s) need regeneration (run 'imacs regen')", total_stale);
        } else {
            println!("\n✓ All specs up to date");
        }

        // Validate unique IDs
        match imacs::validate_unique_ids(&structure) {
            Ok(errors) => {
                if !errors.is_empty() {
                    println!("\n⚠ ID Collisions:");
                    for err in errors {
                        println!("  {}", err);
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to validate IDs: {}", e);
            }
        }
    }

    Ok(())
}

fn cmd_regen() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let all_mode = args.contains(&"--all".to_string());
    let force = args.contains(&"--force".to_string());
    let current_dir = std::env::current_dir().map_err(Error::Io)?;

    if all_mode {
        // Regenerate all imacs folders in project
        let structure = imacs::load_project_structure(&current_dir)?;
        
        if structure.root.is_none() {
            return Err("No IMACS project root found. Run 'imacs init --root' first.".into());
        }

        // Validate unique IDs first (safeguard)
        match imacs::validate_unique_ids(&structure) {
            Ok(id_errors) => {
                if !id_errors.is_empty() {
                    eprintln!("⚠ ID Collisions detected:");
                    for err in &id_errors {
                        eprintln!("  {}", err);
                    }
                    if structure.root.as_ref().unwrap().config.validation.require_unique_ids {
                        return Err("ID collisions found. Fix before regenerating.".into());
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to validate IDs: {}", e);
            }
        }

        let mut total_regenerated = 0;

        // Process root folder
        if let Some(root) = &structure.root {
            total_regenerated += regenerate_folder(root, force)?;
        }

        // Process all child folders
        for folder in &structure.folders {
            total_regenerated += regenerate_folder(folder, force)?;
        }

        println!("\n✓ Regenerated {} spec(s) across {} folder(s)", 
            total_regenerated, structure.folders.len() + 1);
    } else {
        // Regenerate current folder only
        let structure = imacs::load_project_structure(&current_dir)?;
        
        // Find nearest imacs folder
        let folder = if let Some(root) = &structure.root {
            // Check if we're in the root folder
            if current_dir == root.path || current_dir.starts_with(&root.path) {
                root.clone()
            } else {
                // Find matching child folder
                structure.folders.iter()
                    .find(|f| current_dir.starts_with(&f.path))
                    .map(|f| f.clone())
                    .unwrap_or_else(|| root.clone())
            }
        } else {
            return Err("No IMACS project found. Run 'imacs init --root' first.".into());
        };

        regenerate_folder(&folder, force)?;
    }

    Ok(())
}

fn regenerate_folder(folder: &imacs::ImacFolder, force: bool) -> Result<usize> {
    let generated_dir = imacs::get_generated_dir(&folder.path);
    
    // Find stale specs
    let specs_to_regenerate = if force {
        // Force: regenerate all specs
        let entries = fs::read_dir(&folder.path).map_err(Error::Io)?;
        let mut specs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(Error::Io)?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if (ext == "yaml" || ext == "yml") 
                        && path.file_name().and_then(|n| n.to_str()) != Some("config.yaml")
                        && path.file_name().and_then(|n| n.to_str()) != Some(".imacs_root") {
                        specs.push(path);
                    }
                }
            }
        }
        specs
    } else {
        // Check staleness
        imacs::find_stale_specs(&folder.path, &generated_dir)?
    };

    if specs_to_regenerate.is_empty() {
        println!("✓ {}: up to date", folder.path.display());
        return Ok(0);
    }

    // Load or create metadata
    let mut meta = imacs::ImacMeta::load_from_dir(&generated_dir)?
        .unwrap_or_else(imacs::create_meta);

    // Ensure generated directory exists
    fs::create_dir_all(&generated_dir).map_err(Error::Io)?;

    let mut regenerated = 0;

    for spec_path in &specs_to_regenerate {
        let spec_content = fs::read_to_string(spec_path).map_err(Error::Io)?;
        let spec = Spec::from_yaml(&spec_content)?;

        // Apply spec ID prefix if configured
        let spec_id = if !folder.config.spec_id_prefix.is_empty() {
            format!("{}{}", folder.config.spec_id_prefix, spec.id)
        } else {
            spec.id.clone()
        };

        // Generate for each target language
        for target in &folder.config.targets {
            let code = render(&spec, *target);
            let tests = generate_tests(&spec, *target);

            // Apply naming convention
            let code_filename = folder.config.apply_naming(&spec_id, target, false);
            let test_filename = folder.config.apply_naming(&spec_id, target, true);

            let code_path = generated_dir.join(&code_filename);
            let test_path = generated_dir.join(&test_filename);

            // Write code
            fs::write(&code_path, &code).map_err(Error::Io)?;
            
            // Write tests (if any)
            if !tests.trim().is_empty() {
                fs::write(&test_path, &tests).map_err(Error::Io)?;
            }

            // Auto-format if enabled (formatting can be added later)
            if folder.config.auto_format {
                // Formatting will be implemented via format module
                // For now, just write the code as-is
            }

            println!("✓ Generated: {} ({})", code_path.display(), format!("{:?}", target).to_lowercase());
        }

        // Update metadata hash
        meta.update_hash(spec_path, &folder.path)?;
        regenerated += 1;
    }

    // Save metadata
    meta.save_to_dir(&generated_dir)?;

    Ok(regenerated)
}

fn cmd_update() -> Result<()> {
    match update::run_update() {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Update failed: {}", e).into()),
    }
}

fn cmd_selfcheck() -> Result<()> {
    let specs_dir = PathBuf::from("specs");
    let generated_dir = PathBuf::from("src/generated");

    if !specs_dir.exists() {
        return Err("specs/ directory not found".into());
    }

    if !generated_dir.exists() {
        return Err("src/generated/ directory not found. Run 'imacs regen' first.".into());
    }

    let mut passed = 0;
    let mut failed = 0;

    for entry in fs::read_dir(&specs_dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            let spec_content = fs::read_to_string(&path).map_err(Error::Io)?;
            let spec = Spec::from_yaml(&spec_content)?;

            // Check if generated file exists
            let generated_path = generated_dir.join(format!("{}.rs", spec.id));
            if !generated_path.exists() {
                println!(
                    "✗ Missing: {} (expected from {})",
                    generated_path.display(),
                    path.display()
                );
                failed += 1;
                continue;
            }

            // Regenerate expected code
            let expected_code = render(&spec, Target::Rust);
            let expected_tests = generate_tests(&spec, Target::Rust);
            let expected_full = format!(
                "{}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n{}\n}}\n",
                expected_code, expected_tests
            );

            // Read actual generated code
            let actual = fs::read_to_string(&generated_path).map_err(Error::Io)?;

            // Compare (ignoring timestamp and hash comments which vary between runs)
            let filter_metadata =
                |l: &&str| !l.starts_with("// GENERATED:") && !l.starts_with("// SPEC HASH:");
            let expected_lines: Vec<&str> = expected_full.lines().filter(filter_metadata).collect();
            let actual_lines: Vec<&str> = actual.lines().filter(filter_metadata).collect();

            if expected_lines == actual_lines {
                println!("✓ {}: matches spec", spec.id);
                passed += 1;
            } else {
                println!("✗ {}: MISMATCH - regenerate with 'imacs regen'", spec.id);
                failed += 1;
            }
        }
    }

    println!("\nSelfcheck: {} passed, {} failed", passed, failed);

    if failed > 0 {
        Err("Selfcheck failed - generated code does not match specs".into())
    } else {
        Ok(())
    }
}
