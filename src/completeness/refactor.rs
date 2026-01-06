//! High-level refactoring APIs for decision tables
//!
//! This module provides APIs for:
//! - `minimize()` - Simplify redundant rules using Espresso
//! - `decompose()` - Split specs by independent variable groups
//! - `compose()` - Chain multiple specs together
//! - `extract_spec_from_orchestrator()` - Extract decision logic from orchestrators

use super::adapter::{cover_to_cel, rules_to_cover};
use super::espresso::{espresso, Cover};
use super::predicates::{extract_predicates, Predicate, PredicateSet};
use crate::orchestrate::{ChainStep, Orchestrator};
use crate::spec::{ConditionValue, Output, Rule, Spec, VarType, Variable};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// MINIMIZE API
// ============================================================================

/// Result of minimizing a spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimizedSpec {
    /// The simplified spec
    pub spec: Spec,
    /// Original number of rules
    pub original_rule_count: usize,
    /// Minimized number of rules
    pub minimized_rule_count: usize,
    /// Transformations applied (for audit trail)
    pub transformations: Vec<Transformation>,
    /// Whether any simplification was possible
    pub was_simplified: bool,
}

/// A transformation applied during minimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transformation {
    /// Description of the transformation
    pub description: String,
    /// Rules affected
    pub affected_rules: Vec<String>,
    /// Type of transformation
    pub kind: TransformationKind,
}

/// Types of transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationKind {
    /// Rules were merged into one
    Merged,
    /// Rule was removed (redundant)
    Removed,
    /// Rule was expanded (don't care → specific values)
    Expanded,
    /// Rule was simplified (fewer terms)
    Simplified,
}

