// GENERATED TESTS FROM: bool_literal.yaml
// SPEC HASH: sha256:36872e0f7f0af7d1
// GENERATED: 2026-01-06T05:05:48.710543154+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod bool_literal_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_true_py() {
        // true_py: (value == true) && (target == 'Python') → "True"
        assert_eq!(bool_literal(true, "Python".to_string()), "True".to_string());
    }

    #[test]
    fn test_false_py() {
        // false_py: (value == false) && (target == 'Python') → "False"
        assert_eq!(bool_literal(false, "Python".to_string()), "False".to_string());
    }

    #[test]
    fn test_true_default() {
        // true_default: value == true → "true"
        assert_eq!(bool_literal(true, "Rust".to_string()), "true".to_string());
    }

    #[test]
    fn test_false_default() {
        // false_default: value == false → "false"
        assert_eq!(bool_literal(false, "Rust".to_string()), "false".to_string());
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
            fn prop_always_valid_output(value in any::<bool>(), target in any::<String>()) {
                let result = bool_literal(value, target);
                let valid_outputs = vec!["False".to_string(), "True".to_string(), "false".to_string(), "true".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
