// GENERATED FROM: gap_classification.yaml
// SPEC HASH: sha256:a6d8af6b1a3c2298
// GENERATED: 2026-01-02T13:32:40.425218192+00:00
// DO NOT EDIT — regenerate from spec

pub fn gap_classification(rule_found: bool, condition_matches: bool, output_matches: bool, priority_correct: bool) -> (String, String) {
    if ((rule_found && condition_matches) && output_matches) {
        // no_gap
        ("None".to_string(), "None".to_string())
    } else if (!rule_found) {
        // missing
        ("Missing".to_string(), "Error".to_string())
    } else if (rule_found && (!condition_matches)) {
        // condition
        ("ConditionMismatch".to_string(), "Error".to_string())
    } else if ((rule_found && condition_matches) && (!output_matches)) {
        // output
        ("OutputMismatch".to_string(), "Error".to_string())
    } else if (((rule_found && condition_matches) && output_matches) && (!priority_correct)) {
        // priority
        ("WrongPriority".to_string(), "Warning".to_string())
    } else {
        unreachable!("No rule matched")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

// GENERATED TESTS FROM: gap_classification.yaml
// SPEC HASH: sha256:a6d8af6b1a3c2298
// GENERATED: 2026-01-02T13:32:40.478325392+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod gap_classification_tests {
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
            fn prop_always_valid_output(rule_found in any::<bool>(), condition_matches in any::<bool>(), output_matches in any::<bool>(), priority_correct in any::<bool>()) {
                let result = gap_classification(rule_found, condition_matches, output_matches, priority_correct);
                prop_assert!([("ConditionMismatch".to_string(), "Error".to_string()), ("Missing".to_string(), "Error".to_string()), ("None".to_string(), "None".to_string()), ("OutputMismatch".to_string(), "Error".to_string()), ("WrongPriority".to_string(), "Warning".to_string())].contains(&result));
            }
        }
    }
}

}
