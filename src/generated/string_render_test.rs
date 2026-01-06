// GENERATED TESTS FROM: string_render.yaml
// SPEC HASH: sha256:879a9c7c59ff173a
// GENERATED: 2026-01-06T05:05:49.445299579+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod string_render_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_rust_owned() {
        // rust_owned: (target == 'Rust') && (needs_owned) → {"suffix": String(".to_string()"), "quote": String("\"")}
        assert_eq!(string_render("Rust".to_string(), true), ("\"".to_string(), ".to_string()".to_string()));
    }

    #[test]
    fn test_rust_borrowed() {
        // rust_borrowed: (target == 'Rust') && (!needs_owned) → {"quote": String("\""), "suffix": String("")}
        assert_eq!(string_render("Rust".to_string(), false), ("\"".to_string(), "".to_string()));
    }

    #[test]
    fn test_ts() {
        // ts: target == 'TypeScript' → {"quote": String("\""), "suffix": String("")}
        assert_eq!(string_render("TypeScript".to_string(), false), ("\"".to_string(), "".to_string()));
    }

    #[test]
    fn test_py() {
        // py: target == 'Python' → {"suffix": String(""), "quote": String("\"")}
        assert_eq!(string_render("Python".to_string(), false), ("\"".to_string(), "".to_string()));
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
            fn prop_always_valid_output(target in any::<String>(), needs_owned in any::<bool>()) {
                let result = string_render(target, needs_owned);
                let valid_outputs = vec![("\"".to_string(), "".to_string()), ("\"".to_string(), ".to_string()".to_string())];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
