//! Apply fixes to spec files
//!
//! This module provides functionality to apply structured fixes to YAML spec files.
//! It preserves formatting where possible and applies fixes based on FixOperation.

use crate::completeness::{FixConfidence, FixOperation, SpecFix};
use crate::error::Error;
use crate::spec::Spec;

/// Result of applying fixes
#[derive(Debug, Clone)]
pub struct FixApplicationResult {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

/// Apply fixes to a spec
pub fn apply_fixes(spec: &mut Spec, fixes: &[SpecFix], apply_all: bool) -> FixApplicationResult {
    let mut result = FixApplicationResult {
        applied: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
    };

    for fix in fixes {
        // Filter by confidence if not applying all
        if !apply_all {
            match fix.confidence {
                FixConfidence::Low => {
                    result
                        .skipped
                        .push(format!("{} (low confidence)", fix.issue_code));
                    continue;
                }
                FixConfidence::Medium | FixConfidence::High => {}
            }
        }

        match apply_fix(spec, fix) {
            Ok(()) => {
                result.applied.push(fix.issue_code.clone());
            }
            Err(e) => {
                result.errors.push(format!("{}: {}", fix.issue_code, e));
            }
        }
    }

    result
}

/// Apply a single fix to a spec
fn apply_fix(spec: &mut Spec, fix: &SpecFix) -> Result<(), Error> {
    match &fix.operation {
        FixOperation::AddPriority { rule_id, priority } => {
            if let Some(rule) = spec.rules.iter_mut().find(|r| r.id == *rule_id) {
                rule.priority = *priority;
                Ok(())
            } else {
                Err(Error::Other(format!("Rule {} not found", rule_id)))
            }
        }
        FixOperation::DeleteRule { rule_id } => {
            let initial_len = spec.rules.len();
            spec.rules.retain(|r| r.id != *rule_id);
            if spec.rules.len() < initial_len {
                Ok(())
            } else {
                Err(Error::Other(format!("Rule {} not found", rule_id)))
            }
        }
        FixOperation::UpdateRule {
            rule_id,
            field,
            old_value: _,
            new_value,
        } => {
            if let Some(rule) = spec.rules.iter_mut().find(|r| r.id == *rule_id) {
                match field.as_str() {
                    "when" => {
                        if new_value == "null" {
                            rule.when = None;
                        } else {
                            rule.when = Some(crate::spec::WhenClause::Single(new_value.clone()));
                        }
                        Ok(())
                    }
                    "priority" => {
                        if let Ok(p) = new_value.parse::<i32>() {
                            rule.priority = p;
                            Ok(())
                        } else {
                            Err(Error::Other(format!("Invalid priority: {}", new_value)))
                        }
                    }
                    _ => Err(Error::Other(format!("Unknown field: {}", field))),
                }
            } else {
                Err(Error::Other(format!("Rule {} not found", rule_id)))
            }
        }
        FixOperation::UpdateExpression {
            rule_id,
            old_expression: _,
            new_expression,
        } => {
            if let Some(rule) = spec.rules.iter_mut().find(|r| r.id == *rule_id) {
                rule.when = Some(crate::spec::WhenClause::Single(new_expression.clone()));
                Ok(())
            } else {
                Err(Error::Other(format!("Rule {} not found", rule_id)))
            }
        }
        FixOperation::RenameVariable { old_name, new_name } => {
            // Rename in inputs
            for input in &mut spec.inputs {
                if input.name == *old_name {
                    input.name = new_name.clone();
                }
            }
            // Rename in outputs
            for output in &mut spec.outputs {
                if output.name == *old_name {
                    output.name = new_name.clone();
                }
            }
            // Rename in rules (CEL expressions)
            for rule in &mut spec.rules {
                if let Some(cel_expr) = rule.as_cel() {
                    let new_expr = cel_expr.replace(old_name, new_name);
                    rule.when = Some(crate::spec::WhenClause::Single(new_expr));
                }
            }
            Ok(())
        }
    }
}

/// Apply fixes to a YAML file, preserving formatting where possible
pub fn apply_fixes_to_yaml(
    yaml_content: &str,
    fixes: &[SpecFix],
    apply_all: bool,
) -> Result<(String, FixApplicationResult), Error> {
    // Parse YAML
    let mut spec: Spec = Spec::from_yaml(yaml_content)?;

    // Apply fixes
    let result = apply_fixes(&mut spec, fixes, apply_all);

    // Convert back to YAML
    let new_yaml = spec.to_yaml()?;

    Ok((new_yaml, result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConditionValue, Output, VarType};

    fn make_test_spec() -> Spec {
        Spec {
            id: "test".into(),
            name: None,
            description: None,
            inputs: vec![crate::spec::Variable {
                name: "a".into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            }],
            outputs: vec![crate::spec::Variable {
                name: "result".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                crate::spec::Rule {
                    id: "R1".into(),
                    when: Some("a".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                crate::spec::Rule {
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
            scoping: None,
        }
    }

    #[test]
    fn test_add_priority() {
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

        apply_fix(&mut spec, &fix).unwrap();
        assert_eq!(spec.rules[0].priority, 1);
    }

    #[test]
    fn test_delete_rule() {
        let mut spec = make_test_spec();
        let fix = SpecFix {
            issue_code: "V001".into(),
            confidence: FixConfidence::High,
            operation: FixOperation::DeleteRule {
                rule_id: "R2".into(),
            },
            description: "Delete rule".into(),
        };

        apply_fix(&mut spec, &fix).unwrap();
        assert_eq!(spec.rules.len(), 1);
        assert_eq!(spec.rules[0].id, "R1");
    }

    #[test]
    fn test_update_expression() {
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

        apply_fix(&mut spec, &fix).unwrap();
        assert_eq!(spec.rules[0].when, Some("!a".into()));
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

        let result = apply_fixes(&mut spec, &fixes, false);
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(spec.rules[0].priority, 1);
        assert_eq!(spec.rules.len(), 2); // R2 not deleted
    }
}
