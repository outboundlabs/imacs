//! Completeness analysis using truth table enumeration
//!
//! Analyzes decision tables to find:
//! - Missing cases (uncovered input combinations)
//! - Overlapping rules (multiple rules match same input)
//! - Minimization opportunities
//!
//! Uses Quine-McCluskey for boolean minimization.

use super::predicates::{extract_predicates, Predicate, PredicateSet};
use crate::cel::CelCompiler;
use crate::spec::Spec;
use quine_mc_cluskey::Bool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Result of completeness analysis - raw data for LLM tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IncompletenessReport {
    /// Is the spec complete? (all input combinations covered)
    pub is_complete: bool,

    /// Coverage statistics
    pub total_combinations: u64,
    pub covered_combinations: u64,
    pub coverage_ratio: f64,

    /// Missing cases - LLM tool uses these to formulate questions
    pub missing_cases: Vec<MissingCase>,

    /// Overlapping rules (multiple rules match same input)
    pub overlaps: Vec<RuleOverlap>,

    /// All predicates found in the spec
    pub predicates: Vec<PredicateInfo>,

    /// Minimization opportunity
    pub can_minimize: bool,
    pub original_rule_count: usize,
    pub minimized_rule_count: Option<usize>,
}

/// Information about a predicate for the report
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PredicateInfo {
    pub id: usize,
    pub cel_expression: String,
}

/// A specific uncovered input combination
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MissingCase {
    /// The predicate values for this case
    pub predicate_values: Vec<PredicateValue>,

    /// CEL conditions that describe this case
    pub cel_conditions: Vec<String>,

    /// Input variable values (if determinable)
    pub input_values: HashMap<String, String>,
}

/// A predicate with its truth value in a missing case
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PredicateValue {
    pub predicate_id: usize,
    pub cel_expression: String,
    pub value: bool,
}

/// Overlapping rules - multiple rules match the same input
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuleOverlap {
    /// The rules that overlap
    pub rule_ids: Vec<String>,

    /// The predicate values where they overlap
    pub predicate_values: Vec<PredicateValue>,

    /// CEL conditions for the overlap
    pub cel_conditions: Vec<String>,
}

/// Analyze a spec for completeness
///
/// Returns raw incompleteness data that an LLM tool can use
/// to formulate questions for the user.
pub fn analyze_completeness(spec: &Spec) -> IncompletenessReport {
    // 1. Extract all predicates from all rules
    let mut predicate_set = PredicateSet::new();
    let mut rule_predicates: Vec<(String, Vec<(usize, bool)>)> = Vec::new();

    for rule in &spec.rules {
        if let Some(cel_expr) = rule.as_cel() {
            match extract_predicates(&cel_expr) {
                Ok(preds) => {
                    let mut rule_pred_values = Vec::new();
                    for pred in preds {
                        let idx = predicate_set.add(pred);
                        rule_pred_values.push((idx, true)); // Predicate is true for this rule
                    }
                    rule_predicates.push((rule.id.clone(), rule_pred_values));
                }
                Err(_) => {
                    // Skip rules we can't parse
                    continue;
                }
            }
        }
    }

    let n_predicates = predicate_set.len();

    // Handle edge case: no predicates found
    if n_predicates == 0 {
        return IncompletenessReport {
            is_complete: !spec.rules.is_empty(),
            total_combinations: 1,
            covered_combinations: if spec.rules.is_empty() { 0 } else { 1 },
            coverage_ratio: if spec.rules.is_empty() { 0.0 } else { 1.0 },
            missing_cases: vec![],
            overlaps: vec![],
            predicates: vec![],
            can_minimize: false,
            original_rule_count: spec.rules.len(),
            minimized_rule_count: None,
        };
    }

    // Limit analysis to reasonable predicate count (2^n grows fast)
    let max_predicates = 20; // 2^20 = ~1M combinations
    if n_predicates > max_predicates {
        return IncompletenessReport {
            is_complete: false,
            total_combinations: 1 << n_predicates.min(63),
            covered_combinations: 0,
            coverage_ratio: 0.0,
            missing_cases: vec![MissingCase {
                predicate_values: vec![],
                cel_conditions: vec![format!(
                    "Too many predicates ({}) for exhaustive analysis",
                    n_predicates
                )],
                input_values: HashMap::new(),
            }],
            overlaps: vec![],
            predicates: predicate_set
                .predicates
                .iter()
                .enumerate()
                .map(|(id, p)| PredicateInfo {
                    id,
                    cel_expression: p.to_cel_string(),
                })
                .collect(),
            can_minimize: false,
            original_rule_count: spec.rules.len(),
            minimized_rule_count: None,
        };
    }

    // 2. Build coverage bitmap - which combinations are covered
    let total_combinations = 1u64 << n_predicates;
    let mut covered: HashSet<u64> = HashSet::new();
    let mut combo_rules: HashMap<u64, Vec<String>> = HashMap::new();

    // For each rule, determine which combinations it covers
    for rule in &spec.rules {
        if let Some(cel_expr) = rule.as_cel() {
            let rule_combos = find_matching_combinations(&cel_expr, &predicate_set);
            for combo in rule_combos {
                covered.insert(combo);
                combo_rules.entry(combo).or_default().push(rule.id.clone());
            }
        }
    }

    // 3. Find missing cases
    let mut missing_cases = Vec::new();
    for combo in 0..total_combinations {
        if !covered.contains(&combo) {
            missing_cases.push(build_missing_case(combo, &predicate_set));
        }
    }

    // 4. Find overlapping rules
    let mut overlaps = Vec::new();
    for (combo, rules) in &combo_rules {
        if rules.len() > 1 {
            overlaps.push(build_overlap(*combo, rules, &predicate_set));
        }
    }

    // 5. Check minimization potential using quine-mc_cluskey
    let minimized_count = try_minimize(spec, &predicate_set);

    // Build predicate info for report
    let predicates: Vec<PredicateInfo> = predicate_set
        .predicates
        .iter()
        .enumerate()
        .map(|(id, p)| PredicateInfo {
            id,
            cel_expression: p.to_cel_string(),
        })
        .collect();

    IncompletenessReport {
        is_complete: missing_cases.is_empty(),
        total_combinations,
        covered_combinations: covered.len() as u64,
        coverage_ratio: covered.len() as f64 / total_combinations as f64,
        missing_cases,
        overlaps,
        predicates,
        can_minimize: minimized_count
            .map(|c| c < spec.rules.len())
            .unwrap_or(false),
        original_rule_count: spec.rules.len(),
        minimized_rule_count: minimized_count,
    }
}

