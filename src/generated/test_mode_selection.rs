// GENERATED FROM: test_mode_selection.yaml
// SPEC HASH: sha256:ce44baee9e2bf428
// GENERATED: 2026-01-06T05:05:49.479481307+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn test_mode_selection(
    input_count: i64,
    all_inputs_enumerable: bool,
    has_numeric_conditions: bool,
    total_combinations: i64,
) -> (bool, bool, bool) {
    if (all_inputs_enumerable && (total_combinations <= 64)) {
        // exhaustive_yes
        (true, false, true)
    } else if ((!all_inputs_enumerable) || (total_combinations > 64)) {
        // exhaustive_no
        (false, true, true)
    } else if has_numeric_conditions {
        // boundary_numeric
        (false, true, true)
    } else {
        unreachable!("No rule matched")
    }
}
