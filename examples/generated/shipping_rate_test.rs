// GENERATED TESTS FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-05T17:28:47.068261+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod shipping_rate_tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_r1() {
        // R1: member_tier == 'gold' && zone == 'domestic' → 0
        assert_eq!(shipping_rate(0.0, "domestic".to_string(), false, "gold".to_string()), 0.0);
    }

    #[test]
    fn test_r2() {
        // R2: priority && zone == 'international' → "weight_kg * 25.0 + 50.0"
        assert_eq!(shipping_rate(0.0, "international".to_string(), true, "".to_string()), "weight_kg * 25.0 + 50.0".to_string());
    }

    #[test]
    fn test_r3() {
        // R3: priority && zone == 'north_america' → "weight_kg * 15.0 + 20.0"
        assert_eq!(shipping_rate(0.0, "north_america".to_string(), true, "".to_string()), "weight_kg * 15.0 + 20.0".to_string());
    }

    #[test]
    fn test_r4() {
        // R4: priority && zone == 'domestic' → "weight_kg * 8.0 + 10.0"
        assert_eq!(shipping_rate(0.0, "domestic".to_string(), true, "".to_string()), "weight_kg * 8.0 + 10.0".to_string());
    }

    #[test]
    fn test_r5() {
        // R5: member_tier == 'silver' && zone == 'international' → "weight_kg * 16.0 + 30.0"
        assert_eq!(shipping_rate(0.0, "international".to_string(), false, "silver".to_string()), "weight_kg * 16.0 + 30.0".to_string());
    }

    #[test]
    fn test_r6() {
        // R6: member_tier == 'silver' && zone == 'north_america' → "weight_kg * 8.0 + 12.0"
        assert_eq!(shipping_rate(0.0, "north_america".to_string(), false, "silver".to_string()), "weight_kg * 8.0 + 12.0".to_string());
    }

    #[test]
    fn test_r7() {
        // R7: member_tier == 'silver' && zone == 'domestic' → "weight_kg * 4.0 + 5.0"
        assert_eq!(shipping_rate(0.0, "domestic".to_string(), false, "silver".to_string()), "weight_kg * 4.0 + 5.0".to_string());
    }

    #[test]
    fn test_r8() {
        // R8: zone == 'international' → "weight_kg * 20.0 + 40.0"
        assert_eq!(shipping_rate(0.0, "international".to_string(), false, "".to_string()), "weight_kg * 20.0 + 40.0".to_string());
    }

    #[test]
    fn test_r9() {
        // R9: zone == 'north_america' → "weight_kg * 10.0 + 15.0"
        assert_eq!(shipping_rate(0.0, "north_america".to_string(), false, "".to_string()), "weight_kg * 10.0 + 15.0".to_string());
    }

    #[test]
    fn test_r10() {
        // R10: zone == 'domestic' → "weight_kg * 5.0 + 7.0"
        assert_eq!(shipping_rate(0.0, "domestic".to_string(), false, "".to_string()), "weight_kg * 5.0 + 7.0".to_string());
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
            fn prop_always_valid_output(weight_kg in any::<f64>(), zone in any::<String>(), priority in any::<bool>(), member_tier in any::<String>()) {
                let result = shipping_rate(weight_kg, zone, priority, member_tier);
                prop_assert!(["weight_kg * 10.0 + 15.0".to_string(), "weight_kg * 15.0 + 20.0".to_string(), "weight_kg * 16.0 + 30.0".to_string(), "weight_kg * 20.0 + 40.0".to_string(), "weight_kg * 25.0 + 50.0".to_string(), "weight_kg * 4.0 + 5.0".to_string(), "weight_kg * 5.0 + 7.0".to_string(), "weight_kg * 8.0 + 10.0".to_string(), "weight_kg * 8.0 + 12.0".to_string(), 0.0].contains(&result));
            }
        }
    }
}
