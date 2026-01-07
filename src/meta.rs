//! Metadata tracking for staleness detection
//!
//! Stores hashes of spec files in `.imacs_meta.yaml` to detect when
//! generated code needs regeneration.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Metadata file stored in generated/ directory
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImacMeta {
    /// Hash of each spec file (relative path -> SHA256)
    pub spec_hashes: HashMap<String, String>,

    /// Generated files (spec_id -> list of generated file paths)
    /// Used for orphaned file detection with --clean
    #[serde(default)]
    pub generated_files: HashMap<String, Vec<String>>,

    /// When this metadata was generated (ISO 8601 string)
    #[serde(with = "chrono::serde::ts_seconds")]
    #[schemars(with = "String")]
    pub generated_at: DateTime<Utc>,

    /// IMACS tool version that generated this
    pub tool_version: String,
}

impl ImacMeta {
    /// Load metadata from generated directory
    pub fn load_from_dir(generated_dir: &Path) -> Result<Option<Self>> {
        let meta_file = generated_dir.join(".imacs_meta.yaml");
        if !meta_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&meta_file).map_err(Error::Io)?;
        let meta: ImacMeta = serde_norway::from_str(&content)
            .map_err(|e| Error::Other(format!("Failed to parse .imacs_meta.yaml: {}", e)))?;

        Ok(Some(meta))
    }

    /// Save metadata to generated directory
    pub fn save_to_dir(&self, generated_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(generated_dir).map_err(Error::Io)?;
        let meta_file = generated_dir.join(".imacs_meta.yaml");

        // Add header comment warning
        let mut content = String::from(
            "# AUTO-GENERATED - DO NOT EDIT\n\
             # This file tracks spec file hashes for staleness detection\n\
             # Regenerated automatically by 'imacs regen'\n\n",
        );

        let yaml = serde_norway::to_string(self)
            .map_err(|e| Error::Other(format!("Failed to serialize metadata: {}", e)))?;
        content.push_str(&yaml);

        std::fs::write(&meta_file, content).map_err(Error::Io)?;
        Ok(())
    }

    /// Check if a spec file is stale (changed since last generation)
    pub fn is_stale(&self, spec_path: &Path, imacs_dir: &Path) -> bool {
        let relative_path = spec_path
            .strip_prefix(imacs_dir)
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.replace('\\', "/")); // Normalize path separators

        let relative_path = match relative_path {
            Some(p) => p,
            None => return true, // Can't determine relative path, assume stale
        };

        let current_hash = compute_file_hash(spec_path).unwrap_or_default();
        let stored_hash = self.spec_hashes.get(&relative_path);

        stored_hash.map(|h| h != &current_hash).unwrap_or(true)
    }

    /// Update hash for a spec file
    pub fn update_hash(&mut self, spec_path: &Path, imacs_dir: &Path) -> Result<()> {
        let relative_path = spec_path
            .strip_prefix(imacs_dir)
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.replace('\\', "/"))
            .ok_or_else(|| Error::Other("Cannot compute relative path".to_string()))?;

        let hash = compute_file_hash(spec_path)?;
        self.spec_hashes.insert(relative_path, hash);
        Ok(())
    }

    /// Track a generated file for a spec
    pub fn track_generated_file(&mut self, spec_id: &str, file_path: &str) {
        let files = self.generated_files.entry(spec_id.to_string()).or_default();
        if !files.contains(&file_path.to_string()) {
            files.push(file_path.to_string());
        }
    }

    /// Remove a spec's generated files tracking (when spec is deleted)
    pub fn remove_spec(&mut self, spec_id: &str) -> Option<Vec<String>> {
        self.generated_files.remove(spec_id)
    }

    /// Get all tracked generated files
    pub fn all_tracked_files(&self) -> Vec<String> {
        self.generated_files.values().flatten().cloned().collect()
    }

    /// Find orphaned files in the generated directory
    /// (files that exist but aren't tracked for any current spec)
    pub fn find_orphaned_files(
        &self,
        generated_dir: &Path,
        current_spec_ids: &[String],
    ) -> Result<Vec<PathBuf>> {
        let mut orphaned = Vec::new();

        // Get all files we're still tracking for current specs
        let tracked: std::collections::HashSet<String> = current_spec_ids
            .iter()
            .filter_map(|id| self.generated_files.get(id))
            .flatten()
            .cloned()
            .collect();

        // Find files in generated dir that aren't tracked
        if generated_dir.exists() {
            for entry in std::fs::read_dir(generated_dir).map_err(Error::Io)? {
                let entry = entry.map_err(Error::Io)?;
                let path = entry.path();

                // Skip metadata and hidden files
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name == ".imacs_meta.yaml" {
                        continue;
                    }
                }

                if path.is_file() {
                    let relative = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());

                    if let Some(filename) = relative {
                        if !tracked.contains(&filename) {
                            orphaned.push(path);
                        }
                    }
                }
            }
        }

        // Also check for specs that were removed entirely
        for (spec_id, files) in &self.generated_files {
            if !current_spec_ids.contains(spec_id) {
                for file in files {
                    let full_path = generated_dir.join(file);
                    if full_path.exists() && !orphaned.contains(&full_path) {
                        orphaned.push(full_path);
                    }
                }
            }
        }

        Ok(orphaned)
    }
}

