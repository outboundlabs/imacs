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
            if let Some(cel_expr) = rule.as_cel() {
                if let Ok(preds) = extract_predicates(&cel_expr) {
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
///
/// Checks pairs of specs that share common variables to find gaps
/// in their combined coverage. This is faster than exhaustive analysis
/// but may miss some gaps that span more than two specs.
fn analyze_suite_gaps_incremental(specs: &[(String, &Spec)]) -> Vec<SuiteGap> {
    use crate::completeness::predicates::{extract_predicates, PredicateSet};
    use std::collections::HashSet;

    if specs.len() < 2 {
        return Vec::new();
    }

    let mut suite_gaps = Vec::new();

    // For pairwise analysis, we focus on specs with overlapping variables
    for i in 0..specs.len() {
        for j in (i + 1)..specs.len() {
            let (id_a, spec_a) = &specs[i];
            let (id_b, spec_b) = &specs[j];

            // Check if specs share any input variables
            let vars_a: HashSet<_> = spec_a.inputs.iter().map(|v| &v.name).collect();
            let vars_b: HashSet<_> = spec_b.inputs.iter().map(|v| &v.name).collect();
            let shared: Vec<_> = vars_a.intersection(&vars_b).collect();

            if shared.is_empty() {
                continue; // No shared variables, no meaningful pairwise gap
            }

            // Build combined predicate set for shared variables
            let mut combined_predicates = PredicateSet::new();

            for rule in spec_a.rules.iter().chain(spec_b.rules.iter()) {
                if let Some(cel_expr) = rule.as_cel() {
                    if let Ok(preds) = extract_predicates(&cel_expr) {
                        for pred in preds {
                            // Only include predicates involving shared variables
                            if let Some(pred_var) = get_predicate_variable(&pred) {
                                if shared.iter().any(|s| ***s == pred_var) {
                                    combined_predicates.add(pred);
                                }
                            }
                        }
                    }
                }
            }

            let n_predicates = combined_predicates.len();
            if n_predicates == 0 || n_predicates > 15 {
                continue; // Skip if too complex
            }

            // Find coverage for each spec
            let mut covered_by_a: HashSet<u64> = HashSet::new();
            let mut covered_by_b: HashSet<u64> = HashSet::new();

            for rule in &spec_a.rules {
                if let Some(cel_expr) = rule.as_cel() {
                    for combo in find_matching_combinations_for_preds(&cel_expr, &combined_predicates)
                    {
                        covered_by_a.insert(combo);
                    }
                }
            }

            for rule in &spec_b.rules {
                if let Some(cel_expr) = rule.as_cel() {
                    for combo in find_matching_combinations_for_preds(&cel_expr, &combined_predicates)
                    {
                        covered_by_b.insert(combo);
                    }
                }
            }

            // Find gaps (not covered by either spec)
            let total_combinations = 1u64 << n_predicates;
            for combo in 0..total_combinations {
                if !covered_by_a.contains(&combo) && !covered_by_b.contains(&combo) {
                    let cel_condition = build_cel_condition(combo, &combined_predicates);
                    suite_gaps.push(SuiteGap {
                        cel_condition,
                        missing_in_specs: vec![id_a.clone(), id_b.clone()],
                    });
                }
            }
        }
    }

    // Deduplicate gaps
    suite_gaps.sort_by(|a, b| a.cel_condition.cmp(&b.cel_condition));
    suite_gaps.dedup_by(|a, b| a.cel_condition == b.cel_condition);

    // Limit to first 20 gaps
    suite_gaps.truncate(20);
    suite_gaps
}

/// Exhaustive suite gap analysis
///
/// Enumerates all possible predicate combinations across all specs
/// and finds combinations that aren't covered by ANY spec in the suite.
fn analyze_suite_gaps_exhaustive(specs: &[(String, &Spec)]) -> Vec<SuiteGap> {
    use crate::completeness::predicates::{extract_predicates, PredicateSet};
    use std::collections::HashSet;

    if specs.is_empty() {
        return Vec::new();
    }

    // 1. Collect all predicates from all specs
    let mut combined_predicates = PredicateSet::new();

    for (_, spec) in specs {
        for rule in &spec.rules {
            if let Some(cel_expr) = rule.as_cel() {
                if let Ok(preds) = extract_predicates(&cel_expr) {
                    for pred in preds {
                        combined_predicates.add(pred);
                    }
                }
            }
        }
    }

    let n_predicates = combined_predicates.len();

    // Limit to reasonable size
    if n_predicates == 0 || n_predicates > 20 {
        return Vec::new(); // Too large for exhaustive enumeration
    }

    // 2. Find which combinations are covered by each spec
    let mut covered_by_any: HashSet<u64> = HashSet::new();
    let mut spec_coverage: Vec<(String, HashSet<u64>)> = Vec::new();

    for (spec_id, spec) in specs {
        let mut covered: HashSet<u64> = HashSet::new();

        for rule in &spec.rules {
            if let Some(cel_expr) = rule.as_cel() {
                for combo in find_matching_combinations_for_preds(&cel_expr, &combined_predicates) {
                    covered.insert(combo);
                    covered_by_any.insert(combo);
                }
            }
        }

        // If spec has a default, all combinations are covered
        if spec.default.is_some() {
            let total = 1u64 << n_predicates;
            for combo in 0..total {
                covered.insert(combo);
                covered_by_any.insert(combo);
            }
        }

        spec_coverage.push((spec_id.clone(), covered));
    }

    // 3. Find gaps (combinations not covered by any spec)
    let total_combinations = 1u64 << n_predicates;
    let mut suite_gaps = Vec::new();

    for combo in 0..total_combinations {
        if !covered_by_any.contains(&combo) {
            let cel_condition = build_cel_condition(combo, &combined_predicates);

            // Find which specs don't cover this
            let missing_in: Vec<String> = spec_coverage
                .iter()
                .filter(|(_, covered)| !covered.contains(&combo))
                .map(|(id, _)| id.clone())
                .collect();

            suite_gaps.push(SuiteGap {
                cel_condition,
                missing_in_specs: missing_in,
            });
        }
    }

    // Limit to first 50 gaps
    suite_gaps.truncate(50);
    suite_gaps
}

/// Find all combinations that match a CEL expression against a predicate set
fn find_matching_combinations_for_preds(
    cel_expr: &str,
    predicate_set: &crate::completeness::predicates::PredicateSet,
) -> Vec<u64> {
    use crate::cel::CelCompiler;

    let n = predicate_set.len();
    if n == 0 {
        return vec![0];
    }

    // Parse the expression
    let ast = match CelCompiler::parse(cel_expr) {
        Ok(ast) => ast,
        Err(_) => return vec![], // Can't parse - no matches
    };

    // Check each combination
    let mut matches = Vec::new();
    let total = 1u64 << n;

    for combo in 0..total {
        if expression_matches_preds(&ast, combo, predicate_set) {
            matches.push(combo);
        }
    }

    matches
}

/// Check if an expression matches a given predicate combination
fn expression_matches_preds(
    expr: &cel_parser::Expression,
    combo: u64,
    predicate_set: &crate::completeness::predicates::PredicateSet,
) -> bool {
    use crate::completeness::predicates::Predicate;
    use cel_parser::ast::operators;
    use cel_parser::ast::Expr;

    match &expr.expr {
        Expr::Ident(name) => {
            let name_str = name.to_string();
            // Check if this identifier is a predicate
            for (idx, pred) in predicate_set.predicates.iter().enumerate() {
                if let Predicate::BoolVar(var_name) = pred {
                    if var_name == &name_str {
                        return (combo >> idx) & 1 == 1;
                    }
                    if var_name == &format!("!{}", name_str) {
                        return (combo >> idx) & 1 == 0;
                    }
                }
            }
            // Unknown identifier - assume true (conservative)
            true
        }

        Expr::Call(call) => {
            // Handle logical operators
            if call.func_name == operators::LOGICAL_AND {
                if call.args.len() == 2 {
                    return expression_matches_preds(&call.args[0], combo, predicate_set)
                        && expression_matches_preds(&call.args[1], combo, predicate_set);
                }
            } else if call.func_name == operators::LOGICAL_OR {
                if call.args.len() == 2 {
                    return expression_matches_preds(&call.args[0], combo, predicate_set)
                        || expression_matches_preds(&call.args[1], combo, predicate_set);
                }
            } else if call.func_name == operators::LOGICAL_NOT {
                if let Some(inner) = call.args.first() {
                    return !expression_matches_preds(inner, combo, predicate_set);
                }
            } else if call.args.len() == 2 {
                // For comparison operators, check if they match a predicate
                let pred_cel = build_comparison_cel(&call.args[0], &call.func_name, &call.args[1]);
                for (idx, pred) in predicate_set.predicates.iter().enumerate() {
                    if pred.to_cel_string() == pred_cel {
                        return (combo >> idx) & 1 == 1;
                    }
                }
            }
            // Unknown call - assume true
            true
        }

        Expr::Literal(_) => true,
        _ => true, // Unknown - conservative assumption
    }
}

/// Build CEL string for a comparison expression
fn build_comparison_cel(
    left: &cel_parser::Expression,
    op: &str,
    right: &cel_parser::Expression,
) -> String {
    use cel_parser::ast::Expr;
    use cel_parser::reference::Val;

    let left_str = match &left.expr {
        Expr::Ident(name) => name.to_string(),
        _ => return String::new(),
    };

    let right_str = match &right.expr {
        Expr::Literal(Val::Int(i)) => i.to_string(),
        Expr::Literal(Val::String(s)) => format!("\"{}\"", s),
        Expr::Literal(Val::Boolean(b)) => b.to_string(),
        _ => return String::new(),
    };

    format!("{} {} {}", left_str, op, right_str)
}

/// Get the variable name from a predicate
fn get_predicate_variable(pred: &crate::completeness::predicates::Predicate) -> Option<String> {
    use crate::completeness::predicates::Predicate;
    match pred {
        Predicate::BoolVar(name) => Some(name.clone()),
        Predicate::Comparison { var, .. } => Some(var.clone()),
        Predicate::Equality { var, .. } => Some(var.clone()),
        Predicate::Membership { var, .. } => Some(var.clone()),
        Predicate::StringOp { var, .. } => Some(var.clone()),
    }
}

/// Build a CEL condition string from a combination bitmap
fn build_cel_condition(combo: u64, predicate_set: &crate::completeness::predicates::PredicateSet) -> String {
    let conditions: Vec<String> = predicate_set
        .predicates
        .iter()
        .enumerate()
        .map(|(idx, pred)| {
            let value = (combo >> idx) & 1 == 1;
            if value {
                pred.to_cel_string()
            } else {
                pred.negated().to_cel_string()
            }
        })
        .collect();

    conditions.join(" && ")
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
            scoping: None,
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

    fn make_spec_with_rules(id: &str, var_name: &str, rules: Vec<(&str, &str)>) -> Spec {
        Spec {
            id: id.into(),
            name: None,
            description: None,
            inputs: vec![Variable {
                name: var_name.into(),
                typ: VarType::Bool,
                description: None,
                values: None,
            }],
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::String,
                description: None,
                values: None,
            }],
            rules: rules
                .into_iter()
                .map(|(id, when)| Rule {
                    id: id.into(),
                    when: Some(when.into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::String(format!("output_{}", id))),
                    priority: 0,
                    description: None,
                })
                .collect(),
            default: None,
            meta: Default::default(),
            scoping: None,
        }
    }

    #[test]
    fn test_suite_gap_exhaustive_finds_gaps() {
        // Two specs that together don't cover all cases of shared variable 'x'
        let spec_a = make_spec_with_rules("spec_a", "x", vec![("R1", "x")]);
        let spec_b = make_spec_with_rules("spec_b", "x", vec![("R1", "x")]); // Same coverage!

        let specs = vec![
            ("spec_a".into(), spec_a),
            ("spec_b".into(), spec_b),
        ];

        let result = analyze_suite(&specs, true);

        // Should find a gap when x is false (neither spec handles !x)
        assert!(!result.suite_gaps.is_empty(), "Should find gaps when specs have incomplete coverage");
    }

    #[test]
    fn test_suite_gap_exhaustive_no_gaps_with_default() {
        // Spec with a default covers all cases
        let mut spec = make_spec_with_rules("spec_a", "x", vec![("R1", "x")]);
        spec.default = Some(Output::Single(ConditionValue::String("default".into())));

        let specs = vec![("spec_a".into(), spec)];

        let result = analyze_suite(&specs, true);

        // No gaps because default handles everything
        assert!(result.suite_gaps.is_empty(), "No gaps expected when spec has default");
    }

    #[test]
    fn test_suite_gap_exhaustive_complete_coverage() {
        // Two specs that together cover all cases
        let spec_a = make_spec_with_rules("spec_a", "x", vec![("R1", "x")]);
        let spec_b = make_spec_with_rules("spec_b", "x", vec![("R1", "!x")]);

        let specs = vec![
            ("spec_a".into(), spec_a),
            ("spec_b".into(), spec_b),
        ];

        let result = analyze_suite(&specs, true);

        // Spec A handles x=true, Spec B handles x=false
        // Together they cover all combinations
        assert!(result.suite_gaps.is_empty(), "No gaps expected when specs together cover all cases");
    }

    #[test]
    fn test_suite_gap_incremental_shared_variables() {
        // Two specs with shared variable but incomplete coverage
        let spec_a = make_spec_with_rules("spec_a", "shared_var", vec![("R1", "shared_var")]);
        let spec_b = make_spec_with_rules("spec_b", "shared_var", vec![("R1", "shared_var")]);

        let specs = vec![
            ("spec_a".into(), spec_a),
            ("spec_b".into(), spec_b),
        ];

        let result = analyze_suite(&specs, false); // Incremental mode

        // Both specs only handle shared_var=true, leaving !shared_var uncovered
        assert!(!result.suite_gaps.is_empty(), "Should detect gaps in pairwise analysis");
    }

    #[test]
    fn test_suite_gap_no_shared_variables() {
        // Two specs with NO shared variables - no pairwise gaps detected
        let spec_a = make_spec_with_rules("spec_a", "x", vec![("R1", "x")]);

        let mut spec_b = make_spec_with_rules("spec_b", "y", vec![("R1", "y")]);
        spec_b.inputs[0].name = "y".into(); // Different variable

        let specs = vec![
            ("spec_a".into(), spec_a),
            ("spec_b".into(), spec_b),
        ];

        let result = analyze_suite(&specs, false); // Incremental mode

        // No shared variables means incremental analysis skips this pair
        assert!(result.suite_gaps.is_empty(), "No gaps for non-overlapping specs in incremental mode");
    }
}
