//! Project discovery and validation
//!
//! Finds `imacs/` folders, validates structure, and enforces safeguards
//! from FMECA analysis.

use crate::config::{ImacRoot, LocalConfig, MergedConfig};
use crate::error::{Error, Result};
use crate::spec::Spec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// An imacs folder with its configuration
#[derive(Debug, Clone)]
pub struct ImacFolder {
    pub path: PathBuf,
    pub config: MergedConfig,
    pub is_root: bool,
}

/// Project structure discovery result
#[derive(Debug)]
pub struct ProjectStructure {
    pub root: Option<ImacFolder>,
    pub folders: Vec<ImacFolder>,
}

/// Find the project root (folder containing `.imacs_root`)
///
/// Safeguard: Errors if multiple roots found
pub fn find_root(start_dir: &Path) -> Result<Option<PathBuf>> {
    let mut current = start_dir.canonicalize().map_err(Error::Io)?;
    let mut found_root: Option<PathBuf> = None;

    loop {
        // Check for both visible and hidden folders
        let visible = current.join("imacs").join(".imacs_root");
        let hidden = current.join(".imacs").join(".imacs_root");

        // Safeguard: Error if both exist at same level
        if visible.exists() && hidden.exists() {
            return Err(Error::Other(format!(
                "Ambiguous: both imacs/ and .imacs/ exist at {}",
                current.display()
            )));
        }

        if visible.exists() {
            if found_root.is_some() {
                return Err(Error::Other(
                    "Multiple .imacs_root files found - only one allowed per project".to_string(),
                ));
            }
            found_root = Some(current.join("imacs"));
        } else if hidden.exists() {
            if found_root.is_some() {
                return Err(Error::Other(
                    "Multiple .imacs_root files found - only one allowed per project".to_string(),
                ));
            }
            found_root = Some(current.join(".imacs"));
        }

        // Move to parent
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    Ok(found_root)
}

/// Discover all imacs folders in the project
///
/// Starts from root and recursively finds all imacs/ and .imacs/ folders
pub fn discover_all_imacs(root_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut folders = Vec::new();
    let root_path = root_dir.canonicalize().map_err(Error::Io)?;

    // Start from root and walk down
    discover_imacs_recursive(&root_path, &mut folders)?;

    Ok(folders)
}

fn discover_imacs_recursive(dir: &Path, folders: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // Skip directories we can't read
    };

    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check if this is an imacs folder
        if dir_name == "imacs" || dir_name == ".imacs" {
            // Safeguard: Error if both exist at same level
            let parent = path.parent().unwrap();
            let visible = parent.join("imacs");
            let hidden = parent.join(".imacs");
            if visible.exists() && hidden.exists() {
                return Err(Error::Other(format!(
                    "Ambiguous: both imacs/ and .imacs/ exist at {}",
                    parent.display()
                )));
            }

            folders.push(path.clone());
        } else {
            // Recurse into subdirectories (but skip generated/)
            if dir_name != "generated" && !dir_name.starts_with('.') {
                discover_imacs_recursive(&path, folders)?;
            }
        }
    }

    Ok(())
}

/// Load project structure with all configurations
pub fn load_project_structure(start_dir: &Path) -> Result<ProjectStructure> {
    let root_path = match find_root(start_dir)? {
        Some(path) => path,
        None => {
            // No root found - return empty structure
            return Ok(ProjectStructure {
                root: None,
                folders: Vec::new(),
            });
        }
    };

    // Load root config
    let root_config = ImacRoot::load_from_dir(&root_path)?
        .ok_or_else(|| Error::Other(".imacs_root file missing".to_string()))?;

    // Validate tool version
    validate_version(&root_config)?;

    let root_local = LocalConfig::load_from_dir(&root_path)?;
    let root_folder = ImacFolder {
        path: root_path.clone(),
        config: root_config.merge(root_local.as_ref()),
        is_root: true,
    };

    // Discover all imacs folders
    let all_folders = discover_all_imacs(&root_path)?;
    let mut folders = Vec::new();

    for folder_path in all_folders {
        if folder_path == root_path {
            continue; // Already added as root
        }

        let local = LocalConfig::load_from_dir(&folder_path)?;
        let merged = root_config.merge(local.as_ref());

        folders.push(ImacFolder {
            path: folder_path,
            config: merged,
            is_root: false,
        });
    }

    Ok(ProjectStructure {
        root: Some(root_folder),
        folders,
    })
}

/// Validate tool version against constraint
fn validate_version(root: &ImacRoot) -> Result<()> {
    if root.imacs_version.is_empty() {
        return Ok(()); // No constraint
    }

    // Simple version check (can be enhanced with semver parsing)
    let current_version = crate::VERSION;

    // For now, just check if version string is present
    // Full semver parsing can be added later
    if root.imacs_version.starts_with(">=") {
        // Basic check - in production would use semver crate
        let required = root.imacs_version.trim_start_matches(">=").trim();
        if current_version < required {
            return Err(Error::Other(format!(
                "Tool version {} does not meet requirement {}",
                current_version, root.imacs_version
            )));
        }
    }

    Ok(())
}