/// Minimize a spec by simplifying redundant rules
///
/// Uses the Espresso algorithm to find a minimal set of rules
/// that are logically equivalent to the original.
///
/// # Example
///
/// ```ignore
/// use imacs::completeness::minimize;
///
/// let spec = Spec::from_yaml(yaml)?;
/// let result = minimize(&spec);
///
/// if result.was_simplified {
///     println!("Reduced from {} to {} rules",
///         result.original_rule_count,
///         result.minimized_rule_count);
/// }
/// ```
pub fn minimize(spec: &Spec) -> MinimizedSpec {
    let original_rule_count = spec.rules.len();

    // Edge case: no rules or single rule
    if original_rule_count <= 1 {
        return MinimizedSpec {
            spec: spec.clone(),
            original_rule_count,
            minimized_rule_count: original_rule_count,
            transformations: vec![],
            was_simplified: false,
        };
    }

    // Group rules by output value (minimize each group separately)
    let mut rules_by_output: HashMap<String, Vec<&Rule>> = HashMap::new();
    for rule in &spec.rules {
        let output_key = format!("{:?}", rule.then);
        rules_by_output.entry(output_key).or_default().push(rule);
    }

    // Extract predicates from all rules
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

    // Skip if too many predicates (Espresso can handle many, but be safe)
    if predicate_set.len() > 30 {
        return MinimizedSpec {
            spec: spec.clone(),
            original_rule_count,
            minimized_rule_count: original_rule_count,
            transformations: vec![Transformation {
                description: format!(
                    "Too many predicates ({}) for minimization",
                    predicate_set.len()
                ),
                affected_rules: vec![],
                kind: TransformationKind::Simplified,
            }],
            was_simplified: false,
        };
    }

    // Minimize each output group
    let mut new_rules = Vec::new();
    let mut transformations = Vec::new();
    let mut rule_id_counter = 1;

    for (output_key, rules) in &rules_by_output {
        if rules.len() == 1 {
            // Single rule for this output - keep as-is
            new_rules.push((*rules[0]).clone());
            continue;
        }

        // Convert rules to Espresso cover
        let owned_rules: Vec<Rule> = rules.iter().map(|r| (*r).clone()).collect();
        let on_set = rules_to_cover(&owned_rules, &predicate_set);
        let dc_set = Cover::new(predicate_set.len(), 1);

        // Minimize
        let minimized = espresso(&on_set, &dc_set);

        // Convert back to CEL expressions
        let minimized_exprs = cover_to_cel(&minimized, &predicate_set);

        if minimized_exprs.len() < rules.len() {
            // We got simplification!
            transformations.push(Transformation {
                description: format!(
                    "Merged {} rules for output {} into {} rules",
                    rules.len(),
                    output_key,
                    minimized_exprs.len()
                ),
                affected_rules: rules.iter().map(|r| r.id.clone()).collect(),
                kind: TransformationKind::Merged,
            });

            // Create new rules from minimized expressions
            for expr in minimized_exprs {
                new_rules.push(Rule {
                    id: format!("M{}", rule_id_counter),
                    when: Some(crate::spec::WhenClause::Single(expr)),
                    conditions: None,
                    then: rules[0].then.clone(),
                    priority: 0,
                    description: Some(format!(
                        "Minimized from: {}",
                        rules
                            .iter()
                            .map(|r| r.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                });
                rule_id_counter += 1;
            }
        } else {
            // No simplification - keep original rules
            for rule in rules {
                new_rules.push((*rule).clone());
            }
        }
    }

    let minimized_rule_count = new_rules.len();
    let was_simplified = minimized_rule_count < original_rule_count;

    // Build new spec
    let mut minimized_spec = spec.clone();
    minimized_spec.rules = new_rules;

    MinimizedSpec {
        spec: minimized_spec,
        original_rule_count,
        minimized_rule_count,
        transformations,
        was_simplified,
    }
}

// ============================================================================
// DECOMPOSE API
// ============================================================================

/// Result of decomposing a spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    /// Whether decomposition is possible
    pub can_decompose: bool,
    /// Independent variable groups found
    pub variable_groups: Vec<VariableGroup>,
    /// Proposed sub-specs (one per group)
    pub proposed_specs: Vec<Spec>,
    /// How the specs should be chained
    pub chain_order: Vec<String>,
    /// Reason if decomposition not possible
    pub reason: Option<String>,
}

/// A group of variables that are used together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableGroup {
    /// Variable names in this group
    pub variables: Vec<String>,
    /// Rule IDs that use these variables
    pub rule_ids: Vec<String>,
    /// Whether this group is independent from others
    pub is_independent: bool,
}

/// Decompose a spec into independent sub-specs
///
/// Analyzes which variables are used together in rules and
/// splits the spec if there are independent variable groups.
///
/// # Example
///
/// ```ignore
/// use imacs::completeness::decompose;
///
/// let spec = Spec::from_yaml(yaml)?;
/// let result = decompose(&spec);
///
/// if result.can_decompose {
///     for sub_spec in result.proposed_specs {
///         println!("Sub-spec: {} with {} rules", sub_spec.id, sub_spec.rules.len());
///     }
/// }
/// ```
pub fn decompose(spec: &Spec) -> DecompositionResult {
    // Build variable dependency graph
    let mut var_to_rules: HashMap<String, HashSet<String>> = HashMap::new();
    let mut rule_to_vars: HashMap<String, HashSet<String>> = HashMap::new();

    for rule in &spec.rules {
        let mut vars_in_rule = HashSet::new();

        if let Some(cel_expr) = rule.as_cel() {
            // Extract variables from CEL expression
            if let Ok(preds) = extract_predicates(&cel_expr) {
                for pred in preds {
                    let var_name = pred.variable_name();
                    vars_in_rule.insert(var_name.clone());
                    var_to_rules
                        .entry(var_name)
                        .or_default()
                        .insert(rule.id.clone());
                }
            }
        }

        rule_to_vars.insert(rule.id.clone(), vars_in_rule);
    }

    // Find connected components (variable groups)
    let all_vars: Vec<String> = var_to_rules.keys().cloned().collect();
    let groups = find_connected_components(&all_vars, &var_to_rules, &rule_to_vars);

    // Check if decomposition is possible
    if groups.len() <= 1 {
        return DecompositionResult {
            can_decompose: false,
            variable_groups: groups,
            proposed_specs: vec![],
            chain_order: vec![],
            reason: Some("All variables are interconnected - cannot decompose".into()),
        };
    }

    // Create sub-specs for each group
    let mut proposed_specs = Vec::new();
    let mut chain_order = Vec::new();

    for (idx, group) in groups.iter().enumerate() {
        let sub_spec_id = format!("{}_{}", spec.id, idx + 1);
        chain_order.push(sub_spec_id.clone());

        // Filter inputs for this group
        let inputs: Vec<Variable> = spec
            .inputs
            .iter()
            .filter(|v| group.variables.contains(&v.name))
            .cloned()
            .collect();

        // Filter rules for this group
        let rules: Vec<Rule> = spec
            .rules
            .iter()
            .filter(|r| group.rule_ids.contains(&r.id))
            .cloned()
            .collect();

        let sub_spec = Spec {
            id: sub_spec_id,
            name: Some(format!(
                "{} (part {})",
                spec.name.as_deref().unwrap_or(&spec.id),
                idx + 1
            )),
            description: spec.description.clone(),
            inputs,
            outputs: spec.outputs.clone(), // Each sub-spec can produce the same output
            rules,
            default: spec.default.clone(),
            meta: spec.meta.clone(),
            scoping: spec.scoping.clone(),
        };

        proposed_specs.push(sub_spec);
    }

    DecompositionResult {
        can_decompose: true,
        variable_groups: groups,
        proposed_specs,
        chain_order,
        reason: None,
    }
}

/// Find connected components in the variable-rule graph
fn find_connected_components(
    all_vars: &[String],
    var_to_rules: &HashMap<String, HashSet<String>>,
    rule_to_vars: &HashMap<String, HashSet<String>>,
) -> Vec<VariableGroup> {
    let mut visited_vars: HashSet<String> = HashSet::new();
    let mut visited_rules: HashSet<String> = HashSet::new();
    let mut groups = Vec::new();

    for start_var in all_vars {
        if visited_vars.contains(start_var) {
            continue;
        }

        // BFS to find all connected variables and rules
        let mut group_vars = HashSet::new();
        let mut group_rules = HashSet::new();
        let mut queue = vec![start_var.clone()];

        while let Some(var) = queue.pop() {
            if visited_vars.contains(&var) {
                continue;
            }
            visited_vars.insert(var.clone());
            group_vars.insert(var.clone());

            // Find all rules using this variable
            if let Some(rules) = var_to_rules.get(&var) {
                for rule_id in rules {
                    if visited_rules.contains(rule_id) {
                        continue;
                    }
                    visited_rules.insert(rule_id.clone());
                    group_rules.insert(rule_id.clone());

                    // Find all variables used by this rule
                    if let Some(vars) = rule_to_vars.get(rule_id) {
                        for v in vars {
                            if !visited_vars.contains(v) {
                                queue.push(v.clone());
                            }
                        }
                    }
                }
            }
        }

        if !group_vars.is_empty() {
            groups.push(VariableGroup {
                variables: group_vars.into_iter().collect(),
                rule_ids: group_rules.into_iter().collect(),
                is_independent: true,
            });
        }
    }

    groups
}

// ============================================================================
// COMPOSE API
// ============================================================================

/// Definition of how specs should be chained
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDefinition {
    /// Ordered list of spec IDs
    pub spec_ids: Vec<String>,
    /// How outputs map to next spec's inputs
    pub mappings: Vec<OutputToInputMapping>,
}

/// Mapping from one spec's output to another's input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputToInputMapping {
    /// Source spec ID
    pub from_spec: String,
    /// Source output name
    pub from_output: String,
    /// Target spec ID
    pub to_spec: String,
    /// Target input name
    pub to_input: String,
}

