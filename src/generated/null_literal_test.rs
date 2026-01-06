// GENERATED TESTS FROM: null_literal.yaml
// SPEC HASH: sha256:89a8d6dc562fefab
// GENERATED: 2026-01-06T05:05:49.253122888+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod null_literal_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_null_rust() {
        // null_rust: target == 'Rust' → "None"
        assert_eq!(null_literal("Rust".to_string()), "None".to_string());
    }

    #[test]
    fn test_null_ts() {
        // null_ts: target == 'TypeScript' → "null"
        assert_eq!(null_literal("TypeScript".to_string()), "null".to_string());
    }

    #[test]
    fn test_null_py() {
        // null_py: target == 'Python' → "None"
        assert_eq!(null_literal("Python".to_string()), "None".to_string());
    }

    #[test]
    fn test_null_csharp() {
        // null_csharp: target == 'CSharp' → "null"
        assert_eq!(null_literal("CSharp".to_string()), "null".to_string());
    }

    #[test]
    fn test_null_java() {
        // null_java: target == 'Java' → "null"
        assert_eq!(null_literal("Java".to_string()), "null".to_string());
    }

    #[test]
    fn test_null_go() {
        // null_go: target == 'Go' → "nil"
        assert_eq!(null_literal("Go".to_string()), "nil".to_string());
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
            fn prop_always_valid_output(target in any::<String>()) {
                let result = null_literal(target);
                let valid_outputs = vec!["None".to_string(), "nil".to_string(), "null".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
