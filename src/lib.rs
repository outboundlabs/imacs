// Production-quality lints
#![warn(
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::print_stderr
)]
// Deny truly dangerous patterns
#![deny(clippy::mem_forget)]
// Allow common patterns in library code
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

//! # IMACS — Intelligent Model-Assisted Code Synthesis
//!
//! Spec-driven code verification, generation, and testing.
//!
//! ## Core Concept
//!
//! IMACS treats **specifications** as the source of truth. A spec defines
//! decision logic as a set of rules. From this single spec, IMACS can:
//!
//! - **Verify** that code correctly implements all rules
//! - **Generate** code in multiple languages (Rust, TypeScript, Python)
//! - **Generate tests** that cover every rule and edge case
//! - **Detect drift** between implementations (e.g., frontend vs backend)
//! - **Analyze** existing code for complexity and issues
//! - **Extract** specs from existing code
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use imacs::{Spec, verify, render, generate_tests, Target};
//!
//! // Parse a spec
//! let spec = Spec::from_yaml(r#"
//!   id: login_check
//!   inputs:
//!     - name: rate_exceeded
//!       type: bool
//!     - name: valid_creds
//!       type: bool
//!   outputs:
//!     - name: status
//!       type: int
//!   rules:
//!     - id: R1
//!       when: "rate_exceeded"
//!       then: 429
//!     - id: R2
//!       when: "!rate_exceeded && !valid_creds"
//!       then: 401
//!     - id: R3
//!       when: "!rate_exceeded && valid_creds"
//!       then: 200
//! "#)?;
//!
//! // Generate code
//! let rust_code = render(&spec, Target::Rust);
//! let ts_code = render(&spec, Target::TypeScript);
//!
//! // Generate tests
//! let tests = generate_tests(&spec, Target::Rust);
//!
//! // Verify existing code against spec
//! let code_ast = imacs::parse_rust(&existing_code)?;
//! let result = verify(&spec, &code_ast);
//! if result.passed {
//!     println!("✓ All {} rules verified", result.coverage.covered);
//! } else {
//!     for gap in result.gaps() {
//!         println!("✗ Missing: {}", gap);
//!     }
//! }
//! ```
//!
//! ## Spec Format
//!
//! Specs use YAML with CEL (Common Expression Language) for conditions:
//!
//! ```yaml
//! id: checkout_validation
//! inputs:
//!   - name: cart_total
//!     type: float
//!   - name: user_verified
//!     type: bool
//! outputs:
//!   - name: result
//!     type: string
//! rules:
//!   - id: R1
//!     when: "cart_total > 10000 && !user_verified"
//!     then: "requires_review"
//!   - id: R2
//!     when: "cart_total > 10000 && user_verified"
//!     then: "approved"
//!   - id: R3
//!     when: "cart_total <= 10000"
//!     then: "approved"
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                                                             │
//! │  SPEC (YAML + CEL)                                          │
//! │       │                                                     │
//! │       ├──► verify(spec, code) ──► VerificationResult        │
//! │       │                                                     │
//! │       ├──► render(spec, target) ──► Code String             │
//! │       │                                                     │
//! │       └──► generate_tests(spec, target) ──► Test String     │
//! │                                                             │
//! │  CODE                                                       │
//! │       │                                                     │
//! │       ├──► analyze(code) ──► AnalysisReport                 │
//! │       │                                                     │
//! │       ├──► extract(code) ──► ExtractedSpec                  │
//! │       │                                                     │
//! │       └──► compare(code_a, code_b) ──► DriftReport          │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Spec vs Orchestrator Guidelines
//!
//! IMACS uses two logic types with distinct purposes:
//!
//! | Type | Structure | Use Case | Verifiability |
//! |------|-----------|----------|---------------|
//! | **Spec** | Decision table (pure functions) | Business rules, validation | ✓ Complete |
//! | **Orchestrator** | Workflow (control flow) | Process coordination | Partial |
//!
//! ### When to Use Specs
//!
//! Use a **Spec** when the logic:
//! - Is a pure function (same inputs → same outputs)
//! - Can be expressed as if/else or match rules
//! - Has a finite, enumerable input space
//! - Needs verification and test generation
//!
//! **Good Spec examples:**
//! - Status code determination based on conditions
//! - Pricing tier selection
//! - Permission checks
//! - Validation rules
//!
//! ### When to Use Orchestrators
//!
//! Use an **Orchestrator** when the logic:
//! - Coordinates multiple Spec calls
//! - Requires sequential/parallel execution
//! - Involves I/O, side effects, or external services
//! - Needs error handling, retries, or timeouts
//!
//! **Good Orchestrator examples:**
//! - Order processing workflow
//! - Multi-step validation pipeline
//! - Data transformation chain
//!
//! ### Anti-Patterns to Avoid
//!
//! 1. **Business logic in Orchestrators**: Put decision logic in Specs, not Compute steps
//! 2. **Orchestrator loops for rules**: Use Spec rules instead of Loop/Branch for decisions
//! 3. **Specs with I/O**: Specs should be pure; use Orchestrators for external calls
//! 4. **Skipping Specs**: Every Branch/Loop should eventually call a Spec
//!
//! Use `Orchestrator::analyze_complexity()` to detect complexity warnings.