/// Result of composing specs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposedSpec {
    /// The composed spec
    pub spec: Spec,
    /// Original spec IDs that were composed
    pub composed_from: Vec<String>,
    /// Whether composition was successful
    pub success: bool,
    /// Error message if not successful
    pub error: Option<String>,
}

/// Compose multiple specs into a single chained spec
///
/// Takes a chain definition and merges the specs, where outputs
/// of one spec become inputs to the next.
///
/// # Example
///
/// ```ignore
/// use imacs::completeness::{compose, ChainDefinition};
///
/// let chain = ChainDefinition {
///     spec_ids: vec!["validate_user".into(), "check_permissions".into()],
///     mappings: vec![...],
/// };
///
/// let result = compose(&specs, &chain);
/// if result.success {
///     println!("Composed spec has {} rules", result.spec.rules.len());
/// }
/// ```
pub fn compose(specs: &HashMap<String, Spec>, chain: &ChainDefinition) -> ComposedSpec {
    // Validate all specs exist
    for spec_id in &chain.spec_ids {
        if !specs.contains_key(spec_id) {
            return ComposedSpec {
                spec: Spec::default(),
                composed_from: chain.spec_ids.clone(),
                success: false,
                error: Some(format!("Spec '{}' not found", spec_id)),
            };
        }
    }

    if chain.spec_ids.is_empty() {
        return ComposedSpec {
            spec: Spec::default(),
            composed_from: vec![],
            success: false,
            error: Some("No specs to compose".into()),
        };
    }

    // Start with the first spec
    let first_id = &chain.spec_ids[0];
    let first_spec = &specs[first_id];

    let mut composed = first_spec.clone();
    composed.id = chain.spec_ids.join("_");
    composed.name = Some(format!("Composed: {}", chain.spec_ids.join(" → ")));

    // Merge remaining specs
    for spec_id in chain.spec_ids.iter().skip(1) {
        let next_spec = &specs[spec_id];

        // Add inputs that aren't already covered by outputs of previous specs
        for input in &next_spec.inputs {
            // Check if this input is mapped from a previous output
            let is_mapped = chain
                .mappings
                .iter()
                .any(|m| m.to_spec == *spec_id && m.to_input == input.name);

            if !is_mapped {
                // Add as new input if not already present
                if !composed.inputs.iter().any(|i| i.name == input.name) {
                    composed.inputs.push(input.clone());
                }
            }
        }

        // Outputs come from the last spec in chain
        if spec_id == chain.spec_ids.last().unwrap() {
            composed.outputs = next_spec.outputs.clone();
        }

        // Merge rules (prefix with spec ID to avoid conflicts)
        for rule in &next_spec.rules {
            let mut new_rule = rule.clone();
            new_rule.id = format!("{}_{}", spec_id, rule.id);

            // Substitute mapped inputs in the condition
            if let Some(cel_expr) = new_rule.as_cel() {
                let mut new_when = cel_expr;
                for mapping in &chain.mappings {
                    if mapping.to_spec == *spec_id {
                        // Replace input reference with output reference
                        new_when = new_when.replace(
                            &mapping.to_input,
                            &format!("{}_{}", mapping.from_spec, mapping.from_output),
                        );
                    }
                }
                new_rule.when = Some(crate::spec::WhenClause::Single(new_when));
            }

            composed.rules.push(new_rule);
        }
    }

    ComposedSpec {
        spec: composed,
        composed_from: chain.spec_ids.clone(),
        success: true,
        error: None,
    }
}

