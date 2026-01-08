//! Simple CLI commands: verify, render, test, analyze, extract, sexp, drift

use super::util::{parse_output_arg, parse_target_arg, write_output};
use imacs::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Load specs referenced by an orchestrator
///
/// Searches for spec files in:
/// 1. Same directory as the orchestrator file
/// 2. `imacs/` subdirectory relative to orchestrator
/// 3. Parent directory of orchestrator
fn load_referenced_specs(
    orch: &orchestrate::Orchestrator,
    orch_path: &str,
) -> Result<HashMap<String, Spec>> {
    let mut specs = HashMap::new();
    let orch_dir = Path::new(orch_path)
        .parent()
        .unwrap_or(Path::new("."));

    for spec_name in orch.referenced_specs() {
        // Try different locations for the spec file
        let possible_paths = vec![
            orch_dir.join(format!("{}.yaml", spec_name)),
            orch_dir.join(format!("{}.yml", spec_name)),
            orch_dir.join("imacs").join(format!("{}.yaml", spec_name)),
            orch_dir.join("imacs").join(format!("{}.yml", spec_name)),
            orch_dir.parent().unwrap_or(Path::new(".")).join(format!("{}.yaml", spec_name)),
            orch_dir.parent().unwrap_or(Path::new(".")).join(format!("{}.yml", spec_name)),
        ];

        let mut found = false;
        for path in possible_paths {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => match Spec::from_yaml(&content) {
                        Ok(spec) => {
                            specs.insert(spec_name.clone(), spec);
                            found = true;
                            break;
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        if !found {
            eprintln!(
                "Warning: Referenced spec '{}' not found (orchestrator may not render correctly)",
                spec_name
            );
        }
    }

    Ok(specs)
}

pub fn cmd_verify(args: &[String]) -> Result<()> {
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

pub fn cmd_render(args: &[String]) -> Result<()> {
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
        let specs = load_referenced_specs(&orch, spec_path)?;
        orchestrate::render_orchestrator(&orch, &specs, target)
    } else {
        // It's a regular decision table spec
        let spec = Spec::from_yaml(&spec_content)?;
        render(&spec, target)
    };

    write_output(&output, &code)?;
    Ok(())
}

pub fn cmd_test(args: &[String]) -> Result<()> {
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

pub fn cmd_analyze(args: &[String]) -> Result<()> {
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

pub fn cmd_extract(args: &[String]) -> Result<()> {
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

pub fn cmd_sexp(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(
            "Usage: imacs sexp <code.rs> [--lang <rust|typescript|python|go|csharp|java>]".into(),
        );
    }

    let code_path = &args[0];
    let output = parse_output_arg(args);

    // Detect language from file extension or --lang flag
    let lang = if let Some(idx) = args.iter().position(|a| a == "--lang") {
        args.get(idx + 1)
            .map(|s| match s.as_str() {
                "rust" | "rs" => Language::Rust,
                "typescript" | "ts" => Language::TypeScript,
                "python" | "py" => Language::Python,
                "go" => Language::Go,
                "csharp" | "cs" => Language::CSharp,
                "java" => Language::Java,
                _ => detect_language(code_path),
            })
            .unwrap_or_else(|| detect_language(code_path))
    } else {
        detect_language(code_path)
    };

    if lang == Language::Unknown {
        return Err(format!(
            "Cannot detect language for '{}'. Use --lang to specify.",
            code_path
        )
        .into());
    }

    let code_content = fs::read_to_string(code_path).map_err(Error::Io)?;
    let sexp = to_sexp(&code_content, lang)?;

    write_output(&output, &sexp)?;
    Ok(())
}

pub fn cmd_drift(args: &[String]) -> Result<()> {
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
