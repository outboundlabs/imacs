# Test Suite

## Test Files

- `smoke_test.rs` - Basic smoke test to verify functionality
- `completeness_test.rs` - CLI integration tests for completeness command
- `completeness_comprehensive_test.rs` - Data-driven tests using rstest (13 test cases)
- `completeness_coverage_test.rs` - Edge case coverage tests (20+ tests)
- `completeness_property_test.rs` - Property-based tests using proptest (4 properties)
- `suite_test.rs` - Suite analysis integration tests (5 tests)
- `validation_test.rs` - Spec validation tests (contradictions, dead rules, etc.)
- `fix_test.rs` - Fix generation and application tests (8 tests)

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test completeness_comprehensive_test
cargo test --test validation_test
cargo test --test fix_test

# Run with output
cargo test -- --nocapture

# Run library tests only
cargo test --lib completeness
cargo test --lib config
cargo test --lib project
cargo test --lib meta

# Run integration tests only
cargo test --test '*'

# Run tests for specific module
cargo test --lib -- config project meta
```

## Test Coverage

To generate coverage report:
```bash
./scripts/test_coverage.sh
```

Or manually:
```bash
cargo llvm-cov --all-features --tests --html
```

## Test Fixtures

Located in `tests/fixtures/`:
- `complete_spec.yaml` - Fully covered spec
- `incomplete_spec.yaml` - Missing cases
- `overlapping_spec.yaml` - Overlapping rules
- `minimizable_spec.yaml` - Can be minimized
- `suite/` - Cross-spec test fixtures (pricing.yaml, discounts.yaml, shipping.yaml, billing.yaml)

## Expected Test Results

All tests should pass (304 tests total).

## Troubleshooting

### Permission Errors
If you see cargo permission errors:
```bash
# Try using a different cargo home
export CARGO_HOME=/tmp/cargo
cargo test
```

### Compilation Errors
Check that all dependencies are available:
```bash
cargo check --tests
```

### Test Failures
Run with more verbose output:
```bash
cargo test -- --nocapture --test-threads=1
```
