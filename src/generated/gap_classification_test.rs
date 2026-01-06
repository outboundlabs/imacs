// GENERATED TESTS FROM: gap_classification.yaml
// SPEC HASH: sha256:876759ce8c965ce4
// GENERATED: 2026-01-06T05:05:49.151979495+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod gap_classification_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_no_gap() {
        // no_gap: (rule_found) && (condition_matches) && (output_matches) → {"gap_reason": String("None"), "severity": String("None")}
        assert_eq!(gap_classification(true, true, true, false), ("None".to_string(), "None".to_string()));
    }

    #[test]
    fn test_missing() {
        // missing: !rule_found → {"gap_reason": String("Missing"), "severity": String("Error")}
        assert_eq!(gap_classification(false, false, false, false), ("Missing".to_string(), "Error".to_string()));
    }

    #[test]
    fn test_condition() {
        // condition: (rule_found) && (!condition_matches) → {"severity": String("Error"), "gap_reason": String("ConditionMismatch")}
        assert_eq!(gap_classification(true, false, false, false), ("ConditionMismatch".to_string(), "Error".to_string()));
    }

    #[test]
    fn test_output() {
        // output: (rule_found) && (condition_matches) && (!output_matches) → {"severity": String("Error"), "gap_reason": String("OutputMismatch")}
        assert_eq!(gap_classification(true, true, false, false), ("OutputMismatch".to_string(), "Error".to_string()));
    }

    #[test]
    fn test_priority() {
        // priority: (rule_found) && (condition_matches) && (output_matches) && (!priority_correct) → {"gap_reason": String("WrongPriority"), "severity": String("Warning")}
        assert_eq!(gap_classification(true, true, true, false), ("WrongPriority".to_string(), "Warning".to_string()));
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
            fn prop_always_valid_output(rule_found in any::<bool>(), condition_matches in any::<bool>(), output_matches in any::<bool>(), priority_correct in any::<bool>()) {
                let result = gap_classification(rule_found, condition_matches, output_matches, priority_correct);
                let valid_outputs = vec![("ConditionMismatch".to_string(), "Error".to_string()), ("Missing".to_string(), "Error".to_string()), ("None".to_string(), "None".to_string()), ("OutputMismatch".to_string(), "Error".to_string()), ("WrongPriority".to_string(), "Warning".to_string())];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
