//! Spec relationship detection
//!
//! Detects:
//! - Chains: output of spec A matches input of spec B
//! - Merge opportunities: specs with overlapping variables that could be combined

use crate::spec::Spec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A relationship between specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecRelationship {
    pub relationship_type: RelationshipType,
    pub spec_a: String,
    pub spec_b: String,
    pub details: RelationshipDetails,
}

/// Type of relationship
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RelationshipType {
    /// Chain: output of A matches input of B
    Chain,
    /// Merge opportunity: high variable overlap
    MergeOpportunity,
}

/// Details about the relationship
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationshipDetails {
    pub shared_variables: Vec<String>,
    pub output_to_input_mapping: Vec<OutputInputMapping>,
    pub overlap_ratio: f64,
}

/// Mapping from output to input
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OutputInputMapping {
    pub output_name: String,
    pub input_name: String,
}

/// Detect relationships between specs
pub fn detect_relationships(specs: &[(String, &Spec)]) -> Vec<SpecRelationship> {
    let mut relationships = Vec::new();

    for (i, (spec_a_id, spec_a)) in specs.iter().enumerate() {
        for (spec_b_id, spec_b) in specs.iter().skip(i + 1) {
            // Check for chain relationship
            if let Some(chain_details) = check_chain(spec_a, spec_b) {
                relationships.push(SpecRelationship {
                    relationship_type: RelationshipType::Chain,
                    spec_a: spec_a_id.clone(),
                    spec_b: spec_b_id.clone(),
                    details: chain_details,
                });
            }

            // Check for merge opportunity
            if let Some(merge_details) = check_merge_opportunity(spec_a, spec_b) {
                relationships.push(SpecRelationship {
                    relationship_type: RelationshipType::MergeOpportunity,
                    spec_a: spec_a_id.clone(),
                    spec_b: spec_b_id.clone(),
                    details: merge_details,
                });
            }
        }
    }

    relationships
}

/// Check if spec A's outputs match spec B's inputs (chain relationship)
fn check_chain(spec_a: &Spec, spec_b: &Spec) -> Option<RelationshipDetails> {
    let mut mappings = Vec::new();
    let mut shared = Vec::new();

    for output in &spec_a.outputs {
        for input in &spec_b.inputs {
            if output.name == input.name && output.typ == input.typ {
                mappings.push(OutputInputMapping {
                    output_name: output.name.clone(),
                    input_name: input.name.clone(),
                });
                shared.push(output.name.clone());
            }
        }
    }

    if !mappings.is_empty() {
        Some(RelationshipDetails {
            shared_variables: shared,
            output_to_input_mapping: mappings,
            overlap_ratio: 1.0, // All outputs match inputs
        })
    } else {
        None
    }
}

/// Check if specs have high variable overlap (merge opportunity)
fn check_merge_opportunity(spec_a: &Spec, spec_b: &Spec) -> Option<RelationshipDetails> {
    let vars_a: HashSet<String> = spec_a.inputs.iter().map(|v| v.name.clone()).collect();
    let vars_b: HashSet<String> = spec_b.inputs.iter().map(|v| v.name.clone()).collect();

    let intersection: HashSet<_> = vars_a.intersection(&vars_b).cloned().collect();
    let union: HashSet<_> = vars_a.union(&vars_b).cloned().collect();

    if union.is_empty() {
        return None;
    }

    let overlap_ratio = intersection.len() as f64 / union.len() as f64;

    // Consider merge opportunity if >= 50% overlap (2/4 = 0.5)
    if overlap_ratio >= 0.5 {
        Some(RelationshipDetails {
            shared_variables: intersection.into_iter().collect(),
            output_to_input_mapping: Vec::new(),
            overlap_ratio,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{VarType, Variable};

    fn make_test_spec(id: &str, inputs: Vec<Variable>, outputs: Vec<Variable>) -> Spec {
        Spec {
            id: id.into(),
            name: None,
            description: None,
            inputs,
            outputs,
            rules: vec![],
            default: None,
            meta: Default::default(),
        }
    }

    #[test]
    fn test_detect_chain() {
        let spec_a = make_test_spec(
            "spec_a",
            vec![],
            vec![Variable {
                name: "discount_rate".into(),
                typ: VarType::Float,
                description: None,
                values: None,
            }],
        );
        let spec_b = make_test_spec(
            "spec_b",
            vec![Variable {
                name: "discount_rate".into(),
                typ: VarType::Float,
                description: None,
                values: None,
            }],
            vec![],
        );

        let specs = vec![("spec_a".into(), &spec_a), ("spec_b".into(), &spec_b)];

        let relationships = detect_relationships(&specs);
        assert!(!relationships.is_empty());
        assert!(relationships
            .iter()
            .any(|r| matches!(r.relationship_type, RelationshipType::Chain)));
    }

    #[test]
    fn test_detect_merge_opportunity() {
        let spec_a = make_test_spec(
            "spec_a",
            vec![
                Variable {
                    name: "a".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
                Variable {
                    name: "b".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
                Variable {
                    name: "c".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
            ],
            vec![],
        );
        let spec_b = make_test_spec(
            "spec_b",
            vec![
                Variable {
                    name: "a".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
                Variable {
                    name: "b".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
                Variable {
                    name: "d".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
            ],
            vec![],
        );

        let specs = vec![("spec_a".into(), &spec_a), ("spec_b".into(), &spec_b)];

        let relationships = detect_relationships(&specs);
        assert!(!relationships.is_empty());
        assert!(relationships
            .iter()
            .any(|r| matches!(r.relationship_type, RelationshipType::MergeOpportunity)));
    }
}
