// GENERATED TESTS FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.558522705+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod access_level_tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_r1() {
        // R1: role == 'admin' → 100
        assert_eq!(access_level("admin".to_string(), false), 100);
    }

    #[test]
    fn test_r2() {
        // R2: role == 'member' && verified → 50
        assert_eq!(access_level("member".to_string(), true), 50);
    }

    #[test]
    fn test_r3() {
        // R3: role == 'member' && !verified → 25
        assert_eq!(access_level("member".to_string(), false), 25);
    }

    #[test]
    fn test_r4() {
        // R4: role == 'guest' → 10
        assert_eq!(access_level("guest".to_string(), false), 10);
    }

    // ═══════════════════════════════════════════════════════════════
    // Property tests
    // ═══════════════════════════════════════════════════════════════

    #[cfg(feature = "proptest")]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_always_valid_output(role in any::<String>(), verified in any::<bool>()) {
                let result = access_level(role, verified);
                prop_assert!([10, 100, 25, 50].contains(&result));
            }
        }
    }
}
