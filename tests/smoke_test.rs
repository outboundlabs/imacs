//! Smoke test to verify basic functionality

use imacs::completeness::analyze_completeness;
use imacs::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};

#[test]
fn smoke_test_basic_completeness() {
    let spec = Spec {
        id: "smoke".into(),
        name: None,
        description: None,
        inputs: vec![Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![
            Rule {
                id: "R1".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            Rule {
                id: "R2".into(),
                when: Some("!a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(0)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    };

    let report = analyze_completeness(&spec);
    // Basic sanity checks
    assert!(report.total_combinations > 0);
    assert!(report.covered_combinations <= report.total_combinations);
    assert!(report.coverage_ratio >= 0.0);
    assert!(report.coverage_ratio <= 1.0);
}
