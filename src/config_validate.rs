//! Config validation for IMACS
//!
//! Validates `.imacs_root` and `config.yaml` files for correctness.

use crate::config::{ImacRoot, LocalConfig, NamingConfig};
use std::path::Path;

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A validation issue found in config
#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub file: String,
}

impl ConfigIssue {
    pub fn error(code: &str, message: &str, file: &str) -> Self {
        Self {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.to_string(),
            file: file.to_string(),
        }
    }

    pub fn warning(code: &str, message: &str, file: &str) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.to_string(),
            file: file.to_string(),
        }
    }
}

/// Result of config validation
#[derive(Debug, Default)]
pub struct ConfigValidationResult {
    pub issues: Vec<ConfigIssue>,
    pub root_valid: bool,
    pub local_configs_checked: usize,
}

impl ConfigValidationResult {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }
}

/// Validate `.imacs_root` file
pub fn validate_imacs_root(path: &Path) -> ConfigValidationResult {
    let mut result = ConfigValidationResult::default();
    let file_str = path.display().to_string();

    // Check file exists
    if !path.exists() {
        result
            .issues
            .push(ConfigIssue::error("E001", "File does not exist", &file_str));
        return result;
    }

    // Try to read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            result.issues.push(ConfigIssue::error(
                "E002",
                &format!("Cannot read file: {}", e),
                &file_str,
            ));
            return result;
        }
    };

    // Try to parse YAML
    let root: ImacRoot = match serde_norway::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            result.issues.push(ConfigIssue::error(
                "E003",
                &format!("Invalid YAML: {}", e),
                &file_str,
            ));
            return result;
        }
    };

    // Validate version
    if root.version != 1 {
        result.issues.push(ConfigIssue::error(
            "E004",
            &format!(
                "Unsupported version: {}. Only version 1 is supported.",
                root.version
            ),
            &file_str,
        ));
    }

    // Validate project name
    if root.project.name.is_empty() {
        result.issues.push(ConfigIssue::error(
            "E005",
            "Project name cannot be empty",
            &file_str,
        ));
    }

    // Validate targets
    if root.defaults.targets.is_empty() {
        result.issues.push(ConfigIssue::warning(
            "W001",
            "No target languages specified. Defaulting to Rust only.",
            &file_str,
        ));
    }

    // Validate naming patterns
    validate_naming_pattern(&root.defaults.naming, &file_str, &mut result);

    // Validate imacs_version constraint (if specified)
    if !root.imacs_version.is_empty() {
        validate_version_constraint(&root.imacs_version, &file_str, &mut result);
    }

    // Check for overly permissive version constraint
    if root.imacs_version.is_empty() {
        result.issues.push(ConfigIssue::warning(
            "W002",
            "No imacs_version constraint specified. Consider adding one for reproducibility.",
            &file_str,
        ));
    } else if root.imacs_version == ">=0.0.1" || root.imacs_version == "*" {
        result.issues.push(ConfigIssue::warning(
            "W003",
            "imacs_version constraint is very permissive. Consider specifying a minimum version.",
            &file_str,
        ));
    }

    // Check validation settings
    if !root.validation.require_descriptions {
        result.issues.push(ConfigIssue::warning(
            "W004",
            "validation.require_descriptions is false. Consider enabling for better documentation.",
            &file_str,
        ));
    }

    result.root_valid = !result.has_errors();
    result
}

/// Validate a local config.yaml file
pub fn validate_local_config(path: &Path) -> ConfigValidationResult {
    let mut result = ConfigValidationResult::default();
    let file_str = path.display().to_string();

    // Check file exists
    if !path.exists() {
        // Local config is optional, not an error
        return result;
    }

    // Try to read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            result.issues.push(ConfigIssue::error(
                "E002",
                &format!("Cannot read file: {}", e),
                &file_str,
            ));
            return result;
        }
    };

    // Try to parse YAML
    let _config: LocalConfig = match serde_norway::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            result.issues.push(ConfigIssue::error(
                "E003",
                &format!("Invalid YAML: {}", e),
                &file_str,
            ));
            return result;
        }
    };

    result.local_configs_checked = 1;
    result
}

/// Validate naming pattern has required placeholders
fn validate_naming_pattern(naming: &NamingConfig, file: &str, result: &mut ConfigValidationResult) {
    // Code pattern must have {spec_id} and {ext}
    if !naming.code.contains("{spec_id}") {
        result.issues.push(ConfigIssue::error(
            "E006",
            "Naming pattern 'code' must contain {spec_id} placeholder",
            file,
        ));
    }
    if !naming.code.contains("{ext}") {
        result.issues.push(ConfigIssue::error(
            "E007",
            "Naming pattern 'code' must contain {ext} placeholder",
            file,
        ));
    }

    // Test pattern must have {spec_id} and {ext}
    if !naming.tests.contains("{spec_id}") {
        result.issues.push(ConfigIssue::error(
            "E008",
            "Naming pattern 'tests' must contain {spec_id} placeholder",
            file,
        ));
    }
    if !naming.tests.contains("{ext}") {
        result.issues.push(ConfigIssue::error(
            "E009",
            "Naming pattern 'tests' must contain {ext} placeholder",
            file,
        ));
    }
}