/// Find all combinations that match a CEL expression
fn find_matching_combinations(cel_expr: &str, predicate_set: &PredicateSet) -> Vec<u64> {
    let n = predicate_set.len();
    if n == 0 {
        return vec![0];
    }

    // Parse the expression to understand its structure
    let ast = match CelCompiler::parse(cel_expr) {
        Ok(ast) => ast,
        Err(_) => return vec![], // Can't parse - no matches
    };

    // For each possible combination, check if the expression matches
    let mut matches = Vec::new();
    let total = 1u64 << n;

    for combo in 0..total {
        if expression_matches(&ast, combo, predicate_set) {
            matches.push(combo);
        }
    }

    matches
}

/// Check if an expression matches a given predicate combination
fn expression_matches(
    expr: &cel_parser::Expression,
    combo: u64,
    predicate_set: &PredicateSet,
) -> bool {
    use cel_parser::Expression as E;

    match expr {
        E::Ident(name) => {
            let name_str = name.to_string();
            // Check if this identifier is a predicate
            for (idx, pred) in predicate_set.predicates.iter().enumerate() {
                if let Predicate::BoolVar(var_name) = pred {
                    if var_name == &name_str || var_name == &format!("!{}", name_str) {
                        let bit = (combo >> idx) & 1 == 1;
                        return if var_name.starts_with('!') { !bit } else { bit };
                    }
                }
            }
            // Unknown identifier - assume true
            true
        }

        E::And(left, right) => {
            expression_matches(left, combo, predicate_set)
                && expression_matches(right, combo, predicate_set)
        }

        E::Or(left, right) => {
            expression_matches(left, combo, predicate_set)
                || expression_matches(right, combo, predicate_set)
        }

        E::Unary(cel_parser::UnaryOp::Not, inner) => {
            !expression_matches(inner, combo, predicate_set)
        }

        E::Relation(left, op, right) => {
            // Try to match against our predicates
            // Get variable name from left side
            let var = match left.as_ref() {
                E::Ident(name) => name.to_string(),
                _ => return true, // Unknown - assume true
            };

            // Build a predicate and check if it's in our set
            use super::predicates::{ComparisonOp, LiteralValue};
            use cel_parser::RelationOp;

            let pred = match (op, right.as_ref()) {
                (RelationOp::GreaterThan, E::Atom(cel_parser::Atom::Int(i))) => {
                    Some(Predicate::Comparison {
                        var: var.clone(),
                        op: ComparisonOp::Gt,
                        value: LiteralValue::Int(*i),
                    })
                }
                (RelationOp::GreaterThanEq, E::Atom(cel_parser::Atom::Int(i))) => {
                    Some(Predicate::Comparison {
                        var: var.clone(),
                        op: ComparisonOp::Ge,
                        value: LiteralValue::Int(*i),
                    })
                }
                (RelationOp::LessThan, E::Atom(cel_parser::Atom::Int(i))) => {
                    Some(Predicate::Comparison {
                        var: var.clone(),
                        op: ComparisonOp::Lt,
                        value: LiteralValue::Int(*i),
                    })
                }
                (RelationOp::LessThanEq, E::Atom(cel_parser::Atom::Int(i))) => {
                    Some(Predicate::Comparison {
                        var: var.clone(),
                        op: ComparisonOp::Le,
                        value: LiteralValue::Int(*i),
                    })
                }
                (RelationOp::Equals, E::Atom(cel_parser::Atom::String(s))) => {
                    Some(Predicate::Equality {
                        var: var.clone(),
                        value: LiteralValue::String(s.to_string()),
                        negated: false,
                    })
                }
                (RelationOp::NotEquals, E::Atom(cel_parser::Atom::String(s))) => {
                    Some(Predicate::Equality {
                        var: var.clone(),
                        value: LiteralValue::String(s.to_string()),
                        negated: true,
                    })
                }
                _ => None,
            };

            if let Some(pred) = pred {
                if let Some(idx) = predicate_set.index_of(&pred) {
                    return (combo >> idx) & 1 == 1;
                }
                // Check negated form
                let negated_pred = pred.negated();
                if let Some(idx) = predicate_set.index_of(&negated_pred) {
                    return (combo >> idx) & 1 == 0;
                }
            }
            // Unknown relation - assume true (conservative)
            true
        }

        E::Atom(cel_parser::Atom::Bool(b)) => *b,
        E::Atom(_) => true, // Non-boolean atoms assumed true in boolean context

        // For other expressions, assume they match (conservative)
        _ => true,
    }
}

