// GENERATED TESTS FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.830009151+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod order_flow_tests {
    use super::*;

    /// Happy path: all gates pass
    #[test]
    fn test_happy_path() {
        let input = OrderFlowInput {
            role: "test".to_string(),
            verified: true,
            weight_kg: 10.0,
            zone: "test".to_string(),
            priority: true,
            member_tier: "test".to_string(),
        };

        let result = order_flow(input);
        assert!(result.is_ok());
    }

    /// Gate 'require_access' should reject invalid inputs
    #[test]
    fn test_gate_require_access_fails() {
        let input = OrderFlowInput {
            role: String::new(),
            verified: false,
            weight_kg: 0.0,
            zone: String::new(),
            priority: false,
            member_tier: String::new(),
        };

        let result = order_flow(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.step, "require_access");
        assert_eq!(err.error_type, "gate_failed");
    }

}
