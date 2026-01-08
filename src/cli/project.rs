//! Project management CLI commands: init, status, regen, selfcheck

use imacs::*;
use std::fs;
use std::path::PathBuf;

pub fn cmd_init(args: &[String]) -> Result<()> {
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
                loop {
                    let check_imacs = check_dir.join("imacs").join(".imacs_root");
                    let check_hidden = check_dir.join(".imacs").join(".imacs_root");
                    if check_imacs.exists() || check_hidden.exists() {
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

pub fn cmd_status(args: &[String]) -> Result<()> {
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

        if let Ok(stale) =
            imacs::find_stale_specs(&root.path, &imacs::get_generated_dir(&root.path))
        {
            total_stale += stale.len();
        }

        if total_stale > 0 {
            println!(
                "\n⚠ {} stale spec(s) need regeneration (run 'imacs regen')",
                total_stale
            );
        } else {
            println!("\n✓ All specs up to date");
        }

        // Check for orphaned files
        let mut total_orphaned = 0;
        for folder in std::iter::once(root).chain(structure.folders.iter()) {
            // Get current spec IDs
            let current_spec_ids: Vec<String> = {
                let entries = fs::read_dir(&folder.path).ok();
                entries
                    .map(|e| {
                        e.filter_map(|e| e.ok())
                            .filter_map(|e| {
                                let path = e.path();
                                if path.is_file() {
                                    if let Some(ext) = path.extension() {
                                        if (ext == "yaml" || ext == "yml")
                                            && path.file_name().and_then(|n| n.to_str())
                                                != Some("config.yaml")
                                            && path.file_name().and_then(|n| n.to_str())
                                                != Some(".imacs_root")
                                        {
                                            let content = fs::read_to_string(&path).ok()?;
                                            let spec = Spec::from_yaml(&content).ok()?;
                                            if !folder.config.spec_id_prefix.is_empty() {
                                                return Some(format!(
                                                    "{}{}",
                                                    folder.config.spec_id_prefix, spec.id
                                                ));
                                            }
                                            return Some(spec.id);
                                        }
                                    }
                                }
                                None
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            };

            for target in &folder.config.targets {
                let output_dir =
                    imacs::project::get_output_dir(&folder.path, &folder.config, *target);
                if let Ok(Some(meta)) = imacs::ImacMeta::load_from_dir(&output_dir) {
                    if let Ok(orphaned) = meta.find_orphaned_files(&output_dir, &current_spec_ids) {
                        total_orphaned += orphaned.len();
                    }
                }
            }
        }

        if total_orphaned > 0 {
            println!(
                "\n⚠ {} orphaned file(s) found (run 'imacs regen --clean' to remove)",
                total_orphaned
            );
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

pub fn cmd_regen() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let all_mode = args.contains(&"--all".to_string());
    let force = args.contains(&"--force".to_string());
    let clean = args.contains(&"--clean".to_string());
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
                    if structure
                        .root
                        .as_ref()
                        .unwrap()
                        .config
                        .validation
                        .require_unique_ids
                    {
                        return Err("ID collisions found. Fix before regenerating.".into());
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to validate IDs: {}", e);
            }
        }

        // Detect output path conflicts (safeguard)
        if structure
            .root
            .as_ref()
            .unwrap()
            .config
            .validation
            .detect_output_conflicts
        {
            let conflicts = imacs::project::detect_output_conflicts(&structure);
            if !conflicts.is_empty() {
                eprintln!("⚠ Output Path Conflicts detected:");
                for err in &conflicts {
                    eprintln!("  {}", err);
                }
                return Err("Output path conflicts found. Multiple specs would write to the same file. Fix output configuration before regenerating.".into());
            }
        }

        let mut total_regenerated = 0;
        let mut total_cleaned = 0;

        // Process root folder
        if let Some(root) = &structure.root {
            let (regenerated, cleaned) = regenerate_folder(root, force, clean)?;
            total_regenerated += regenerated;
            total_cleaned += cleaned;
        }

        // Process all child folders
        for folder in &structure.folders {
            let (regenerated, cleaned) = regenerate_folder(folder, force, clean)?;
            total_regenerated += regenerated;
            total_cleaned += cleaned;
        }

        println!(
            "\n✓ Regenerated {} spec(s) across {} folder(s)",
            total_regenerated,
            structure.folders.len() + 1
        );
        if clean && total_cleaned > 0 {
            println!("🧹 Cleaned {} orphaned file(s)", total_cleaned);
        }
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
                structure
                    .folders
                    .iter()
                    .find(|f| current_dir.starts_with(&f.path))
                    .cloned()
                    .unwrap_or_else(|| root.clone())
            }
        } else {
            return Err("No IMACS project found. Run 'imacs init --root' first.".into());
        };

        let (_, cleaned) = regenerate_folder(&folder, force, clean)?;
        if clean && cleaned > 0 {
            println!("🧹 Cleaned {} orphaned file(s)", cleaned);
        }
    }

    Ok(())
}

/// Regenerate specs in a folder, returns (regenerated_count, cleaned_count)
fn regenerate_folder(
    folder: &imacs::ImacFolder,
    force: bool,
    clean: bool,
) -> Result<(usize, usize)> {
    // Collect all current spec IDs for orphan detection
    let all_specs: Vec<PathBuf> = {
        let entries = fs::read_dir(&folder.path).map_err(Error::Io)?;
        let mut specs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(Error::Io)?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if (ext == "yaml" || ext == "yml")
                        && path.file_name().and_then(|n| n.to_str()) != Some("config.yaml")
                        && path.file_name().and_then(|n| n.to_str()) != Some(".imacs_root")
                    {
                        specs.push(path);
                    }
                }
            }
        }
        specs
    };

    // Get current spec IDs for orphan detection
    let current_spec_ids: Vec<String> = all_specs
        .iter()
        .filter_map(|p| {
            let content = fs::read_to_string(p).ok()?;
            let spec = Spec::from_yaml(&content).ok()?;
            if !folder.config.spec_id_prefix.is_empty() {
                Some(format!("{}{}", folder.config.spec_id_prefix, spec.id))
            } else {
                Some(spec.id.clone())
            }
        })
        .collect();

    // Find stale specs (need to check all possible output directories)
    let specs_to_regenerate = if force {
        all_specs.clone()
    } else {
        // Check staleness - need to check all output directories
        // For now, use default generated dir for staleness check
        let default_dir = imacs::get_generated_dir(&folder.path);
        imacs::find_stale_specs(&folder.path, &default_dir)?
    };

    let mut cleaned = 0;

    // Clean orphaned files if requested
    if clean {
        for target in &folder.config.targets {
            let output_dir = imacs::project::get_output_dir(&folder.path, &folder.config, *target);
            if let Ok(Some(meta)) = imacs::ImacMeta::load_from_dir(&output_dir) {
                match meta.find_orphaned_files(&output_dir, &current_spec_ids) {
                    Ok(orphaned) => {
                        for orphan in orphaned {
                            if let Err(e) = fs::remove_file(&orphan) {
                                eprintln!(
                                    "Warning: Could not remove orphaned file {}: {}",
                                    orphan.display(),
                                    e
                                );
                            } else {
                                println!("🗑  Removed orphaned: {}", orphan.display());
                                cleaned += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not check for orphaned files: {}", e);
                    }
                }
            }
        }
    }

    if specs_to_regenerate.is_empty() {
        if cleaned == 0 {
            println!("✓ {}: up to date", folder.path.display());
        }
        return Ok((0, cleaned));
    }

    let mut regenerated = 0;

    for spec_path in &specs_to_regenerate {
        let spec_content = fs::read_to_string(spec_path).map_err(Error::Io)?;

        // Check if this is an orchestrator (has 'chain:' or 'uses:' key) or a regular spec
        let is_orchestrator = spec_content.contains("\nchain:") || spec_content.contains("\nuses:");

        // Get the ID based on type
        let spec_id = if is_orchestrator {
            let orch = orchestrate::Orchestrator::from_yaml(&spec_content)?;
            if !folder.config.spec_id_prefix.is_empty() {
                format!("{}{}", folder.config.spec_id_prefix, orch.id)
            } else {
                orch.id.clone()
            }
        } else {
            let spec = Spec::from_yaml(&spec_content)?;
            if !folder.config.spec_id_prefix.is_empty() {
                format!("{}{}", folder.config.spec_id_prefix, spec.id)
            } else {
                spec.id.clone()
            }
        };

        // Generate for each target language
        for target in &folder.config.targets {
            // Get output directory for this language
            let output_dir = imacs::project::get_output_dir(&folder.path, &folder.config, *target);

            // Load or create metadata for this output directory
            let mut meta =
                imacs::ImacMeta::load_from_dir(&output_dir)?.unwrap_or_else(imacs::create_meta);

            // Ensure output directory exists
            fs::create_dir_all(&output_dir).map_err(Error::Io)?;

            // Generate code based on type
            let (code, tests) = if is_orchestrator {
                let orch = orchestrate::Orchestrator::from_yaml(&spec_content)?;
                let specs_map = std::collections::HashMap::new();
                (
                    orchestrate::render_orchestrator(&orch, &specs_map, *target),
                    testgen::orchestrator::generate_orchestrator_tests(&orch, *target),
                )
            } else {
                let spec = Spec::from_yaml(&spec_content)?;
                (render(&spec, *target), generate_tests(&spec, *target))
            };

            // Apply naming convention
            let code_filename = folder.config.apply_naming(&spec_id, target, false);
            let test_filename = folder.config.apply_naming(&spec_id, target, true);

            let code_path = output_dir.join(&code_filename);
            let test_path = output_dir.join(&test_filename);

            // Write code
            fs::write(&code_path, &code).map_err(Error::Io)?;

            // Track generated files for --clean support
            meta.track_generated_file(&spec_id, &code_filename);

            // Write tests (if any)
            if !tests.trim().is_empty() {
                fs::write(&test_path, &tests).map_err(Error::Io)?;
                meta.track_generated_file(&spec_id, &test_filename);
            }

            // Auto-format if enabled (formatting can be added later)
            if folder.config.auto_format {
                // Formatting will be implemented via format module
                // For now, just write the code as-is
            }

            // Update metadata hash
            meta.update_hash(spec_path, &folder.path)?;

            // Save metadata for this output directory
            meta.save_to_dir(&output_dir)?;

            println!(
                "✓ Generated: {} ({})",
                code_path.display(),
                format!("{:?}", target).to_lowercase()
            );
        }

        regenerated += 1;
    }

    Ok((regenerated, cleaned))
}

pub fn cmd_selfcheck() -> Result<()> {
    // Check IMACS's own internal specs (dogfooding) - uses imacs/ convention
    // IMACS's own specs are in src/imacs/ (special case)
    let current_dir = std::env::current_dir().map_err(Error::Io)?;
    let imacs_dir = current_dir.join("src").join("imacs");

    if !imacs_dir.exists() {
        return Err(
            "IMACS internal specs not found. Expected src/imacs/ folder with .imacs_root".into(),
        );
    }

    // Verify .imacs_root exists
    if !imacs_dir.join(".imacs_root").exists() {
        return Err("src/imacs/.imacs_root not found".into());
    }

    // For IMACS's own specs, generated code goes to src/generated/ (sibling to src/imacs/)
    let generated_dir = PathBuf::from("src/generated");

    if !generated_dir.exists() {
        return Err("src/generated/ directory not found. Run 'imacs regen' from project root to regenerate internal specs.".into());
    }

    let mut passed = 0;
    let mut failed = 0;

    // List all specs in the imacs folder
    let specs = imacs::list_specs(&imacs_dir)?;

    for spec_path in specs {
        let spec_content = fs::read_to_string(&spec_path).map_err(Error::Io)?;
        let spec = Spec::from_yaml(&spec_content)?;

        // Check if generated file exists
        let generated_path = generated_dir.join(format!("{}.rs", spec.id));
        if !generated_path.exists() {
            println!(
                "✗ Missing: {} (expected from {})",
                generated_path.display(),
                spec_path.display()
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

    println!("\nSelfcheck: {} passed, {} failed", passed, failed);

    if failed > 0 {
        Err("Selfcheck failed - generated code does not match specs".into())
    } else {
        Ok(())
    }
}
