# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.3] - 2026-01-08

### Added

- **Enhanced Code Extraction Pipeline** (All 6 Phases complete)
  - `ast_to_cel()` function for converting AST nodes to CEL expressions
  - `pattern_to_cel()` for converting match patterns to CEL (including Or patterns)
  - `Output::Expression(String)` variant for computed outputs
  - `ExtractionReport` with coverage metrics and skipped node tracking
  - `ParseDiagnostics` for tracking unknown nodes and syntax errors during parsing

- **Data Flow Tracking**
  - `ast_to_cel_with_ctx()` for variable inlining
  - `ExtractionContext.var_definitions` tracks let bindings
  - Variable substitution: `let x = a + b; if x > 10` → CEL `(a + b) > 10`

- **Loop and Control Flow Extraction**
  - ForEach loops with early return → CEL `collection.exists(item, condition)`
  - Try/Catch blocks extract with `_error` condition for error handling paths
  - While loops properly skipped with diagnostic

- **Parsing Improvements** (Phase 6)
  - ERROR nodes from tree-sitter → `AstNode::SyntaxError` with source text
  - `macro_invocation` → `AstNode::MacroCall` (reports as unsupported in CEL)
  - `reference_expression` → `AstNode::Ref` (dereferenced in CEL: `&x` → `x`)
  - `type_cast_expression` → `AstNode::Cast` (annotated in CEL: `x as i32` → `x/*as i32*/`)

- **CLI Improvements**
  - `--sexp` flag for raw S-expression AST output
  - `--diagnostics` flag for extraction diagnostics
  - `detect_language()` for automatic language detection
  - Orchestrator rendering now automatically loads referenced specs from filesystem

- **Suite Gap Detection** (was stub, now implemented)
  - `analyze_suite_gaps_exhaustive()` finds input combinations not covered by ANY spec
  - `analyze_suite_gaps_incremental()` pairwise analysis for specs with shared variables
  - Reports gaps with CEL conditions and affected spec IDs

- **Completeness Improvements**
  - Branch coverage analysis for orchestrators (was hardcoded true)
  - Duplicate rule confidence calculation based on predicate overlap similarity (was hardcoded 0.8)

- **Test Coverage**
  - 6 new tests for suite gap detection (exhaustive, incremental, defaults, shared variables)
  - Total test count: 280+ tests passing

### Changed

- Extraction now uses CEL `when` clauses instead of structured `conditions`
- `extract()` now uses context-aware extraction by default (enables variable tracking)
- Field access, function calls, and enum patterns now extracted correctly

### Fixed

- Field access like `user.verified` now produces CEL `user.verified` (was dropped)
- Function calls like `is_valid(x)` now produce CEL `is_valid(x)` (was dropped)
- Enum patterns like `Status::Active | Status::Pending` → CEL `status in ["Active", "Pending"]`
- Computed outputs like `return calculate(x)` → `Output::Expression("calculate(x)")`
- Syntax errors no longer silently ignored (now tracked with source location)
- Suite gap detection no longer returns empty results (was stub)
- Orchestrator branch coverage verification now works correctly (was stub)
- Duplicate rule detection confidence now reflects actual similarity (was hardcoded)
- Orchestrator rendering loads referenced specs from filesystem (was empty HashMap)
- Removed ~170 lines of dead code from extraction module (old pre-context API)

## [0.0.2] - 2026-01-08

### Changed

- Updated all external crate versions

## [0.0.1] - 2026-01-04

### Added

- **Core Specification System**
  - YAML spec parsing with JSON Schema validation
  - CEL (Common Expression Language) conditions with compilation to target languages
  - Support for inputs, outputs, rules, and default values
  - Spec hashing for change detection

- **Code Generation** (6 languages)
  - Rust with `prettyplease` formatting
  - TypeScript with optional `prettier` formatting
  - Python with optional `black`/`ruff` formatting
  - Go with optional `gofmt` formatting
  - Java using `genco`
  - C# using `genco`

