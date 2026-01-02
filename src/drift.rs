//! Drift detection — compare implementations across languages/files
//!
//! Detects when frontend and backend implementations diverge.
//! Compares decision logic regardless of syntax differences.

use crate::ast::*;
use crate::spec::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Compare two code implementations
pub fn compare(code_a: &CodeAst, code_b: &CodeAst) -> DriftReport {
    DriftDetector::new().compare(code_a, code_b)
}

/// Drift detector
pub struct DriftDetector {
    config: DriftConfig,
}

/// Drift detection configuration
#[derive(Debug, Clone)]
pub struct DriftConfig {
    /// Function name to compare (if None, compare first function)
    pub function_name: Option<String>,
    /// Tolerance for numeric differences
    pub numeric_tolerance: f64,
}

impl Default for DriftConfig {
    fn default() -> Self {
        Self {
            function_name: None,
            numeric_tolerance: 0.0,
        }
    }
}

/// Drift report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Overall status
    pub status: DriftStatus,
    /// File A info
    pub file_a: FileInfo,
    /// File B info
    pub file_b: FileInfo,
    /// Differences found
    pub differences: Vec<Difference>,
    /// Summary statistics
    pub summary: DriftSummary,
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub language: String,
    pub function: String,
    pub hash: String,
}

/// Overall drift status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftStatus {
    /// Implementations match
    Synced,
    /// Minor differences (warnings only)
    MinorDrift,
    /// Significant differences
    MajorDrift,
    /// Cannot compare
    Incomparable,
}

impl std::fmt::Display for DriftStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftStatus::Synced => write!(f, "SYNCED"),
            DriftStatus::MinorDrift => write!(f, "MINOR DRIFT"),
            DriftStatus::MajorDrift => write!(f, "MAJOR DRIFT"),
            DriftStatus::Incomparable => write!(f, "INCOMPARABLE"),
        }
    }
}

/// A difference between implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    /// Type of difference
    pub kind: DifferenceKind,
    /// Severity
    pub severity: DiffSeverity,
    /// Description
    pub description: String,
    /// Value in file A
    pub value_a: Option<String>,
    /// Value in file B
    pub value_b: Option<String>,
    /// Location hint
    pub location: Option<String>,
}

/// Types of differences
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferenceKind {
    /// Different number of rules/branches
    RuleCount,
    /// Condition differs
    Condition,
    /// Output value differs
    Output,
    /// Rule ordering differs
    Order,
    /// Missing case in one implementation
    MissingCase,
    /// Extra case in one implementation
    ExtraCase,
    /// Structural difference
    Structure,
}

/// Severity of difference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffSeverity {
    Info,
    Warning,
    Error,
}

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftSummary {
    pub total_differences: usize,
    pub errors: usize,
    pub warnings: usize,
    pub rules_a: usize,
    pub rules_b: usize,
    pub matching_rules: usize,
}

/// Internal representation of extracted rule for comparison
#[derive(Debug, Clone)]
struct NormalizedRule {
    conditions: HashMap<String, NormalizedCondition>,
    output: NormalizedOutput,
}

#[derive(Debug, Clone, PartialEq)]
struct NormalizedCondition {
    op: ConditionOp,
    value: NormalizedValue,
}

#[derive(Debug, Clone, PartialEq)]
enum NormalizedValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Any,
}

#[derive(Debug, Clone, PartialEq)]
enum NormalizedOutput {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Unknown,
}

impl DriftDetector {
    pub fn new() -> Self {
        Self {
            config: DriftConfig::default(),
        }
    }

    pub fn with_config(config: DriftConfig) -> Self {
        Self { config }
    }

