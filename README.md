# IMACS

**Intelligent Model-Assisted Code Synthesis**

Spec-driven code verification, generation, and testing.

[![Crates.io](https://img.shields.io/crates/v/imacs.svg)](https://crates.io/crates/imacs)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## What is IMACS?

IMACS treats **specifications** as the source of truth for decision logic. From a single YAML spec, you can:

- ✅ **Verify** that code correctly implements all rules
- 🔄 **Generate** code in 6 languages (Rust, TypeScript, Python, Go, Java, C#)
- 🧪 **Generate tests** that cover every rule and edge case
- 🔍 **Detect drift** between frontend and backend implementations
- 📊 **Analyze** existing code for complexity
- 📝 **Extract** specs from legacy code
- 🧮 **Analyze completeness** to find missing cases and overlapping rules
- ⚡ **Minimize rules** using Espresso Boolean optimization

## Two Spec Types

IMACS supports two types of specifications:

| Type | Purpose | Complexity | Verification |
|------|---------|------------|--------------|
| **Decision Table** | Pure decision logic (if/else/match) | O(n) rules | Full |
| **Orchestrator** | Workflow composition (sequence, branch, loop) | Turing-complete | Partial |

### Decision Tables (Specs)

For pure decision logic — no side effects, no I/O:

```yaml
id: login_attempt
rules:
  - when: "rate_exceeded"
    then: 429
  - when: "!rate_exceeded && locked"
    then: 423
  - when: "!rate_exceeded && !locked && valid_creds"
    then: 200
```

### Orchestrators

For composing multiple specs into workflows:

```yaml
id: order_flow
uses: [access_level, shipping_rate]

chain:
  - step: call
    id: check_access
    spec: access_level
    inputs: { role: "role", verified: "verified" }

  - step: gate
    condition: "check_access.level >= 50"

  - step: call
    id: calc_shipping
    spec: shipping_rate
    inputs: { weight_kg: "weight_kg", zone: "zone" }
```

Orchestrator step types: `call`, `gate`, `branch`, `parallel`, `loop`, `compute`, `try`

## Quick Start

### Installation

```bash
cargo install imacs
```

### Define a Spec

```yaml
# login_attempt.yaml
id: login_attempt
name: "Login Attempt Validation"

inputs:
  - name: rate_exceeded
    type: bool
  - name: locked
    type: bool
  - name: valid_creds
    type: bool

outputs:
  - name: status
    type: int

rules:
  - id: R1
    when: "rate_exceeded"
    then: 429
    description: "Rate limited"

  - id: R2
    when: "!rate_exceeded && locked"
    then: 423
    description: "Account locked"

  - id: R3
    when: "!rate_exceeded && !locked && !valid_creds"
    then: 401
    description: "Invalid credentials"

  - id: R4
    when: "!rate_exceeded && !locked && valid_creds"
    then: 200
    description: "Success"
```

### Generate Code

```bash
# Rust
imacs render login_attempt.yaml --lang rust

# TypeScript
imacs render login_attempt.yaml --lang typescript

# Python
imacs render login_attempt.yaml --lang python
```

### Generate Tests

```bash
imacs test login_attempt.yaml --lang rust > tests/login_attempt_test.rs
```

### Verify Implementation

```bash
imacs verify login_attempt.yaml src/login_attempt.rs
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `verify <spec> <code>` | Check code implements spec correctly |
| `render <spec> [--lang]` | Generate code from spec |
| `test <spec> [--lang]` | Generate tests from spec |
| `analyze <code>` | Analyze code complexity |
| `extract <code>` | Extract spec from existing code |
| `drift <code_a> <code_b>` | Compare two implementations |

## Library Usage

```rust
use imacs::{Spec, verify, render, generate_tests, Target};

// Parse spec
let spec = Spec::from_yaml(include_str!("login_attempt.yaml"))?;

// Generate Rust code
let rust_code = render(&spec, Target::Rust);

// Generate tests
let tests = generate_tests(&spec, Target::Rust);

// Verify existing code
let code_ast = imacs::parse_rust(&existing_code)?;
let result = verify(&spec, &code_ast);

if result.passed {
    println!("✓ All {} rules verified", result.coverage.covered);
} else {
    for gap in &result.gaps {
        println!("✗ Missing: {} - {}", gap.rule_id, gap.suggestion);
    }
}
```

## Spec Format

Specs use YAML with CEL (Common Expression Language) for conditions:

```yaml
id: checkout_validation
inputs:
  - name: cart_total
    type: float
  - name: user_verified
    type: bool
outputs:
  - name: result
    type: string
rules:
  - id: R1
    when: "cart_total > 10000 && !user_verified"
    then: "requires_review"
  - id: R2
    when: "cart_total > 10000 && user_verified"
    then: "approved"
  - id: R3
    when: "cart_total <= 10000"
    then: "approved"
```

### Supported Types

- `bool` - Boolean
- `int` - Integer
- `float` - Floating point
- `string` - String
- `enum` - Enumeration with specific values
- `list` - List/array
- `object` - Key-value map

### CEL Expressions

IMACS uses [CEL](https://cel.dev/) for condition expressions:

```yaml
# Comparisons
when: "amount > 1000"
when: "status == 'active'"

# Logical operators
when: "verified && amount > 100"
when: "locked || suspended"
when: "!rate_exceeded"

# Membership
when: "status in ['pending', 'review']"

# String functions
when: "email.endsWith('@company.com')"
```

## Use Cases

### 1. Verified AI Code Generation

Use IMACS specs with AI coding tools to ensure generated code is correct:

```
Human: Generate code for this spec: [paste spec]
AI: [generates code]
Human: imacs verify spec.yaml generated.rs
✓ All 4 rules verified
```

### 2. Frontend/Backend Sync

Keep frontend and backend decision logic in sync:

```bash
imacs drift src/backend/auth.rs src/frontend/auth.ts
# Detects when implementations diverge
```

### 3. Legacy Code Documentation

Extract specs from existing code to document behavior:

```bash
imacs extract src/legacy_validator.rs > validator.yaml
# Creates spec from existing code with confidence scores
```

### 4. Test Generation

Generate comprehensive tests from specs:

```bash
imacs test payment.yaml --lang python > test_payment.py
# Creates: rule tests, exhaustive tests, boundary tests, property tests
```

## Completeness Analysis

IMACS uses the **Espresso algorithm** (same as used in hardware logic optimization) to analyze decision tables:

```rust
use imacs::{analyze_completeness, extract_predicates, minimize_rules};

let spec = Spec::from_yaml(yaml)?;
let report = analyze_completeness(&spec);

if !report.is_complete {
    for case in &report.missing_cases {
        // LLM tool uses these to ask clarifying questions
        println!("Missing case: {:?}", case.cel_conditions);
    }
}

for overlap in &report.overlaps {
    println!("Rules {} and {} overlap", overlap.rule_a, overlap.rule_b);
}
```

### What It Detects

| Analysis | Description |
|----------|-------------|
| **Missing cases** | Input combinations with no matching rule |
| **Overlapping rules** | Multiple rules match the same input |
| **Minimization opportunities** | Redundant rules that can be simplified |

### How It Works

1. **Predicate extraction**: Parse CEL expressions into atomic boolean predicates
2. **Truth table analysis**: Enumerate all 2^n combinations
3. **Gap detection**: Find uncovered input patterns
4. **Espresso minimization**: EXPAND → REDUCE → IRREDUNDANT phases

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  DECISION TABLE (YAML + CEL)                                │
│       │                                                     │
│       ├──► render(spec, target) ──► Code String             │
│       ├──► generate_tests(spec) ──► Test String             │
│       ├──► verify(spec, code) ──► VerificationResult        │
│       └──► analyze_completeness(spec) ──► IncompletenessReport│
│                                                             │
│  ORCHESTRATOR (YAML)                                        │
│       │                                                     │
│       └──► render_orchestrator(orch, specs) ──► Code String │
│                                                             │
│  CODE                                                       │
│       │                                                     │
│       ├──► analyze(code) ──► AnalysisReport                 │
│       ├──► extract(code) ──► ExtractedSpec                  │
│       └──► compare(code_a, code_b) ──► DriftReport          │
│                                                             │
│  COMPLETENESS (Espresso)                                    │
│       │                                                     │
│       ├──► extract_predicates(spec) ──► PredicateSet        │
│       ├──► rules_to_cover(rules) ──► Espresso Cover         │
│       └──► minimize_rules(rules) ──► Simplified CEL         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE)
