//! IMACS CLI - Command-line interface
//!
//! Commands:
//!   verify   - Check code against spec
//!   render   - Generate code from spec
//!   test     - Generate tests from spec
//!   analyze  - Analyze code complexity
//!   extract  - Extract spec from code
//!   sexp     - Output raw tree-sitter S-expression
//!   drift    - Compare implementations
//!   update   - Update to latest version

mod cli;
mod update;

use imacs::*;
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
        "verify" => cli::cmd_verify(&args[2..]),
        "render" => cli::cmd_render(&args[2..]),
        "test" => cli::cmd_test(&args[2..]),
        "analyze" => cli::cmd_analyze(&args[2..]),
        "extract" => cli::cmd_extract(&args[2..]),
        "sexp" => cli::cmd_sexp(&args[2..]),
        "drift" => cli::cmd_drift(&args[2..]),
        "completeness" => cli::cmd_completeness(&args[2..]),
        "validate" => cli::cmd_validate(&args[2..]),
        "config" => cli::cmd_config(&args[2..]),
        "schema" => cli::cmd_schema(&args[2..]),
        "init" => cli::cmd_init(&args[2..]),
        "regen" => cli::cmd_regen(),
        "status" => cli::cmd_status(&args[2..]),
        "selfcheck" => cli::cmd_selfcheck(),
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
    sexp <code.rs> [--lang]           Output raw tree-sitter S-expression
    drift <code_a.rs> <code_b.rs>    Compare implementations
    completeness <spec.yaml|dir>     Analyze spec(s) for missing cases
                                      Use directory for suite analysis
    validate <spec.yaml> [--strict]  Validate spec for impossible situations
    config check [--json]            Validate .imacs_root and config.yaml files
    config schema [name]             Print JSON schema for config type
    schema [name]                     Print JSON schema for output type
    init [--root]                    Initialize imacs/ folder (--root for project root)
    regen [--all] [--force] [--clean] Regenerate code from specs (--clean removes orphaned files)
    status [--json]                  Show project status and stale specs
    selfcheck                        Verify IMACS internal generated code (from imacs/) matches
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

fn cmd_update() -> Result<()> {
    match update::run_update() {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Update failed: {}", e).into()),
    }
}
