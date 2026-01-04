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
                            .or_insert_with(Vec::new)
                            .push(path.clone());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Get generated directory path for an imacs folder
pub fn get_generated_dir(imacs_dir: &Path) -> PathBuf {
    imacs_dir
        .parent()
        .unwrap_or(imacs_dir)
        .join("generated")
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
        fs::write(root_imacs.join(".imacs_root"), "version: 1\nproject:\n  name: test").unwrap();

        let found = find_root(temp.path()).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap(), root_imacs);
    }

    #[test]
    fn test_validate_unique_ids() {
        // This would require creating test specs - defer to integration tests
    }
}

