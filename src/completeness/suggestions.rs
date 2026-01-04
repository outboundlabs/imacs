//! Generate actionable suggestions from analysis findings
//!
//! Takes collisions, duplicates, and relationships and generates
//! concrete suggestions for fixing issues.

use crate::completeness::collision::{Collision, CollisionType};
use crate::completeness::duplicate::Duplicate;
use crate::completeness::relationship::{RelationshipType, SpecRelationship};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A suggestion for fixing an issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Suggestion {
    pub code: String,
    pub category: SuggestionCategory,
    pub description: String,
    pub affected_specs: Vec<String>,
    pub fix: SuggestedFix,
}

/// Category of suggestion
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SuggestionCategory {
    Rename,
    Namespace,
    Merge,
    Extract,
    DefineChain,
}

/// A concrete fix suggestion
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SuggestedFix {
    Rename {
        from: String,
        to: String,
        in_spec: String,
    },
    Namespace {
        prefix: String,
        variables: Vec<String>,
    },
    Merge {
        specs: Vec<String>,
        into: String,
    },
    Extract {
        rules: Vec<String>,
        into: String,
    },
    DefineChain {
        specs: Vec<String>,
        as_orchestrator: String,
    },
}

/// Generate suggestions from analysis findings
pub fn generate_suggestions(
    collisions: &[Collision],
    duplicates: &[Duplicate],
    relationships: &[SpecRelationship],
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    let mut code_counter = 1;

    // Generate suggestions from collisions
    for collision in collisions {
        match &collision.collision_type {
            CollisionType::SameNameDifferentValues | CollisionType::SameNameDifferentTypes => {
                // Suggest renaming or namespacing
                let affected_specs: Vec<String> = collision
                    .occurrences
                    .iter()
                    .map(|occ| occ.spec_id.clone())
                    .collect();

                // Suggestion 1: Rename to domain-specific names
                for occ in &collision.occurrences {
                    let new_name = format!("{}_{}", occ.spec_id, collision.variable_name);
                    suggestions.push(Suggestion {
                        code: format!("S{:03}", code_counter),
                        category: SuggestionCategory::Rename,
                        description: format!(
                            "Rename '{}' to '{}' in {}",
                            collision.variable_name, new_name, occ.spec_id
                        ),
                        affected_specs: vec![occ.spec_id.clone()],
                        fix: SuggestedFix::Rename {
                            from: collision.variable_name.clone(),
                            to: new_name,
                            in_spec: occ.spec_id.clone(),
                        },
                    });
                    code_counter += 1;
                }

                // Suggestion 2: Namespace
                suggestions.push(Suggestion {
                    code: format!("S{:03}", code_counter),
                    category: SuggestionCategory::Namespace,
                    description: format!(
                        "Add namespace prefix to '{}' in all specs",
                        collision.variable_name
                    ),
                    affected_specs: affected_specs.clone(),
                    fix: SuggestedFix::Namespace {
                        prefix: "domain".into(), // TODO: infer from spec context
                        variables: vec![collision.variable_name.clone()],
                    },
                });
                code_counter += 1;
            }
            CollisionType::AmbiguousSemantics => {
                // Suggest adding values or renaming
                let affected_specs: Vec<String> = collision
                    .occurrences
                    .iter()
                    .map(|occ| occ.spec_id.clone())
                    .collect();
                suggestions.push(Suggestion {
                    code: format!("S{:03}", code_counter),
                    category: SuggestionCategory::Rename,
                    description: format!(
                        "Clarify semantics of '{}' by renaming or adding enum values",
                        collision.variable_name
                    ),
                    affected_specs,
                    fix: SuggestedFix::Rename {
                        from: collision.variable_name.clone(),
                        to: format!("clarified_{}", collision.variable_name),
                        in_spec: "all".into(),
                    },
                });
                code_counter += 1;
            }
        }
    }

    // Generate suggestions from duplicates
    for duplicate in duplicates {
        suggestions.push(Suggestion {
            code: format!("S{:03}", code_counter),
            category: SuggestionCategory::Extract,
            description: format!(
                "Extract duplicate rules {}:{} and {}:{} to shared spec",
                duplicate.rule_a.spec_id,
                duplicate.rule_a.rule_id,
                duplicate.rule_b.spec_id,
                duplicate.rule_b.rule_id
            ),
            affected_specs: vec![
                duplicate.rule_a.spec_id.clone(),
                duplicate.rule_b.spec_id.clone(),
            ],
            fix: SuggestedFix::Extract {
                rules: vec![
                    format!("{}:{}", duplicate.rule_a.spec_id, duplicate.rule_a.rule_id),
                    format!("{}:{}", duplicate.rule_b.spec_id, duplicate.rule_b.rule_id),
                ],
                into: format!(
                    "shared_{}_{}",
                    duplicate.rule_a.spec_id, duplicate.rule_b.spec_id
                ),
            },
        });
        code_counter += 1;
    }

    // Generate suggestions from relationships
    for relationship in relationships {
        match &relationship.relationship_type {
            RelationshipType::Chain => {
                suggestions.push(Suggestion {
                    code: format!("S{:03}", code_counter),
                    category: SuggestionCategory::DefineChain,
                    description: format!(
                        "Define orchestrator chain: {} → {}",
                        relationship.spec_a, relationship.spec_b
                    ),
                    affected_specs: vec![relationship.spec_a.clone(), relationship.spec_b.clone()],
                    fix: SuggestedFix::DefineChain {
                        specs: vec![relationship.spec_a.clone(), relationship.spec_b.clone()],
                        as_orchestrator: format!("{}_chain", relationship.spec_a),
                    },
                });
                code_counter += 1;
            }
            RelationshipType::MergeOpportunity => {
                suggestions.push(Suggestion {
                    code: format!("S{:03}", code_counter),
                    category: SuggestionCategory::Merge,
                    description: format!(
                        "Consider merging {} and {} ({}% variable overlap)",
                        relationship.spec_a,
                        relationship.spec_b,
                        (relationship.details.overlap_ratio * 100.0) as u32
                    ),
                    affected_specs: vec![relationship.spec_a.clone(), relationship.spec_b.clone()],
                    fix: SuggestedFix::Merge {
                        specs: vec![relationship.spec_a.clone(), relationship.spec_b.clone()],
                        into: format!("merged_{}_{}", relationship.spec_a, relationship.spec_b),
                    },
                });
                code_counter += 1;
            }
        }
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completeness::collision::{CollisionType, VariableOccurrence};
    use crate::spec::{VarType, Variable};

    #[test]
    fn test_generate_rename_suggestion() {
        let collisions = vec![Collision {
            variable_name: "customer_type".into(),
            occurrences: vec![VariableOccurrence {
                spec_id: "pricing".into(),
                variable: Variable {
                    name: "customer_type".into(),
                    typ: VarType::String,
                    description: None,
                    values: Some(vec!["standard".into()]),
                },
            }],
            collision_type: CollisionType::SameNameDifferentValues,
        }];

        let suggestions = generate_suggestions(&collisions, &[], &[]);
        assert!(!suggestions.is_empty());
    }
}
