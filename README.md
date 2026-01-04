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

## Getting Started

IMACS can be used in two ways:
- **CLI Tool**: Install and use from the command line
- **Library**: Add as a dependency to your Rust project

### Installation

#### As a CLI Tool

```bash
cargo install imacs
```

Or build from source:

```bash
git clone https://github.com/anthropics/imacs.git
cd imacs
cargo build --release
```

#### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
imacs = "0.0.1"
```

Then use in your Rust code:

```rust
use imacs::{Spec, render, Target};

let spec = Spec::from_yaml(yaml_content)?;
let code = render(&spec, Target::Rust);
```

### Project Structure

IMACS uses a convention-based folder structure:

```
project/
├── imacs/                          # ROOT folder (or .imacs/)
│   ├── .imacs_root                 # Project root config + version lock
│   ├── common/                     # Shared specs
│   │   └── validation.yaml
│   └── example.yaml
├── services/
│   ├── auth/
│   │   ├── imacs/                  # Child folder
│   │   │   ├── config.yaml         # Optional local overrides
│   │   │   └── login.yaml
│   │   └── generated/              # Auto-generated code
│   │       ├── .imacs_meta.yaml    # Staleness tracking
│   │       └── login.rs
│   └── billing/
│       ├── imacs/
│       │   └── invoice.yaml
│       └── generated/
│           └── invoice.rs
```

#### Initialize a Project

```bash
# Create project root
imacs init --root

# Create local imacs folder (inherits root config)
cd services/auth
imacs init
```

#### Project Configuration

The `.imacs_root` file in the root `imacs/` folder defines project-wide settings:

```yaml
version: 1
imacs_version: ">=0.1.0"

project:
  name: my-project
  spec_id_prefix: ""                # Optional prefix to avoid ID collisions

defaults:
  targets: [rust, typescript]       # Languages to generate
  auto_format: true
  naming:
    code: "{spec_id}.{ext}"
    tests: "{spec_id}_test.{ext}"

validation:
  require_unique_ids: true          # Error on ID collision
  require_descriptions: false
  max_rules_per_spec: 50
```

Child folders can override defaults with `config.yaml`:

```yaml
# services/auth/imacs/config.yaml
targets: [rust]  # Override: only generate Rust for this folder
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

### Analyze Completeness

```bash
# Single spec analysis
imacs completeness login_attempt.yaml

# Suite analysis (multiple specs in a directory)
imacs completeness specs/

# JSON output for LLM integration
imacs completeness login_attempt.yaml --json
```

### Validate Spec

```bash
# Check for impossible/invalid conditions
imacs validate login_attempt.yaml

# Strict mode (treat warnings as errors)
imacs validate login_attempt.yaml --strict

# Generate and apply fixes automatically
imacs validate login_attempt.yaml --fix

# Preview fixes without applying
imacs validate login_attempt.yaml --fix --dry-run

# Apply all fixes including low-confidence ones
imacs validate login_attempt.yaml --fix --all
```

## CLI Commands

### Core Commands

| Command | Description | Options |
|---------|-------------|---------|
| `verify <spec> <code>` | Check code implements spec correctly | `--json` |
| `render <spec>` | Generate code from spec | `--lang <lang>`, `--output <file>` |
| `test <spec>` | Generate tests from spec | `--lang <lang>`, `--output <file>` |
| `analyze <code>` | Analyze code complexity | `--json` |
| `extract <code>` | Extract spec from existing code | `--json` |
| `drift <code_a> <code_b>` | Compare two implementations | `--json` |

### Analysis Commands

| Command | Description | Options |
|---------|-------------|---------|
| `completeness <spec\|dir>` | Analyze spec(s) for missing cases and overlaps | `--json`, `--full` |
| `validate <spec>` | Validate spec for impossible situations | `--strict`, `--json`, `--fix`, `--dry-run`, `--all` |
| `schema [name]` | Print JSON schema for output type | (none) |

### Utility Commands

