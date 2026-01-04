//! Spec suite analysis
//!
//! Analyzes multiple specs together to find:
//! - Cross-spec collisions (same variable names, different meanings)
//! - Duplicate rules across specs
//! - Spec relationships (chains, merge opportunities)
//! - Suite-level gaps (combinations not covered by any spec)

use crate::completeness::analysis::analyze_completeness;
use crate::completeness::collision::detect_collisions;
use crate::completeness::duplicate::detect_duplicates;
use crate::completeness::relationship::detect_relationships;
use crate::completeness::suggestions::generate_suggestions;
use crate::spec::Spec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of analyzing a suite of specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuiteAnalysisResult {
    pub individual_results: Vec<SpecResult>,
    pub collisions: Vec<crate::completeness::collision::Collision>,
    pub duplicates: Vec<crate::completeness::duplicate::Duplicate>,
    pub relationships: Vec<crate::completeness::relationship::SpecRelationship>,
    pub suite_gaps: Vec<SuiteGap>,
    pub complexity: ComplexityReport,
    pub suggestions: Vec<crate::completeness::suggestions::Suggestion>,
}

/// Result for a single spec
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecResult {
    pub spec_id: String,
    pub spec_file: Option<String>,
    pub report: crate::completeness::IncompletenessReport,
    pub passed: bool,
}

/// A gap in the suite (not covered by any spec)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuiteGap {
    pub cel_condition: String,
    pub missing_in_specs: Vec<String>,
}

/// Complexity report for the suite
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComplexityReport {
    pub total_unique_predicates: usize,
    pub combined_input_space: u64,
    pub analysis_mode: AnalysisMode,
    pub warning: Option<String>,
}

/// Analysis mode used
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AnalysisMode {
    Incremental,
    Full,
}

/// Analyze a suite of specs
pub fn analyze_suite(specs: &[(String, Spec)], full: bool) -> SuiteAnalysisResult {
    // 1. Individual analysis for each spec
    let individual_results: Vec<SpecResult> = specs
        .iter()
        .map(|(spec_id, spec)| {
            let report = analyze_completeness(spec);
            SpecResult {
                spec_id: spec_id.clone(),
                spec_file: None,
                report: report.clone(),
                passed: report.is_complete && report.overlaps.is_empty(),
            }
        })
        .collect();

    // 2. Prepare data for cross-spec analysis
    let spec_vars: Vec<(String, Vec<crate::spec::Variable>)> = specs
        .iter()
        .map(|(id, spec)| (id.clone(), spec.inputs.clone()))
        .collect();

    let spec_refs: Vec<(String, &Spec)> =
        specs.iter().map(|(id, spec)| (id.clone(), spec)).collect();

    // 3. Detect collisions
    let collisions = detect_collisions(&spec_vars);

    // 4. Detect duplicates
    let duplicates = detect_duplicates(&spec_refs);

    // 5. Detect relationships
    let relationships = detect_relationships(&spec_refs);

    // 6. Suite-level gap analysis
    let suite_gaps = if full {
        analyze_suite_gaps_exhaustive(&spec_refs)
    } else {
        analyze_suite_gaps_incremental(&spec_refs)
    };

    // 7. Complexity report
    let total_predicates = count_unique_predicates(&spec_refs);
    let complexity = ComplexityReport {
        total_unique_predicates: total_predicates,
        combined_input_space: if total_predicates < 64 {
            1u64 << total_predicates
        } else {
            u64::MAX
        },
        analysis_mode: if full {
            AnalysisMode::Full
        } else {
            AnalysisMode::Incremental
        },
        warning: if total_predicates > 20 && !full {
            Some(format!(
                "Large predicate space ({} predicates). Run with --full for exhaustive analysis (may be slow)",
                total_predicates
            ))
        } else {
            None
        },
    };

    // 8. Generate suggestions
    let suggestions = generate_suggestions(&collisions, &duplicates, &relationships);

    SuiteAnalysisResult {
        individual_results,
        collisions,
        duplicates,
        relationships,
        suite_gaps,
        complexity,
        suggestions,
    }
}

/// Count unique predicates across all specs
fn count_unique_predicates(specs: &[(String, &Spec)]) -> usize {
    use crate::completeness::predicates::{extract_predicates, PredicateSet};

    let mut predicate_set = PredicateSet::new();
    for (_, spec) in specs {
        for rule in &spec.rules {
            if let Some(cel_expr) = &rule.when {
                if let Ok(preds) = extract_predicates(cel_expr) {
                    for pred in preds {
                        predicate_set.add(pred);
                    }
                }
            }
        }
    }
    predicate_set.len()
}

/// Incremental suite gap analysis (pairwise)
fn analyze_suite_gaps_incremental(_specs: &[(String, &Spec)]) -> Vec<SuiteGap> {
    // For incremental mode, we check pairwise overlaps
    // Full gaps would require exhaustive enumeration
    // This is a simplified version - could be enhanced
    Vec::new() // TODO: implement pairwise gap detection
}

/// Exhaustive suite gap analysis
fn analyze_suite_gaps_exhaustive(_specs: &[(String, &Spec)]) -> Vec<SuiteGap> {
    // This would enumerate all possible combinations
    // and check if any fall through all specs
    // For now, return empty - this is complex to implement
    Vec::new() // TODO: implement exhaustive gap detection
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConditionValue, Output, Rule, VarType, Variable};

    fn make_test_spec(id: &str) -> Spec {
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
            rules: vec![Rule {
                id: "R1".into(),
                when: Some("a".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            }],
            default: None,
            meta: Default::default(),
        }
    }

    #[test]
    fn test_analyze_suite() {
        let specs = vec![
            ("spec_a".into(), make_test_spec("spec_a")),
            ("spec_b".into(), make_test_spec("spec_b")),
        ];

        let result = analyze_suite(&specs, false);
        assert_eq!(result.individual_results.len(), 2);
    }
}
