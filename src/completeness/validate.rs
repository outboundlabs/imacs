//! Spec validation - detect impossible/invalid specs
//!
//! Detects:
//! - Contradictory rules (same condition, different outputs, no priority)
//! - Unsatisfiable conditions (can never be true)
//! - Tautology conditions (always match, not marked as default)
//! - Dead rules (covered by earlier rules)
//! - Type mismatches (wrong types in comparisons)

use super::adapter::rules_to_cover;
use super::espresso::Cover;
use super::predicates::{extract_predicates, PredicateSet};
use crate::spec::{Rule, Spec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of spec validation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub fixes: Vec<SpecFix>,
    pub error_count: usize,
    pub warning_count: usize,
}

/// A validation issue found in the spec
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationIssue {
    pub code: String,
    pub severity: Severity,
    pub issue_type: IssueType,
    pub message: String,
    pub affected_rules: Vec<String>,

    /// Detailed explanation of why this is a problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,

    /// Step-by-step suggestion for fixing the issue
    pub suggestion: Option<String>,

    /// Concrete example showing the fix (YAML snippet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_example: Option<String>,

    /// Additional context (CEL expressions, variable names, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<IssueContext>,
}

/// Additional context for validation issues
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueContext {
    /// The problematic CEL expression(s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cel_expressions: Option<Vec<String>>,

    /// Variable names involved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<String>>,

    /// Type information (for type mismatches)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_info: Option<String>,

    /// Example input that triggers the issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_input: Option<HashMap<String, String>>,

    /// What happens with current spec
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_behavior: Option<String>,

    /// What should happen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_behavior: Option<String>,
}

/// Severity of a validation issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum Severity {
    Error,
    Warning,
}

/// Type of validation issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum IssueType {
    ContradictoryRules,
    UnsatisfiableCondition,
    TautologyCondition,
    DeadRule,
    TypeMismatch,
}

/// A concrete fix that can be applied to a spec
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecFix {
    pub issue_code: String,
    pub confidence: FixConfidence,
    pub operation: FixOperation,
    pub description: String,
}

/// Confidence level for a fix
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FixConfidence {
    /// Safe to auto-apply
    High,
    /// Likely correct, review recommended
    Medium,
    /// Suggestion only, requires human judgment
    Low,
}

/// Operation to fix an issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum FixOperation {
    /// Update a field in a rule
    UpdateRule {
        rule_id: String,
        field: String,
        old_value: String,
        new_value: String,
    },
    /// Delete a rule
    DeleteRule { rule_id: String },
    /// Add or update priority on a rule
    AddPriority { rule_id: String, priority: i32 },
    /// Rename a variable
    RenameVariable { old_name: String, new_name: String },
    /// Update a CEL expression
    UpdateExpression {
        rule_id: String,
        old_expression: String,
        new_expression: String,
    },
}

/// Validate a spec for impossible/invalid situations
pub fn validate_spec(spec: &Spec, strict: bool) -> ValidationReport {
    let mut issues = Vec::new();
    let mut code_counter = 1;

    // 1. Type mismatch detection
    issues.extend(detect_type_mismatches(spec, &mut code_counter));

    // 2. Unsatisfiable condition detection
    issues.extend(detect_unsatisfiable(spec, &mut code_counter));

    // 3. Tautology detection
    issues.extend(detect_tautologies(spec, &mut code_counter));

    // 4. Dead rule detection
    issues.extend(detect_dead_rules(spec, &mut code_counter));

    // 5. Contradictory rules detection
    issues.extend(detect_contradictions(spec, &mut code_counter));

    // Generate fixes for each issue
    let fixes = generate_fixes(&issues, spec);

    // Count errors and warnings
    let error_count = issues
        .iter()
        .filter(|i| matches!(i.severity, Severity::Error))
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| matches!(i.severity, Severity::Warning))
        .count();

    // In strict mode, warnings become errors
    if strict {
        for issue in &mut issues {
            if matches!(issue.severity, Severity::Warning) {
                issue.severity = Severity::Error;
            }
        }
    }

    let is_valid = if strict {
        issues.is_empty()
    } else {
        error_count == 0
    };

    ValidationReport {
        is_valid,
        issues,
        fixes,
        error_count: if strict {
            error_count + warning_count
        } else {
            error_count
        },
        warning_count: if strict { 0 } else { warning_count },
    }
}