| Command | Description |
|---------|-------------|
| `regen` | Regenerate src/generated/ from specs/ |
| `selfcheck` | Verify generated code matches specs |
| `version`, `-v` | Show version |
| `help`, `-h` | Show usage |

### Command Options

- `--lang <rust\|typescript\|python\|csharp\|java\|go>` - Target language (default: rust)
- `--output <file>` - Output file (default: stdout)
- `--json` - JSON output format (verify, analyze, extract, drift, completeness, validate)
- `--full` - Full exhaustive analysis for completeness suite mode
- `--strict` - Strict mode: treat warnings as errors (validate command)
- `--fix` - Apply fixes automatically (validate command)
- `--dry-run` - Preview changes without applying (validate command)
- `--all` - Apply all fixes including low-confidence ones (validate command)

### Examples

```bash
# Initialize project
imacs init --root                    # Create root imacs/ folder
imacs init                           # Create local imacs/ folder

# Generate code
imacs render login_attempt.yaml --lang rust
imacs regen                          # Regenerate current folder
imacs regen --all                    # Regenerate entire project
imacs regen --force                  # Force regenerate (ignore staleness)

# Check status
imacs status                         # Show project status
imacs status --json                  # JSON output

# Generate tests
imacs test login_attempt.yaml --lang typescript --output tests/login.test.ts

# Verify implementation
imacs verify login_attempt.yaml src/login.rs --json

# Analyze completeness
imacs completeness specs/ --full

# Validate and auto-fix
imacs validate login_attempt.yaml --fix

# Get JSON schema
imacs schema validate
```

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
    println!("Rules {} and {} overlap", overlap.rule_ids);
}
```

### What It Detects

| Analysis | Description |
|----------|-------------|
| **Missing cases** | Input combinations with no matching rule |
| **Overlapping rules** | Multiple rules match the same input |
| **Minimization opportunities** | Redundant rules that can be simplified |

### Suite Analysis

Analyze multiple specs together to find cross-cutting issues:

```bash
imacs completeness specs/ --full
```

Detects:
- **Collisions**: Same variable names with different meanings/types across specs
- **Duplicates**: Identical logic implemented in multiple specs
- **Relationships**: Chains (output of one spec is input to another) and merge opportunities
- **Suite gaps**: Missing cases across the entire spec suite

### How It Works

1. **Predicate extraction**: Parse CEL expressions into atomic boolean predicates
2. **Truth table analysis**: Enumerate all 2^n combinations
3. **Gap detection**: Find uncovered input patterns
4. **Espresso minimization**: EXPAND → REDUCE → IRREDUNDANT phases
5. **Cross-spec analysis**: Compare predicates, variables, and rules across multiple specs

## Spec Validation

IMACS can detect impossible or invalid spec conditions:

```bash
imacs validate spec.yaml
```

### What It Detects

| Issue Type | Description | Fix Confidence |
|------------|-------------|----------------|
| **Contradictory rules** | Same condition, different outputs, no priority | High |
| **Dead rules** | Covered by earlier rules, can never fire | High |
| **Tautology conditions** | Always match, not marked as default | Medium |
| **Type mismatches** | Wrong types in CEL comparisons | Medium |
| **Unsatisfiable conditions** | Can never be true | Low |

### Auto-Fix

IMACS can automatically fix many issues:

```bash
# Preview fixes
imacs validate spec.yaml --fix --dry-run

# Apply high-confidence fixes
imacs validate spec.yaml --fix

# Apply all fixes including low-confidence
imacs validate spec.yaml --fix --all
```

Fixes are structured and machine-readable, perfect for LLM integration:

```json
{
  "fixes": [
    {
      "issue_code": "V001",
      "confidence": "High",
      "operation": {
        "type": "AddPriority",
        "rule_id": "R1",
        "priority": 1
      },
      "description": "Add priority 1 to rule R1 to resolve conflict with R2"
    }
  ]
}
```

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