// ============================================================================
// EXTRACT FROM ORCHESTRATOR API
// ============================================================================

/// Result of extracting specs from an orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorExtractionResult {
    /// Specs extracted from decision logic in the orchestrator
    pub extracted_specs: Vec<Spec>,
    /// The simplified orchestrator (decision logic removed)
    pub simplified_orchestrator: Option<Orchestrator>,
    /// Confidence in the extraction (0.0 - 1.0)
    pub confidence: f64,
    /// Warnings or notes about the extraction
    pub notes: Vec<String>,
    /// Whether any decision logic was found
    pub found_decision_logic: bool,
}

/// Extract decision table specs from an orchestrator
///
/// Analyzes Branch and Gate steps to identify embedded decision logic
/// that could be extracted into proper decision table specs.
///
/// # Example
///
/// ```ignore
/// use imacs::completeness::extract_spec_from_orchestrator;
///
/// let orch = Orchestrator::from_yaml(yaml)?;
/// let result = extract_spec_from_orchestrator(&orch);
///
/// for spec in result.extracted_specs {
///     println!("Extracted: {}", spec.id);
/// }
/// ```
pub fn extract_spec_from_orchestrator(orch: &Orchestrator) -> OrchestratorExtractionResult {
    let mut extracted_specs = Vec::new();
    let mut notes = Vec::new();
    let mut found_decision_logic = false;

    // Analyze the chain for decision logic
    let branch_steps = find_branch_steps(&orch.chain);
    let gate_steps = find_gate_steps(&orch.chain);

    // Extract specs from Branch steps
    for (idx, branch) in branch_steps.iter().enumerate() {
        if let Some(spec) = extract_spec_from_branch(branch, &orch.id, idx) {
            found_decision_logic = true;
            extracted_specs.push(spec);
            notes.push(format!(
                "Extracted decision table from branch step {}",
                idx + 1
            ));
        }
    }

    // Analyze Gate steps for potential rules
    if !gate_steps.is_empty() {
        if let Some(spec) = extract_spec_from_gates(&gate_steps, &orch.id) {
            found_decision_logic = true;
            extracted_specs.push(spec);
            notes.push("Extracted gate conditions as rules".to_string());
        }
    }

    // Calculate confidence based on extraction success
    let confidence = if extracted_specs.is_empty() {
        0.0
    } else {
        // Higher confidence if we extracted clean specs
        0.7 + (0.3
            * (extracted_specs.len() as f64
                / (branch_steps.len() + gate_steps.len()).max(1) as f64))
    };

    OrchestratorExtractionResult {
        extracted_specs,
        simplified_orchestrator: None, // TODO: Build simplified orchestrator
        confidence,
        notes,
        found_decision_logic,
    }
}