/// Generate fixes for validation issues
fn generate_fixes(issues: &[ValidationIssue], spec: &Spec) -> Vec<SpecFix> {
    let mut fixes = Vec::new();

    for issue in issues {
        match issue.issue_type {
            IssueType::ContradictoryRules => {
                if let Some(fix) = generate_contradiction_fix(issue, spec) {
                    fixes.push(fix);
                }
            }
            IssueType::DeadRule => {
                if let Some(fix) = generate_dead_rule_fix(issue) {
                    fixes.push(fix);
                }
            }
            IssueType::TautologyCondition => {
                if let Some(fix) = generate_tautology_fix(issue, spec) {
                    fixes.push(fix);
                }
            }
            IssueType::TypeMismatch => {
                if let Some(fix) = generate_type_mismatch_fix(issue, spec) {
                    fixes.push(fix);
                }
            }
            IssueType::UnsatisfiableCondition => {
                // Low confidence - requires manual review
                if let Some(fix) = generate_unsatisfiable_fix(issue) {
                    fixes.push(fix);
                }
            }
        }
    }

    fixes
}

/// Generate fix for contradictory rules
fn generate_contradiction_fix(issue: &ValidationIssue, spec: &Spec) -> Option<SpecFix> {
    if issue.affected_rules.len() >= 2 {
        let rule_id = &issue.affected_rules[0];
        // Find the rule to get its current priority
        let rule = spec.rules.iter().find(|r| r.id == *rule_id)?;
        let new_priority = rule.priority + 1;

        Some(SpecFix {
            issue_code: issue.code.clone(),
            confidence: FixConfidence::High,
            operation: FixOperation::AddPriority {
                rule_id: rule_id.clone(),
                priority: new_priority,
            },
            description: format!(
                "Add priority {} to rule {} to resolve conflict with {}",
                new_priority, rule_id, issue.affected_rules[1]
            ),
        })
    } else {
        None
    }
}

/// Generate fix for dead rule
fn generate_dead_rule_fix(issue: &ValidationIssue) -> Option<SpecFix> {
    if !issue.affected_rules.is_empty() {
        let rule_id = &issue.affected_rules[0];
        Some(SpecFix {
            issue_code: issue.code.clone(),
            confidence: FixConfidence::High,
            operation: FixOperation::DeleteRule {
                rule_id: rule_id.clone(),
            },
            description: format!("Delete rule {} as it can never fire", rule_id),
        })
    } else {
        None
    }
}

/// Generate fix for tautology condition
fn generate_tautology_fix(issue: &ValidationIssue, spec: &Spec) -> Option<SpecFix> {
    if !issue.affected_rules.is_empty() {
        let rule_id = &issue.affected_rules[0];
        let rule = spec.rules.iter().find(|r| r.id == *rule_id)?;

        Some(SpecFix {
            issue_code: issue.code.clone(),
            confidence: FixConfidence::Medium,
            operation: FixOperation::UpdateRule {
                rule_id: rule_id.clone(),
                field: "when".to_string(),
                old_value: rule.as_cel()?,
                new_value: "null".to_string(), // Remove condition, make it default
            },
            description: format!(
                "Remove condition from rule {} and mark as default rule",
                rule_id
            ),
        })
    } else {
        None
    }
}

/// Generate fix for type mismatch
fn generate_type_mismatch_fix(issue: &ValidationIssue, spec: &Spec) -> Option<SpecFix> {
    if !issue.affected_rules.is_empty() {
        let rule_id = &issue.affected_rules[0];
        let rule = spec.rules.iter().find(|r| r.id == *rule_id)?;

        // Extract the problematic expression
        let old_expr = rule.as_cel()?;

        // Try to infer a fix based on context
        // This is a simplified version - in practice, we'd need more sophisticated type inference
        let new_expr = suggest_type_fix(&old_expr, &issue.context);

        Some(SpecFix {
            issue_code: issue.code.clone(),
            confidence: FixConfidence::Medium,
            operation: FixOperation::UpdateExpression {
                rule_id: rule_id.clone(),
                old_expression: old_expr,
                new_expression: new_expr,
            },
            description: format!(
                "Fix type mismatch in rule {} by correcting the CEL expression",
                rule_id
            ),
        })
    } else {
        None
    }
}

