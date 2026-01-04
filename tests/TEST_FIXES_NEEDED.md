# Test Fixes Needed

## Test Expectations That May Need Adjustment

### 1. Minimizable Spec (`minimizable_spec`)
**Current expectation**: `covered_combinations: 2`
**Rules**: `(a && b) → 1` and `(a && !b) → 1`
**Analysis**: 
- Total combinations: 4 (a,b), (a,!b), (!a,b), (!a,!b)
- Covered: 2 (only when a=true)
- **Status**: ✅ Likely CORRECT

### 2. Membership Predicates (`membership_predicates`)
**Current expectation**: `total_combinations: 3`
**Rules**: `region in ["US", "EU"]` and `region == "APAC"`
**Analysis**:
- This depends on how predicates are extracted
- If extracted as 3 separate equality predicates: 2^3 = 8 combinations
- If extracted as 1 membership predicate: 2^1 = 2 combinations  
- **Status**: ⚠️ May need adjustment - depends on implementation

### 3. Comparison Predicates (`comparison_predicates`)
**Current expectation**: `total_combinations: 2`
**Rules**: `amount > 1000` and `amount <= 1000`
**Analysis**:
- Creates 1 boolean predicate: `amount > 1000`
- Total: 2^1 = 2 combinations
- **Status**: ✅ Likely CORRECT

## Potential Runtime Issues

### 1. Property Tests
The `proptest!` macro may need configuration. If tests fail, add:
```rust
proptest::proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    // ... tests
}
```

### 2. Test Data Validity
Some helper functions create specs that may not be valid:
- Check that all CEL expressions are parseable
- Verify variable names match what's used in rules

### 3. Import Issues
All imports use `imacs::completeness::*` which should work, but verify:
- `Cube` and `CubeValue` are re-exported
- All public APIs are accessible

## Steps to Fix Tests

1. **Run tests**:
   ```bash
   cargo test --lib completeness
   cargo test --test '*'
   ```

2. **Identify failures**:
   - Check which assertions fail
   - Note actual vs expected values

3. **Fix expectations**:
   - Adjust test expectations to match actual behavior
   - Or fix implementation if behavior is wrong

4. **Verify fixes**:
   - Re-run tests
   - Check coverage

## Known Issues to Check

1. **Empty predicate set**: Some tests may fail if predicate extraction returns empty
2. **Invalid CEL**: Tests with invalid CEL should handle gracefully
3. **Edge cases**: Empty specs, single rules, etc.

## Test Execution Order

Run tests in this order to identify issues:
1. `smoke_test.rs` - Basic functionality
2. `completeness_coverage_test.rs` - Edge cases
3. `completeness_comprehensive_test.rs` - Data-driven tests
4. `suite_test.rs` - Suite analysis
5. `completeness_property_test.rs` - Property tests (may be slow)