/// Find all Branch steps in the chain (recursively)
fn find_branch_steps(steps: &[ChainStep]) -> Vec<&ChainStep> {
    let mut branches = Vec::new();

    for step in steps {
        match step {
            ChainStep::Branch(_) => {
                branches.push(step);
            }
            ChainStep::Parallel(par) => {
                branches.extend(find_branch_steps(&par.steps));
            }
            ChainStep::Loop(lp) => {
                branches.extend(find_branch_steps(&lp.steps));
            }
            ChainStep::Try(tr) => {
                branches.extend(find_branch_steps(&tr.try_steps));
                if let Some(catch) = &tr.catch {
                    branches.extend(find_branch_steps(&catch.steps));
                }
                if let Some(finally) = &tr.finally {
                    branches.extend(find_branch_steps(finally));
                }
            }
            _ => {}
        }
    }

    branches
}

/// Find all Gate steps in the chain
fn find_gate_steps(steps: &[ChainStep]) -> Vec<&ChainStep> {
    let mut gates = Vec::new();

    for step in steps {
        match step {
            ChainStep::Gate(_) => {
                gates.push(step);
            }
            ChainStep::Parallel(par) => {
                gates.extend(find_gate_steps(&par.steps));
            }
            ChainStep::Loop(lp) => {
                gates.extend(find_gate_steps(&lp.steps));
            }
            ChainStep::Try(tr) => {
                gates.extend(find_gate_steps(&tr.try_steps));
                if let Some(catch) = &tr.catch {
                    gates.extend(find_gate_steps(&catch.steps));
                }
            }
            _ => {}
        }
    }

    gates
}

