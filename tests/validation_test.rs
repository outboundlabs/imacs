//! Tests for spec validation - impossible situation detection

use imacs::completeness::{validate_spec, IssueType};
use imacs::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};

fn make_base_spec() -> Spec {
    Spec {
        id: "test".into(),
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
        rules: vec![],
        default: None,
        meta: Default::default(),
    }
}

#[test]
fn test_validate_empty_spec() {
    let spec = make_base_spec();
    let report = validate_spec(&spec, false);
    assert!(report.is_valid);
    assert_eq!(report.error_count, 0);
    assert_eq!(report.warning_count, 0);
}

#[test]
fn test_detect_contradictory_rules() {
    let mut spec = make_base_spec();
    spec.rules = vec![
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
            when: Some("a".into()), // Same condition!
            conditions: None,
            then: Output::Single(ConditionValue::Int(2)), // Different output!
            priority: 0,                                  // Same priority!
            description: None,
        },
    ];

    let report = validate_spec(&spec, false);
    assert!(!report.is_valid);
    assert!(report
        .issues
        .iter()
        .any(|i| matches!(i.issue_type, IssueType::ContradictoryRules)));
    assert!(report.error_count > 0);
}

#[test]
fn test_contradiction_with_priority() {
    let mut spec = make_base_spec();
    spec.rules = vec![
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
            when: Some("a".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(2)),
            priority: 1, // Different priority - not a contradiction!
            description: None,
        },
    ];

    let report = validate_spec(&spec, false);
    // Should not be a contradiction because priorities differ
    assert!(!report
        .issues
        .iter()
        .any(|i| matches!(i.issue_type, IssueType::ContradictoryRules)));
}

#[test]
fn test_detect_dead_rule() {
    let mut spec = make_base_spec();
    spec.rules = vec![
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
            then: Output::Single(ConditionValue::Int(2)),
            priority: 0,
            description: None,
        },
        Rule {
            id: "R3".into(),
            when: Some("a".into()), // Covered by R1!
            conditions: None,
            then: Output::Single(ConditionValue::Int(3)),
            priority: 0,
            description: None,
        },
    ];

    let report = validate_spec(&spec, false);
    // Should detect R3 as dead rule
    assert!(report
        .issues
        .iter()
        .any(|i| matches!(i.issue_type, IssueType::DeadRule)));
}

#[test]
fn test_detect_tautology() {
    let mut spec = make_base_spec();
    spec.rules = vec![Rule {
        id: "R1".into(),
        when: Some("a || !a".into()), // Always true!
        conditions: None,
        then: Output::Single(ConditionValue::Int(1)),
        priority: 0,
        description: None,
    }];

    let report = validate_spec(&spec, false);
    // Should detect tautology
    assert!(report
        .issues
        .iter()
        .any(|i| matches!(i.issue_type, IssueType::TautologyCondition)));
}

#[test]
fn test_detect_type_mismatch() {
    let mut spec = make_base_spec();
    spec.inputs = vec![Variable {
        name: "amount".into(),
        typ: VarType::Int,
        description: None,
        values: None,
    }];
    spec.rules = vec![Rule {
        id: "R1".into(),
        when: Some(r#"amount == "string""#.into()), // Type mismatch!
        conditions: None,
        then: Output::Single(ConditionValue::Int(1)),
        priority: 0,
        description: None,
    }];

    let report = validate_spec(&spec, false);
    // Should detect type mismatch
    assert!(report
        .issues
        .iter()
        .any(|i| matches!(i.issue_type, IssueType::TypeMismatch)));
}

#[test]
fn test_strict_mode() {
    let mut spec = make_base_spec();
    spec.rules = vec![Rule {
        id: "R1".into(),
        when: Some("a || !a".into()), // Tautology (warning)
        conditions: None,
        then: Output::Single(ConditionValue::Int(1)),
        priority: 0,
        description: None,
    }];

    let report_normal = validate_spec(&spec, false);
    let report_strict = validate_spec(&spec, true);

    // In normal mode, tautology is a warning
    assert_eq!(report_normal.warning_count, 1);
    assert_eq!(report_normal.error_count, 0);

    // In strict mode, warning becomes error
    assert_eq!(report_strict.warning_count, 0);
    assert_eq!(report_strict.error_count, 1);
    assert!(!report_strict.is_valid);
}

#[test]
fn test_valid_spec() {
    let mut spec = make_base_spec();
    spec.rules = vec![
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
            then: Output::Single(ConditionValue::Int(2)),
            priority: 0,
            description: None,
        },
    ];

    let report = validate_spec(&spec, false);
    assert!(report.is_valid);
    assert_eq!(report.error_count, 0);
}