/// Compute SHA256 hash of a file
fn compute_file_hash(path: &Path) -> Result<String> {
    let content = std::fs::read(path).map_err(Error::Io)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Create new metadata with current timestamp
pub fn create_meta() -> ImacMeta {
    ImacMeta {
        spec_hashes: HashMap::new(),
        generated_files: HashMap::new(),
        generated_at: Utc::now(),
        tool_version: crate::VERSION.to_string(),
    }
}

/// Find stale specs in an imacs directory
pub fn find_stale_specs(imacs_dir: &Path, generated_dir: &Path) -> Result<Vec<PathBuf>> {
    let meta = ImacMeta::load_from_dir(generated_dir)?;
    let mut stale = Vec::new();

    // If no metadata exists, all specs are considered stale
    if meta.is_none() {
        return collect_all_specs(imacs_dir);
    }

    let meta = meta.unwrap();

    // Check each spec file
    let entries = std::fs::read_dir(imacs_dir).map_err(Error::Io)?;
    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    // Skip config files
                    if path.file_name().and_then(|n| n.to_str()) == Some("config.yaml") {
                        continue;
                    }
                    if path.file_name().and_then(|n| n.to_str()) == Some(".imacs_root") {
                        continue;
                    }

                    if meta.is_stale(&path, imacs_dir) {
                        stale.push(path);
                    }
                }
            }
        }
    }

    Ok(stale)
}

/// Collect all spec files in a directory
fn collect_all_specs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut specs = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(Error::Io)?;

    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    // Skip config files
                    if path.file_name().and_then(|n| n.to_str()) == Some("config.yaml") {
                        continue;
                    }
                    if path.file_name().and_then(|n| n.to_str()) == Some(".imacs_root") {
                        continue;
                    }
                    specs.push(path);
                }
            }
        }
    }

    Ok(specs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hash_computation() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.yaml");
        fs::write(&file, "id: test").unwrap();

        let hash1 = compute_file_hash(&file).unwrap();
        let hash2 = compute_file_hash(&file).unwrap();
        assert_eq!(hash1, hash2); // Same content = same hash

        fs::write(&file, "id: changed").unwrap();
        let hash3 = compute_file_hash(&file).unwrap();
        assert_ne!(hash1, hash3); // Different content = different hash
    }

    #[test]
    fn test_staleness_detection() {
        let temp = TempDir::new().unwrap();
        let imacs_dir = temp.path().join("imacs");
        let generated_dir = temp.path().join("generated");
        fs::create_dir_all(&imacs_dir).unwrap();
        fs::create_dir_all(&generated_dir).unwrap();

        let spec_file = imacs_dir.join("test.yaml");
        fs::write(&spec_file, "id: test").unwrap();

        // No metadata = all stale
        let stale = find_stale_specs(&imacs_dir, &generated_dir).unwrap();
        assert_eq!(stale.len(), 1);

        // Create metadata
        let mut meta = create_meta();
        meta.update_hash(&spec_file, &imacs_dir).unwrap();
        meta.save_to_dir(&generated_dir).unwrap();

        // Should not be stale now
        let stale = find_stale_specs(&imacs_dir, &generated_dir).unwrap();
        assert_eq!(stale.len(), 0);

        // Change spec
        fs::write(&spec_file, "id: changed").unwrap();
        let stale = find_stale_specs(&imacs_dir, &generated_dir).unwrap();
        assert_eq!(stale.len(), 1);
    }
}
