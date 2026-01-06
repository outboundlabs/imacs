//! Property-based tests for completeness analysis
//!
//! Uses proptest to generate random specs and verify invariants

use imacs::completeness::analyze_completeness;
use imacs::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_coverage_ratio_bounds(spec in any_spec()) {
        let report = analyze_completeness(&spec);
        // Coverage ratio should always be between 0 and 1
        prop_assert!(report.coverage_ratio >= 0.0);
        prop_assert!(report.coverage_ratio <= 1.0);
    }

    #[test]
    fn test_covered_never_exceeds_total(spec in any_spec()) {
        let report = analyze_completeness(&spec);
        // Covered combinations should never exceed total
        prop_assert!(report.covered_combinations <= report.total_combinations);
    }

    #[test]
    fn test_complete_implies_no_missing(spec in any_spec()) {
        let report = analyze_completeness(&spec);
        // If complete, should have no missing cases
        if report.is_complete {
            prop_assert_eq!(report.missing_cases.len(), 0);
        }
    }

    #[test]
    fn test_predicates_match_rules(spec in any_spec()) {
        let report = analyze_completeness(&spec);
        // Number of predicates should be reasonable
        prop_assert!(report.predicates.len() <= 100); // Sanity check
    }
}

fn any_spec() -> impl Strategy<Value = Spec> {
    let var_strategy = prop_oneof![
        Just(Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }),
        Just(Variable {
            name: "b".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }),
    ];

    let rule_strategy = prop::collection::vec(
        prop_oneof![
            Just(Rule {
                id: "R1".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            }),
            Just(Rule {
                id: "R2".into(),
                when: Some("!a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(2)),
                priority: 0,
                description: None,
            }),
            Just(Rule {
                id: "R3".into(),
                when: Some("a && b".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(3)),
                priority: 0,
                description: None,
            }),
        ],
        0..5,
    );

    (var_strategy, rule_strategy).prop_map(|(input, rules)| Spec {
        scoping: None,
        id: "test".into(),
        name: None,
        description: None,
        inputs: vec![input],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules,
        default: None,
        meta: Default::default(),
    })
}