/// Suggest a type fix for a CEL expression
fn suggest_type_fix(expr: &str, context: &Option<IssueContext>) -> String {
    // Basic heuristics for common type mismatches
    // This is a simplified version - could be enhanced with proper AST analysis

    // If context has type info, use it
    if let Some(ctx) = context {
        if let Some(type_info) = &ctx.type_info {
            // Try to infer fix from type info
            if type_info.contains("string") && expr.contains(">") {
                // Likely comparing string with number
                return format!("int({})", expr.replace(">", "").trim());
            }
        }
    }

    // Default: return original (requires manual fix)
    expr.to_string()
}

/// Generate fix for unsatisfiable condition
fn generate_unsatisfiable_fix(issue: &ValidationIssue) -> Option<SpecFix> {
    if !issue.affected_rules.is_empty() {
        let rule_id = &issue.affected_rules[0];
        Some(SpecFix {
            issue_code: issue.code.clone(),
            confidence: FixConfidence::Low,
            operation: FixOperation::DeleteRule {
                rule_id: rule_id.clone(),
            },
            description: format!(
                "Rule {} has unsatisfiable condition - review and fix or delete",
                rule_id
            ),
        })
    } else {
        None
    }
}

/// Detect type mismatches in CEL expressions
fn detect_type_mismatches(spec: &Spec, code_counter: &mut usize) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let var_types: HashMap<String, crate::spec::VarType> = spec
        .inputs
        .iter()
        .map(|v| (v.name.clone(), v.typ.clone()))
        .collect();

    for rule in &spec.rules {
        if let Some(when) = rule.as_cel() {
            if let Err(e) = check_cel_types(&when, &var_types) {
                // Extract variable names from the expression using CEL AST
                let vars_in_expr: Vec<String> = crate::cel::CelCompiler::extract_variables(&when)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|v| var_types.contains_key(v))
                    .collect();

                // Get type information
                let type_info = extract_type_mismatch_info(&when, &var_types);

                issues.push(ValidationIssue {
                    code: format!("V{:03}", {
                        let c = *code_counter;
                        *code_counter += 1;
                        c
                    }),
                    severity: Severity::Error,
                    issue_type: IssueType::TypeMismatch,
                    message: format!("Type mismatch in rule {}: {}", rule.id, e),
                    affected_rules: vec![rule.id.clone()],
                    explanation: Some(format!(
                        "The condition '{}' compares incompatible types. This will cause a runtime error when evaluating the expression.",
                        when
                    )),
                    suggestion: Some(
                        "1. Check the variable types in your spec inputs\n2. Ensure comparisons use compatible types (int with int/float, string with string, etc.)\n3. Convert types explicitly if needed (e.g., int(user.age) or string(user.id))".into()
                    ),
                    fix_example: Some(format!(
                        "# Example fix - ensure types match:\nrules:\n  - id: {}\n    when: \"# Fix: Use compatible types\"\n    then: ...",
                        rule.id
                    )),
                    context: Some(IssueContext {
                        cel_expressions: Some(vec![when.clone()]),
                        variables: if vars_in_expr.is_empty() { None } else { Some(vars_in_expr) },
                        type_info,
                        example_input: None,
                        current_behavior: Some("Expression will fail to evaluate due to type mismatch".into()),
                        expected_behavior: Some("Expression should compare compatible types".into()),
                    }),
                });
            }
        }
    }

    issues
}

/// Check CEL expression for type mismatches
fn check_cel_types(
    expr: &str,
    var_types: &HashMap<String, crate::spec::VarType>,
) -> Result<(), String> {
    // Parse the expression
    let ast = match cel_parser::Parser::new().parse(expr) {
        Ok(ast) => ast,
        Err(_) => return Ok(()), // Can't parse - skip type checking
    };

    // Walk AST and check types
    check_ast_types(&ast, var_types)
}

