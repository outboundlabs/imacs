//! Completeness Analysis for Decision Tables
//!
//! This module analyzes specs to find:
//! - Missing cases (uncovered input combinations)
//! - Overlapping rules (conflicts)
//! - Minimization opportunities
//!
//! The analysis decomposes CEL expressions into atomic predicates,
//! then uses truth table analysis to find gaps.
//!
//! ## Submodules
//!
//! - `predicates` - CEL → atomic predicate extraction
//! - `analysis` - Completeness checking and gap detection
//! - `espresso` - Heuristic Boolean minimization (Espresso algorithm)
//!
//! ## Example
//!
//! ```ignore
//! use imacs::completeness::analyze_completeness;
//!
//! let report = analyze_completeness(&spec);
//! if !report.is_complete {
//!     for case in &report.missing_cases {
//!         // LLM tool uses case.cel_conditions to formulate questions
//!         println!("Missing: {:?}", case.cel_conditions);
//!     }
//! }
//! ```

mod predicates;
mod analysis;
pub mod espresso;
mod adapter;

pub use predicates::{Predicate, PredicateSet, extract_predicates};
pub use analysis::{analyze_completeness, IncompletenessReport, MissingCase, RuleOverlap, PredicateValue, PredicateInfo};

// Re-export key espresso types
pub use espresso::{Cover, Cube, CubeValue, espresso as espresso_minimize, EspressoOptions};

// Re-export adapter functions
pub use adapter::{rules_to_cover, expression_to_cube, cover_to_cel, cube_to_cel, minimize_rules};
