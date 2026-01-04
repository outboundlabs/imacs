//! IMACS project configuration
//!
//! Handles loading and merging of `.imacs_root` (project root config)
//! and local `config.yaml` files in each `imacs/` folder.

use crate::cel::Target;
use crate::error::{Error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Root project configuration (`.imacs_root`)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImacRoot {
    /// Schema version for migrations
    pub version: u32,

    /// Tool version constraint (e.g., ">=0.1.0")
    #[serde(default)]
    pub imacs_version: String,

    /// Project-level settings
    pub project: ProjectConfig,

    /// Default settings for all child imacs folders
    pub defaults: DefaultsConfig,

    /// Validation rules
    #[serde(default)]
    pub validation: ValidationConfig,
}

/// Project-level configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Optional prefix for all spec IDs to avoid collisions
    #[serde(default)]
    pub spec_id_prefix: String,
}

/// Default settings applied to all imacs folders
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DefaultsConfig {
    /// Target languages to generate
    #[serde(default = "default_targets")]
    pub targets: Vec<Target>,

    /// Auto-format generated code
    #[serde(default = "default_true")]
    pub auto_format: bool,

    /// File naming conventions
    #[serde(default)]
    pub naming: NamingConfig,
}

fn default_targets() -> Vec<Target> {
    vec![Target::Rust]
}

fn default_true() -> bool {
    true
}

/// Naming convention for generated files
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NamingConfig {
    /// Code file pattern: {spec_id}, {lang}, {ext}
    #[serde(default = "default_code_naming")]
    pub code: String,

    /// Test file pattern: {spec_id}, {lang}, {ext}
    #[serde(default = "default_test_naming")]
    pub tests: String,
}

fn default_code_naming() -> String {
    "{spec_id}.{ext}".to_string()
}

fn default_test_naming() -> String {
    "{spec_id}_test.{ext}".to_string()
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            code: default_code_naming(),
            tests: default_test_naming(),
        }
    }
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationConfig {
    /// Require unique spec IDs across entire project
    #[serde(default = "default_true")]
    pub require_unique_ids: bool,

    /// Require descriptions on specs (warn if missing)
    #[serde(default)]
    pub require_descriptions: bool,

    /// Maximum rules per spec (warn if exceeded)
    #[serde(default = "default_max_rules")]
    pub max_rules_per_spec: usize,
}

fn default_max_rules() -> usize {
    50
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            require_unique_ids: true,
            require_descriptions: false,
            max_rules_per_spec: 50,
        }
    }
}

/// Local configuration in an imacs folder (merges with root defaults)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocalConfig {
    /// Override target languages
    pub targets: Option<Vec<Target>>,

    /// Override auto-format setting
    pub auto_format: Option<bool>,

    /// Override naming conventions
    pub naming: Option<NamingConfig>,
}

/// Merged configuration for a specific imacs folder
#[derive(Debug, Clone)]
pub struct MergedConfig {
    pub targets: Vec<Target>,
    pub auto_format: bool,
    pub naming: NamingConfig,
    pub validation: ValidationConfig,
    pub spec_id_prefix: String,
}

impl ImacRoot {
    /// Load `.imacs_root` from a directory
    pub fn load_from_dir(dir: &Path) -> Result<Option<Self>> {
        let root_file = dir.join(".imacs_root");
        if !root_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&root_file).map_err(Error::Io)?;
        let root: ImacRoot = serde_yaml::from_str(&content)
            .map_err(|e| Error::Other(format!("Failed to parse .imacs_root: {}", e)))?;

        // Validate version
        if root.version != 1 {
            return Err(Error::Other(format!(
                "Unsupported .imacs_root version: {}",
                root.version
            )));
        }

        Ok(Some(root))
    }

    /// Merge with local config to produce final config
    pub fn merge(&self, local: Option<&LocalConfig>) -> MergedConfig {
        let local = local.unwrap_or(&LocalConfig {
            targets: None,
            auto_format: None,
            naming: None,
        });

        MergedConfig {
            targets: local
                .targets
                .clone()
                .unwrap_or_else(|| self.defaults.targets.clone()),
            auto_format: local
                .auto_format
                .unwrap_or(self.defaults.auto_format),
            naming: local
                .naming
                .clone()
                .unwrap_or_else(|| self.defaults.naming.clone()),
            validation: self.validation.clone(),
            spec_id_prefix: self.project.spec_id_prefix.clone(),
        }
    }
}

impl LocalConfig {
    /// Load `config.yaml` from an imacs directory
    pub fn load_from_dir(dir: &Path) -> Result<Option<Self>> {
        let config_file = dir.join("config.yaml");
        if !config_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_file).map_err(Error::Io)?;
        let config: LocalConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Other(format!("Failed to parse config.yaml: {}", e)))?;

        Ok(Some(config))
    }
}

impl MergedConfig {
    /// Apply naming pattern to generate output filename
    pub fn apply_naming(&self, spec_id: &str, lang: &Target, is_test: bool) -> String {
        let pattern = if is_test {
            &self.naming.tests
        } else {
            &self.naming.code
        };

        let ext = match lang {
            Target::Rust => "rs",
            Target::TypeScript => "ts",
            Target::Python => "py",
            Target::Go => "go",
            Target::Java => "java",
            Target::CSharp => "cs",
        };

        pattern
            .replace("{spec_id}", spec_id)
            .replace("{lang}", &format!("{:?}", lang).to_lowercase())
            .replace("{ext}", ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_pattern() {
        let config = MergedConfig {
            targets: vec![Target::Rust],
            auto_format: true,
            naming: NamingConfig::default(),
            validation: ValidationConfig::default(),
            spec_id_prefix: "".to_string(),
        };

        assert_eq!(
            config.apply_naming("login", &Target::Rust, false),
            "login.rs"
        );
        assert_eq!(
            config.apply_naming("login", &Target::Rust, true),
            "login_test.rs"
        );
        assert_eq!(
            config.apply_naming("pricing", &Target::TypeScript, false),
            "pricing.ts"
        );
    }

    #[test]
    fn test_merge_config() {
        let root = ImacRoot {
            version: 1,
            imacs_version: ">=0.1.0".to_string(),
            project: ProjectConfig {
                name: "test".to_string(),
                spec_id_prefix: "".to_string(),
            },
            defaults: DefaultsConfig {
                targets: vec![Target::Rust],
                auto_format: true,
                naming: NamingConfig::default(),
            },
            validation: ValidationConfig::default(),
        };

        let local = LocalConfig {
            targets: Some(vec![Target::Rust, Target::TypeScript]),
            auto_format: None,
            naming: None,
        };

        let merged = root.merge(Some(&local));
        assert_eq!(merged.targets.len(), 2);
        assert_eq!(merged.auto_format, true); // Inherited from root
    }
}