/// Recursively check AST for type mismatches
fn check_ast_types(
    expr: &cel_parser::Expression,
    var_types: &HashMap<String, crate::spec::VarType>,
) -> Result<(), String> {
    use cel_parser::ast::operators;
    use cel_parser::ast::Expr as E;

    match &expr.expr {
        E::Ident(id) => {
            // Check if variable exists and has valid type
            if let Some(_var_type) = var_types.get(id.as_str()) {
                Ok(())
            } else {
                Err(format!("Unknown variable: {}", id))
            }
        }
        E::Call(call) => {
            // Check if this is a relation operator
            if call.func_name == operators::EQUALS || call.func_name == operators::NOT_EQUALS {
                if call.args.len() == 2 {
                    check_same_types(&call.args[0], &call.args[1], var_types)?;
                }
            } else if call.func_name == operators::GREATER
                || call.func_name == operators::LESS
                || call.func_name == operators::GREATER_EQUALS
                || call.func_name == operators::LESS_EQUALS
            {
                if call.args.len() == 2 {
                    check_comparable_types(&call.args[0], &call.args[1], var_types)?;
                }
            } else if call.func_name == operators::LOGICAL_AND
                || call.func_name == operators::LOGICAL_OR
            {
                if call.args.len() == 2 {
                    check_ast_types(&call.args[0], var_types)?;
                    check_ast_types(&call.args[1], var_types)?;
                }
            } else if call.func_name == operators::LOGICAL_NOT {
                if let Some(operand) = call.args.first() {
                    check_ast_types(operand, var_types)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Check if two expressions have comparable types
fn check_comparable_types(
    left: &cel_parser::Expression,
    right: &cel_parser::Expression,
    var_types: &HashMap<String, crate::spec::VarType>,
) -> Result<(), String> {
    let left_type = infer_type(left, var_types);
    let right_type = infer_type(right, var_types);

    match (left_type, right_type) {
        (Some(crate::spec::VarType::Int), Some(crate::spec::VarType::Int)) => Ok(()),
        (Some(crate::spec::VarType::Float), Some(crate::spec::VarType::Float)) => Ok(()),
        (Some(crate::spec::VarType::Int), Some(crate::spec::VarType::Float)) => Ok(()),
        (Some(crate::spec::VarType::Float), Some(crate::spec::VarType::Int)) => Ok(()),
        (Some(l), Some(r)) => Err(format!(
            "Cannot compare {} with {}",
            format_type(&l),
            format_type(&r)
        )),
        _ => Ok(()), // Can't infer - skip
    }
}

/// Check if two expressions have the same type
fn check_same_types(
    left: &cel_parser::Expression,
    right: &cel_parser::Expression,
    var_types: &HashMap<String, crate::spec::VarType>,
) -> Result<(), String> {
    let left_type = infer_type(left, var_types);
    let right_type = infer_type(right, var_types);

    if let (Some(l), Some(r)) = (left_type, right_type) {
        if l != r {
            return Err(format!(
                "Type mismatch: {} vs {}",
                format_type(&l),
                format_type(&r)
            ));
        }
    }

    Ok(())
}

/// Infer the type of an expression
fn infer_type(
    expr: &cel_parser::Expression,
    var_types: &HashMap<String, crate::spec::VarType>,
) -> Option<crate::spec::VarType> {
    use cel_parser::ast::Expr as E;
    use cel_parser::reference::Val;

    match &expr.expr {
        E::Ident(id) => var_types.get(id.as_str()).cloned(),
        E::Literal(val) => match val {
            Val::Int(_) | Val::UInt(_) => Some(crate::spec::VarType::Int),
            Val::Double(_) => Some(crate::spec::VarType::Float),
            Val::String(_) => Some(crate::spec::VarType::String),
            Val::Boolean(_) => Some(crate::spec::VarType::Bool),
            _ => None,
        },
        _ => None,
    }
}

/// Format a type for error messages
fn format_type(typ: &crate::spec::VarType) -> &str {
    match typ {
        crate::spec::VarType::Int => "int",
        crate::spec::VarType::Float => "float",
        crate::spec::VarType::String => "string",
        crate::spec::VarType::Bool => "bool",
        crate::spec::VarType::Enum(_) => "enum",
        crate::spec::VarType::List(_) => "list",
        crate::spec::VarType::Object => "object",
    }
}

/// Extract type mismatch information from a CEL expression
fn extract_type_mismatch_info(
    _expr: &str,
    _var_types: &HashMap<String, crate::spec::VarType>,
) -> Option<String> {
    // Placeholder - could be enhanced to provide detailed type info
    None
}

/// Detect unsatisfiable conditions (can never be true)
fn detect_unsatisfiable(spec: &Spec, code_counter: &mut usize) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Build predicate set
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

    // Check each rule
    for rule in &spec.rules {
        if let Some(cel_expr) = rule.as_cel() {
            // Convert to cover
            let rule_cover = rules_to_cover(std::slice::from_ref(rule), &predicate_set);

            // Check if cover has zero minterms (unsatisfiable)
            if !predicate_set.is_empty() {
                let combinations_covered =
                    count_combinations_in_cover(&rule_cover, predicate_set.len());
                if combinations_covered == 0 {
                    issues.push(ValidationIssue {
                        code: format!("V{:03}", {
                            let c = *code_counter;
                            *code_counter += 1;
                            c
                        }),
                        severity: Severity::Error,
                        issue_type: IssueType::UnsatisfiableCondition,
                        message: format!(
                            "Rule {} has unsatisfiable condition: {}",
                            rule.id, cel_expr
                        ),
                        affected_rules: vec![rule.id.clone()],
                        explanation: None,
                        suggestion: Some("Fix the condition logic - it can never be true".into()),
                        fix_example: None,
                        context: None,
                    });
                }
            }
        }
    }

    issues
}

/// Detect tautology conditions (always match)
fn detect_tautologies(spec: &Spec, code_counter: &mut usize) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Build predicate set
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

    let total_combinations = 1u64 << predicate_set.len();

    // Check each rule
    for rule in &spec.rules {
        if let Some(cel_expr) = rule.as_cel() {
            // Count how many combinations this rule covers
            let cover = rules_to_cover(std::slice::from_ref(rule), &predicate_set);
            let combinations_covered = count_combinations_in_cover(&cover, predicate_set.len());

            // If it covers all combinations, it's a tautology
            if combinations_covered == total_combinations && total_combinations > 0 {
                issues.push(ValidationIssue {
                    code: format!("V{:03}", {
                        let c = *code_counter;
                        *code_counter += 1;
                        c
                    }),
                    severity: Severity::Warning,
                    issue_type: IssueType::TautologyCondition,
                    message: format!("Rule {} always matches (tautology): {}", rule.id, cel_expr),
                    affected_rules: vec![rule.id.clone()],
                    explanation: None,
                    suggestion: Some(
                        "Consider removing the condition or marking as default rule".into(),
                    ),
                    fix_example: None,
                    context: None,
                });
            }
        }
    }

    issues
}

/// Count combinations covered by a cover
fn count_combinations_in_cover(cover: &Cover, num_predicates: usize) -> u64 {
    if num_predicates == 0 {
        return 1;
    }

    let total = 1u64 << num_predicates;
    let mut count = 0u64;

    for combo in 0..total {
        if cover.covers_minterm(combo) {
            count += 1;
        }
    }

    count
}

/// Detect dead rules (covered by earlier rules)
fn detect_dead_rules(spec: &Spec, code_counter: &mut usize) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Build predicate set
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

    // Track what's been covered so far
    let mut covered_so_far = Cover::new(predicate_set.len(), 1);

    for (idx, rule) in spec.rules.iter().enumerate() {
        if rule.as_cel().is_some() {
            let rule_cover = rules_to_cover(std::slice::from_ref(rule), &predicate_set);

            // Check if this rule's cover is a subset of what's already covered
            if is_subset(&rule_cover, &covered_so_far, predicate_set.len()) {
                // Find which earlier rules cover this
                let covering_rules = find_covering_rules(&spec.rules[..idx], rule, &predicate_set);

                issues.push(ValidationIssue {
                    code: format!("V{:03}", {
                        let c = *code_counter;
                        *code_counter += 1;
                        c
                    }),
                    severity: Severity::Warning,
                    issue_type: IssueType::DeadRule,
                    message: format!("Rule {} can never fire (covered by earlier rules)", rule.id),
                    affected_rules: {
                        let mut affected = vec![rule.id.clone()];
                        affected.extend(covering_rules);
                        affected
                    },
                    explanation: None,
                    suggestion: Some("Remove this rule or adjust its conditions".into()),
                    fix_example: None,
                    context: None,
                });
            }

            // Add this rule's cover to what's covered so far
            covered_so_far = union_covers(&covered_so_far, &rule_cover);
        }
    }

    issues
}

/// Check if cover_a is a subset of cover_b
fn is_subset(cover_a: &Cover, cover_b: &Cover, num_predicates: usize) -> bool {
    let total = 1u64 << num_predicates;
    for combo in 0..total {
        if cover_a.covers_minterm(combo) && !cover_b.covers_minterm(combo) {
            return false;
        }
    }
    true
}

/// Find which earlier rules cover a given rule
fn find_covering_rules(
    earlier_rules: &[Rule],
    rule: &Rule,
    predicate_set: &PredicateSet,
) -> Vec<String> {
    let mut covering = Vec::new();
    let rule_cover = rules_to_cover(std::slice::from_ref(rule), predicate_set);

    for earlier_rule in earlier_rules {
        if earlier_rule.as_cel().is_some() {
            let earlier_cover = rules_to_cover(std::slice::from_ref(earlier_rule), predicate_set);
            // Check if earlier_cover covers rule_cover
            if covers_cover(&earlier_cover, &rule_cover, predicate_set.len()) {
                covering.push(earlier_rule.id.clone());
            }
        }
    }

    covering
}

/// Check if cover_a covers all minterms in cover_b
fn covers_cover(cover_a: &Cover, cover_b: &Cover, num_predicates: usize) -> bool {
    let total = 1u64 << num_predicates;
    for combo in 0..total {
        if cover_b.covers_minterm(combo) && !cover_a.covers_minterm(combo) {
            return false;
        }
    }
    true
}

/// Union two covers
fn union_covers(cover_a: &Cover, cover_b: &Cover) -> Cover {
    // Create a new cover with all cubes from both
    let mut result = Cover::new(cover_a.num_inputs(), cover_a.num_outputs());

    // Add all cubes from cover_a
    for i in 0..cover_a.len() {
        if let Some(cube) = cover_a.get(i) {
            result.add(cube.clone());
        }
    }

    // Add all cubes from cover_b
    for i in 0..cover_b.len() {
        if let Some(cube) = cover_b.get(i) {
            result.add(cube.clone());
        }
    }

    result
}

/// Detect contradictory rules (same condition, different outputs, no priority)
fn detect_contradictions(spec: &Spec, code_counter: &mut usize) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Build predicate set
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

    // Compare all pairs of rules
    for (i, rule_a) in spec.rules.iter().enumerate() {
        for rule_b in spec.rules.iter().skip(i + 1) {
            if rule_a.as_cel().is_some() && rule_b.as_cel().is_some() {
                // Check if they have overlapping covers
                let cover_a = rules_to_cover(std::slice::from_ref(rule_a), &predicate_set);
                let cover_b = rules_to_cover(std::slice::from_ref(rule_b), &predicate_set);

                if covers_intersect(&cover_a, &cover_b, predicate_set.len()) {
                    // Check if outputs differ and priorities are same
                    if rule_a.then != rule_b.then && rule_a.priority == rule_b.priority {
                        issues.push(ValidationIssue {
                            code: format!("V{:03}", {
                                let c = *code_counter;
                                *code_counter += 1;
                                c
                            }),
                            severity: Severity::Error,
                            issue_type: IssueType::ContradictoryRules,
                            message: format!(
                                "Contradictory rules: {} and {} match same inputs with different outputs",
                                rule_a.id, rule_b.id
                            ),
                            affected_rules: vec![rule_a.id.clone(), rule_b.id.clone()],
                            explanation: None,
                            suggestion: Some(
                                "Set priority on one rule or merge with conditional output".into(),
                            ),
                            fix_example: None,
                            context: None,
                        });
                    }
                }
            }
        }
    }

    issues
}

/// Check if two covers intersect (have overlapping minterms)
fn covers_intersect(cover_a: &Cover, cover_b: &Cover, num_predicates: usize) -> bool {
    let total = 1u64 << num_predicates;
    for combo in 0..total {
        if cover_a.covers_minterm(combo) && cover_b.covers_minterm(combo) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConditionValue, Output, VarType, WhenClause};

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
            rules: vec![],
            default: None,
            meta: Default::default(),
            scoping: None,
        }
    }

    #[test]
    fn test_validate_empty_spec() {
        let spec = make_test_spec();
        let report = validate_spec(&spec, false);
        assert!(report.is_valid);
    }

    #[test]
    fn test_detect_contradiction() {
        let mut spec = make_test_spec();
        spec.rules = vec![
            Rule {
                id: "R1".into(),
                when: Some(WhenClause::from("a")),
                conditions: None,
                then: Output::Single(ConditionValue::Int(1)),
                priority: 0,
                description: None,
            },
            Rule {
                id: "R2".into(),
                when: Some(WhenClause::from("a")), // Same condition!
                conditions: None,
                then: Output::Single(ConditionValue::Int(2)), // Different output!
                priority: 0,                                  // Same priority!
                description: None,
            },
        ];

        let report = validate_spec(&spec, false);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i.issue_type, IssueType::ContradictoryRules)));
    }
}
