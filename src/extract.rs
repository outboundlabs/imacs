//! Spec extraction — extract specs from existing code
//!
//! Analyzes code to reverse-engineer decision tables.
//! Useful for documenting/specifying existing systems.

use crate::ast::*;
use crate::spec::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Extract spec from code AST
pub fn extract(code: &CodeAst) -> ExtractedSpec {
    Extractor::new().extract(code)
}

/// Spec extractor
pub struct Extractor {
    #[allow(dead_code)]
    config: ExtractorConfig,
}

/// Extractor configuration
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    /// Minimum confidence to include a rule
    pub min_confidence: f32,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
        }
    }
}

/// Result of extraction
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedSpec {
    /// The extracted spec
    pub spec: Spec,
    /// Confidence in extraction (0.0-1.0)
    pub confidence: Confidence,
    /// Questions for human review
    pub questions: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Confidence levels
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Confidence {
    /// Overall confidence
    pub overall: f32,
    /// Per-rule confidence
    pub rules: Vec<RuleConfidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuleConfidence {
    pub rule_id: String,
    pub confidence: f32,
    pub reason: String,
}

impl Extractor {
    pub fn new() -> Self {
        Self {
            config: ExtractorConfig::default(),
        }
    }

    pub fn with_config(config: ExtractorConfig) -> Self {
        Self { config }
    }

    /// Extract spec from code
    pub fn extract(&self, code: &CodeAst) -> ExtractedSpec {
        if code.functions.is_empty() {
            return ExtractedSpec {
                spec: Spec {
                    id: "unknown".into(),
                    name: None,
                    description: None,
                    inputs: vec![],
                    outputs: vec![],
                    rules: vec![],
                    default: None,
                    meta: SpecMeta::default(),
                    scoping: None,
                },
                confidence: Confidence {
                    overall: 0.0,
                    rules: vec![],
                },
                questions: vec!["No functions found in code".into()],
                warnings: vec![],
            };
        }

        // Use first function
        let func = &code.functions[0];
        self.extract_from_function(func)
    }

    fn extract_from_function(&self, func: &Function) -> ExtractedSpec {
        let mut rules = Vec::new();
        let mut questions = Vec::new();
        let mut warnings = Vec::new();
        let mut rule_confidences = Vec::new();

        // Extract inputs from parameters
        let inputs: Vec<Variable> = func
            .params
            .iter()
            .map(|p| Variable {
                name: p.name.clone(),
                typ: self.infer_type(&p.typ),
                description: None,
                values: None,
            })
            .collect();

        // Extract rules from body
        let mut rule_counter = 0;
        self.extract_rules(
            &func.body,
            &inputs,
            &mut vec![],
            &mut rules,
            &mut rule_counter,
            &mut rule_confidences,
            &mut warnings,
        );

        // Infer output type
        let output_type = self.infer_output_type(&rules);
        let outputs = vec![Variable {
            name: "result".into(),
            typ: output_type,
            description: None,
            values: None,
        }];

        // Generate questions
        if inputs.iter().any(|i| i.typ == VarType::String) {
            questions.push("Some inputs are strings - should they be enums?".into());
        }

        if rules.len() > 10 {
            questions.push("Many rules detected - is this the right granularity?".into());
        }

        // Check for missing default
        if !self.has_catch_all(&func.body) {
            warnings.push("No default/catch-all case found".into());
            questions.push("What should happen for uncovered cases?".into());
        }

        let overall_confidence = if rule_confidences.is_empty() {
            0.0
        } else {
            rule_confidences.iter().map(|r| r.confidence).sum::<f32>()
                / rule_confidences.len() as f32
        };

        ExtractedSpec {
            spec: Spec {
                id: func.name.clone(),
                name: Some(humanize(&func.name)),
                description: None,
                inputs,
                outputs,
                rules,
                default: None,
                meta: SpecMeta::default(),
                scoping: None,
            },
            confidence: Confidence {
                overall: overall_confidence,
                rules: rule_confidences,
            },
            questions,
            warnings,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_rules(
        &self,
        node: &AstNode,
        inputs: &[Variable],
        current_conditions: &mut Vec<Condition>,
        rules: &mut Vec<Rule>,
        counter: &mut usize,
        confidences: &mut Vec<RuleConfidence>,
        _warnings: &mut Vec<String>,
    ) {
        match node {
            AstNode::Match { arms, .. } => {
                for arm in arms {
                    let mut arm_conditions = current_conditions.clone();
                    let conf =
                        self.extract_pattern_conditions(&arm.pattern, inputs, &mut arm_conditions);

                    if arm.pattern.is_catch_all() {
                        // Default case
                        if let Some(output) = self.extract_output(&arm.body) {
                            *counter += 1;
                            let rule_id = format!("R{}", counter);
                            rules.push(Rule {
                                id: rule_id.clone(),
                                when: None,
                                conditions: if arm_conditions.is_empty() {
                                    None
                                } else {
                                    Some(arm_conditions)
                                },
                                then: Output::Single(output),
                                priority: *counter as i32,
                                description: Some("Default case".into()),
                            });
                            confidences.push(RuleConfidence {
                                rule_id,
                                confidence: conf * 0.8, // Slightly lower for catch-all
                                reason: "Catch-all pattern".into(),
                            });
                        }
                    } else if let Some(output) = self.extract_output(&arm.body) {
                        *counter += 1;
                        let rule_id = format!("R{}", counter);
                        rules.push(Rule {
                            id: rule_id.clone(),
                            when: None,
                            conditions: if arm_conditions.is_empty() {
                                None
                            } else {
                                Some(arm_conditions)
                            },
                            then: Output::Single(output),
                            priority: *counter as i32,
                            description: None,
                        });
                        confidences.push(RuleConfidence {
                            rule_id,
                            confidence: conf,
                            reason: "Direct pattern match".into(),
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
                let mut then_conditions = current_conditions.clone();
                let conf = self.extract_expr_conditions(condition, &mut then_conditions, false);

                if let Some(output) = self.extract_output(then_branch) {
                    *counter += 1;
                    let rule_id = format!("R{}", counter);
                    rules.push(Rule {
                        id: rule_id.clone(),
                        when: None,
                        conditions: if then_conditions.is_empty() {
                            None
                        } else {
                            Some(then_conditions)
                        },
                        then: Output::Single(output),
                        priority: *counter as i32,
                        description: None,
                    });
                    confidences.push(RuleConfidence {
                        rule_id,
                        confidence: conf,
                        reason: "If condition".into(),
                    });
                } else {
                    // Recurse into then branch
                    let mut then_conds = current_conditions.clone();
                    self.extract_expr_conditions(condition, &mut then_conds, false);
                    self.extract_rules(
                        then_branch,
                        inputs,
                        &mut then_conds,
                        rules,
                        counter,
                        confidences,
                        _warnings,
                    );
                }

                // Else branch
                if let Some(else_node) = else_branch {
                    let mut else_conditions = current_conditions.clone();
                    self.extract_expr_conditions(condition, &mut else_conditions, true);
                    self.extract_rules(
                        else_node,
                        inputs,
                        &mut else_conditions,
                        rules,
                        counter,
                        confidences,
                        _warnings,
                    );
                }
            }

            AstNode::Block {
                result: Some(inner),
                ..
            } => {
                self.extract_rules(
                    inner,
                    inputs,
                    current_conditions,
                    rules,
                    counter,
                    confidences,
                    _warnings,
                );
            }

            AstNode::Return {
                value: Some(inner), ..
            } => {
                if let Some(output) = self.extract_output(inner) {
                    if !current_conditions.is_empty() {
                        *counter += 1;
                        let rule_id = format!("R{}", counter);
                        rules.push(Rule {
                            id: rule_id.clone(),
                            when: None,
                            conditions: Some(current_conditions.clone()),
                            then: Output::Single(output),
                            priority: *counter as i32,
                            description: None,
                        });
                        confidences.push(RuleConfidence {
                            rule_id,
                            confidence: 0.7,
                            reason: "Early return".into(),
                        });
                    }
                }
            }

            _ => {}
        }
    }

    fn extract_pattern_conditions(
        &self,
        pattern: &Pattern,
        inputs: &[Variable],
        conditions: &mut Vec<Condition>,
    ) -> f32 {
        match pattern {
            Pattern::Tuple(elements) => {
                let mut conf = 1.0;
                for (i, elem) in elements.iter().enumerate() {
                    if let Some(input) = inputs.get(i) {
                        conf *= self.extract_single_pattern(&input.name, elem, conditions);
                    }
                }
                conf
            }

            Pattern::Literal(lit) => {
                if let Some(input) = inputs.first() {
                    conditions.push(Condition {
                        var: input.name.clone(),
                        op: ConditionOp::Eq,
                        value: self.literal_to_value(lit),
                    });
                }
                1.0
            }

            Pattern::Wildcard | Pattern::Binding(_) => 0.9,

            _ => 0.5,
        }
    }

    fn extract_single_pattern(
        &self,
        var_name: &str,
        pattern: &Pattern,
        conditions: &mut Vec<Condition>,
    ) -> f32 {
        match pattern {
            Pattern::Literal(lit) => {
                conditions.push(Condition {
                    var: var_name.to_string(),
                    op: ConditionOp::Eq,
                    value: self.literal_to_value(lit),
                });
                1.0
            }
            Pattern::Wildcard | Pattern::Binding(_) => 1.0,
            _ => 0.5,
        }
    }

    fn extract_expr_conditions(
        &self,
        expr: &AstNode,
        conditions: &mut Vec<Condition>,
        negated: bool,
    ) -> f32 {
        match expr {
            AstNode::Binary {
                op: BinaryOp::And,
                left,
                right,
                ..
            } if !negated => {
                let c1 = self.extract_expr_conditions(left, conditions, false);
                let c2 = self.extract_expr_conditions(right, conditions, false);
                (c1 + c2) / 2.0
            }

            AstNode::Binary {
                op, left, right, ..
            } => {
                if let AstNode::Var { name, .. } = left.as_ref() {
                    if let Some(value) = self.node_to_value(right) {
                        let cond_op = if negated {
                            self.negate_op(self.binary_to_op(*op))
                        } else {
                            self.binary_to_op(*op)
                        };
                        conditions.push(Condition {
                            var: name.clone(),
                            op: cond_op,
                            value,
                        });
                        return 1.0;
                    }
                }
                0.5
            }

            AstNode::Unary {
                op: UnaryOp::Not,
                operand,
                ..
            } => self.extract_expr_conditions(operand, conditions, !negated),

            AstNode::Var { name, .. } => {
                conditions.push(Condition {
                    var: name.clone(),
                    op: ConditionOp::Eq,
                    value: ConditionValue::Bool(!negated),
                });
                1.0
            }

            _ => 0.5,
        }
    }

    fn extract_output(&self, node: &AstNode) -> Option<ConditionValue> {
        match node {
            AstNode::Literal { value, .. } => Some(self.literal_to_value(value)),
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

    fn has_catch_all(&self, node: &AstNode) -> bool {
        match node {
            AstNode::Match { arms, .. } => arms.iter().any(|a| a.pattern.is_catch_all()),
            AstNode::If { else_branch, .. } => else_branch
                .as_ref()
                .map(|e| self.is_terminal(e))
                .unwrap_or(false),
            AstNode::Block { result, .. } => result
                .as_ref()
                .map(|r| self.has_catch_all(r))
                .unwrap_or(false),
            _ => false,
        }
    }

    fn is_terminal(&self, node: &AstNode) -> bool {
        match node {
            AstNode::Literal { .. } => true,
            AstNode::Return { .. } => true,
            AstNode::Block { result, .. } => result
                .as_ref()
                .map(|r| self.is_terminal(r))
                .unwrap_or(false),
            _ => false,
        }
    }

    fn infer_type(&self, type_str: &str) -> VarType {
        match type_str.to_lowercase().as_str() {
            "bool" => VarType::Bool,
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" | "int" | "number" => VarType::Int,
            "f32" | "f64" | "float" => VarType::Float,
            "string" | "&str" | "str" => VarType::String,
            _ => VarType::String,
        }
    }

    fn infer_output_type(&self, rules: &[Rule]) -> VarType {
        for rule in rules {
            match &rule.then {
                Output::Single(ConditionValue::Bool(_)) => return VarType::Bool,
                Output::Single(ConditionValue::Int(_)) => return VarType::Int,
                Output::Single(ConditionValue::Float(_)) => return VarType::Float,
                Output::Single(ConditionValue::String(_)) => return VarType::String,
                _ => {}
            }
        }
        VarType::String
    }

    fn literal_to_value(&self, lit: &LiteralValue) -> ConditionValue {
        match lit {
            LiteralValue::Bool(b) => ConditionValue::Bool(*b),
            LiteralValue::Int(i) => ConditionValue::Int(*i),
            LiteralValue::Float(f) => ConditionValue::Float(*f),
            LiteralValue::String(s) => ConditionValue::String(s.clone()),
            LiteralValue::Char(c) => ConditionValue::String(c.to_string()),
            LiteralValue::Unit => ConditionValue::Null,
        }
    }

    fn node_to_value(&self, node: &AstNode) -> Option<ConditionValue> {
        match node {
            AstNode::Literal { value, .. } => Some(self.literal_to_value(value)),
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

impl Default for Extractor {
    fn default() -> Self {
        Self::new()
    }
}

fn humanize(s: &str) -> String {
    s.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl ExtractedSpec {
    /// Format as YAML
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();

        out.push_str("# EXTRACTED SPEC\n");
        out.push_str(&format!(
            "# Confidence: {:.0}%\n",
            self.confidence.overall * 100.0
        ));

        if !self.warnings.is_empty() {
            out.push_str("# Warnings:\n");
            for w in &self.warnings {
                out.push_str(&format!("#   - {}\n", w));
            }
        }

        if !self.questions.is_empty() {
            out.push_str("# Review needed:\n");
            for q in &self.questions {
                out.push_str(&format!("#   ? {}\n", q));
            }
        }

        out.push('\n');
        out.push_str(&self.spec.to_yaml().unwrap_or_default());

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_rust;

    #[test]
    fn test_extract_simple_match() {
        let code = r#"
fn check(x: bool) -> i32 {
    match x {
        true => 1,
        false => 0,
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let extracted = extract(&ast);

        assert_eq!(extracted.spec.id, "check");
        assert_eq!(extracted.spec.rules.len(), 2);
        assert!(extracted.confidence.overall > 0.5);
    }

    #[test]
    fn test_extract_tuple_match() {
        let code = r#"
fn check(a: bool, b: bool) -> i32 {
    match (a, b) {
        (true, true) => 1,
        (true, false) => 2,
        (false, _) => 3,
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let extracted = extract(&ast);

        assert_eq!(extracted.spec.inputs.len(), 2);
        assert_eq!(extracted.spec.rules.len(), 3);
    }
}
