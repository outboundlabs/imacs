// GENERATED TESTS FROM: operator_mapping.yaml
// SPEC HASH: sha256:a20f57d199b9c1a4
// GENERATED: 2026-01-06T05:05:49.352980731+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod operator_mapping_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_eq_ts() {
        // eq_ts: (op == 'Eq') && (target == 'TypeScript') → "==="
        assert_eq!(operator_mapping("Eq".to_string(), "TypeScript".to_string()), "===".to_string());
    }

    #[test]
    fn test_eq_default() {
        // eq_default: op == 'Eq' → "=="
        assert_eq!(operator_mapping("Eq".to_string(), "Rust".to_string()), "==".to_string());
    }

    #[test]
    fn test_ne_ts() {
        // ne_ts: (op == 'Ne') && (target == 'TypeScript') → "!=="
        assert_eq!(operator_mapping("Ne".to_string(), "TypeScript".to_string()), "!==".to_string());
    }

    #[test]
    fn test_ne_default() {
        // ne_default: op == 'Ne' → "!="
        assert_eq!(operator_mapping("Ne".to_string(), "Rust".to_string()), "!=".to_string());
    }

    #[test]
    fn test_lt() {
        // lt: op == 'Lt' → "<"
        assert_eq!(operator_mapping("Lt".to_string(), "Rust".to_string()), "<".to_string());
    }

    #[test]
    fn test_le() {
        // le: op == 'Le' → "<="
        assert_eq!(operator_mapping("Le".to_string(), "Rust".to_string()), "<=".to_string());
    }

    #[test]
    fn test_gt() {
        // gt: op == 'Gt' → ">"
        assert_eq!(operator_mapping("Gt".to_string(), "Rust".to_string()), ">".to_string());
    }

    #[test]
    fn test_ge() {
        // ge: op == 'Ge' → ">="
        assert_eq!(operator_mapping("Ge".to_string(), "Rust".to_string()), ">=".to_string());
    }

    #[test]
    fn test_and_py() {
        // and_py: (op == 'And') && (target == 'Python') → "and"
        assert_eq!(operator_mapping("And".to_string(), "Python".to_string()), "and".to_string());
    }

    #[test]
    fn test_and_default() {
        // and_default: op == 'And' → "&&"
        assert_eq!(operator_mapping("And".to_string(), "Rust".to_string()), "&&".to_string());
    }

    #[test]
    fn test_or_py() {
        // or_py: (op == 'Or') && (target == 'Python') → "or"
        assert_eq!(operator_mapping("Or".to_string(), "Python".to_string()), "or".to_string());
    }

    #[test]
    fn test_or_default() {
        // or_default: op == 'Or' → "||"
        assert_eq!(operator_mapping("Or".to_string(), "Rust".to_string()), "||".to_string());
    }

    #[test]
    fn test_not_py() {
        // not_py: (op == 'Not') && (target == 'Python') → "not "
        assert_eq!(operator_mapping("Not".to_string(), "Python".to_string()), "not ".to_string());
    }

    #[test]
    fn test_not_default() {
        // not_default: op == 'Not' → "!"
        assert_eq!(operator_mapping("Not".to_string(), "Rust".to_string()), "!".to_string());
    }

    #[test]
    fn test_in_rust() {
        // in_rust: (op == 'In') && (target == 'Rust') → ".contains(&{})"
        assert_eq!(operator_mapping("In".to_string(), "Rust".to_string()), ".contains(&{})".to_string());
    }

    #[test]
    fn test_in_ts() {
        // in_ts: (op == 'In') && (target == 'TypeScript') → ".includes({})"
        assert_eq!(operator_mapping("In".to_string(), "TypeScript".to_string()), ".includes({})".to_string());
    }

    #[test]
    fn test_in_py() {
        // in_py: (op == 'In') && (target == 'Python') → " in "
        assert_eq!(operator_mapping("In".to_string(), "Python".to_string()), " in ".to_string());
    }

    #[test]
    fn test_in_csharp() {
        // in_csharp: (op == 'In') && (target == 'CSharp') → ".Contains({})"
        assert_eq!(operator_mapping("In".to_string(), "CSharp".to_string()), ".Contains({})".to_string());
    }

    #[test]
    fn test_in_java() {
        // in_java: (op == 'In') && (target == 'Java') → ".contains({})"
        assert_eq!(operator_mapping("In".to_string(), "Java".to_string()), ".contains({})".to_string());
    }

    #[test]
    fn test_in_go() {
        // in_go: (op == 'In') && (target == 'Go') → "contains({}, {})"
        assert_eq!(operator_mapping("In".to_string(), "Go".to_string()), "contains({}, {})".to_string());
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
            fn prop_always_valid_output(op in any::<String>(), target in any::<String>()) {
                let result = operator_mapping(op, target);
                let valid_outputs = vec![" in ".to_string(), "!".to_string(), "!=".to_string(), "!==".to_string(), "&&".to_string(), ".Contains({})".to_string(), ".contains(&{})".to_string(), ".contains({})".to_string(), ".includes({})".to_string(), "<".to_string(), "<=".to_string(), "==".to_string(), "===".to_string(), ">".to_string(), ">=".to_string(), "and".to_string(), "contains({}, {})".to_string(), "not ".to_string(), "or".to_string(), "||".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