/// Detect output path conflicts (multiple specs writing to same file)
///
/// Safeguard: Prevents overwriting files from different specs
/// Returns empty vec if no conflicts, or list of error messages if conflicts found
pub fn detect_output_conflicts(structure: &ProjectStructure) -> Vec<String> {
    use std::collections::HashMap;
    let mut output_map: HashMap<PathBuf, Vec<(String, PathBuf)>> = HashMap::new();
    let mut errors = Vec::new();

    // Collect all output paths
    for folder in &structure.folders {
        if let Some(root) = &structure.root {
            if folder.path == root.path {
                continue; // Skip root, already processed
            }
        }
        collect_output_paths(folder, &mut output_map);
    }

    if let Some(root) = &structure.root {
        collect_output_paths(root, &mut output_map);
    }

    // Check for conflicts
    for (output_path, sources) in output_map {
        if sources.len() > 1 {
            let spec_list: Vec<String> = sources
                .iter()
                .map(|(spec_id, spec_path)| format!("{} ({})", spec_id, spec_path.display()))
                .collect();
            errors.push(format!(
                "Output path conflict: {} would be written by multiple specs: {}",
                output_path.display(),
                spec_list.join(", ")
            ));
        }
    }

    errors
}

fn collect_output_paths(
    folder: &ImacFolder,
    output_map: &mut std::collections::HashMap<PathBuf, Vec<(String, PathBuf)>>,
) {
    use crate::spec::Spec;

    if let Ok(specs) = list_specs(&folder.path) {
        for spec_path in specs {
            if let Ok(content) = std::fs::read_to_string(&spec_path) {
                if let Ok(spec) = Spec::from_yaml(&content) {
                    let spec_id = if !folder.config.spec_id_prefix.is_empty() {
                        format!("{}{}", folder.config.spec_id_prefix, spec.id)
                    } else {
                        spec.id.clone()
                    };

                    // Check each target language
                    for target in &folder.config.targets {
                        let output_dir = get_output_dir(&folder.path, &folder.config, *target);
                        let code_filename = folder.config.apply_naming(&spec_id, target, false);
                        let output_path = output_dir.join(&code_filename);

                        output_map
                            .entry(output_path)
                            .or_default()
                            .push((spec_id.clone(), spec_path.clone()));
                    }
                }
            }
        }
    }
}

