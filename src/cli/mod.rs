//! CLI command implementations
//!
//! This module contains all CLI command handlers, organized by category:
//! - `simple`: Basic commands (verify, render, test, analyze, extract, sexp, drift)
//! - `completeness`: Completeness analysis commands
//! - `validate`: Spec validation commands
//! - `config`: Configuration and schema commands
//! - `project`: Project management commands (init, status, regen, selfcheck)
//! - `util`: Shared utility functions

pub mod completeness;
pub mod config;
pub mod project;
pub mod simple;
pub mod util;
pub mod validate;

// Re-export all command functions for convenient access
pub use completeness::cmd_completeness;
pub use config::{cmd_config, cmd_schema};
pub use project::{cmd_init, cmd_regen, cmd_selfcheck, cmd_status};
pub use simple::{cmd_analyze, cmd_drift, cmd_extract, cmd_render, cmd_sexp, cmd_test, cmd_verify};
pub use validate::cmd_validate;
