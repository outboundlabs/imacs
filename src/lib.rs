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

// Core modules (Layer 0: hand-crafted bootstrap)
pub mod ast;
pub mod cel;
pub mod error;
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
pub use cel::{CelCompiler, CelExpr};
pub use drift::{compare, Difference, DriftDetector, DriftReport, DriftStatus};
pub use error::{Error, Result};
pub use extract::{extract, Confidence, ExtractedSpec, Extractor};
pub use parse::parse_rust;
pub use cel::Target;
pub use render::{render, Renderer};
pub use spec::{Condition, ConditionOp, ConditionValue, Output, Rule, Spec, VarType, Variable};
pub use testgen::{generate_tests, TestConfig, TestGenerator, TestMode};
pub use verify::{verify, Coverage, CoverageGap, VerificationResult, Verifier};

// Code formatting
pub use format::{format_code, format_rust, FormatError};

// Completeness analysis
pub use completeness::{
    analyze_completeness, extract_predicates, IncompletenessReport, MissingCase,
    Predicate, PredicateInfo, PredicateSet, PredicateValue, RuleOverlap,
    // Espresso integration
    Cover, Cube, CubeValue, espresso_minimize, EspressoOptions,
    rules_to_cover, expression_to_cube, cover_to_cel, cube_to_cel, minimize_rules,
};

// Orchestration
pub use orchestrate::{
    render_orchestrator, ChainStep, Orchestrator, OrchestratorInput, OrchestratorOutput,
};
pub use testgen_orchestrate::{
    generate_orchestrator_tests, verify_orchestrator, OrchestratorTests, OrchestratorVerification,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