/// Validate unique spec IDs across entire project
///
/// Safeguard: Prevents ID collisions
/// Returns empty vec if valid, or list of error messages if collisions found
pub fn validate_unique_ids(structure: &ProjectStructure) -> Result<Vec<String>> {
    let mut id_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut errors = Vec::new();

    // Collect all spec IDs
    for folder in &structure.folders {
        if let Some(root) = &structure.root {
            if folder.path == root.path {
                continue; // Skip root, already processed
            }
        }
        collect_spec_ids(&folder.path, &mut id_map)?;
    }

    if let Some(root) = &structure.root {
        collect_spec_ids(&root.path, &mut id_map)?;
    }

    // Check for collisions
    for (id, paths) in id_map {
        if paths.len() > 1 {
            errors.push(format!(
                "Spec ID '{}' found in multiple locations: {}",
                id,
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    Ok(errors)
}

fn collect_spec_ids(dir: &Path, id_map: &mut HashMap<String, Vec<PathBuf>>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(Error::Io)?;

    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    let content = std::fs::read_to_string(&path).map_err(Error::Io)?;
                    if let Ok(spec) = Spec::from_yaml(&content) {
                        id_map
                            .entry(spec.id.clone())
                            .or_default()
                            .push(path.clone());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Get generated directory path for an imacs folder
/// Get output directory for a specific language using config
pub fn get_output_dir(
    imacs_dir: &Path,
    config: &crate::config::MergedConfig,
    target: crate::cel::Target,
) -> PathBuf {
    let base = imacs_dir.parent().unwrap_or(imacs_dir);

    // 1. Check per-language override
    let lang_override = match target {
        crate::cel::Target::Rust => &config.output.rust,
        crate::cel::Target::TypeScript => &config.output.typescript,
        crate::cel::Target::Python => &config.output.python,
        crate::cel::Target::Go => &config.output.go,
        crate::cel::Target::Java => &config.output.java,
        crate::cel::Target::CSharp => &config.output.csharp,
    };

    if let Some(path) = lang_override {
        return base.join(path);
    }

    // 2. Check default override
    if let Some(default) = &config.output.default {
        return base.join(default);
    }

    // 3. Fallback to convention
    base.join("generated")
}

/// Legacy function for backward compatibility
/// Use get_output_dir() instead
pub fn get_generated_dir(imacs_dir: &Path) -> PathBuf {
    imacs_dir.parent().unwrap_or(imacs_dir).join("generated")
}

/// List all spec files in an imacs directory
pub fn list_specs(imacs_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut specs = Vec::new();

    if !imacs_dir.exists() {
        return Ok(specs);
    }

    let entries = std::fs::read_dir(imacs_dir).map_err(Error::Io)?;
    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    // Skip config files
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == "config.yaml" || name == ".imacs_root" {
                            continue;
                        }
                    }
                    specs.push(path);
                }
            }
        } else if path.is_dir() {
            // Recursively search subdirectories
            let mut sub_specs = list_specs(&path)?;
            specs.append(&mut sub_specs);
        }
    }

    Ok(specs)
}

/// Discover specs directory (imacs/ or .imacs/)
///
/// Checks current directory and parents for imacs/ or .imacs/ folder
pub fn discover_specs_dir(start_dir: &Path) -> Result<Option<PathBuf>> {
    let mut current = start_dir.canonicalize().map_err(Error::Io)?;

    loop {
        let visible = current.join("imacs");
        let hidden = current.join(".imacs");

        // Check if either exists
        if visible.exists() && visible.is_dir() {
            return Ok(Some(visible));
        }
        if hidden.exists() && hidden.is_dir() {
            return Ok(Some(hidden));
        }

        // Move to parent
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    Ok(None)
}

/// Discover or create generated directory for an imacs folder
pub fn discover_generated_dir(imacs_dir: &Path) -> Result<PathBuf> {
    let generated_dir = get_generated_dir(imacs_dir);

    // Create if it doesn't exist
    if !generated_dir.exists() {
        std::fs::create_dir_all(&generated_dir).map_err(Error::Io)?;
    }

    Ok(generated_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_root() {
        let temp = TempDir::new().unwrap();
        let root_imacs = temp.path().join("imacs");
        fs::create_dir_all(&root_imacs).unwrap();
        fs::write(
            root_imacs.join(".imacs_root"),
            "version: 1\nproject:\n  name: test",
        )
        .unwrap();

        let found = find_root(temp.path()).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap(), root_imacs);
    }

    #[test]
    fn test_validate_unique_ids() {
        // This would require creating test specs - defer to integration tests
    }

    #[test]
    fn test_get_output_dir_default() {
        use crate::cel::Target;
        use crate::config::{MergedConfig, NamingConfig, OutputConfig, ValidationConfig};

        let temp = TempDir::new().unwrap();
        let imacs_dir = temp.path().join("imacs");
        fs::create_dir_all(&imacs_dir).unwrap();

        let config = MergedConfig {
            targets: vec![Target::Rust],
            auto_format: true,
            naming: NamingConfig::default(),
            validation: ValidationConfig::default(),
            spec_id_prefix: "".to_string(),
            output: OutputConfig::default(),
        };

        let output_dir = get_output_dir(&imacs_dir, &config, Target::Rust);
        assert_eq!(output_dir, temp.path().join("generated"));
    }

    #[test]
    fn test_get_output_dir_per_language() {
        use crate::cel::Target;
        use crate::config::{MergedConfig, NamingConfig, OutputConfig, ValidationConfig};

        let temp = TempDir::new().unwrap();
        let imacs_dir = temp.path().join("imacs");
        fs::create_dir_all(&imacs_dir).unwrap();

        // Use simple paths without .. for testing
        let output = OutputConfig {
            rust: Some("./rust_output".to_string()),
            typescript: Some("./ts_output".to_string()),
            ..Default::default()
        };

        let config = MergedConfig {
            targets: vec![Target::Rust, Target::TypeScript],
            auto_format: true,
            naming: NamingConfig::default(),
            validation: ValidationConfig::default(),
            spec_id_prefix: "".to_string(),
            output,
        };

        let rust_dir = get_output_dir(&imacs_dir, &config, Target::Rust);
        let expected_rust = temp.path().join("rust_output");
        assert_eq!(rust_dir, expected_rust, "Rust output dir mismatch");

        let ts_dir = get_output_dir(&imacs_dir, &config, Target::TypeScript);
        let expected_ts = temp.path().join("ts_output");
        assert_eq!(ts_dir, expected_ts, "TypeScript output dir mismatch");
    }

    #[test]
    fn test_get_output_dir_default_override() {
        use crate::cel::Target;
        use crate::config::{MergedConfig, NamingConfig, OutputConfig, ValidationConfig};

        let temp = TempDir::new().unwrap();
        let imacs_dir = temp.path().join("imacs");
        fs::create_dir_all(&imacs_dir).unwrap();

        // Use simple path without .. for testing
        let output = OutputConfig {
            default: Some("./custom_output".to_string()),
            ..Default::default()
        };

        let config = MergedConfig {
            targets: vec![Target::Rust],
            auto_format: true,
            naming: NamingConfig::default(),
            validation: ValidationConfig::default(),
            spec_id_prefix: "".to_string(),
            output,
        };

        let output_dir = get_output_dir(&imacs_dir, &config, Target::Rust);
        let expected = temp.path().join("custom_output");
        assert_eq!(output_dir, expected);
    }
}
