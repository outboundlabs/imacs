//! Verification — check that code implements spec correctly
//!
//! Analyzes code AST to determine which spec rules are covered.
//! Reports gaps (uncovered rules) and coverage statistics.

use crate::ast::*;
use crate::spec::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Verify code against spec
pub fn verify(spec: &Spec, code: &CodeAst) -> VerificationResult {
    Verifier::new().verify(spec, code)
}

/// Code verifier
pub struct Verifier {
    config: VerifierConfig,
}

/// Verifier configuration
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Require all rules to be covered
    pub require_complete: bool,
    /// Allow extra branches not in spec
    pub allow_extra: bool,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            require_complete: true,
            allow_extra: true,
        }
    }
}

/// Result of verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether verification passed
    pub passed: bool,
    /// Coverage information
    pub coverage: Coverage,
    /// Gaps (uncovered rules)
    pub gaps: Vec<CoverageGap>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Spec hash
    pub spec_hash: String,
    /// Code hash
    pub code_hash: String,
}

/// Coverage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    pub total: usize,
    pub covered: usize,
    pub percentage: f32,
    pub covered_rules: Vec<String>,
    pub uncovered_rules: Vec<String>,
}

/// A gap in coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageGap {
    pub rule_id: String,
    pub reason: GapReason,
    pub expected_condition: String,
    pub expected_output: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GapReason {
    Missing,
    ConditionMismatch,
    OutputMismatch,
}

/// Extracted rule from code
#[derive(Debug, Clone)]
struct CodeRule {
    conditions: Vec<ExtractedCondition>,
    output: ExtractedOutput,
}

#[derive(Debug, Clone)]
struct ExtractedCondition {
    var: String,
    op: ConditionOp,
    value: ConditionValue,
}

#[derive(Debug, Clone)]
enum ExtractedOutput {
    Literal(ConditionValue),
}

enum MatchResult {
    Exact,
    ConditionMismatch(String),
    OutputMismatch(String),
    NotFound,
}

impl Verifier {
    pub fn new() -> Self {
        Self {
            config: VerifierConfig::default(),
        }
    }

    pub fn with_config(config: VerifierConfig) -> Self {
        Self { config }
    }

    pub fn verify(&self, spec: &Spec, code: &CodeAst) -> VerificationResult {
        let func = code
            .get_function(&spec.id)
            .or_else(|| code.functions.first());

        let func = match func {
            Some(f) => f,
            None => {
                return VerificationResult {
                    passed: false,
                    coverage: Coverage {
                        total: spec.rules.len(),
                        covered: 0,
                        percentage: 0.0,
                        covered_rules: vec![],
                        uncovered_rules: spec.rules.iter().map(|r| r.id.clone()).collect(),
                    },
                    gaps: spec
                        .rules
                        .iter()
                        .map(|r| CoverageGap {
                            rule_id: r.id.clone(),
                            reason: GapReason::Missing,
                            expected_condition: r.as_cel().unwrap_or_default(),
                            expected_output: r.then.to_string(),
                            suggestion: format!("Add function '{}'", spec.id),
                        })
                        .collect(),
                    warnings: vec!["No matching function found".into()],
                    spec_hash: spec.hash(),
                    code_hash: code.source_hash.clone(),
                };
            }
        };

        // Extract rules from code
        let code_rules = self.extract_code_rules(&func.body, &spec.inputs);

        // Match spec rules against code
        let mut covered = HashSet::new();
        let mut gaps = Vec::new();
        let warnings = Vec::new();

        for spec_rule in &spec.rules {
            match self.find_matching_rule(spec_rule, &code_rules, spec) {
                MatchResult::Exact => {
                    covered.insert(spec_rule.id.clone());
                }
                MatchResult::ConditionMismatch(detail) => {
                    gaps.push(CoverageGap {
                        rule_id: spec_rule.id.clone(),
                        reason: GapReason::ConditionMismatch,
                        expected_condition: spec_rule.as_cel().unwrap_or_default(),
                        expected_output: spec_rule.then.to_string(),
                        suggestion: format!("Fix condition: {}", detail),
                    });
                }
                MatchResult::OutputMismatch(detail) => {
                    gaps.push(CoverageGap {
                        rule_id: spec_rule.id.clone(),
                        reason: GapReason::OutputMismatch,
                        expected_condition: spec_rule.as_cel().unwrap_or_default(),
                        expected_output: spec_rule.then.to_string(),
                        suggestion: format!("Fix output: {}", detail),
                    });
                }
                MatchResult::NotFound => {
                    gaps.push(CoverageGap {
                        rule_id: spec_rule.id.clone(),
                        reason: GapReason::Missing,
                        expected_condition: spec_rule.as_cel().unwrap_or_default(),
                        expected_output: spec_rule.then.to_string(),
                        suggestion: "Add this rule to the code".into(),
                    });
                }
            }
        }

        let covered_rules: Vec<_> = covered.into_iter().collect();
        let uncovered_rules: Vec<_> = spec
            .rules
            .iter()
            .filter(|r| !covered_rules.contains(&r.id))
            .map(|r| r.id.clone())
            .collect();

        let coverage = Coverage {
            total: spec.rules.len(),
            covered: covered_rules.len(),
            percentage: if spec.rules.is_empty() {
                100.0
            } else {
                (covered_rules.len() as f32 / spec.rules.len() as f32) * 100.0
            },
            covered_rules,
            uncovered_rules,
        };

        let passed = gaps.is_empty() || (!self.config.require_complete && coverage.covered > 0);

        VerificationResult {
            passed,
            coverage,
            gaps,
            warnings,
            spec_hash: spec.hash(),
            code_hash: code.source_hash.clone(),
        }
    }

