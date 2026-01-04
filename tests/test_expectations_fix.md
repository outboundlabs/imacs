# Test Expectations Review

## Potential Issues to Fix

### 1. Minimizable Spec Coverage
The `minimizable_spec` test expects `covered_combinations: 2`, but:
- Rules: `(a && b) → 1` and `(a && !b) → 1`
- This covers: `a=true, b=true` and `a=true, b=false`
- Total combinations: 4 (a,b), (a,!b), (!a,b), (!a,!b)
- Covered: 2 (only when a=true)
- **Expected**: `covered_combinations: 2` is CORRECT

### 2. Comparison Predicates
The `comparison_predicates` test expects `total_combinations: 2`:
- Input: `amount` (int, no enum values)
- Predicates extracted: `amount > 1000`, `amount <= 1000`
- These are boolean predicates, so 2^1 = 2 combinations
- **Expected**: CORRECT

### 3. Membership Predicates  
The `membership_predicates` test expects `total_combinations: 3`:
- Input: `region` with values `["US", "EU", "APAC"]`
- Rules: `region in ["US", "EU"]` and `region == "APAC"`
- This creates 3 distinct cases, not 2^n boolean combinations
- **May need adjustment** - depends on how predicates are extracted

## Action Items

1. Run tests and see which fail
2. Adjust expectations based on actual behavior
3. Document the predicate extraction logic for non-boolean types
4. Add property-based tests for edge cases

