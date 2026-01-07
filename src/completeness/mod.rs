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
//! ```rust
//! use imacs::{Spec, analyze_completeness};
//!
//! let spec = Spec::from_yaml(r#"
//!   id: example
//!   inputs:
//!     - name: a
//!       type: bool
//!     - name: b
//!       type: bool
//!   outputs:
//!     - name: result
//!       type: int
//!   rules:
//!     - id: R1
//!       when: "a && b"
//!       then: 1
//! "#).unwrap();
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
mod collision;
mod duplicate;
pub mod espresso;
mod fix;
mod orchestrator_suite;
mod predicates;
mod refactor;
mod relationship;
mod suggestions;
mod suite;
mod validate;
mod variable_match;

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

// Re-export suite analysis APIs
pub use collision::{detect_collisions, Collision, CollisionType, VariableOccurrence};
pub use duplicate::{detect_duplicates, Duplicate, RuleRef};
pub use fix::{apply_fixes, apply_fixes_to_yaml, FixApplicationResult};
pub use orchestrator_suite::{
    analyze_directory_with_orchestrators, analyze_orchestrator_suite, DirectorySuiteResult,
    MappingIssue, MappingIssueType, OrchestratorSuiteResult,
};
pub use relationship::{
    detect_relationships, OutputInputMapping, RelationshipDetails, RelationshipType,
    SpecRelationship,
};
pub use suggestions::{generate_suggestions, SuggestedFix, Suggestion, SuggestionCategory};
pub use suite::{
    analyze_suite, AnalysisMode, ComplexityReport, SpecResult, SuiteAnalysisResult, SuiteGap,
};
pub use validate::{
    validate_spec, FixConfidence, FixOperation, IssueType, Severity, SpecFix, ValidationIssue,
    ValidationReport,
};
pub use variable_match::{match_variables, MatchType, VariableMatch, VariableMatchResult};