- **Code Verification**
  - Verify existing code against specifications
  - Tree-sitter based AST parsing for all supported languages
  - Structural matching of conditions and outputs

- **Test Generation**
  - Generate test cases from specs for all target languages
  - Boundary value analysis
  - Edge case coverage

- **Completeness Analysis**
  - Predicate extraction from CEL expressions
  - Truth table analysis for gap detection
  - Overlap detection for conflicting rules
  - Espresso-style Boolean minimization (EXPAND, REDUCE, IRREDUNDANT)
  - CEL to Cube conversion bridge
  - Suite analysis for analyzing multiple specs together
  - Cross-spec collision detection (same variable names with different meanings)
  - Duplicate rule detection across specs
  - Relationship detection (chains, merge opportunities)
  - Suite gap detection (missing cases across entire spec suite)
  - Orchestrator-aware suite analysis (automatically discovers referenced specs)
  - Complexity reporting for spec suites
  - Actionable suggestions for fixing suite issues

- **Spec Validation**
  - Validation for impossible/invalid spec conditions
  - Detection of contradictory rules (same condition, different outputs, no priority)
  - Detection of unsatisfiable conditions (can never be true)
  - Detection of tautology conditions (always match, not marked as default)
  - Detection of dead rules (covered by earlier rules)
  - Type mismatch detection in CEL expressions
  - Structured fix generation with confidence levels (High/Medium/Low)
  - Automatic fix application with `--fix` flag
  - Dry-run mode with `--dry-run` flag
  - Selective fix application with `--all` flag

- **Fix Application System**
  - Structured fix operations: UpdateRule, DeleteRule, AddPriority, RenameVariable, UpdateExpression
  - Confidence-based filtering (High/Medium/Low)
  - Fix application to spec objects and YAML files
  - Error tracking and reporting for failed fixes

- **JSON Output Support**
  - `--json` flag for machine-readable output across all commands
  - JSON Schema generation via `imacs schema` command
  - Structured output for verification, analysis, extraction, drift, completeness, and validation
  - LLM-friendly format for integration with AI tools

- **Project Conventions & Configuration**
  - Hierarchical `imacs/` folder structure (supports both `imacs/` and `.imacs/`)
  - Root marker (`.imacs_root`) with version locking and project-wide defaults
  - Config inheritance (child folders inherit root defaults)
  - Local `config.yaml` files for folder-specific overrides
  - Staleness detection via hash-based tracking in `.imacs_meta.yaml`
  - Unique spec ID validation across entire project
  - Multiple target languages per folder
  - Customizable naming patterns for generated files
  - `imacs init [--root]` command to initialize project structure
  - `imacs regen [--all] [--force]` command with config-aware regeneration
  - `imacs status [--json]` command for project status

- **Orchestrator Support**
  - Workflow definitions with sequential and parallel steps
  - Control flow: conditionals, loops, gates
  - Try/catch/finally error handling
  - Complexity analysis with cyclomatic complexity calculation

- **Additional Features**
  - Drift detection between spec and implementation
  - Spec extraction from existing code
  - External formatter integration with graceful fallbacks
  - CLI tool for all operations
  - Enhanced CLI with improved error messages and actionable suggestions

### Changed

- Completeness analysis now returns exit code 1 for incomplete specs
- Validation report includes fixes array for structured fix data
- Suite analysis automatically discovers orchestrator-referenced specs

### Documentation

- Comprehensive lib.rs module documentation
- Usage examples in README
- Spec vs Orchestrator guidelines
- Project conventions documentation

## [Unreleased]

[Unreleased]: https://github.com/anthropics/imacs/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/anthropics/imacs/releases/tag/v0.0.3
[0.0.2]: https://github.com/anthropics/imacs/releases/tag/v0.0.2
[0.0.1]: https://github.com/anthropics/imacs/releases/tag/v0.0.1
