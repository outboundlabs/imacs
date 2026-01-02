# IMACS

**Intelligent Model-Assisted Code Synthesis**

Spec-driven code verification, generation, and testing.

[![Crates.io](https://img.shields.io/crates/v/imacs.svg)](https://crates.io/crates/imacs)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## What is IMACS?

IMACS treats **specifications** as the source of truth for decision logic. From a single YAML spec, you can:

- ✅ **Verify** that code correctly implements all rules
- 🔄 **Generate** code in multiple languages (Rust, TypeScript, Python)
- 🧪 **Generate tests** that cover every rule and edge case
- 🔍 **Detect drift** between frontend and backend implementations
- 📊 **Analyze** existing code for complexity
- 📝 **Extract** specs from legacy code

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

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  SPEC (YAML + CEL)                                          │
│       │                                                     │
│       ├──► verify(spec, code) ──► VerificationResult        │
│       │                                                     │
│       ├──► render(spec, target) ──► Code String             │
│       │                                                     │
│       └──► generate_tests(spec, target) ──► Test String     │
│                                                             │
│  CODE                                                       │
│       │                                                     │
│       ├──► analyze(code) ──► AnalysisReport                 │
│       │                                                     │
│       ├──► extract(code) ──► ExtractedSpec                  │
│       │                                                     │
│       └──► compare(code_a, code_b) ──► DriftReport          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE)
