# Test Suite

## Test Files

- `smoke_test.rs` - Basic smoke test to verify functionality
- `completeness_test.rs` - CLI integration tests (10 tests)
- `completeness_comprehensive_test.rs` - Data-driven tests (13 test cases)
- `completeness_coverage_test.rs` - Edge case coverage (20+ tests)
- `completeness_property_test.rs` - Property-based tests (4 properties)
- `suite_test.rs` - Suite analysis tests (5 tests)

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test completeness_comprehensive_test

# Run with output
cargo test -- --nocapture

# Run library tests only
cargo test --lib completeness

# Run integration tests only
cargo test --test '*'
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
- `suite/` - Cross-spec test fixtures

## Expected Test Results

All tests should pass. If tests fail:

1. Check `TEST_FIXES_NEEDED.md` for known issues
2. Review test expectations vs actual behavior
3. Adjust expectations or fix implementation as needed
4. See `COVERAGE_AND_CORRECTNESS.md` for coverage goals

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