    fn extract_code_rules(&self, body: &AstNode, inputs: &[Variable]) -> Vec<CodeRule> {
        let mut rules = Vec::new();
        self.extract_from_node(body, inputs, &mut vec![], &mut rules);
        rules
    }

    fn extract_from_node(
        &self,
        node: &AstNode,
        inputs: &[Variable],
        current_conditions: &mut Vec<ExtractedCondition>,
        rules: &mut Vec<CodeRule>,
    ) {
        match node {
            AstNode::Match { arms, .. } => {
                for arm in arms {
                    let mut arm_conditions = current_conditions.clone();
                    self.extract_pattern_conditions(&arm.pattern, inputs, &mut arm_conditions);

                    let output = self.extract_output(&arm.body);

                    rules.push(CodeRule {
                        conditions: arm_conditions,
                        output,
                    });
                }
            }

            AstNode::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                // Then branch
                let mut then_conditions = current_conditions.clone();
                self.extract_expr_conditions(condition, &mut then_conditions, false);

                let then_output = self.extract_output(then_branch);
                rules.push(CodeRule {
                    conditions: then_conditions,
                    output: then_output,
                });

                // Else branch
                if let Some(else_node) = else_branch {
                    let mut else_conditions = current_conditions.clone();
                    self.extract_expr_conditions(condition, &mut else_conditions, true);
                    self.extract_from_node(else_node, inputs, &mut else_conditions, rules);
                }
            }

            AstNode::Block { result, .. } => {
                if let Some(inner) = result {
                    self.extract_from_node(inner, inputs, current_conditions, rules);
                }
            }

            _ => {
                // Leaf node - extract as output
                let output = self.extract_output(node);
                if !current_conditions.is_empty() {
                    rules.push(CodeRule {
                        conditions: current_conditions.clone(),
                        output,
                    });
                }
            }
        }
    }

    fn extract_pattern_conditions(
        &self,
        pattern: &Pattern,
        inputs: &[Variable],
        conditions: &mut Vec<ExtractedCondition>,
    ) {
        match pattern {
            Pattern::Tuple(elements) => {
                for (i, elem) in elements.iter().enumerate() {
                    if let Some(input) = inputs.get(i) {
                        self.extract_single_pattern(&input.name, elem, conditions);
                    }
                }
            }
            Pattern::Literal(lit) => {
                if let Some(input) = inputs.first() {
                    conditions.push(ExtractedCondition {
                        var: input.name.clone(),
                        op: ConditionOp::Eq,
                        value: literal_to_condition_value(lit),
                    });
                }
            }
            Pattern::Wildcard | Pattern::Binding(_) => {
                // Matches anything
            }
            _ => {}
        }
    }

    fn extract_single_pattern(
        &self,
        var_name: &str,
        pattern: &Pattern,
        conditions: &mut Vec<ExtractedCondition>,
    ) {
        match pattern {
            Pattern::Literal(lit) => {
                conditions.push(ExtractedCondition {
                    var: var_name.to_string(),
                    op: ConditionOp::Eq,
                    value: literal_to_condition_value(lit),
                });
            }
            Pattern::Wildcard | Pattern::Binding(_) => {
                // Matches anything - no condition
            }
            _ => {}
        }
    }

    fn extract_expr_conditions(
        &self,
        expr: &AstNode,
        conditions: &mut Vec<ExtractedCondition>,
        negated: bool,
    ) {
        match expr {
            AstNode::Binary {
                op: BinaryOp::And,
                left,
                right,
                ..
            } => {
                if negated {
                    // De Morgan: !(a && b) = !a || !b - harder to handle
                    // For now, just mark as complex
                } else {
                    self.extract_expr_conditions(left, conditions, false);
                    self.extract_expr_conditions(right, conditions, false);
                }
            }

            AstNode::Binary {
                op: BinaryOp::Or,
                left,
                right,
                ..
            } => {
                if negated {
                    // De Morgan: !(a || b) = !a && !b
                    self.extract_expr_conditions(left, conditions, true);
                    self.extract_expr_conditions(right, conditions, true);
                }
            }

            AstNode::Binary {
                op, left, right, ..
            } => {
                if let AstNode::Var { name, .. } = left.as_ref() {
                    if let Some(value) = self.extract_literal_value(right) {
                        let actual_op = if negated {
                            negate_op(*op)
                        } else {
                            binary_to_condition_op(*op)
                        };
                        conditions.push(ExtractedCondition {
                            var: name.clone(),
                            op: actual_op,
                            value,
                        });
                    }
                }
            }

            AstNode::Unary {
                op: UnaryOp::Not,
                operand,
                ..
            } => {
                self.extract_expr_conditions(operand, conditions, !negated);
            }

            AstNode::Var { name, .. } => {
                // Bare variable = truthy check
                conditions.push(ExtractedCondition {
                    var: name.clone(),
                    op: ConditionOp::Eq,
                    value: ConditionValue::Bool(!negated),
                });
            }

            _ => {}
        }
    }

    fn extract_literal_value(&self, node: &AstNode) -> Option<ConditionValue> {
        match node {
            AstNode::Literal { value, .. } => Some(literal_to_condition_value(value)),
            _ => None,
        }
    }

    fn extract_output(&self, node: &AstNode) -> ExtractedOutput {
        match node {
            AstNode::Literal { value, .. } => {
                ExtractedOutput::Literal(literal_to_condition_value(value))
            }
            AstNode::Block {
                result: Some(inner),
                ..
            } => self.extract_output(inner),
            AstNode::Return {
                value: Some(inner), ..
            } => self.extract_output(inner),
            _ => ExtractedOutput::Literal(ConditionValue::Null),
        }
    }

    fn find_matching_rule(
        &self,
        spec_rule: &Rule,
        code_rules: &[CodeRule],
        spec: &Spec,
    ) -> MatchResult {
        // Build expected conditions from spec rule
        let expected = self.build_expected_conditions(spec_rule, spec);

        for code_rule in code_rules {
            if self.conditions_match(&expected, &code_rule.conditions) {
                // Check output matches
                if self.output_matches(&spec_rule.then, &code_rule.output) {
                    return MatchResult::Exact;
                } else {
                    return MatchResult::OutputMismatch(format!(
                        "expected {}, got {:?}",
                        spec_rule.then, code_rule.output
                    ));
                }
            }
        }

        // Check if there's a partial match
        for code_rule in code_rules {
            if self.conditions_partial_match(&expected, &code_rule.conditions) {
                return MatchResult::ConditionMismatch("Partial match found".into());
            }
        }

        MatchResult::NotFound
    }

    fn build_expected_conditions(&self, rule: &Rule, _spec: &Spec) -> Vec<ExtractedCondition> {
        let mut conditions = Vec::new();

        if let Some(conds) = &rule.conditions {
            for cond in conds {
                conditions.push(ExtractedCondition {
                    var: cond.var.clone(),
                    op: cond.op,
                    value: cond.value.clone(),
                });
            }
        }

        // TODO: Parse CEL conditions from rule.when

        conditions
    }

    fn conditions_match(
        &self,
        expected: &[ExtractedCondition],
        actual: &[ExtractedCondition],
    ) -> bool {
        if expected.len() != actual.len() {
            return false;
        }

        for exp in expected {
            let found = actual
                .iter()
                .any(|act| act.var == exp.var && act.op == exp.op && act.value == exp.value);
            if !found {
                return false;
            }
        }

        true
    }

    fn conditions_partial_match(
        &self,
        expected: &[ExtractedCondition],
        actual: &[ExtractedCondition],
    ) -> bool {
        // At least one condition matches
        for exp in expected {
            let found = actual.iter().any(|act| act.var == exp.var);
            if found {
                return true;
            }
        }
        false
    }

    fn output_matches(&self, expected: &Output, actual: &ExtractedOutput) -> bool {
        match (expected, actual) {
            (Output::Single(exp_val), ExtractedOutput::Literal(act_val)) => exp_val == act_val,
            _ => false,
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

fn literal_to_condition_value(lit: &LiteralValue) -> ConditionValue {
    match lit {
        LiteralValue::Bool(b) => ConditionValue::Bool(*b),
        LiteralValue::Int(i) => ConditionValue::Int(*i),
        LiteralValue::Float(f) => ConditionValue::Float(*f),
        LiteralValue::String(s) => ConditionValue::String(s.clone()),
        LiteralValue::Char(c) => ConditionValue::String(c.to_string()),
        LiteralValue::Unit => ConditionValue::Null,
    }
}

fn binary_to_condition_op(op: BinaryOp) -> ConditionOp {
    match op {
        BinaryOp::Eq => ConditionOp::Eq,
        BinaryOp::Ne => ConditionOp::Ne,
        BinaryOp::Lt => ConditionOp::Lt,
        BinaryOp::Le => ConditionOp::Le,
        BinaryOp::Gt => ConditionOp::Gt,
        BinaryOp::Ge => ConditionOp::Ge,
        _ => ConditionOp::Eq,
    }
}

fn negate_op(op: BinaryOp) -> ConditionOp {
    match op {
        BinaryOp::Eq => ConditionOp::Ne,
        BinaryOp::Ne => ConditionOp::Eq,
        BinaryOp::Lt => ConditionOp::Ge,
        BinaryOp::Le => ConditionOp::Gt,
        BinaryOp::Gt => ConditionOp::Le,
        BinaryOp::Ge => ConditionOp::Lt,
        _ => ConditionOp::Eq,
    }
}

impl VerificationResult {
    /// Get list of gap descriptions
    pub fn gap_descriptions(&self) -> Vec<String> {
        self.gaps
            .iter()
            .map(|g| {
                format!(
                    "{}: when {} → {} ({})",
                    g.rule_id, g.expected_condition, g.expected_output, g.suggestion
                )
            })
            .collect()
    }

    /// Format as human-readable report
    pub fn to_report(&self) -> String {
        let mut out = String::new();

        let status = if self.passed {
            "✓ PASSED"
        } else {
            "✗ FAILED"
        };
        out.push_str(&format!("Verification: {}\n", status));
        out.push_str(&format!(
            "Coverage: {}/{} ({:.0}%)\n",
            self.coverage.covered, self.coverage.total, self.coverage.percentage
        ));

        if !self.gaps.is_empty() {
            out.push_str("\nGaps:\n");
            for gap in &self.gaps {
                out.push_str(&format!(
                    "  {} [{}]: {} → {}\n",
                    gap.rule_id,
                    match gap.reason {
                        GapReason::Missing => "MISSING",
                        GapReason::ConditionMismatch => "CONDITION",
                        GapReason::OutputMismatch => "OUTPUT",
                    },
                    gap.expected_condition,
                    gap.expected_output
                ));
                out.push_str(&format!("    → {}\n", gap.suggestion));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_rust;

    #[test]
    fn test_verify_simple_match() {
        let spec = Spec::from_yaml(
            r#"
id: check
inputs:
  - name: x
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    conditions:
      - var: x
        value: true
    then: 1
  - id: R2
    conditions:
      - var: x
        value: false
    then: 0
"#,
        )
        .unwrap();

        let code = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        false => 0,
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let result = verify(&spec, &ast);

        assert!(result.passed);
        assert_eq!(result.coverage.covered, 2);
    }

    #[test]
    fn test_verify_missing_rule() {
        let spec = Spec::from_yaml(
            r#"
id: check
inputs:
  - name: x
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    conditions:
      - var: x
        value: true
    then: 1
  - id: R2
    conditions:
      - var: x
        value: false
    then: 0
"#,
        )
        .unwrap();

        let code = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        _ => 999,
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let result = verify(&spec, &ast);

        assert!(!result.passed);
        assert!(result.gaps.iter().any(|g| g.rule_id == "R2"));
    }
}