/// Extract a spec from a Branch step
fn extract_spec_from_branch(step: &ChainStep, orch_id: &str, idx: usize) -> Option<Spec> {
    if let ChainStep::Branch(branch) = step {
        let spec_id = format!("{}_branch_{}", orch_id, idx + 1);

        // Build rules from branch cases
        let mut rules = Vec::new();
        let mut rule_idx = 1;

        for condition in branch.cases.keys() {
            rules.push(Rule {
                id: format!("R{}", rule_idx),
                when: Some(crate::spec::WhenClause::Single(condition.clone())),
                conditions: None,
                then: Output::Single(ConditionValue::String(format!("case_{}", rule_idx))),
                priority: 0,
                description: Some(format!("Branch case: {}", condition)),
            });
            rule_idx += 1;
        }

        // Add default case if present
        if branch.default.is_some() {
            rules.push(Rule {
                id: format!("R{}", rule_idx),
                when: None,
                conditions: None,
                then: Output::Single(ConditionValue::String("default".into())),
                priority: 0,
                description: Some("Default branch case".into()),
            });
        }

        if rules.is_empty() {
            return None;
        }

        // Try to infer inputs from conditions
        let mut inputs = Vec::new();
        let mut seen_vars = HashSet::new();

        for rule in &rules {
            if let Some(cel_expr) = rule.as_cel() {
                if let Ok(preds) = extract_predicates(&cel_expr) {
                    for pred in preds {
                        let var_name = pred.variable_name();
                        if !seen_vars.contains(&var_name) {
                            seen_vars.insert(var_name.clone());
                            inputs.push(Variable {
                                name: var_name,
                                typ: pred.infer_type(),
                                description: None,
                                values: None,
                            });
                        }
                    }
                }
            }
        }

        Some(Spec {
            id: spec_id,
            name: Some(format!("Extracted from {} branch {}", orch_id, idx + 1)),
            description: None,
            inputs,
            outputs: vec![Variable {
                name: "result".into(),
                typ: VarType::String,
                description: Some("Branch result".into()),
                values: None,
            }],
            rules,
            default: None,
            meta: Default::default(),
            scoping: None,
        })
    } else {
        None
    }
}

/// Extract a spec from Gate steps
fn extract_spec_from_gates(steps: &[&ChainStep], orch_id: &str) -> Option<Spec> {
    let mut rules = Vec::new();

    for (idx, step) in steps.iter().enumerate() {
        if let ChainStep::Gate(gate) = step {
            rules.push(Rule {
                id: format!("G{}", idx + 1),
                when: Some(crate::spec::WhenClause::Single(gate.condition.clone())),
                conditions: None,
                then: Output::Single(ConditionValue::Bool(true)),
                priority: 0,
                description: Some(gate.id.clone()),
            });
        }
    }

    if rules.is_empty() {
        return None;
    }

    // Infer inputs from gate conditions
    let mut inputs = Vec::new();
    let mut seen_vars = HashSet::new();

    for rule in &rules {
        if let Some(cel_expr) = rule.as_cel() {
            if let Ok(preds) = extract_predicates(&cel_expr) {
                for pred in preds {
                    let var_name = pred.variable_name();
                    if !seen_vars.contains(&var_name) {
                        seen_vars.insert(var_name.clone());
                        inputs.push(Variable {
                            name: var_name,
                            typ: pred.infer_type(),
                            description: None,
                            values: None,
                        });
                    }
                }
            }
        }
    }

    Some(Spec {
        id: format!("{}_gates", orch_id),
        name: Some(format!("Gate conditions from {}", orch_id)),
        description: Some("Extracted gate conditions as decision rules".into()),
        inputs,
        outputs: vec![Variable {
            name: "allowed".into(),
            typ: VarType::Bool,
            description: Some("Whether the gate condition passed".into()),
            values: None,
        }],
        rules,
        default: Some(Output::Single(ConditionValue::Bool(false))),
        meta: Default::default(),
        scoping: None,
    })
}

