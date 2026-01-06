//! Tests for fix generation and application

use imacs::completeness::{
    apply_fixes, apply_fixes_to_yaml, validate_spec, FixConfidence, FixOperation, SpecFix,
};
use imacs::spec::{ConditionValue, Output, Spec, VarType};

fn make_test_spec() -> Spec {
    Spec {
        scoping: None,
        id: "test".into(),
        name: None,
        description: None,
        inputs: vec![imacs::spec::Variable {
            name: "a".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![imacs::spec::Variable {
            name: "result".into(),
            typ: VarType::Int,
            description: None,
            values: None,
        }],
        rules: vec![
            imacs::spec::Rule {
                id: "R1".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            imacs::spec::Rule {
                id: "R2".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(2)),
                priority: 0,
                description: None,
            },
        ],
        default: None,
        meta: Default::default(),
    }
}

#[test]
fn test_fix_generation_contradiction() {
    let spec = make_test_spec();
    let report = validate_spec(&spec, false);

    // Should detect contradiction and generate fix
    assert!(!report.is_valid);
    assert!(!report.fixes.is_empty());

    let contradiction_fix = report
        .fixes
        .iter()
        .find(|f| matches!(f.operation, FixOperation::AddPriority { .. }));
    assert!(contradiction_fix.is_some());

    let fix = contradiction_fix.unwrap();
    assert_eq!(fix.confidence, FixConfidence::High);

    // Verify the fix corresponds to a contradiction issue
    let contradiction_issue = report.issues.iter().find(|i| {
        matches!(
            i.issue_type,
            imacs::completeness::IssueType::ContradictoryRules
        )
    });
    assert!(contradiction_issue.is_some());
    assert_eq!(fix.issue_code, contradiction_issue.unwrap().code);
}

#[test]
fn test_apply_priority_fix() {
    let mut spec = make_test_spec();
    let fix = SpecFix {
        issue_code: "V001".into(),
        confidence: FixConfidence::High,
        operation: FixOperation::AddPriority {
            rule_id: "R1".into(),
            priority: 1,
        },
        description: "Add priority".into(),
    };

    let result = apply_fixes(&mut spec, &[fix], false);
    assert_eq!(result.applied.len(), 1);
    assert_eq!(spec.rules[0].priority, 1);
    assert_eq!(spec.rules[1].priority, 0);
}

#[test]
fn test_apply_delete_rule_fix() {
    let mut spec = make_test_spec();
    let fix = SpecFix {
        issue_code: "V001".into(),
        confidence: FixConfidence::High,
        operation: FixOperation::DeleteRule {
            rule_id: "R2".into(),
        },
        description: "Delete rule".into(),
    };

    let result = apply_fixes(&mut spec, &[fix], false);
    assert_eq!(result.applied.len(), 1);
    assert_eq!(spec.rules.len(), 1);
    assert_eq!(spec.rules[0].id, "R1");
}

#[test]
fn test_apply_fixes_filtering() {
    let mut spec = make_test_spec();
    let fixes = vec![
        SpecFix {
            issue_code: "V001".into(),
            confidence: FixConfidence::High,
            operation: FixOperation::AddPriority {
                rule_id: "R1".into(),
                priority: 1,
            },
            description: "High confidence".into(),
        },
        SpecFix {
            issue_code: "V002".into(),
            confidence: FixConfidence::Low,
            operation: FixOperation::DeleteRule {
                rule_id: "R2".into(),
            },
            description: "Low confidence".into(),
        },
    ];

    // Without --all, should skip low confidence
    let result = apply_fixes(&mut spec, &fixes, false);
    assert_eq!(result.applied.len(), 1);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(spec.rules[0].priority, 1);
    assert_eq!(spec.rules.len(), 2); // R2 not deleted

    // With --all, should apply both
    let mut spec2 = make_test_spec();
    let result2 = apply_fixes(&mut spec2, &fixes, true);
    assert_eq!(result2.applied.len(), 2);
    assert_eq!(result2.skipped.len(), 0);
    assert_eq!(spec2.rules.len(), 1); // R2 deleted
}

#[test]
fn test_apply_fixes_to_yaml() {
    let yaml = r#"
id: test_spec
inputs:
  - name: a
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "a"
    then: 1
    priority: 0
  - id: R2
    when: "a"
    then: 2
    priority: 0
"#;

    let fixes = vec![SpecFix {
        issue_code: "V001".into(),
        confidence: FixConfidence::High,
        operation: FixOperation::AddPriority {
            rule_id: "R1".into(),
            priority: 1,
        },
        description: "Add priority".into(),
    }];

    let (new_yaml, result) = apply_fixes_to_yaml(yaml, &fixes, false).unwrap();
    assert_eq!(result.applied.len(), 1);
    assert!(new_yaml.contains("priority: 1"));
}

#[test]
fn test_update_expression_fix() {
    let mut spec = make_test_spec();
    let fix = SpecFix {
        issue_code: "V001".into(),
        confidence: FixConfidence::Medium,
        operation: FixOperation::UpdateExpression {
            rule_id: "R1".into(),
            old_expression: "a".into(),
            new_expression: "!a".into(),
        },
        description: "Update expression".into(),
    };

    let result = apply_fixes(&mut spec, &[fix], false);
    assert_eq!(result.applied.len(), 1);
    assert_eq!(spec.rules[0].when, Some("!a".into()));
}

#[test]
fn test_rename_variable_fix() {
    let mut spec = Spec {
        scoping: None,
        id: "test".into(),
        name: None,
        description: None,
        inputs: vec![imacs::spec::Variable {
            name: "old_name".into(),
            typ: VarType::Bool,
            description: None,
            values: None,
        }],
        outputs: vec![],
        rules: vec![imacs::spec::Rule {
            id: "R1".into(),
            when: Some("old_name".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Bool(true)),
            priority: 0,
            description: None,
        }],
        default: None,
        meta: Default::default(),
    };

    let fix = SpecFix {
        issue_code: "V001".into(),
        confidence: FixConfidence::High,
        operation: FixOperation::RenameVariable {
            old_name: "old_name".into(),
            new_name: "new_name".into(),
        },
        description: "Rename variable".into(),
    };

    let result = apply_fixes(&mut spec, &[fix], false);
    assert_eq!(result.applied.len(), 1);
    assert_eq!(spec.inputs[0].name, "new_name");
    assert_eq!(spec.rules[0].when, Some("new_name".into()));
}

#[test]
fn test_fix_error_handling() {
    let mut spec = make_test_spec();
    let fix = SpecFix {
        issue_code: "V001".into(),
        confidence: FixConfidence::High,
        operation: FixOperation::AddPriority {
            rule_id: "R999".into(), // Non-existent rule
            priority: 1,
        },
        description: "Add priority".into(),
    };

    let result = apply_fixes(&mut spec, &[fix], false);
    assert_eq!(result.applied.len(), 0);
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("R999"));
}
