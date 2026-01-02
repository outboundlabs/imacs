// GENERATED FROM: test_mode_selection.yaml
// SPEC HASH: sha256:3ac976b78aef9a2d
// GENERATED: 2026-01-02T13:32:40.932156365+00:00
// DO NOT EDIT — regenerate from spec

pub fn test_mode_selection(input_count: i64, all_inputs_enumerable: bool, has_numeric_conditions: bool, total_combinations: i64) -> (bool, bool, bool) {
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


#[cfg(test)]
mod tests {
    use super::*;

// GENERATED TESTS FROM: test_mode_selection.yaml
// SPEC HASH: sha256:3ac976b78aef9a2d
// GENERATED: 2026-01-02T13:32:40.960453004+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod test_mode_selection_tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    // ═══════════════════════════════════════════════════════════════
    // Property tests
    // ═══════════════════════════════════════════════════════════════

    #[cfg(feature = "proptest")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_always_valid_output(input_count in any::<i64>(), all_inputs_enumerable in any::<bool>(), has_numeric_conditions in any::<bool>(), total_combinations in any::<i64>()) {
                let result = test_mode_selection(input_count, all_inputs_enumerable, has_numeric_conditions, total_combinations);
                prop_assert!([(false, true, true), (true, false, true)].contains(&result));
            }
        }
    }
}

}
