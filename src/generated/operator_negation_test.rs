// GENERATED TESTS FROM: operator_negation.yaml
// SPEC HASH: sha256:719d81f5844361a1
// GENERATED: 2026-01-06T05:05:49.407152348+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod operator_negation_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_eq_to_ne() {
        // eq_to_ne: op == 'Eq' → "Ne"
        assert_eq!(operator_negation("Eq".to_string()), "Ne".to_string());
    }

    #[test]
    fn test_ne_to_eq() {
        // ne_to_eq: op == 'Ne' → "Eq"
        assert_eq!(operator_negation("Ne".to_string()), "Eq".to_string());
    }

    #[test]
    fn test_lt_to_ge() {
        // lt_to_ge: op == 'Lt' → "Ge"
        assert_eq!(operator_negation("Lt".to_string()), "Ge".to_string());
    }

    #[test]
    fn test_le_to_gt() {
        // le_to_gt: op == 'Le' → "Gt"
        assert_eq!(operator_negation("Le".to_string()), "Gt".to_string());
    }

    #[test]
    fn test_gt_to_le() {
        // gt_to_le: op == 'Gt' → "Le"
        assert_eq!(operator_negation("Gt".to_string()), "Le".to_string());
    }

    #[test]
    fn test_ge_to_lt() {
        // ge_to_lt: op == 'Ge' → "Lt"
        assert_eq!(operator_negation("Ge".to_string()), "Lt".to_string());
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
            fn prop_always_valid_output(op in any::<String>()) {
                let result = operator_negation(op);
                let valid_outputs = vec!["Eq".to_string(), "Ge".to_string(), "Gt".to_string(), "Le".to_string(), "Lt".to_string(), "Ne".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