/// Validate version constraint syntax
fn validate_version_constraint(constraint: &str, file: &str, result: &mut ConfigValidationResult) {
    // Simple validation - just check for common patterns
    let valid_prefixes = [">=", "<=", ">", "<", "=", "^", "~", "*"];
    let has_valid_prefix = valid_prefixes.iter().any(|p| constraint.starts_with(p));

    // Also allow bare version numbers like "0.1.0"
    let looks_like_version = constraint
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false);

    if !has_valid_prefix && !looks_like_version && constraint != "*" {
        result.issues.push(ConfigIssue::warning(
            "W005",
            &format!(
                "imacs_version constraint '{}' may not be valid semver syntax",
                constraint
            ),
            file,
        ));
    }
}

/// Validate all configs in a project directory
pub fn validate_project(root_dir: &Path) -> ConfigValidationResult {
    let mut result = ConfigValidationResult::default();

    // Find and validate .imacs_root
    let root_file = root_dir.join(".imacs_root");
    if root_file.exists() {
        let root_result = validate_imacs_root(&root_file);
        result.issues.extend(root_result.issues);
        result.root_valid = root_result.root_valid;
    } else {
        // Try to find .imacs_root in parent directories
        let mut current = root_dir.to_path_buf();
        let mut found = false;
        while let Some(parent) = current.parent() {
            let root_file = parent.join(".imacs_root");
            if root_file.exists() {
                let root_result = validate_imacs_root(&root_file);
                result.issues.extend(root_result.issues);
                result.root_valid = root_result.root_valid;
                found = true;
                break;
            }
            current = parent.to_path_buf();
        }
        if !found {
            result.issues.push(ConfigIssue::warning(
                "W006",
                "No .imacs_root found. Run 'imacs init --root' to create one.",
                &root_dir.display().to_string(),
            ));
        }
    }

    // Find and validate local config.yaml files
    if let Ok(entries) = std::fs::read_dir(root_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let config_file = path.join("config.yaml");
                if config_file.exists() {
                    let local_result = validate_local_config(&config_file);
                    result.issues.extend(local_result.issues);
                    result.local_configs_checked += local_result.local_configs_checked;
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_valid_imacs_root() {
        let dir = TempDir::new().unwrap();
        let root_file = dir.path().join(".imacs_root");

        let content = r#"
version: 1
project:
  name: test-project
defaults:
  targets: [rust, typescript]
"#;
        std::fs::write(&root_file, content).unwrap();

        let result = validate_imacs_root(&root_file);
        assert!(
            !result.has_errors(),
            "Expected no errors: {:?}",
            result.issues
        );
    }

    #[test]
    fn test_validate_invalid_version() {
        let dir = TempDir::new().unwrap();
        let root_file = dir.path().join(".imacs_root");

        let content = r#"
version: 99
project:
  name: test-project
defaults:
  targets: [rust]
"#;
        std::fs::write(&root_file, content).unwrap();

        let result = validate_imacs_root(&root_file);
        assert!(result.has_errors());
        assert!(result.issues.iter().any(|i| i.code == "E004"));
    }

    #[test]
    fn test_validate_empty_project_name() {
        let dir = TempDir::new().unwrap();
        let root_file = dir.path().join(".imacs_root");

        let content = r#"
version: 1
project:
  name: ""
defaults:
  targets: [rust]
"#;
        std::fs::write(&root_file, content).unwrap();

        let result = validate_imacs_root(&root_file);
        assert!(result.has_errors());
        assert!(result.issues.iter().any(|i| i.code == "E005"));
    }

    #[test]
    fn test_validate_invalid_naming_pattern() {
        let dir = TempDir::new().unwrap();
        let root_file = dir.path().join(".imacs_root");

        let content = r#"
version: 1
project:
  name: test
defaults:
  targets: [rust]
  naming:
    code: "output.txt"
    tests: "test.txt"
"#;
        std::fs::write(&root_file, content).unwrap();

        let result = validate_imacs_root(&root_file);
        assert!(result.has_errors());
        // Should have errors for missing {spec_id} and {ext}
        assert!(result.issues.iter().any(|i| i.code == "E006"));
        assert!(result.issues.iter().any(|i| i.code == "E007"));
    }
}