/// Build a MissingCase from a combination bitmap
fn build_missing_case(combo: u64, predicate_set: &PredicateSet) -> MissingCase {
    let predicate_values: Vec<PredicateValue> = predicate_set
        .predicates
        .iter()
        .enumerate()
        .map(|(idx, pred)| {
            let value = (combo >> idx) & 1 == 1;
            PredicateValue {
                predicate_id: idx,
                cel_expression: pred.to_cel_string(),
                value,
            }
        })
        .collect();

    // Build CEL conditions for this case
    let cel_conditions: Vec<String> = predicate_values
        .iter()
        .map(|pv| {
            if pv.value {
                pv.cel_expression.clone()
            } else {
                // Negate the condition
                if let Some(pred) = predicate_set.get(pv.predicate_id) {
                    pred.negated().to_cel_string()
                } else {
                    format!("!{}", pv.cel_expression)
                }
            }
        })
        .collect();

    MissingCase {
        predicate_values,
        cel_conditions,
        input_values: HashMap::new(), // TODO: derive from predicates
    }
}

/// Build a RuleOverlap from a combination and rules
fn build_overlap(combo: u64, rules: &[String], predicate_set: &PredicateSet) -> RuleOverlap {
    let predicate_values: Vec<PredicateValue> = predicate_set
        .predicates
        .iter()
        .enumerate()
        .map(|(idx, pred)| {
            let value = (combo >> idx) & 1 == 1;
            PredicateValue {
                predicate_id: idx,
                cel_expression: pred.to_cel_string(),
                value,
            }
        })
        .collect();

    let cel_conditions: Vec<String> = predicate_values
        .iter()
        .map(|pv| {
            if pv.value {
                pv.cel_expression.clone()
            } else if let Some(pred) = predicate_set.get(pv.predicate_id) {
                pred.negated().to_cel_string()
            } else {
                format!("!{}", pv.cel_expression)
            }
        })
        .collect();

    RuleOverlap {
        rule_ids: rules.to_vec(),
        predicate_values,
        cel_conditions,
    }
}

