//! Duplicate rule detection across specs
//!
//! Finds rules in different specs that cover the same input space,
//! suggesting they might be duplicates or should be extracted to a shared spec.

use crate::completeness::predicates::{extract_predicates, PredicateSet};
use crate::spec::Spec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A duplicate detected between specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Duplicate {
    pub rule_a: RuleRef,
    pub rule_b: RuleRef,
    pub overlap_cel: String,
    pub confidence: f64,
}

/// Reference to a rule in a spec
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuleRef {
    pub spec_id: String,
    pub rule_id: String,
    pub cel_condition: Option<String>,
}

/// Detect duplicate rules across specs
pub fn detect_duplicates(specs: &[(String, &Spec)]) -> Vec<Duplicate> {
    let mut duplicates = Vec::new();

    // Build predicate sets for each spec
    let mut spec_predicates: Vec<(String, PredicateSet)> = Vec::new();
    for (spec_id, spec) in specs {
        let mut predicate_set = PredicateSet::new();
        for rule in &spec.rules {
            if let Some(cel_expr) = rule.as_cel() {
                if let Ok(preds) = extract_predicates(&cel_expr) {
                    for pred in preds {
                        predicate_set.add(pred);
                    }
                }
            }
        }
        spec_predicates.push((spec_id.clone(), predicate_set));
    }

    // Compare all pairs of specs
    for (i, (spec_a_id, spec_a)) in specs.iter().enumerate() {
        for (j, (spec_b_id, spec_b)) in specs.iter().enumerate().skip(i + 1) {
            // Find common predicates
            let pred_set_a = &spec_predicates[i].1;
            let pred_set_b = &spec_predicates[j].1;

            // Build combined predicate set
            let mut combined_set = PredicateSet::new();
            for pred in &pred_set_a.predicates {
                combined_set.add(pred.clone());
            }
            for pred in &pred_set_b.predicates {
                combined_set.add(pred.clone());
            }

            // Compare rules
            for rule_a in &spec_a.rules {
                for rule_b in &spec_b.rules {
                    if let (Some(cel_a), Some(cel_b)) = (rule_a.as_cel(), rule_b.as_cel()) {
                        if let Some((overlap, confidence)) =
                            check_rule_overlap(&cel_a, &cel_b, &combined_set)
                        {
                            duplicates.push(Duplicate {
                                rule_a: RuleRef {
                                    spec_id: spec_a_id.clone(),
                                    rule_id: rule_a.id.clone(),
                                    cel_condition: Some(cel_a.clone()),
                                },
                                rule_b: RuleRef {
                                    spec_id: spec_b_id.clone(),
                                    rule_id: rule_b.id.clone(),
                                    cel_condition: Some(cel_b.clone()),
                                },
                                overlap_cel: overlap,
                                confidence,
                            });
                        }
                    }
                }
            }
        }
    }

    duplicates
}

/// Check if two CEL expressions overlap (cover same input space)
/// Returns (overlap_cel, confidence) if overlap exists
fn check_rule_overlap(
    cel_a: &str,
    cel_b: &str,
    predicate_set: &PredicateSet,
) -> Option<(String, f64)> {
    // Convert both to cubes
    let cube_a = match crate::completeness::adapter::expression_to_cube(cel_a, predicate_set) {
        Ok(c) => c,
        Err(_) => return None,
    };
    let cube_b = match crate::completeness::adapter::expression_to_cube(cel_b, predicate_set) {
        Ok(c) => c,
        Err(_) => return None,
    };

    // Check if cubes overlap (both have 1 for same predicates)
    let mut overlap_predicates = Vec::new();
    let mut matching_predicates = 0;
    let mut total_set_predicates = 0;
    let mut conflicting_predicates = 0;

    for (idx, pred) in predicate_set.predicates.iter().enumerate() {
        let val_a = cube_a.input(idx);
        let val_b = cube_b.input(idx);

        let dc = crate::completeness::espresso::CubeValue::DontCare;

        // Count predicates that are set (not don't-care) in either rule
        if val_a != dc || val_b != dc {
            total_set_predicates += 1;
        }

        // Both set and same value = matching
        if val_a == val_b && val_a != dc {
            matching_predicates += 1;
            if val_a == crate::completeness::espresso::CubeValue::One {
                overlap_predicates.push(pred.to_cel_string());
            } else {
                overlap_predicates.push(format!("!{}", pred.to_cel_string()));
            }
        }
        // Both set but different value = conflict
        else if val_a != dc && val_b != dc && val_a != val_b {
            conflicting_predicates += 1;
        }
    }

    // Rules with conflicts can't be duplicates
    if conflicting_predicates > 0 {
        return None;
    }

    // Need some overlap to be considered duplicate
    if matching_predicates == 0 {
        return None;
    }

    // Compute confidence based on how similar the rules are
    // Higher confidence = more predicates matching, fewer don't-cares
    let confidence = if total_set_predicates > 0 {
        // Base confidence on matching ratio
        let base = matching_predicates as f64 / total_set_predicates as f64;

        // Boost confidence if rules are identical (same number of predicates)
        if matching_predicates == total_set_predicates {
            1.0 // Exact duplicate
        } else {
            // Scale between 0.5 and 0.95 based on overlap
            0.5 + (base * 0.45)
        }
    } else {
        0.5 // Default for edge case
    };

    Some((overlap_predicates.join(" && "), confidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConditionValue, Output, Rule, VarType, Variable};

    fn make_test_spec(id: &str, rules: Vec<Rule>) -> Spec {
        Spec {
            id: id.into(),
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
            rules,
            default: None,
            meta: Default::default(),
            scoping: None,
        }
    }

    #[test]
    fn test_detect_exact_duplicate() {
        let spec_a = make_test_spec(
            "spec_a",
            vec![Rule {
                id: "R1".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            }],
        );
        let spec_b = make_test_spec(
            "spec_b",
            vec![Rule {
                id: "R2".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(2)),
                priority: 0,
                description: None,
            }],
        );

        let specs = vec![("spec_a".into(), &spec_a), ("spec_b".into(), &spec_b)];

        let duplicates = detect_duplicates(&specs);
        assert!(!duplicates.is_empty());
    }
}