    /// Compare two code ASTs
    pub fn compare(&self, code_a: &CodeAst, code_b: &CodeAst) -> DriftReport {
        let func_a = self.get_function(code_a);
        let func_b = self.get_function(code_b);

        let (func_a, func_b) = match (func_a, func_b) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                return DriftReport {
                    status: DriftStatus::Incomparable,
                    file_a: FileInfo {
                        language: format!("{:?}", code_a.language),
                        function: "none".into(),
                        hash: code_a.source_hash.clone(),
                    },
                    file_b: FileInfo {
                        language: format!("{:?}", code_b.language),
                        function: "none".into(),
                        hash: code_b.source_hash.clone(),
                    },
                    differences: vec![Difference {
                        kind: DifferenceKind::Structure,
                        severity: DiffSeverity::Error,
                        description: "Cannot find matching functions to compare".into(),
                        value_a: None,
                        value_b: None,
                        location: None,
                    }],
                    summary: DriftSummary {
                        total_differences: 1,
                        errors: 1,
                        warnings: 0,
                        rules_a: 0,
                        rules_b: 0,
                        matching_rules: 0,
                    },
                };
            }
        };

        // Extract normalized rules from both
        let rules_a = self.extract_normalized_rules(&func_a.body, &func_a.params);
        let rules_b = self.extract_normalized_rules(&func_b.body, &func_b.params);

        let mut differences = Vec::new();

        // Compare rule counts
        if rules_a.len() != rules_b.len() {
            differences.push(Difference {
                kind: DifferenceKind::RuleCount,
                severity: DiffSeverity::Warning,
                description: format!(
                    "Different number of rules: {} vs {}",
                    rules_a.len(),
                    rules_b.len()
                ),
                value_a: Some(rules_a.len().to_string()),
                value_b: Some(rules_b.len().to_string()),
                location: None,
            });
        }

        // Compare individual rules
        let mut matched_b: HashSet<usize> = HashSet::new();
        let mut matching_rules = 0;

        for (i, rule_a) in rules_a.iter().enumerate() {
            let mut found_match = false;

            for (j, rule_b) in rules_b.iter().enumerate() {
                if matched_b.contains(&j) {
                    continue;
                }

                if self.conditions_equivalent(&rule_a.conditions, &rule_b.conditions) {
                    matched_b.insert(j);
                    found_match = true;

                    // Check if outputs match
                    if !self.outputs_equivalent(&rule_a.output, &rule_b.output) {
                        differences.push(Difference {
                            kind: DifferenceKind::Output,
                            severity: DiffSeverity::Error,
                            description: format!("Rule {} has different output", i + 1),
                            value_a: Some(format!("{:?}", rule_a.output)),
                            value_b: Some(format!("{:?}", rule_b.output)),
                            location: Some(format!("Rule {}", i + 1)),
                        });
                    } else {
                        matching_rules += 1;
                    }

                    // Check ordering
                    if i != j {
                        differences.push(Difference {
                            kind: DifferenceKind::Order,
                            severity: DiffSeverity::Warning,
                            description: format!(
                                "Rule ordering differs: rule {} in A matches rule {} in B",
                                i + 1,
                                j + 1
                            ),
                            value_a: Some((i + 1).to_string()),
                            value_b: Some((j + 1).to_string()),
                            location: None,
                        });
                    }

                    break;
                }
            }

            if !found_match {
                differences.push(Difference {
                    kind: DifferenceKind::MissingCase,
                    severity: DiffSeverity::Error,
                    description: format!("Rule {} in A has no match in B", i + 1),
                    value_a: Some(format!("{:?}", rule_a.conditions)),
                    value_b: None,
                    location: Some(format!("Rule {}", i + 1)),
                });
            }
        }

        // Check for extra rules in B
        for (j, rule_b) in rules_b.iter().enumerate() {
            if !matched_b.contains(&j) {
                differences.push(Difference {
                    kind: DifferenceKind::ExtraCase,
                    severity: DiffSeverity::Warning,
                    description: format!("Rule {} in B has no match in A", j + 1),
                    value_a: None,
                    value_b: Some(format!("{:?}", rule_b.conditions)),
                    location: Some(format!("Rule {}", j + 1)),
                });
            }
        }

        // Determine overall status
        let errors = differences
            .iter()
            .filter(|d| d.severity == DiffSeverity::Error)
            .count();
        let warnings = differences
            .iter()
            .filter(|d| d.severity == DiffSeverity::Warning)
            .count();

        let status = if differences.is_empty() {
            DriftStatus::Synced
        } else if errors > 0 {
            DriftStatus::MajorDrift
        } else {
            DriftStatus::MinorDrift
        };

        DriftReport {
            status,
            file_a: FileInfo {
                language: format!("{:?}", code_a.language),
                function: func_a.name.clone(),
                hash: code_a.source_hash.clone(),
            },
            file_b: FileInfo {
                language: format!("{:?}", code_b.language),
                function: func_b.name.clone(),
                hash: code_b.source_hash.clone(),
            },
            differences,
            summary: DriftSummary {
                total_differences: errors + warnings,
                errors,
                warnings,
                rules_a: rules_a.len(),
                rules_b: rules_b.len(),
                matching_rules,
            },
        }
    }

    fn get_function<'a>(&self, code: &'a CodeAst) -> Option<&'a Function> {
        if let Some(name) = &self.config.function_name {
            code.get_function(name)
        } else {
            code.functions.first()
        }
    }

    fn extract_normalized_rules(
        &self,
        body: &AstNode,
        params: &[Parameter],
    ) -> Vec<NormalizedRule> {
        let mut rules = Vec::new();
        self.extract_rules_recursive(body, params, &mut HashMap::new(), &mut rules);
        rules
    }

    fn extract_rules_recursive(
        &self,
        node: &AstNode,
        params: &[Parameter],
        current: &mut HashMap<String, NormalizedCondition>,
        rules: &mut Vec<NormalizedRule>,
    ) {
        match node {
            AstNode::Match { arms, .. } => {
                for arm in arms {
                    let mut arm_conditions = current.clone();
                    self.extract_pattern_conditions(&arm.pattern, params, &mut arm_conditions);

                    if let Some(output) = self.extract_output(&arm.body) {
                        rules.push(NormalizedRule {
                            conditions: arm_conditions,
                            output,
                        });
                    }
                }
            }

            AstNode::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                // Then branch
                let mut then_conditions = current.clone();
                self.extract_expr_conditions(condition, &mut then_conditions, false);

                if let Some(output) = self.extract_output(then_branch) {
                    rules.push(NormalizedRule {
                        conditions: then_conditions,
                        output,
                    });
                } else {
                    self.extract_rules_recursive(then_branch, params, &mut then_conditions, rules);
                }

                // Else branch
                if let Some(else_node) = else_branch {
                    let mut else_conditions = current.clone();
                    self.extract_expr_conditions(condition, &mut else_conditions, true);
                    self.extract_rules_recursive(else_node, params, &mut else_conditions, rules);
                }
            }

            AstNode::Block { result, .. } => {
                if let Some(inner) = result {
                    self.extract_rules_recursive(inner, params, current, rules);
                }
            }

            _ => {}
        }
    }

    fn extract_pattern_conditions(
        &self,
        pattern: &Pattern,
        params: &[Parameter],
        conditions: &mut HashMap<String, NormalizedCondition>,
    ) {
        match pattern {
            Pattern::Tuple(elements) => {
                for (i, elem) in elements.iter().enumerate() {
                    if let Some(param) = params.get(i) {
                        self.extract_single_pattern(&param.name, elem, conditions);
                    }
                }
            }
            Pattern::Literal(lit) => {
                if let Some(param) = params.first() {
                    conditions.insert(
                        param.name.clone(),
                        NormalizedCondition {
                            op: ConditionOp::Eq,
                            value: self.literal_to_normalized(lit),
                        },
                    );
                }
            }
            Pattern::Wildcard | Pattern::Binding(_) => {
                // Matches anything - insert wildcard
                if let Some(param) = params.first() {
                    conditions.insert(
                        param.name.clone(),
                        NormalizedCondition {
                            op: ConditionOp::Eq,
                            value: NormalizedValue::Any,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    fn extract_single_pattern(
        &self,
        var: &str,
        pattern: &Pattern,
        conditions: &mut HashMap<String, NormalizedCondition>,
    ) {
        match pattern {
            Pattern::Literal(lit) => {
                conditions.insert(
                    var.to_string(),
                    NormalizedCondition {
                        op: ConditionOp::Eq,
                        value: self.literal_to_normalized(lit),
                    },
                );
            }
            Pattern::Wildcard | Pattern::Binding(_) => {
                conditions.insert(
                    var.to_string(),
                    NormalizedCondition {
                        op: ConditionOp::Eq,
                        value: NormalizedValue::Any,
                    },
                );
            }
            _ => {}
        }
    }

    fn extract_expr_conditions(
        &self,
        expr: &AstNode,
        conditions: &mut HashMap<String, NormalizedCondition>,
        negated: bool,
    ) {
        match expr {
            AstNode::Binary {
                op: BinaryOp::And,
                left,
                right,
                ..
            } if !negated => {
                self.extract_expr_conditions(left, conditions, false);
                self.extract_expr_conditions(right, conditions, false);
            }

            AstNode::Binary {
                op, left, right, ..
            } => {
                if let AstNode::Var { name, .. } = left.as_ref() {
                    if let Some(value) = self.node_to_normalized(right) {
                        let cond_op = if negated {
                            self.negate_op(self.binary_to_op(*op))
                        } else {
                            self.binary_to_op(*op)
                        };
                        conditions.insert(name.clone(), NormalizedCondition { op: cond_op, value });
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
                conditions.insert(
                    name.clone(),
                    NormalizedCondition {
                        op: ConditionOp::Eq,
                        value: NormalizedValue::Bool(!negated),
                    },
                );
            }

            _ => {}
        }
    }

    fn extract_output(&self, node: &AstNode) -> Option<NormalizedOutput> {
        match node {
            AstNode::Literal { value, .. } => Some(self.literal_to_output(value)),
            AstNode::Block {
                result: Some(inner),
                ..
            } => self.extract_output(inner),
            AstNode::Return {
                value: Some(inner), ..
            } => self.extract_output(inner),
            _ => None,
        }
    }

    fn conditions_equivalent(
        &self,
        a: &HashMap<String, NormalizedCondition>,
        b: &HashMap<String, NormalizedCondition>,
    ) -> bool {
        if a.len() != b.len() {
            return false;
        }

        for (key, cond_a) in a {
            match b.get(key) {
                Some(cond_b) => {
                    if !self.condition_equal(cond_a, cond_b) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }

    fn condition_equal(&self, a: &NormalizedCondition, b: &NormalizedCondition) -> bool {
        if a.op != b.op {
            return false;
        }

        match (&a.value, &b.value) {
            (NormalizedValue::Any, _) | (_, NormalizedValue::Any) => true,
            (NormalizedValue::Bool(va), NormalizedValue::Bool(vb)) => va == vb,
            (NormalizedValue::Int(va), NormalizedValue::Int(vb)) => va == vb,
            (NormalizedValue::Float(va), NormalizedValue::Float(vb)) => {
                (va - vb).abs() <= self.config.numeric_tolerance
            }
            (NormalizedValue::String(va), NormalizedValue::String(vb)) => va == vb,
            _ => false,
        }
    }

    fn outputs_equivalent(&self, a: &NormalizedOutput, b: &NormalizedOutput) -> bool {
        match (a, b) {
            (NormalizedOutput::Bool(va), NormalizedOutput::Bool(vb)) => va == vb,
            (NormalizedOutput::Int(va), NormalizedOutput::Int(vb)) => va == vb,
            (NormalizedOutput::Float(va), NormalizedOutput::Float(vb)) => {
                (va - vb).abs() <= self.config.numeric_tolerance
            }
            (NormalizedOutput::String(va), NormalizedOutput::String(vb)) => va == vb,
            (NormalizedOutput::Unknown, NormalizedOutput::Unknown) => true,
            _ => false,
        }
    }

    fn literal_to_normalized(&self, lit: &LiteralValue) -> NormalizedValue {
        match lit {
            LiteralValue::Bool(b) => NormalizedValue::Bool(*b),
            LiteralValue::Int(i) => NormalizedValue::Int(*i),
            LiteralValue::Float(f) => NormalizedValue::Float(*f),
            LiteralValue::String(s) => NormalizedValue::String(s.clone()),
            LiteralValue::Char(c) => NormalizedValue::String(c.to_string()),
            LiteralValue::Unit => NormalizedValue::Any,
        }
    }

    fn literal_to_output(&self, lit: &LiteralValue) -> NormalizedOutput {
        match lit {
            LiteralValue::Bool(b) => NormalizedOutput::Bool(*b),
            LiteralValue::Int(i) => NormalizedOutput::Int(*i),
            LiteralValue::Float(f) => NormalizedOutput::Float(*f),
            LiteralValue::String(s) => NormalizedOutput::String(s.clone()),
            LiteralValue::Char(c) => NormalizedOutput::String(c.to_string()),
            LiteralValue::Unit => NormalizedOutput::Unknown,
        }
    }

    fn node_to_normalized(&self, node: &AstNode) -> Option<NormalizedValue> {
        match node {
            AstNode::Literal { value, .. } => Some(self.literal_to_normalized(value)),
            _ => None,
        }
    }

    fn binary_to_op(&self, op: BinaryOp) -> ConditionOp {
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

    fn negate_op(&self, op: ConditionOp) -> ConditionOp {
        match op {
            ConditionOp::Eq => ConditionOp::Ne,
            ConditionOp::Ne => ConditionOp::Eq,
            ConditionOp::Lt => ConditionOp::Ge,
            ConditionOp::Le => ConditionOp::Gt,
            ConditionOp::Gt => ConditionOp::Le,
            ConditionOp::Ge => ConditionOp::Lt,
            other => other,
        }
    }
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DriftReport {
    /// Format as human-readable report
    pub fn to_report(&self) -> String {
        let mut out = String::new();

        out.push_str("DRIFT REPORT\n");
        out.push_str("═══════════════════════════════════════════════════════════════\n\n");

        out.push_str(&format!("Status: {}\n\n", self.status));

        out.push_str("Files:\n");
        out.push_str(&format!(
            "  A: {} ({}) [{}]\n",
            self.file_a.function, self.file_a.language, self.file_a.hash
        ));
        out.push_str(&format!(
            "  B: {} ({}) [{}]\n\n",
            self.file_b.function, self.file_b.language, self.file_b.hash
        ));

        out.push_str(&format!(
            "Rules: {} in A, {} in B, {} matching\n\n",
            self.summary.rules_a, self.summary.rules_b, self.summary.matching_rules
        ));

        if !self.differences.is_empty() {
            out.push_str("Differences:\n");
            for diff in &self.differences {
                let severity = match diff.severity {
                    DiffSeverity::Error => "ERROR",
                    DiffSeverity::Warning => "WARN",
                    DiffSeverity::Info => "INFO",
                };
                out.push_str(&format!("  [{}] {}\n", severity, diff.description));
                if let Some(a) = &diff.value_a {
                    out.push_str(&format!("    A: {}\n", a));
                }
                if let Some(b) = &diff.value_b {
                    out.push_str(&format!("    B: {}\n", b));
                }
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
    fn test_compare_identical() {
        let code = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        false => 0,
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let report = compare(&ast, &ast);

        assert_eq!(report.status, DriftStatus::Synced);
        assert_eq!(report.differences.len(), 0);
    }

    #[test]
    fn test_compare_different_output() {
        let code_a = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        false => 0,
    }
}
"#;
        let code_b = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        false => 99,
    }
}
"#;
        let ast_a = parse_rust(code_a).unwrap();
        let ast_b = parse_rust(code_b).unwrap();
        let report = compare(&ast_a, &ast_b);

        assert_eq!(report.status, DriftStatus::MajorDrift);
        assert!(report
            .differences
            .iter()
            .any(|d| d.kind == DifferenceKind::Output));
    }
}
