//! Variable collision detection across specs
//!
//! Detects when the same variable name is used in multiple specs with
//! different meanings, types, or values - indicating a naming conflict.

use crate::spec::Variable;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A collision detected between specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Collision {
    pub variable_name: String,
    pub occurrences: Vec<VariableOccurrence>,
    pub collision_type: CollisionType,
}

/// Type of collision detected
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CollisionType {
    /// Same name, different enum values
    SameNameDifferentValues,
    /// Same name, different types (int vs string)
    SameNameDifferentTypes,
    /// Same name, unclear if same meaning (no values defined)
    AmbiguousSemantics,
}

/// An occurrence of a variable in a spec
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableOccurrence {
    pub spec_id: String,
    pub variable: Variable,
}

/// Detect collisions across specs
pub fn detect_collisions(specs: &[(String, Vec<Variable>)]) -> Vec<Collision> {
    let mut var_map: HashMap<String, Vec<VariableOccurrence>> = HashMap::new();

    // Index all variables by name
    for (spec_id, vars) in specs {
        for var in vars {
            var_map
                .entry(var.name.clone())
                .or_default()
                .push(VariableOccurrence {
                    spec_id: spec_id.clone(),
                    variable: var.clone(),
                });
        }
    }

    // Find collisions (same name in multiple specs)
    let mut collisions = Vec::new();
    for (var_name, occurrences) in var_map {
        if occurrences.len() > 1 {
            // Check if they're actually different
            if let Some(collision_type) = classify_collision(&occurrences) {
                collisions.push(Collision {
                    variable_name: var_name,
                    occurrences,
                    collision_type,
                });
            }
        }
    }

    collisions
}

/// Classify the type of collision
fn classify_collision(occurrences: &[VariableOccurrence]) -> Option<CollisionType> {
    if occurrences.len() < 2 {
        return None;
    }

    let first = &occurrences[0].variable;

    // Check for type differences
    let has_type_diff = occurrences.iter().any(|occ| occ.variable.typ != first.typ);
    if has_type_diff {
        return Some(CollisionType::SameNameDifferentTypes);
    }

    // Check for value differences
    if let Some(first_vals) = &first.values {
        let has_value_diff = occurrences.iter().any(|occ| {
            if let Some(occ_vals) = &occ.variable.values {
                occ_vals != first_vals
            } else {
                false
            }
        });
        if has_value_diff {
            return Some(CollisionType::SameNameDifferentValues);
        }
    } else {
        // No values defined - ambiguous
        return Some(CollisionType::AmbiguousSemantics);
    }

    // If all same, no collision
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::VarType;

    #[test]
    fn test_detect_type_collision() {
        let specs = vec![
            (
                "spec_a".into(),
                vec![Variable {
                    name: "customer_type".into(),
                    typ: VarType::String,
                    description: None,
                    values: Some(vec!["standard".into()]),
                }],
            ),
            (
                "spec_b".into(),
                vec![Variable {
                    name: "customer_type".into(),
                    typ: VarType::Int,
                    description: None,
                    values: None,
                }],
            ),
        ];

        let collisions = detect_collisions(&specs);
        assert_eq!(collisions.len(), 1);
        assert!(matches!(
            collisions[0].collision_type,
            CollisionType::SameNameDifferentTypes
        ));
    }

    #[test]
    fn test_detect_value_collision() {
        let specs = vec![
            (
                "spec_a".into(),
                vec![Variable {
                    name: "customer_type".into(),
                    typ: VarType::String,
                    description: None,
                    values: Some(vec!["standard".into(), "premium".into()]),
                }],
            ),
            (
                "spec_b".into(),
                vec![Variable {
                    name: "customer_type".into(),
                    typ: VarType::String,
                    description: None,
                    values: Some(vec!["new".into(), "returning".into()]),
                }],
            ),
        ];

        let collisions = detect_collisions(&specs);
        assert_eq!(collisions.len(), 1);
        assert!(matches!(
            collisions[0].collision_type,
            CollisionType::SameNameDifferentValues
        ));
    }
}
