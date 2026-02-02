//! Completeness analysis CLI commands

use imacs::*;
use std::fs;
use std::path::PathBuf;

pub fn cmd_completeness(args: &[String]) -> Result<()> {
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
                    if let Ok(spec) = Spec::from_yaml(&spec_content) {
                        let spec_id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        specs.push((spec_id, spec));
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
                    "    [{}] {}:{:?} - {}",
                    issue.step_id, issue.spec_id, issue.issue_type, issue.details
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