// ============================================================================
// HELPER TRAIT IMPLEMENTATIONS
// ============================================================================

impl Predicate {
    /// Get the variable name from a predicate
    pub fn variable_name(&self) -> String {
        match self {
            Predicate::BoolVar(name) => name.trim_start_matches('!').to_string(),
            Predicate::Comparison { var, .. } => var.clone(),
            Predicate::Equality { var, .. } => var.clone(),
            Predicate::Membership { var, .. } => var.clone(),
            Predicate::StringOp { var, .. } => var.clone(),
        }
    }

    /// Infer the variable type from a predicate
    pub fn infer_type(&self) -> VarType {
        match self {
            Predicate::BoolVar(_) => VarType::Bool,
            Predicate::Comparison { value, .. } => match value {
                super::predicates::LiteralValue::Bool(_) => VarType::Bool,
                super::predicates::LiteralValue::Int(_) => VarType::Int,
                super::predicates::LiteralValue::Float(_) => VarType::Float,
                super::predicates::LiteralValue::String(_) => VarType::String,
            },
            Predicate::Equality { value, .. } => match value {
                super::predicates::LiteralValue::Bool(_) => VarType::Bool,
                super::predicates::LiteralValue::Int(_) => VarType::Int,
                super::predicates::LiteralValue::Float(_) => VarType::Float,
                super::predicates::LiteralValue::String(_) => VarType::String,
            },
            Predicate::Membership { .. } => VarType::String,
            Predicate::StringOp { .. } => VarType::String,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_spec() -> Spec {
        Spec {
            id: "test".into(),
            name: None,
            description: None,
            inputs: vec![Variable {
                name: "flag".into(),
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
                    when: Some("flag".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("!flag".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(0)),
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
    fn test_minimize_simple_spec() {
        let spec = make_test_spec();
        let result = minimize(&spec);

        assert_eq!(result.original_rule_count, 2);
        // Simple spec may not be minimizable
        assert!(result.minimized_rule_count <= 2);
    }

    #[test]
    fn test_decompose_independent_groups() {
        // Spec with two independent variable groups
        let spec = Spec {
            id: "test".into(),
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
                name: "out".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![
                Rule {
                    id: "R1".into(),
                    when: Some("a".into()),
                    conditions: None,
                    then: Output::Single(ConditionValue::Int(1)),
                    priority: 0,
                    description: None,
                },
                Rule {
                    id: "R2".into(),
                    when: Some("b".into()),
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

        let result = decompose(&spec);

        // Two independent rules using different variables should decompose
        assert!(result.can_decompose);
        assert_eq!(result.variable_groups.len(), 2);
    }

    #[test]
    fn test_decompose_connected_variables() {
        // Spec where variables are connected through a rule
        let spec = Spec {
            id: "test".into(),
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
                name: "out".into(),
                typ: VarType::Int,
                description: None,
                values: None,
            }],
            rules: vec![Rule {
                id: "R1".into(),
                when: Some("a && b".into()),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            }],
            default: None,
            meta: Default::default(),
            scoping: None,
        };

        let result = decompose(&spec);

        // Variables used together cannot be decomposed
        assert!(!result.can_decompose);
    }

    #[test]
    fn test_compose_specs() {
        let mut specs = HashMap::new();
        specs.insert("spec_a".into(), make_test_spec());

        let chain = ChainDefinition {
            spec_ids: vec!["spec_a".into()],
            mappings: vec![],
        };

        let result = compose(&specs, &chain);
        assert!(result.success);
    }
}
