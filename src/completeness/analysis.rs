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
use crate::error::{Error, Result};
use crate::spec::{Rule, Spec};
use quine_mc_cluskey::Bool;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Result of completeness analysis - raw data for LLM tool
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateInfo {
    pub id: usize,
    pub cel_expression: String,
}

/// A specific uncovered input combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingCase {
    /// The predicate values for this case
    pub predicate_values: Vec<PredicateValue>,

    /// CEL conditions that describe this case
    pub cel_conditions: Vec<String>,

    /// Input variable values (if determinable)
    pub input_values: HashMap<String, String>,
}

/// A predicate with its truth value in a missing case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateValue {
    pub predicate_id: usize,
    pub cel_expression: String,
    pub value: bool,
}

/// Overlapping rules - multiple rules match the same input
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        if let Some(cel_expr) = &rule.when {
            match extract_predicates(cel_expr) {
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
        if let Some(cel_expr) = &rule.when {
            let rule_combos = find_matching_combinations(cel_expr, &predicate_set);
            for combo in rule_combos {
                covered.insert(combo);
                combo_rules
                    .entry(combo)
                    .or_insert_with(Vec::new)
                    .push(rule.id.clone());
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
        can_minimize: minimized_count.map(|c| c < spec.rules.len()).unwrap_or(false),
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

        E::Unary(cel_parser::UnaryOp::Not, inner) => !expression_matches(inner, combo, predicate_set),

        E::Relation(left, op, right) => {
            // Try to match against our predicates
            // Get variable name from left side
            let var = match left.as_ref() {
                E::Ident(name) => name.to_string(),
                _ => return true, // Unknown - assume true
            };

            // Build a predicate and check if it's in our set
            use cel_parser::RelationOp;
            use super::predicates::{ComparisonOp, LiteralValue};

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

        E::Atom(atom) => {
            match atom {
                cel_parser::Atom::Bool(b) => *b,
                _ => true, // Non-boolean atoms assumed true in boolean context
            }
        }

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
            } else {
                if let Some(pred) = predicate_set.get(pv.predicate_id) {
                    pred.negated().to_cel_string()
                } else {
                    format!("!{}", pv.cel_expression)
                }
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

    // Build boolean expression for each output value
    // For now, just check if any simplification is possible
    let mut all_rules_as_terms: Vec<Bool> = Vec::new();

    for rule in &spec.rules {
        if let Some(cel_expr) = &rule.when {
            if let Some(bool_expr) = cel_to_bool(cel_expr, predicate_set) {
                all_rules_as_terms.push(bool_expr);
            }
        }
    }

    if all_rules_as_terms.is_empty() {
        return None;
    }

    // Combine all rules with OR and simplify
    let combined = if all_rules_as_terms.len() == 1 {
        all_rules_as_terms.pop().unwrap()
    } else {
        Bool::Or(all_rules_as_terms)
    };

    // Simplify and count resulting terms
    let simplified = combined.simplify();

    // Count the number of terms in the simplified result
    let simplified_count = count_terms(&simplified);

    Some(simplified_count)
}

/// Convert CEL expression to quine-mc_cluskey Bool
fn cel_to_bool(cel_expr: &str, predicate_set: &PredicateSet) -> Option<Bool> {
    let ast = CelCompiler::parse(cel_expr).ok()?;
    ast_to_bool(&ast, predicate_set)
}

/// Convert CEL AST to quine-mc_cluskey Bool
fn ast_to_bool(expr: &cel_parser::Expression, predicate_set: &PredicateSet) -> Option<Bool> {
    use cel_parser::Expression as E;

    match expr {
        E::Ident(name) => {
            let name_str = name.to_string();
            // Find this identifier in predicates
            for (idx, pred) in predicate_set.predicates.iter().enumerate() {
                if let Predicate::BoolVar(var_name) = pred {
                    if var_name == &name_str {
                        return Some(Bool::Term(idx as u8));
                    }
                    if var_name == &format!("!{}", name_str) {
                        return Some(Bool::Not(Box::new(Bool::Term(idx as u8))));
                    }
                }
            }
            // Try to find as a general predicate
            if let Ok(preds) = extract_predicates(&name_str) {
                if let Some(pred) = preds.into_iter().next() {
                    if let Some(idx) = predicate_set.index_of(&pred) {
                        return Some(Bool::Term(idx as u8));
                    }
                }
            }
            Some(Bool::True) // Unknown - assume true
        }

        E::And(left, right) => {
            let l = ast_to_bool(left, predicate_set)?;
            let r = ast_to_bool(right, predicate_set)?;
            Some(Bool::And(vec![l, r]))
        }

        E::Or(left, right) => {
            let l = ast_to_bool(left, predicate_set)?;
            let r = ast_to_bool(right, predicate_set)?;
            Some(Bool::Or(vec![l, r]))
        }

        E::Unary(cel_parser::UnaryOp::Not, inner) => {
            let i = ast_to_bool(inner, predicate_set)?;
            Some(Bool::Not(Box::new(i)))
        }

        E::Relation(_, _, _) => {
            // Find this relation as a predicate
            let cel_str = format!("{:?}", expr);
            if let Ok(preds) = extract_predicates(&cel_str) {
                if let Some(pred) = preds.into_iter().next() {
                    if let Some(idx) = predicate_set.index_of(&pred) {
                        return Some(Bool::Term(idx as u8));
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

/// Count terms in a simplified boolean expression
fn count_terms(exprs: &[Bool]) -> usize {
    exprs.len().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Output, Rule, Spec, Variable, VarType, ConditionValue};

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
}