// Core modules (Layer 0: hand-crafted bootstrap)
pub mod ast;
pub mod cel;
pub mod config;
pub mod error;
pub mod meta;
pub mod project;
pub mod spec;

// Operations (Layer 0: hand-crafted)
pub mod analyze;
pub mod drift;
pub mod extract;
pub mod format;
pub mod orchestrate;
pub mod parse;
pub mod render;
pub mod testgen;
pub mod testgen_orchestrate;
pub mod verify;

// Completeness analysis (Phase 5)
pub mod completeness;

// Generated from specs (Layer 1: dogfooding)
// Regenerate with: imacs regen
// Verify with: imacs selfcheck
pub mod generated;

// Re-exports
pub use analyze::{analyze, AnalysisReport, Analyzer, FunctionMetrics, Issue, Severity};
pub use ast::{
    AstNode, BinaryOp, CodeAst, Function, LiteralValue, MatchArm, Pattern, Span, UnaryOp,
};
pub use cel::Target;
pub use cel::{CelCompiler, CelExpr};
pub use drift::{compare, Difference, DriftDetector, DriftReport, DriftStatus};
pub use error::{Error, Result};
pub use extract::{extract, Confidence, ExtractedSpec, Extractor};
pub use parse::parse_rust;
pub use render::{render, Renderer};
pub use spec::{Condition, ConditionOp, ConditionValue, Output, Rule, Spec, VarType, Variable};
pub use testgen::{generate_tests, TestConfig, TestGenerator, TestMode};
pub use verify::{verify, Coverage, CoverageGap, VerificationResult, Verifier};

// Code formatting
pub use format::{
    available_formatters, format_code, format_go, format_python, format_rust, format_typescript,
    is_formatter_available, FormatError,
};

// Completeness analysis
pub use completeness::{
    analyze_completeness,
    analyze_suite,
    compose,
    cover_to_cel,
    cube_to_cel,
    decompose,
    espresso_minimize,
    expression_to_cube,
    extract_predicates,
    extract_spec_from_orchestrator,
    // Refactoring APIs
    minimize,
    minimize_rules,
    rules_to_cover,
    validate_spec,
    AnalysisMode,
    ChainDefinition,
    ComparisonOp,
    ComposedSpec,
    // Espresso integration
    Cover,
    Cube,
    CubeValue,
    DecompositionResult,
    EspressoOptions,
    IncompletenessReport,
    MinimizedSpec,
    MissingCase,
    OrchestratorExtractionResult,
    OutputToInputMapping,
    Predicate,
    PredicateInfo,
    PredicateSet,
    PredicateValue,
    RuleOverlap,
    SpecResult,
    StringOpKind,
    SuiteAnalysisResult,
    SuiteGap,
    Transformation,
    TransformationKind,
    ValidationReport,
    VariableGroup,
};
// Re-export predicate LiteralValue under a distinct name to avoid conflict with ast::LiteralValue
pub use completeness::LiteralValue as PredicateLiteralValue;

// Orchestration
pub use orchestrate::{
    calculate_complexity, count_steps, render_orchestrator, ChainStep, ComplexityReport,
    Orchestrator, OrchestratorInput, OrchestratorOutput,
};
pub use testgen_orchestrate::{
    generate_orchestrator_tests, verify_orchestrator, OrchestratorTests, OrchestratorVerification,
};

// Project management
pub use config::{ImacRoot, LocalConfig, MergedConfig, ProjectConfig, ValidationConfig};
pub use meta::{create_meta, find_stale_specs, ImacMeta};
pub use project::{
    discover_all_imacs, discover_generated_dir, discover_specs_dir, find_root,
    get_generated_dir, list_specs, load_project_structure, validate_unique_ids, ImacFolder,
    ProjectStructure,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
