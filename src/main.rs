//! IMACS CLI - Command-line interface
//!
//! Commands:
//!   verify   - Check code against spec
//!   render   - Generate code from spec
//!   test     - Generate tests from spec
//!   analyze  - Analyze code complexity
//!   extract  - Extract spec from code
//!   drift    - Compare implementations

use imacs::*;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
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
        "regen" => cmd_regen(),
        "selfcheck" => cmd_selfcheck(),
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
    verify <spec.yaml> <code.rs>   Check code implements spec
    render <spec.yaml> [--lang]    Generate code from spec
    test <spec.yaml> [--lang]      Generate tests from spec
    analyze <code.rs>              Analyze code complexity
    extract <code.rs>              Extract spec from code
    drift <code_a.rs> <code_b.rs>  Compare implementations
    regen                          Regenerate src/generated/ from specs/
    selfcheck                      Verify generated code matches specs

OPTIONS:
    --lang <rust|typescript|python|csharp|java|go>   Target language (default: rust)
    --output <file>                   Output file (default: stdout)
    --json                            JSON output format

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

    let spec_content = fs::read_to_string(spec_path).map_err(|e| Error::Io(e))?;
    let code_content = fs::read_to_string(code_path).map_err(|e| Error::Io(e))?;

    let spec = Spec::from_yaml(&spec_content)?;
    let code = parse_rust(&code_content)?;

    let result = verify(&spec, &code);

    println!("{}", result.to_report());

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

    let spec_content = fs::read_to_string(spec_path).map_err(|e| Error::Io(e))?;

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

    let spec_content = fs::read_to_string(spec_path).map_err(|e| Error::Io(e))?;
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

    let code_content = fs::read_to_string(code_path).map_err(|e| Error::Io(e))?;
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
    let output = parse_output_arg(args);

    let code_content = fs::read_to_string(code_path).map_err(|e| Error::Io(e))?;
    let code = parse_rust(&code_content)?;

    let extracted = extract(&code);

    write_output(&output, &extracted.to_yaml())?;
    Ok(())
}

fn cmd_drift(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err("Usage: imacs drift <code_a.rs> <code_b.rs>".into());
    }

    let path_a = &args[0];
    let path_b = &args[1];
    let json_output = args.contains(&"--json".to_string());

    let content_a = fs::read_to_string(path_a).map_err(|e| Error::Io(e))?;
    let content_b = fs::read_to_string(path_b).map_err(|e| Error::Io(e))?;

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
            fs::write(p, content).map_err(|e| Error::Io(e))?;
            eprintln!("Written to: {}", p.display());
        }
        None => {
            println!("{}", content);
        }
    }
    Ok(())
}

fn cmd_regen() -> Result<()> {
    let specs_dir = PathBuf::from("specs");
    let generated_dir = PathBuf::from("src/generated");

    if !specs_dir.exists() {
        return Err("specs/ directory not found".into());
    }

    // Ensure generated directory exists
    fs::create_dir_all(&generated_dir).map_err(|e| Error::Io(e))?;

    let mut modules = Vec::new();

    // Process each spec file
    for entry in fs::read_dir(&specs_dir).map_err(|e| Error::Io(e))? {
        let entry = entry.map_err(|e| Error::Io(e))?;
        let path = entry.path();

        if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
            let spec_content = fs::read_to_string(&path).map_err(|e| Error::Io(e))?;
            let spec = Spec::from_yaml(&spec_content)?;

            // Generate Rust code
            let code = render(&spec, Target::Rust);

            // Generate tests
            let tests = generate_tests(&spec, Target::Rust);

            // Combine code and tests
            let full_code = format!("{}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n{}\n}}\n", code, tests);

            // Write to generated file
            let output_name = format!("{}.rs", spec.id);
            let output_path = generated_dir.join(&output_name);
            fs::write(&output_path, &full_code).map_err(|e| Error::Io(e))?;

            println!("✓ Generated: {}", output_path.display());
            modules.push(spec.id.clone());
        }
    }

    // Generate mod.rs
    let mut mod_content = String::from("//! Generated modules from specs/\n//!\n//! DO NOT EDIT — regenerate with: imacs regen\n\n");
    for module in &modules {
        mod_content.push_str(&format!("pub mod {};\n", module));
    }

    let mod_path = generated_dir.join("mod.rs");
    fs::write(&mod_path, &mod_content).map_err(|e| Error::Io(e))?;
    println!("✓ Generated: {}", mod_path.display());

    println!("\n✓ Regenerated {} modules from specs/", modules.len());
    Ok(())
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

    for entry in fs::read_dir(&specs_dir).map_err(|e| Error::Io(e))? {
        let entry = entry.map_err(|e| Error::Io(e))?;
        let path = entry.path();

        if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
            let spec_content = fs::read_to_string(&path).map_err(|e| Error::Io(e))?;
            let spec = Spec::from_yaml(&spec_content)?;

            // Check if generated file exists
            let generated_path = generated_dir.join(format!("{}.rs", spec.id));
            if !generated_path.exists() {
                println!("✗ Missing: {} (expected from {})", generated_path.display(), path.display());
                failed += 1;
                continue;
            }

            // Regenerate expected code
            let expected_code = render(&spec, Target::Rust);
            let expected_tests = generate_tests(&spec, Target::Rust);
            let expected_full = format!("{}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n{}\n}}\n", expected_code, expected_tests);

            // Read actual generated code
            let actual = fs::read_to_string(&generated_path).map_err(|e| Error::Io(e))?;

            // Compare (ignoring timestamp and hash comments which vary between runs)
            let filter_metadata = |l: &&str| {
                !l.starts_with("// GENERATED:") && !l.starts_with("// SPEC HASH:")
            };
            let expected_lines: Vec<&str> = expected_full.lines()
                .filter(filter_metadata)
                .collect();
            let actual_lines: Vec<&str> = actual.lines()
                .filter(filter_metadata)
                .collect();

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
