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

mod adapter;
mod analysis;
pub mod espresso;
mod predicates;
mod refactor;

pub use analysis::{
    analyze_completeness, IncompletenessReport, MissingCase, PredicateInfo, PredicateValue,
    RuleOverlap,
};
pub use predicates::{
    extract_predicates, ComparisonOp, LiteralValue, Predicate, PredicateSet, StringOpKind,
};

// Re-export key espresso types
pub use espresso::{espresso as espresso_minimize, Cover, Cube, CubeValue, EspressoOptions};

// Re-export adapter functions
pub use adapter::{cover_to_cel, cube_to_cel, expression_to_cube, minimize_rules, rules_to_cover};

// Re-export refactoring APIs
pub use refactor::{
    // Compose API
    compose,
    // Decompose API
    decompose,
    // Extract from Orchestrator API
    extract_spec_from_orchestrator,
    // Minimize API
    minimize,
    ChainDefinition,
    ComposedSpec,
    DecompositionResult,
    MinimizedSpec,
    OrchestratorExtractionResult,
    OutputToInputMapping,
    Transformation,
    TransformationKind,
    VariableGroup,
};
