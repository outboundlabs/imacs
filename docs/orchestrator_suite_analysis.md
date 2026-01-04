# Orchestrator-Aware Suite Analysis

## Overview

When analyzing a directory containing both specs and orchestrators, IMACS automatically discovers the relationships between them and performs comprehensive cross-spec analysis.

## How It Works

### 1. Discovery Phase

When you run:
```bash
imacs completeness specs/
```

IMACS:
1. **Scans the directory** for all YAML files
2. **Detects orchestrators** (files with `chain:` or `uses:` fields)
3. **Extracts referenced specs** from each orchestrator using `orchestrator.referenced_specs()`

### 2. Spec References

Orchestrators reference specs in two ways:

#### Explicit References (`uses:` field)
```yaml
id: checkout_flow
uses:
  - validate_cart
  - calculate_total
  - process_payment
chain:
  - step: call
    spec: validate_cart
    ...
```

#### Implicit References (in `chain:` steps)
```yaml
id: checkout_flow
chain:
  - step: call
    id: validate
    spec: validate_cart  # <-- Referenced here
    inputs:
      items: "cart_items"
  - step: call
    id: calculate
    spec: calculate_total  # <-- And here
    inputs:
      items: "cart_items"
```

The `referenced_specs()` method collects all spec IDs from both sources.

### 3. Analysis Phase

For each orchestrator found:

1. **Load referenced specs** from the directory
2. **Analyze them as a suite** (collisions, duplicates, relationships, gaps)
3. **Check input/output mappings**:
   - Missing required inputs
   - Type mismatches
   - Unused outputs

### 4. Output

The analysis reports:

```
ORCHESTRATOR: checkout_flow

  Referenced specs: validate_cart, calculate_total, process_payment
  Found: validate_cart, calculate_total
  ⚠ Missing: process_payment

  MAPPING ISSUES:
    [validate] validate_cart:MissingInput - Required input 'cart_id' (type: String) not provided

  Suite analysis for referenced specs:
    COLLISIONS:
      [C001] Variable 'customer_type' used in 2 specs with different values:
             • validate_cart: values ["standard", "premium"]
             • calculate_total: values ["new", "returning"]
    ...
```

## Example Workflow

```bash
# Directory structure:
specs/
├── validate_cart.yaml
├── calculate_total.yaml
├── process_payment.yaml
└── checkout_flow.yaml  # Orchestrator

# Run analysis
imacs completeness specs/

# Output shows:
# 1. Individual spec completeness
# 2. Cross-spec issues (collisions, duplicates)
# 3. Orchestrator-specific issues (missing specs, mapping problems)
# 4. Suggestions for fixes
```

## Benefits

1. **Automatic Discovery**: No need to manually list related specs
2. **Comprehensive Analysis**: Checks both individual specs AND their relationships
3. **Mapping Validation**: Ensures orchestrator inputs/outputs match spec expectations
4. **Missing Spec Detection**: Warns if orchestrator references non-existent specs

## Use Cases

- **CI/CD Integration**: Run `imacs completeness specs/` in CI to catch issues before deployment
- **Refactoring**: When splitting/merging specs, verify orchestrators still work
- **Documentation**: Understand which specs are used together
- **Quality Assurance**: Ensure all referenced specs are complete and compatible