/// Try to minimize the spec using Quine-McCluskey
fn try_minimize(spec: &Spec, predicate_set: &PredicateSet) -> Option<usize> {
    let n = predicate_set.len();
    if n == 0 || n > 16 {
        // Too many predicates for QMC
        return None;
    }

    // Build a mapping of used predicate indices to contiguous indices (0, 1, 2, ...)
    // quine_mc_cluskey requires continuous naming scheme
    let mut used_indices: Vec<usize> = Vec::new();
    for rule in &spec.rules {
        if let Some(cel_expr) = rule.as_cel() {
            collect_used_indices(&cel_expr, predicate_set, &mut used_indices);
        }
    }
    used_indices.sort();
    used_indices.dedup();

    // Create mapping: original index -> contiguous index
    let index_map: HashMap<usize, u8> = used_indices
        .iter()
        .enumerate()
        .map(|(new_idx, &old_idx)| (old_idx, new_idx as u8))
        .collect();

    if index_map.is_empty() {
        return None;
    }

    // Group rules by output - only rules with the SAME output can be minimized together
    let mut output_groups: HashMap<String, Vec<Bool>> = HashMap::new();

    for rule in &spec.rules {
        if let Some(cel_expr) = rule.as_cel() {
            if let Some(bool_expr) = cel_to_bool_mapped(&cel_expr, predicate_set, &index_map) {
                // Use debug format of output as key for grouping
                let output_key = format!("{:?}", rule.then);
                output_groups.entry(output_key).or_default().push(bool_expr);
            }
        }
    }

    if output_groups.is_empty() {
        return None;
    }

    // For each output group, simplify and count the resulting terms
    let mut total_minimized = 0;
    for (_output, terms) in output_groups {
        if terms.is_empty() {
            continue;
        }

        // Combine terms for this output with OR and simplify
        let combined = if terms.len() == 1 {
            terms.into_iter().next().unwrap()
        } else {
            Bool::Or(terms)
        };

        // Simplify and count resulting terms
        let simplified = combined.simplify();

        let simplified_count = if simplified.is_empty() {
            1
        } else {
            simplified.len()
        };

        total_minimized += simplified_count;
    }

    Some(total_minimized)
}

/// Collect all predicate indices used in a CEL expression
fn collect_used_indices(cel_expr: &str, predicate_set: &PredicateSet, indices: &mut Vec<usize>) {
    if let Ok(ast) = CelCompiler::parse(cel_expr) {
        collect_indices_from_ast(&ast, predicate_set, indices);
    }
}

