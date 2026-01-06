// GENERATED TESTS FROM: test_mode_selection.yaml
// SPEC HASH: sha256:ce44baee9e2bf428
// GENERATED: 2026-01-06T05:05:49.479578105+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod test_mode_selection_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_exhaustive_yes() {
        // exhaustive_yes: (all_inputs_enumerable) && (total_combinations <= 64) → {"generate_property": Bool(true), "generate_exhaustive": Bool(true), "generate_boundary": Bool(false)}
        assert_eq!(test_mode_selection(0, true, false, 0), (true, false, true));
    }

    #[test]
    fn test_exhaustive_no() {
        // exhaustive_no: !all_inputs_enumerable || total_combinations > 64 → {"generate_boundary": Bool(true), "generate_property": Bool(true), "generate_exhaustive": Bool(false)}
        assert_eq!(test_mode_selection(0, false, false, 0), (false, true, true));
    }

    #[test]
    fn test_boundary_numeric() {
        // boundary_numeric: has_numeric_conditions → {"generate_property": Bool(true), "generate_exhaustive": Bool(false), "generate_boundary": Bool(true)}
        assert_eq!(test_mode_selection(0, false, true, 0), (false, true, true));
    }

    // ═══════════════════════════════════════════════════════════════
    // Property tests
    // ═══════════════════════════════════════════════════════════════

    #[cfg(feature = "proptest")]
    mod property_tests {
        #[allow(unused_imports)]
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_always_valid_output(input_count in any::<i64>(), all_inputs_enumerable in any::<bool>(), has_numeric_conditions in any::<bool>(), total_combinations in any::<i64>()) {
                let result = test_mode_selection(input_count, all_inputs_enumerable, has_numeric_conditions, total_combinations);
                let valid_outputs = vec![(false, true, true), (true, false, true)];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