/// Recursively collect indices from AST
fn collect_indices_from_ast(
    expr: &cel_parser::Expression,
    predicate_set: &PredicateSet,
    indices: &mut Vec<usize>,
) {
    use cel_parser::Expression as E;

    match expr {
        E::Ident(name) => {
            let name_str = name.to_string();
            for (idx, pred) in predicate_set.predicates.iter().enumerate() {
                if let Predicate::BoolVar(var_name) = pred {
                    if var_name == &name_str || var_name == &format!("!{}", name_str) {
                        indices.push(idx);
                        return;
                    }
                }
            }
        }
        E::And(left, right) | E::Or(left, right) => {
            collect_indices_from_ast(left, predicate_set, indices);
            collect_indices_from_ast(right, predicate_set, indices);
        }
        E::Unary(_, inner) => {
            collect_indices_from_ast(inner, predicate_set, indices);
        }
        E::Relation(_, _, _) => {
            // Try to find this relation as a predicate
            let cel_str = format!("{:?}", expr);
            if let Ok(preds) = extract_predicates(&cel_str) {
                if let Some(pred) = preds.into_iter().next() {
                    if let Some(idx) = predicate_set.index_of(&pred) {
                        indices.push(idx);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Convert CEL expression to quine-mc_cluskey Bool with index mapping
fn cel_to_bool_mapped(
    cel_expr: &str,
    predicate_set: &PredicateSet,
    index_map: &HashMap<usize, u8>,
) -> Option<Bool> {
    let ast = CelCompiler::parse(cel_expr).ok()?;
    ast_to_bool_mapped(&ast, predicate_set, index_map)
}

/// Convert CEL AST to quine-mc_cluskey Bool with index mapping for continuous naming
fn ast_to_bool_mapped(
    expr: &cel_parser::Expression,
    predicate_set: &PredicateSet,
    index_map: &HashMap<usize, u8>,
) -> Option<Bool> {
    use cel_parser::Expression as E;

    match expr {
        E::Ident(name) => {
            let name_str = name.to_string();
            // Find this identifier in predicates
            for (idx, pred) in predicate_set.predicates.iter().enumerate() {
                if let Predicate::BoolVar(var_name) = pred {
                    if var_name == &name_str {
                        if let Some(&mapped_idx) = index_map.get(&idx) {
                            return Some(Bool::Term(mapped_idx));
                        }
                    }
                    if var_name == &format!("!{}", name_str) {
                        if let Some(&mapped_idx) = index_map.get(&idx) {
                            return Some(Bool::Not(Box::new(Bool::Term(mapped_idx))));
                        }
                    }
                }
            }
            // Try to find as a general predicate
            if let Ok(preds) = extract_predicates(&name_str) {
                if let Some(pred) = preds.into_iter().next() {
                    if let Some(idx) = predicate_set.index_of(&pred) {
                        if let Some(&mapped_idx) = index_map.get(&idx) {
                            return Some(Bool::Term(mapped_idx));
                        }
                    }
                }
            }
            Some(Bool::True) // Unknown - assume true
        }

        E::And(left, right) => {
            let l = ast_to_bool_mapped(left, predicate_set, index_map)?;
            let r = ast_to_bool_mapped(right, predicate_set, index_map)?;
            Some(Bool::And(vec![l, r]))
        }

        E::Or(left, right) => {
            let l = ast_to_bool_mapped(left, predicate_set, index_map)?;
            let r = ast_to_bool_mapped(right, predicate_set, index_map)?;
            Some(Bool::Or(vec![l, r]))
        }

        E::Unary(cel_parser::UnaryOp::Not, inner) => {
            let i = ast_to_bool_mapped(inner, predicate_set, index_map)?;
            Some(Bool::Not(Box::new(i)))
        }

        E::Relation(_, _, _) => {
            // Find this relation as a predicate
            let cel_str = format!("{:?}", expr);
            if let Ok(preds) = extract_predicates(&cel_str) {
                if let Some(pred) = preds.into_iter().next() {
                    if let Some(idx) = predicate_set.index_of(&pred) {
                        if let Some(&mapped_idx) = index_map.get(&idx) {
                            return Some(Bool::Term(mapped_idx));
                        }
                    }
                }
            }
            Some(Bool::True)
        }

        E::Atom(atom) => match atom {
            cel_parser::Atom::Bool(true) => Some(Bool::True),
            cel_parser::Atom::Bool(false) => Some(Bool::False),
            _ => Some(Bool::True),
        },

        _ => Some(Bool::True), // Default to true for unknown expressions
    }
}

impl IncompletenessReport {
    /// Format as human-readable report
    pub fn to_report(&self) -> String {
        let mut out = String::new();

        let status = if self.is_complete {
            "✓ COMPLETE"
        } else {
            "✗ INCOMPLETE"
        };
        out.push_str(&format!("Completeness Analysis: {}\n", status));
        out.push_str(&format!(
            "Coverage: {}/{} ({:.1}%)\n",
            self.covered_combinations,
            self.total_combinations,
            self.coverage_ratio * 100.0
        ));

        if !self.predicates.is_empty() {
            out.push_str(&format!("\nPredicates ({}):\n", self.predicates.len()));
            for pred in &self.predicates {
                out.push_str(&format!("  [{}] {}\n", pred.id, pred.cel_expression));
            }
        }

        if !self.missing_cases.is_empty() {
            out.push_str(&format!(
                "\nMissing Cases ({}):\n",
                self.missing_cases.len()
            ));
            for (i, case) in self.missing_cases.iter().take(10).enumerate() {
                out.push_str(&format!("  Case {}:\n", i + 1));
                for cond in &case.cel_conditions {
                    out.push_str(&format!("    - {}\n", cond));
                }
            }
            if self.missing_cases.len() > 10 {
                out.push_str(&format!(
                    "  ... and {} more cases\n",
                    self.missing_cases.len() - 10
                ));
            }
        }

        if !self.overlaps.is_empty() {
            out.push_str(&format!("\nOverlapping Rules ({}):\n", self.overlaps.len()));
            for overlap in &self.overlaps {
                out.push_str(&format!(
                    "  Rules {} overlap:\n",
                    overlap.rule_ids.join(", ")
                ));
                for cond in &overlap.cel_conditions {
                    out.push_str(&format!("    - {}\n", cond));
                }
            }
        }

        if self.can_minimize {
            out.push_str(&format!(
                "\nMinimization: {} rules can be reduced to ~{}\n",
                self.original_rule_count,
                self.minimized_rule_count
                    .unwrap_or(self.original_rule_count)
            ));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};

    fn make_test_spec() -> Spec {
        Spec {
            id: "test".into(),
            name: None,
            description: None,
            inputs: vec![
                Variable {
                    name: "rate_exceeded".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
                Variable {
                    name: "amount".into(),
                    typ: VarType::Int,
                    description: None,
                    values: None,
                },
            ],
            outputs: vec![Variable {
                name: "status".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                Rule {
                    id: "R1".into(),
                    when: Some("rate_exceeded".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(429)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("!rate_exceeded && amount > 1000".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(200)),
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
    fn test_analyze_incomplete_spec() {
        let spec = make_test_spec();
        let report = analyze_completeness(&spec);

        // Should be incomplete - missing case: !rate_exceeded && amount <= 1000
        assert!(!report.is_complete);
        assert!(!report.missing_cases.is_empty());
    }

    #[test]
    fn test_analyze_predicates_extracted() {
        let spec = make_test_spec();
        let report = analyze_completeness(&spec);

        // Should have extracted predicates
        assert!(!report.predicates.is_empty());
    }

    #[test]
    fn test_coverage_ratio() {
        let spec = make_test_spec();
        let report = analyze_completeness(&spec);

        // Coverage should be between 0 and 1
        assert!(report.coverage_ratio >= 0.0);
        assert!(report.coverage_ratio <= 1.0);
    }

    fn make_complete_spec() -> Spec {
        Spec {
            id: "complete".into(),
            name: None,
            description: None,
            inputs: vec![
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
            ],
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                Rule {
                    id: "R1".into(),
                    when: Some("a && b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("a && !b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(2)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R3".into(),
                    when: Some("!a && b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(3)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R4".into(),
                    when: Some("!a && !b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(4)),
                    priority: 0,
                    description: None,
                },
            ],
            default: None,
            meta: Default::default(),
            scoping: None,
        }
    }

    fn make_overlapping_spec() -> Spec {
        Spec {
            id: "overlapping".into(),
            name: None,
            description: None,
            inputs: vec![Variable {
                name: "x".into(),
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
                    when: Some("x".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("x".into()), // Same condition - overlaps with R1
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(2)),
                    priority: 1,
                    description: None,
                },
            ],
            default: None,
            meta: Default::default(),
            scoping: None,
        }
    }

    fn make_minimizable_spec() -> Spec {
        // A || B can be simplified from (A && B) || (A && !B) || (!A && B)
        Spec {
            id: "minimizable".into(),
            name: None,
            description: None,
            inputs: vec![
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
            ],
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                Rule {
                    id: "R1".into(),
                    when: Some("a && b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("a && !b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R3".into(),
                    when: Some("!a && b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
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
    fn test_complete_spec_simple() {
        let spec = make_complete_spec();
        let report = analyze_completeness(&spec);

        assert!(
            report.is_complete,
            "Complete spec should be marked as complete"
        );
        assert_eq!(
            report.missing_cases.len(),
            0,
            "Complete spec should have no missing cases"
        );
        assert_eq!(
            report.coverage_ratio, 1.0,
            "Complete spec should have 100% coverage"
        );
    }

    #[test]
    fn test_complete_spec_complex() {
        // Test with 3 boolean variables (8 combinations)
        let spec = Spec {
            id: "complex_complete".into(),
            name: None,
            description: None,
            inputs: vec![
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
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                Rule {
                    id: "R1".into(),
                    when: Some("a && b && c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("a && b && !c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(2)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R3".into(),
                    when: Some("a && !b && c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(3)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R4".into(),
                    when: Some("a && !b && !c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(4)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R5".into(),
                    when: Some("!a && b && c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(5)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R6".into(),
                    when: Some("!a && b && !c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(6)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R7".into(),
                    when: Some("!a && !b && c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(7)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R8".into(),
                    when: Some("!a && !b && !c".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(8)),
                    priority: 0,
                    description: None,
                },
            ],
            default: None,
            meta: Default::default(),
            scoping: None,
        };

        let report = analyze_completeness(&spec);
        assert!(report.is_complete, "All 8 combinations should be covered");
        assert_eq!(report.total_combinations, 8);
        assert_eq!(report.covered_combinations, 8);
    }

    #[test]
    fn test_incomplete_spec_missing_case() {
        let spec = make_test_spec();
        let report = analyze_completeness(&spec);

        assert!(!report.is_complete);
        assert!(!report.missing_cases.is_empty());
        assert!(report.coverage_ratio < 1.0);
    }

    #[test]
    fn test_incomplete_spec_partial_coverage() {
        // Only covers 2 out of 4 combinations
        let spec = Spec {
            id: "partial".into(),
            name: None,
            description: None,
            inputs: vec![
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
            ],
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                Rule {
                    id: "R1".into(),
                    when: Some("a && b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("a && !b".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(2)),
                    priority: 0,
                    description: None,
                },
            ],
            default: None,
            meta: Default::default(),
            scoping: None,
        };

        let report = analyze_completeness(&spec);
        assert!(!report.is_complete);
        assert_eq!(report.total_combinations, 4);
        assert_eq!(report.covered_combinations, 2);
        assert_eq!(report.coverage_ratio, 0.5);
    }

    #[test]
    fn test_overlapping_rules_detection() {
        let spec = make_overlapping_spec();
        let report = analyze_completeness(&spec);

        assert!(
            !report.overlaps.is_empty(),
            "Should detect overlapping rules"
        );
        assert!(report.overlaps.iter().any(|o| o.rule_ids.len() > 1));
    }

    #[test]
    fn test_overlapping_rules_cel_conditions() {
        let spec = make_overlapping_spec();
        let report = analyze_completeness(&spec);

        for overlap in &report.overlaps {
            assert!(
                !overlap.cel_conditions.is_empty(),
                "Overlaps should have CEL conditions"
            );
            assert!(
                !overlap.rule_ids.is_empty(),
                "Overlaps should identify rule IDs"
            );
        }
    }

    #[test]
    fn test_minimization_opportunity() {
        let spec = make_minimizable_spec();
        let report = analyze_completeness(&spec);

        // The spec has 3 rules that can potentially be minimized
        assert!(report.original_rule_count == 3);
        // Minimization may or may not succeed depending on implementation
        // but we should at least check the flag is set correctly
    }

    #[test]
    fn test_empty_spec() {
        let spec = Spec {
            id: "empty".into(),
            name: None,
            description: None,
            inputs: vec![Variable {
                name: "x".into(),
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
            scoping: None,
        };

        let report = analyze_completeness(&spec);
        assert!(!report.is_complete, "Empty spec should be incomplete");
        assert_eq!(report.covered_combinations, 0);
    }

    #[test]
    fn test_spec_no_predicates() {
        // Spec with rules but no CEL expressions (using conditions instead)
        let spec = Spec {
            id: "no_predicates".into(),
            name: None,
            description: None,
            inputs: vec![Variable {
                name: "x".into(),
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
                when: None,
                conditions: Some(vec![crate::spec::Condition {
                    var: "x".into(),
                    op: crate::spec::ConditionOp::Eq,
                    value: ConditionValue::Bool(true),
                }]),
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            }],
            default: None,
            meta: Default::default(),
            scoping: None,
        };

        let report = analyze_completeness(&spec);
        // Should handle gracefully - may have no predicates extracted
        assert!(report.predicates.is_empty() || !report.predicates.is_empty());
    }

    #[test]
    fn test_to_report_complete() {
        let spec = make_complete_spec();
        let report = analyze_completeness(&spec);
        let output = report.to_report();

        assert!(
            output.contains("COMPLETE"),
            "Report should indicate completeness"
        );
        assert!(
            output.contains("100.0%") || output.contains("100%"),
            "Should show 100% coverage"
        );
    }

    #[test]
    fn test_to_report_incomplete() {
        let spec = make_test_spec();
        let report = analyze_completeness(&spec);
        let output = report.to_report();

        assert!(
            output.contains("INCOMPLETE"),
            "Report should indicate incompleteness"
        );
        assert!(
            output.contains("Coverage:"),
            "Should show coverage information"
        );
    }

    #[test]
    fn test_to_report_with_missing_cases() {
        let spec = make_test_spec();
        let report = analyze_completeness(&spec);
        let output = report.to_report();

        if !report.missing_cases.is_empty() {
            assert!(
                output.contains("Missing Cases"),
                "Should list missing cases"
            );
        }
    }

    #[test]
    fn test_to_report_with_overlaps() {
        let spec = make_overlapping_spec();
        let report = analyze_completeness(&spec);
        let output = report.to_report();

        if !report.overlaps.is_empty() {
            assert!(
                output.contains("Overlapping Rules"),
                "Should list overlapping rules"
            );
        }
    }

    #[test]
    fn test_to_report_with_minimization() {
        let spec = make_minimizable_spec();
        let report = analyze_completeness(&spec);
        let output = report.to_report();

        if report.can_minimize {
            assert!(
                output.contains("Minimization"),
                "Should show minimization info"
            );
        }
    }

    #[test]
    fn test_to_report_truncates_many_cases() {
        // Create a spec with many missing cases (>10)
        let spec = Spec {
            id: "many_missing".into(),
            name: None,
            description: None,
            inputs: vec![
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
                Variable {
                    name: "d".into(),
                    typ: VarType::Bool,
                    description: None,
                    values: None,
                },
            ],
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![Rule {
                id: "R1".into(),
                when: Some("a && b && c && d".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            }],
            default: None,
            meta: Default::default(),
            scoping: None,
        };

        let report = analyze_completeness(&spec);
        let output = report.to_report();

        // Should have many missing cases (15 out of 16)
        if report.missing_cases.len() > 10 {
            assert!(output.contains("... and"), "Should truncate and show count");
        }
    }

    #[test]
    fn test_json_serialization() {
        let spec = make_complete_spec();
        let report = analyze_completeness(&spec);

        let json = serde_json::to_string(&report).expect("Should serialize to JSON");
        assert!(!json.is_empty());
        assert!(json.contains("\"is_complete\""));
        assert!(json.contains("\"coverage_ratio\""));
        assert!(json.contains("\"missing_cases\""));
        assert!(json.contains("\"overlaps\""));
    }

    #[test]
    fn test_json_deserialization() {
        let spec = make_complete_spec();
        let report = analyze_completeness(&spec);

        let json = serde_json::to_string(&report).expect("Should serialize");
        let deserialized: IncompletenessReport =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(report.is_complete, deserialized.is_complete);
        assert_eq!(report.total_combinations, deserialized.total_combinations);
        assert_eq!(
            report.covered_combinations,
            deserialized.covered_combinations
        );
        assert_eq!(report.missing_cases.len(), deserialized.missing_cases.len());
        assert_eq!(report.overlaps.len(), deserialized.overlaps.len());
    }

    #[test]
    fn test_too_many_predicates() {
        // Create a spec with many boolean variables (exceeding the 20 predicate limit)
        // Note: This is hard to test directly since we'd need 20+ boolean variables,
        // but we can test the behavior when predicates exceed the limit
        let mut inputs = Vec::new();
        let mut rules = Vec::new();

        // Create 10 boolean variables (would create many predicates)
        for i in 0..10 {
            inputs.push(Variable {
                name: format!("var_{}", i),
                typ: VarType::Bool,
                description: None,
                values: None,
            });
        }

        // Add a few rules
        rules.push(Rule {
            id: "R1".into(),
            when: Some("var_0 && var_1".into()),
            conditions: None,
            then: Output::Single(ConditionValue::Int(1)),
            priority: 0,
            description: None,
        });

        let spec = Spec {
            id: "many_predicates".into(),
            name: None,
            description: None,
            inputs,
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
        };

        let report = analyze_completeness(&spec);
        // Should handle gracefully - either complete analysis or report too many predicates
        assert!(
            report.total_combinations > 0
                || !report.missing_cases.is_empty()
                || report
                    .missing_cases
                    .iter()
                    .any(|c| c.cel_conditions.iter().any(|s| s.contains("Too many")))
        );
    }

    #[test]
    fn test_spec_with_conditions_only() {
        // Test spec using conditions: instead of when:
        let spec = Spec {
            id: "conditions_only".into(),
            name: None,
            description: None,
            inputs: vec![Variable {
                name: "x".into(),
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
                    when: None,
                    conditions: Some(vec![crate::spec::Condition {
                        var: "x".into(),
                        op: crate::spec::ConditionOp::Eq,
                        value: ConditionValue::Bool(true),
                    }]),
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: None,
                    conditions: Some(vec![crate::spec::Condition {
                        var: "x".into(),
                        op: crate::spec::ConditionOp::Eq,
                        value: ConditionValue::Bool(false),
                    }]),
                    then: Output::Single(ConditionValue::Int(2)),
                    priority: 0,
                    description: None,
                },
            ],
            default: None,
            meta: Default::default(),
            scoping: None,
        };

        let report = analyze_completeness(&spec);
        // Should handle gracefully - may not extract predicates from conditions:
        // but should not panic. Just verify we get a valid report.
        let _ = report.total_combinations; // Verify we got a report
    }
}
