//! Spec extraction — extract specs from existing code
//!
//! Analyzes code to reverse-engineer decision tables.
//! Useful for documenting/specifying existing systems.

use crate::ast::*;
use crate::spec::{
    Condition, ConditionOp, ConditionValue, Output, Rule, Spec, SpecMeta, VarType, Variable,
    WhenClause,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// AST to CEL Conversion
// ============================================================================

/// Error that can occur when converting AST to CEL expression
#[derive(Debug, Clone)]
pub enum ConversionError {
    /// Node type cannot be represented in CEL
    Unsupported(String),
    /// Unknown AST node encountered
    UnknownNode { kind: String, span: Span },
    /// Control flow node (if/match/loop) - not a simple expression
    ControlFlow(String),
    /// Operator not supported in CEL
    UnsupportedOp(String),
    /// Syntax error in source code
    SyntaxError { message: String, span: Span },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            ConversionError::UnknownNode { kind, span } => {
                write!(f, "Unknown node '{}' at line {}", kind, span.start_line)
            }
            ConversionError::ControlFlow(msg) => write!(f, "Control flow: {}", msg),
            ConversionError::UnsupportedOp(op) => write!(f, "Unsupported operator: {}", op),
            ConversionError::SyntaxError { message, span } => {
                write!(f, "Syntax error at line {}: {}", span.start_line, message)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

/// Convert an AST node to a CEL expression string.
///
/// This is the core function that enables extraction of conditions and outputs
/// from arbitrary AST nodes. It handles:
/// - Literals: `true`, `42`, `"hello"`, `3.14`
/// - Variables: `x`, `user_id`
/// - Field access: `user.name`, `order.items.count`
/// - Function calls: `is_valid(x)`, `len(items)`
/// - Binary operations: `x > 10 && y < 20`
/// - Unary operations: `!enabled`, `-value`
/// - Index access: `items[0]`, `map["key"]`
/// - Arrays/Tuples: `[1, 2, 3]`
///
/// # Examples
/// ```ignore
/// // Literal
/// ast_to_cel(&AstNode::Literal { value: LiteralValue::Int(42), .. }) // => "42"
///
/// // Field access
/// ast_to_cel(&AstNode::Field { object: user_var, field: "name", .. }) // => "user.name"
///
/// // Binary comparison
/// ast_to_cel(&AstNode::Binary { op: Gt, left: x, right: 10, .. }) // => "(x > 10)"
/// ```
pub fn ast_to_cel(node: &AstNode) -> Result<String, ConversionError> {
    match node {
        // === Literals ===
        AstNode::Literal { value, .. } => Ok(literal_to_cel(value)),

        // === Variables ===
        AstNode::Var { name, .. } => Ok(name.clone()),

        // === Field access: object.field ===
        AstNode::Field { object, field, .. } => {
            let obj_cel = ast_to_cel(object)?;
            Ok(format!("{}.{}", obj_cel, field))
        }

        // === Function calls: func(args...) ===
        AstNode::Call { function, args, .. } => {
            let args_cel: Result<Vec<_>, _> = args.iter().map(ast_to_cel).collect();
            Ok(format!("{}({})", function, args_cel?.join(", ")))
        }

        // === Index access: object[index] ===
        AstNode::Index { object, index, .. } => {
            let obj_cel = ast_to_cel(object)?;
            let idx_cel = ast_to_cel(index)?;
            Ok(format!("{}[{}]", obj_cel, idx_cel))
        }

        // === Binary operations ===
        AstNode::Binary {
            op, left, right, ..
        } => {
            let op_str = binary_op_to_cel(*op)?;
            let left_cel = ast_to_cel(left)?;
            let right_cel = ast_to_cel(right)?;
            Ok(format!("({} {} {})", left_cel, op_str, right_cel))
        }

        // === Unary operations ===
        AstNode::Unary { op, operand, .. } => {
            let operand_cel = ast_to_cel(operand)?;
            match op {
                UnaryOp::Not => Ok(format!("!({})", operand_cel)),
                UnaryOp::Neg => Ok(format!("-({})", operand_cel)),
                UnaryOp::BitNot => Err(ConversionError::UnsupportedOp("bitwise NOT".to_string())),
            }
        }

        // === Tuple/Array: [elem1, elem2, ...] ===
        AstNode::Tuple { elements, .. } | AstNode::Array { elements, .. } => {
            let elems_cel: Result<Vec<_>, _> = elements.iter().map(ast_to_cel).collect();
            Ok(format!("[{}]", elems_cel?.join(", ")))
        }

        // === Control flow - these are not simple expressions ===
        AstNode::If { .. } => Err(ConversionError::ControlFlow(
            "if expression - extract condition separately".to_string(),
        )),
        AstNode::Match { .. } => Err(ConversionError::ControlFlow(
            "match expression - extract arms separately".to_string(),
        )),
        AstNode::Block { result, .. } => {
            // For blocks with a result, try to convert the result expression
            if let Some(inner) = result {
                ast_to_cel(inner)
            } else {
                Err(ConversionError::ControlFlow(
                    "block without result expression".to_string(),
                ))
            }
        }
        AstNode::Return { value, .. } => {
            if let Some(inner) = value {
                ast_to_cel(inner)
            } else {
                Err(ConversionError::ControlFlow("empty return".to_string()))
            }
        }

        // === Let bindings - need data flow tracking ===
        AstNode::Let { name, .. } => Err(ConversionError::Unsupported(format!(
            "let binding '{}' - requires data flow tracking",
            name
        ))),

        // === Loops - not expressible as CEL ===
        AstNode::For { .. } => Err(ConversionError::ControlFlow("for loop".to_string())),
        AstNode::ForEach { .. } => Err(ConversionError::ControlFlow("for-each loop".to_string())),
        AstNode::While { .. } => Err(ConversionError::ControlFlow("while loop".to_string())),

        // === Other ===
        AstNode::Try { .. } => Err(ConversionError::ControlFlow("try/catch".to_string())),
        AstNode::Assign { .. } => Err(ConversionError::Unsupported("assignment".to_string())),
        AstNode::Await { expr, .. } => {
            // For await, try to extract the inner expression
            ast_to_cel(expr)
        }
        AstNode::Closure { .. } => Err(ConversionError::Unsupported("closure".to_string())),

        // === New Phase 6 node types ===
        AstNode::MacroCall { name, .. } => {
            Err(ConversionError::Unsupported(format!("macro call: {}!", name)))
        }
        AstNode::Ref { expr, .. } => {
            // References can be dereferenced for CEL - just use the inner expression
            ast_to_cel(expr)
        }
        AstNode::Cast { expr, target_type, .. } => {
            // Type casts are runtime operations - convert the inner expression
            // and note the cast in the output
            let inner = ast_to_cel(expr)?;
            Ok(format!("{}/*as {}*/", inner, target_type))
        }
        AstNode::SyntaxError { message, span, .. } => Err(ConversionError::SyntaxError {
            message: message.clone(),
            span: *span,
        }),

        // === Unknown nodes ===
        AstNode::Unknown { kind, span } => Err(ConversionError::UnknownNode {
            kind: kind.clone(),
            span: *span,
        }),
    }
}

/// Convert a literal value to CEL string representation
fn literal_to_cel(lit: &LiteralValue) -> String {
    match lit {
        LiteralValue::Bool(b) => b.to_string(),
        LiteralValue::Int(i) => i.to_string(),
        LiteralValue::Float(f) => {
            // Ensure float has decimal point
            let s = f.to_string();
            if s.contains('.') {
                s
            } else {
                format!("{}.0", s)
            }
        }
        LiteralValue::String(s) => format!("\"{}\"", escape_cel_string(s)),
        LiteralValue::Char(c) => format!("\"{}\"", c), // CEL uses strings for chars
        LiteralValue::Unit => "null".to_string(),
    }
}

/// Escape special characters in CEL strings
fn escape_cel_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Convert binary operator to CEL string
fn binary_op_to_cel(op: BinaryOp) -> Result<&'static str, ConversionError> {
    match op {
        // Arithmetic
        BinaryOp::Add => Ok("+"),
        BinaryOp::Sub => Ok("-"),
        BinaryOp::Mul => Ok("*"),
        BinaryOp::Div => Ok("/"),
        BinaryOp::Mod => Ok("%"),
        // Comparison
        BinaryOp::Eq => Ok("=="),
        BinaryOp::Ne => Ok("!="),
        BinaryOp::Lt => Ok("<"),
        BinaryOp::Le => Ok("<="),
        BinaryOp::Gt => Ok(">"),
        BinaryOp::Ge => Ok(">="),
        // Logical
        BinaryOp::And => Ok("&&"),
        BinaryOp::Or => Ok("||"),
        // Bitwise - not supported in standard CEL
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            Err(ConversionError::UnsupportedOp(format!("{:?}", op)))
        }
    }
}

/// Convert a Pattern to a CEL condition string
///
/// Returns an optional condition because wildcards and bindings don't generate conditions.
pub fn pattern_to_cel(pattern: &Pattern, input_var: &str) -> Option<String> {
    match pattern {
        Pattern::Wildcard | Pattern::Binding(_) | Pattern::Rest => None,

        Pattern::Literal(lit) => {
            let lit_cel = literal_to_cel(lit);
            Some(format!("{} == {}", input_var, lit_cel))
        }

        Pattern::Constructor { name, fields } => {
            // For enum constructors like Status::Active
            // Generate: status == "Active" or status == "Status.Active"
            if fields.is_empty() {
                Some(format!("{} == \"{}\"", input_var, name))
            } else {
                // Constructor with fields - more complex
                // For now, just match the variant name
                Some(format!("{} == \"{}\"", input_var, name))
            }
        }

        Pattern::Or(patterns) => {
            // Generate: input in ["val1", "val2", ...]
            let values: Vec<String> = patterns
                .iter()
                .filter_map(|p| match p {
                    Pattern::Literal(lit) => Some(literal_to_cel(lit)),
                    Pattern::Constructor { name, .. } => Some(format!("\"{}\"", name)),
                    _ => None,
                })
                .collect();

            if values.is_empty() {
                None
            } else if values.len() == 1 {
                Some(format!("{} == {}", input_var, values[0]))
            } else {
                Some(format!("{} in [{}]", input_var, values.join(", ")))
            }
        }

        Pattern::Tuple(elements) => {
            // For tuples, we'd need multiple input variables
            // This is handled at a higher level with indexed access
            // e.g., (a, b) matching (true, false) → a == true && b == false
            // For now, return None and let the caller handle it
            if elements.is_empty() {
                None
            } else {
                // Try to convert first element as a hint
                pattern_to_cel(&elements[0], &format!("{}[0]", input_var))
            }
        }
    }
}

/// Convert an AST node to a CEL expression string with variable substitution.
///
/// This version uses the ExtractionContext to resolve variable definitions,
/// enabling data flow tracking. For example:
/// ```ignore
/// let x = a + b;
/// if x > 10 { ... }
/// ```
/// Will produce CEL `(a + b) > 10` instead of `x > 10`.
fn ast_to_cel_with_ctx(node: &AstNode, ctx: &ExtractionContext) -> Result<String, ConversionError> {
    match node {
        // === Variables - check for definition and inline ===
        AstNode::Var { name, .. } => {
            if let Some(definition) = ctx.resolve_var(name) {
                // Inline the variable definition
                // Wrap in parens to preserve precedence
                let def_cel = ast_to_cel_with_ctx(&definition, ctx)?;
                Ok(format!("({})", def_cel))
            } else {
                // No definition found, use variable name as-is
                Ok(name.clone())
            }
        }

        // === Literals ===
        AstNode::Literal { value, .. } => Ok(literal_to_cel(value)),

        // === Field access: object.field ===
        AstNode::Field { object, field, .. } => {
            let obj_cel = ast_to_cel_with_ctx(object, ctx)?;
            Ok(format!("{}.{}", obj_cel, field))
        }

        // === Function calls: func(args...) ===
        AstNode::Call { function, args, .. } => {
            let args_cel: Result<Vec<_>, _> = args.iter().map(|a| ast_to_cel_with_ctx(a, ctx)).collect();
            Ok(format!("{}({})", function, args_cel?.join(", ")))
        }

        // === Index access: object[index] ===
        AstNode::Index { object, index, .. } => {
            let obj_cel = ast_to_cel_with_ctx(object, ctx)?;
            let idx_cel = ast_to_cel_with_ctx(index, ctx)?;
            Ok(format!("{}[{}]", obj_cel, idx_cel))
        }

        // === Binary operations ===
        AstNode::Binary {
            op, left, right, ..
        } => {
            let op_str = binary_op_to_cel(*op)?;
            let left_cel = ast_to_cel_with_ctx(left, ctx)?;
            let right_cel = ast_to_cel_with_ctx(right, ctx)?;
            Ok(format!("({} {} {})", left_cel, op_str, right_cel))
        }

        // === Unary operations ===
        AstNode::Unary { op, operand, .. } => {
            let operand_cel = ast_to_cel_with_ctx(operand, ctx)?;
            match op {
                UnaryOp::Not => Ok(format!("!({})", operand_cel)),
                UnaryOp::Neg => Ok(format!("-({})", operand_cel)),
                UnaryOp::BitNot => Err(ConversionError::UnsupportedOp("bitwise NOT".to_string())),
            }
        }

        // === Tuple/Array: [elem1, elem2, ...] ===
        AstNode::Tuple { elements, .. } | AstNode::Array { elements, .. } => {
            let elems_cel: Result<Vec<_>, _> = elements.iter().map(|e| ast_to_cel_with_ctx(e, ctx)).collect();
            Ok(format!("[{}]", elems_cel?.join(", ")))
        }

        // === Reference expression - dereference for CEL ===
        AstNode::Ref { expr, .. } => ast_to_cel_with_ctx(expr, ctx),

        // === Type cast - convert inner with context ===
        AstNode::Cast { expr, target_type, .. } => {
            let inner = ast_to_cel_with_ctx(expr, ctx)?;
            Ok(format!("{}/*as {}*/", inner, target_type))
        }

        // === Control flow and other nodes - delegate to base function ===
        _ => ast_to_cel(node),
    }
}

// ============================================================================
// End AST to CEL Conversion
// ============================================================================

/// Extract spec from code AST with detailed report
pub fn extract_with_report(code: &CodeAst, source: &str) -> ExtractionReport {
    Extractor::new().extract_with_report(code, source)
}

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

/// Report from extraction with diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractionReport {
    /// The extracted spec (same as ExtractedSpec)
    pub extracted: ExtractedSpec,
    /// Nodes that were skipped during extraction
    pub skipped_nodes: Vec<SkippedNode>,
    /// Total decision points found in the code
    pub total_decision_points: usize,
    /// Decision points successfully extracted
    pub extracted_decision_points: usize,
    /// Coverage percentage (0.0 - 100.0)
    pub coverage_percent: f32,
}

/// Information about a node that was skipped during extraction
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkippedNode {
    /// The AST node kind (e.g., "Call", "Field", "Unknown")
    pub kind: String,
    /// Why this node was skipped
    pub reason: String,
    /// The source text (if available)
    pub source_text: String,
    /// Source location
    pub span: Span,
}

/// Context for tracking skipped nodes and variable definitions during extraction
struct ExtractionContext {
    source: String,
    skipped: RefCell<Vec<SkippedNode>>,
    total_decision_points: RefCell<usize>,
    extracted_decision_points: RefCell<usize>,
    /// Variable definitions for data flow tracking
    /// Maps variable name to its definition (AST node)
    /// e.g., `let x = a + b` → var_definitions["x"] = Binary(Add, a, b)
    var_definitions: RefCell<HashMap<String, AstNode>>,
}

impl ExtractionContext {
    fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            skipped: RefCell::new(Vec::new()),
            total_decision_points: RefCell::new(0),
            extracted_decision_points: RefCell::new(0),
            var_definitions: RefCell::new(HashMap::new()),
        }
    }

    /// Record a variable definition (from a let binding)
    fn define_var(&self, name: &str, value: AstNode) {
        self.var_definitions
            .borrow_mut()
            .insert(name.to_string(), value);
    }

    /// Resolve a variable to its definition, if tracked
    fn resolve_var(&self, name: &str) -> Option<AstNode> {
        self.var_definitions.borrow().get(name).cloned()
    }

    fn record_skipped(&self, kind: &str, reason: &str, span: Span) {
        let source_text = self.extract_source_text(span);
        self.skipped.borrow_mut().push(SkippedNode {
            kind: kind.to_string(),
            reason: reason.to_string(),
            source_text,
            span,
        });
    }

    fn record_decision_point(&self, extracted: bool) {
        *self.total_decision_points.borrow_mut() += 1;
        if extracted {
            *self.extracted_decision_points.borrow_mut() += 1;
        }
    }

    fn extract_source_text(&self, span: Span) -> String {
        let lines: Vec<&str> = self.source.lines().collect();
        if span.start_line > 0 && span.start_line <= lines.len() {
            let line = lines[span.start_line - 1];
            // Extract the relevant portion, limit to 100 chars
            let start = span.start_col.saturating_sub(1).min(line.len());
            let end = (start + 100).min(line.len());
            line[start..end].to_string()
        } else {
            String::new()
        }
    }

    fn into_results(self) -> (Vec<SkippedNode>, usize, usize) {
        let skipped = self.skipped.into_inner();
        let total = *self.total_decision_points.borrow();
        let extracted = *self.extracted_decision_points.borrow();
        (skipped, total, extracted)
    }
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

    /// Extract spec from code with detailed extraction report
    pub fn extract_with_report(&self, code: &CodeAst, source: &str) -> ExtractionReport {
        let ctx = ExtractionContext::new(source);
        let extracted = self.extract_with_context(code, &ctx);
        let (skipped_nodes, total, extracted_count) = ctx.into_results();

        let coverage_percent = if total > 0 {
            (extracted_count as f32 / total as f32) * 100.0
        } else {
            100.0 // No decision points = fully covered
        };

        ExtractionReport {
            extracted,
            skipped_nodes,
            total_decision_points: total,
            extracted_decision_points: extracted_count,
            coverage_percent,
        }
    }

    /// Extract spec from code (internal, with context)
    fn extract_with_context(&self, code: &CodeAst, ctx: &ExtractionContext) -> ExtractedSpec {
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
        self.extract_from_function_with_context(func, ctx)
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
        // Use context-aware extraction to support ForEach, Try, and variable tracking
        let ctx = ExtractionContext::new("");
        self.extract_from_function_with_context(func, &ctx)
    }

    fn extract_from_function_with_context(
        &self,
        func: &Function,
        ctx: &ExtractionContext,
    ) -> ExtractedSpec {
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

        // Extract rules from body (with context tracking)
        let mut rule_counter = 0;
        self.extract_rules_with_context(
            &func.body,
            &inputs,
            &mut vec![],
            &mut rules,
            &mut rule_counter,
            &mut rule_confidences,
            &mut warnings,
            ctx,
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
    fn extract_rules_with_context(
        &self,
        node: &AstNode,
        inputs: &[Variable],
        current_conditions: &mut Vec<Condition>,
        rules: &mut Vec<Rule>,
        counter: &mut usize,
        confidences: &mut Vec<RuleConfidence>,
        _warnings: &mut Vec<String>,
        ctx: &ExtractionContext,
    ) {
        match node {
            AstNode::Match { arms, span, .. } => {
                for arm in arms {
                    ctx.record_decision_point(true); // Match arm is a decision point
                    let mut arm_conditions = current_conditions.clone();
                    let conf = self.extract_pattern_conditions_with_context(
                        &arm.pattern,
                        inputs,
                        &mut arm_conditions,
                        ctx,
                    );

                    if arm.pattern.is_catch_all() {
                        // Default case
                        if let Some(output) = self.extract_output_with_context(&arm.body, ctx) {
                            *counter += 1;
                            let rule_id = format!("R{}", counter);
                            let when_clause = self.conditions_to_when(&arm_conditions);
                            rules.push(Rule {
                                id: rule_id.clone(),
                                when: when_clause,
                                conditions: None,
                                then: Output::Single(output),
                                priority: *counter as i32,
                                description: Some("Default case".into()),
                            });
                            confidences.push(RuleConfidence {
                                rule_id,
                                confidence: conf * 0.8,
                                reason: "Catch-all pattern".into(),
                            });
                        }
                    } else if let Some(output) = self.extract_output_with_context(&arm.body, ctx) {
                        *counter += 1;
                        let rule_id = format!("R{}", counter);
                        let when_clause = self.conditions_to_when(&arm_conditions);
                        rules.push(Rule {
                            id: rule_id.clone(),
                            when: when_clause,
                            conditions: None,
                            then: Output::Single(output),
                            priority: *counter as i32,
                            description: None,
                        });
                        confidences.push(RuleConfidence {
                            rule_id,
                            confidence: conf,
                            reason: "Direct pattern match".into(),
                        });
                    } else {
                        ctx.record_skipped(
                            "MatchArm",
                            "Could not extract output from match arm body",
                            *span,
                        );
                    }
                }
            }

            AstNode::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                ctx.record_decision_point(true); // If is a decision point
                                                 // Then branch
                let mut then_conditions = current_conditions.clone();
                let conf = self.extract_expr_conditions_with_context(
                    condition,
                    &mut then_conditions,
                    false,
                    ctx,
                );

                if let Some(output) = self.extract_output_with_context(then_branch, ctx) {
                    *counter += 1;
                    let rule_id = format!("R{}", counter);
                    // Prefer `when` (CEL string) over structured `conditions`
                    let when_clause = self.conditions_to_when(&then_conditions);
                    rules.push(Rule {
                        id: rule_id.clone(),
                        when: when_clause,
                        conditions: None, // Prefer CEL `when` clause
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
                    self.extract_expr_conditions_with_context(
                        condition,
                        &mut then_conds,
                        false,
                        ctx,
                    );
                    self.extract_rules_with_context(
                        then_branch,
                        inputs,
                        &mut then_conds,
                        rules,
                        counter,
                        confidences,
                        _warnings,
                        ctx,
                    );
                }

                // Else branch
                if let Some(else_node) = else_branch {
                    let mut else_conditions = current_conditions.clone();
                    self.extract_expr_conditions_with_context(
                        condition,
                        &mut else_conditions,
                        true,
                        ctx,
                    );
                    self.extract_rules_with_context(
                        else_node,
                        inputs,
                        &mut else_conditions,
                        rules,
                        counter,
                        confidences,
                        _warnings,
                        ctx,
                    );
                } else {
                    // No else branch - this is a potential gap
                    ctx.record_skipped("If", "No else branch - potential uncovered case", *span);
                }
            }

            AstNode::Block {
                statements,
                result: Some(inner),
                ..
            } => {
                // Process statements for let bindings - track variable definitions
                for stmt in statements {
                    if let AstNode::Let { name, value, .. } = stmt {
                        // Track the variable definition for later inlining
                        ctx.define_var(name, *value.clone());
                    }
                }
                self.extract_rules_with_context(
                    inner,
                    inputs,
                    current_conditions,
                    rules,
                    counter,
                    confidences,
                    _warnings,
                    ctx,
                );
            }

            AstNode::Return {
                value: Some(inner),
                span,
            } => {
                if let Some(output) = self.extract_output_with_context(inner, ctx) {
                    if !current_conditions.is_empty() {
                        ctx.record_decision_point(true);
                        *counter += 1;
                        let rule_id = format!("R{}", counter);
                        let when_clause = self.conditions_to_when(current_conditions);
                        rules.push(Rule {
                            id: rule_id.clone(),
                            when: when_clause,
                            conditions: None,
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
                } else {
                    ctx.record_decision_point(false);
                    ctx.record_skipped("Return", "Could not extract output from return", *span);
                }
            }

            AstNode::Call { function, span, .. } => {
                ctx.record_decision_point(false);
                ctx.record_skipped(
                    "Call",
                    &format!("Function call '{}' not extractable", function),
                    *span,
                );
            }

            AstNode::Field { field, span, .. } => {
                ctx.record_decision_point(false);
                ctx.record_skipped(
                    "Field",
                    &format!("Field access '.{}' not extractable", field),
                    *span,
                );
            }

            AstNode::ForEach {
                item,
                collection,
                body,
                span,
                ..
            } => {
                // Try to extract "exists" pattern from for-each loops with early returns
                // Pattern: for item in collection { if cond { return val } } return default
                if let Some((condition_cel, return_value)) =
                    self.find_early_return_pattern(body, item, ctx)
                {
                    ctx.record_decision_point(true);
                    if let Ok(collection_cel) = ast_to_cel_with_ctx(collection, ctx) {
                        *counter += 1;
                        let rule_id = format!("R{}", counter);
                        let exists_cel =
                            format!("{}.exists({}, {})", collection_cel, item, condition_cel);

                        // Combine with current conditions
                        let mut all_conditions = current_conditions.clone();
                        all_conditions.push(Condition {
                            var: "_cel".to_string(),
                            op: ConditionOp::Eq,
                            value: ConditionValue::String(exists_cel),
                        });

                        rules.push(Rule {
                            id: rule_id.clone(),
                            when: self.conditions_to_when(&all_conditions),
                            conditions: None,
                            then: Output::Single(return_value),
                            priority: *counter as i32,
                            description: Some("Exists pattern from for-each loop".into()),
                        });
                        confidences.push(RuleConfidence {
                            rule_id,
                            confidence: 0.7,
                            reason: "For-each exists pattern".into(),
                        });
                    }
                } else {
                    ctx.record_decision_point(false);
                    ctx.record_skipped("ForEach", "Loop body does not match exists pattern", *span);
                }
            }

            AstNode::While { span, .. } => {
                // While loops are harder to extract - skip for now
                ctx.record_decision_point(false);
                ctx.record_skipped("While", "While loop not extractable", *span);
            }

            AstNode::Try {
                try_block,
                catch_block,
                finally_block,
                span,
                ..
            } => {
                ctx.record_decision_point(true);

                // Extract rules from the try block
                self.extract_rules_with_context(
                    try_block,
                    inputs,
                    current_conditions,
                    rules,
                    counter,
                    confidences,
                    _warnings,
                    ctx,
                );

                // Extract rules from the catch block with an _error condition
                if let Some(catch) = catch_block {
                    let mut error_conditions = current_conditions.clone();
                    error_conditions.push(Condition {
                        var: "_error".to_string(),
                        op: ConditionOp::Eq,
                        value: ConditionValue::Bool(true),
                    });

                    self.extract_rules_with_context(
                        catch,
                        inputs,
                        &mut error_conditions,
                        rules,
                        counter,
                        confidences,
                        _warnings,
                        ctx,
                    );
                }

                // Finally block doesn't affect conditions, but we could extract from it
                if let Some(finally) = finally_block {
                    // For now, just record that we saw a finally block
                    ctx.record_skipped(
                        "Finally",
                        "Finally block skipped (always executes, not conditional)",
                        *span,
                    );
                    // Could potentially extract side effects here in the future
                    let _ = finally;
                }
            }

            AstNode::Unknown { kind, span } => {
                ctx.record_decision_point(false);
                ctx.record_skipped("Unknown", &format!("Unknown AST node: {}", kind), *span);
            }

            _ => {
                // Other nodes are structural, not decision points
            }
        }
    }

    fn extract_pattern_conditions_with_context(
        &self,
        pattern: &Pattern,
        inputs: &[Variable],
        conditions: &mut Vec<Condition>,
        ctx: &ExtractionContext,
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

            Pattern::Constructor { name, .. } => {
                // Enum constructors - record as skipped but return partial confidence
                ctx.record_skipped(
                    "Constructor",
                    &format!("Enum constructor pattern '{}' not fully extractable", name),
                    Span::default(),
                );
                0.5
            }

            Pattern::Or(patterns) => {
                // Or patterns - record as skipped
                ctx.record_skipped(
                    "OrPattern",
                    &format!(
                        "Or pattern with {} alternatives not extractable",
                        patterns.len()
                    ),
                    Span::default(),
                );
                0.5
            }

            Pattern::Rest => 0.5,
        }
    }

    /// Try to convert an expression to a CEL condition string
    /// Returns Some(cel_string) if successful, None otherwise
    fn try_expr_to_cel(&self, expr: &AstNode, negated: bool, ctx: &ExtractionContext) -> Option<String> {
        match ast_to_cel_with_ctx(expr, ctx) {
            Ok(cel) => {
                if negated {
                    Some(format!("!({})", cel))
                } else {
                    Some(cel)
                }
            }
            Err(_) => None,
        }
    }

    fn extract_expr_conditions_with_context(
        &self,
        expr: &AstNode,
        conditions: &mut Vec<Condition>,
        negated: bool,
        ctx: &ExtractionContext,
    ) -> f32 {
        match expr {
            AstNode::Binary {
                op: BinaryOp::And,
                left,
                right,
                ..
            } if !negated => {
                let c1 = self.extract_expr_conditions_with_context(left, conditions, false, ctx);
                let c2 = self.extract_expr_conditions_with_context(right, conditions, false, ctx);
                (c1 + c2) / 2.0
            }

            AstNode::Binary {
                op, left, right, ..
            } => {
                // First, try to convert the entire expression with variable substitution
                // This handles cases like `let sum = a + b; if sum > 10`
                if let Some(cel) = self.try_expr_to_cel(expr, negated, ctx) {
                    conditions.push(Condition {
                        var: "_cel".to_string(),
                        op: ConditionOp::Eq,
                        value: ConditionValue::String(cel),
                    });
                    return 0.95;
                }
                // Fallback: simple Var op Literal pattern (for inputs without definitions)
                if let AstNode::Var { name, .. } = left.as_ref() {
                    // Only use structured condition if var has no definition
                    if ctx.resolve_var(name).is_none() {
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
                }
                ctx.record_skipped(
                    "Binary",
                    "Binary expression could not be converted",
                    expr.span(),
                );
                0.5
            }

            AstNode::Unary {
                op: UnaryOp::Not,
                operand,
                ..
            } => self.extract_expr_conditions_with_context(operand, conditions, !negated, ctx),

            AstNode::Var { name, .. } => {
                // Check if this variable has a tracked definition
                if let Some(definition) = ctx.resolve_var(name) {
                    // Inline the variable definition as a CEL expression
                    if let Ok(cel) = ast_to_cel_with_ctx(&definition, ctx) {
                        let final_cel = if negated {
                            format!("!({})", cel)
                        } else {
                            cel
                        };
                        conditions.push(Condition {
                            var: "_cel".to_string(),
                            op: ConditionOp::Eq,
                            value: ConditionValue::String(final_cel),
                        });
                        return 1.0;
                    }
                }
                // No definition or couldn't inline - use as boolean condition
                conditions.push(Condition {
                    var: name.clone(),
                    op: ConditionOp::Eq,
                    value: ConditionValue::Bool(!negated),
                });
                1.0
            }

            // For Call, Field, and other complex nodes, try ast_to_cel
            AstNode::Call { .. } | AstNode::Field { .. } | AstNode::Index { .. } => {
                if let Some(cel) = self.try_expr_to_cel(expr, negated, ctx) {
                    conditions.push(Condition {
                        var: "_cel".to_string(),
                        op: ConditionOp::Eq,
                        value: ConditionValue::String(cel),
                    });
                    return 0.9;
                }
                ctx.record_skipped(
                    &format!("{:?}", std::mem::discriminant(expr)),
                    "Could not convert to CEL expression",
                    expr.span(),
                );
                0.3
            }

            _ => {
                // Try ast_to_cel as a last resort
                if let Some(cel) = self.try_expr_to_cel(expr, negated, ctx) {
                    conditions.push(Condition {
                        var: "_cel".to_string(),
                        op: ConditionOp::Eq,
                        value: ConditionValue::String(cel),
                    });
                    return 0.8;
                }
                ctx.record_skipped(
                    &format!("{:?}", std::mem::discriminant(expr)),
                    "Unknown expression type in condition",
                    expr.span(),
                );
                0.5
            }
        }
    }

    /// Convert a list of conditions to a WhenClause, handling special _cel conditions
    fn conditions_to_when(&self, conditions: &[Condition]) -> Option<WhenClause> {
        if conditions.is_empty() {
            return None;
        }

        let cel_parts: Vec<String> = conditions
            .iter()
            .map(|c| {
                if c.var == "_cel" {
                    // This is a raw CEL expression stored via try_expr_to_cel
                    if let ConditionValue::String(cel) = &c.value {
                        cel.clone()
                    } else {
                        format!("{} {} {:?}", c.var, c.op, c.value)
                    }
                } else {
                    // Convert structured condition to CEL
                    let value_str = match &c.value {
                        ConditionValue::Bool(b) => b.to_string(),
                        ConditionValue::Int(i) => i.to_string(),
                        ConditionValue::Float(f) => f.to_string(),
                        ConditionValue::String(s) => format!("\"{}\"", s),
                        ConditionValue::Null => "null".to_string(),
                        ConditionValue::List(_) | ConditionValue::Map(_) => return String::new(),
                    };
                    format!("{} {} {}", c.var, c.op, value_str)
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        if cel_parts.is_empty() {
            None
        } else if cel_parts.len() == 1 {
            Some(WhenClause::Single(cel_parts[0].clone()))
        } else {
            Some(WhenClause::Multiple(cel_parts))
        }
    }

    fn extract_output_with_context(
        &self,
        node: &AstNode,
        ctx: &ExtractionContext,
    ) -> Option<ConditionValue> {
        match node {
            AstNode::Literal { value, .. } => Some(self.literal_to_value(value)),
            AstNode::Block {
                result: Some(inner),
                ..
            } => self.extract_output_with_context(inner, ctx),
            AstNode::Return {
                value: Some(inner), ..
            } => self.extract_output_with_context(inner, ctx),
            // For complex expressions, try ast_to_cel conversion
            AstNode::Call { .. }
            | AstNode::Field { .. }
            | AstNode::Binary { .. }
            | AstNode::Unary { .. }
            | AstNode::Index { .. }
            | AstNode::Var { .. } => {
                // Try to convert to CEL expression - this allows computed outputs
                // The caller will wrap this in Output::Expression
                match ast_to_cel(node) {
                    Ok(cel_expr) => {
                        // Return a special string marker that indicates this is an expression
                        // The caller should check for this and use Output::Expression instead
                        Some(ConditionValue::String(format!("${{cel:{}}}", cel_expr)))
                    }
                    Err(e) => {
                        ctx.record_skipped(
                            &format!("{:?}", std::mem::discriminant(node)),
                            &e.to_string(),
                            node.span(),
                        );
                        None
                    }
                }
            }
            _ => None,
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

    /// Find early return pattern in a for-each loop body
    ///
    /// Looks for patterns like:
    /// ```ignore
    /// for item in items {
    ///     if condition(item) {
    ///         return value;
    ///     }
    /// }
    /// ```
    /// Returns (condition_cel, return_value) if found
    fn find_early_return_pattern(
        &self,
        body: &AstNode,
        item_var: &str,
        ctx: &ExtractionContext,
    ) -> Option<(String, ConditionValue)> {
        match body {
            // Direct if statement in loop body
            AstNode::If {
                condition,
                then_branch,
                ..
            } => {
                // Try to convert the condition to CEL
                if let Ok(condition_cel) = ast_to_cel_with_ctx(condition, ctx) {
                    // Check if the then branch is a return
                    if let Some(value) = self.extract_return_value(then_branch) {
                        return Some((condition_cel, value));
                    }
                }
                None
            }

            // Block with statements - look for if inside
            AstNode::Block { statements, .. } => {
                for stmt in statements {
                    if let Some(result) = self.find_early_return_pattern(stmt, item_var, ctx) {
                        return Some(result);
                    }
                }
                None
            }

            _ => None,
        }
    }

    /// Extract the return value from a return statement or block with return
    fn extract_return_value(&self, node: &AstNode) -> Option<ConditionValue> {
        match node {
            AstNode::Return {
                value: Some(inner), ..
            } => match inner.as_ref() {
                AstNode::Literal { value, .. } => Some(self.literal_to_value(value)),
                _ => None,
            },
            AstNode::Block {
                statements,
                result,
                ..
            } => {
                // Check statements for a return
                for stmt in statements {
                    if let Some(value) = self.extract_return_value(stmt) {
                        return Some(value);
                    }
                }
                // Check result expression
                if let Some(inner) = result {
                    return self.extract_return_value(inner);
                }
                None
            }
            AstNode::Literal { value, .. } => Some(self.literal_to_value(value)),
            _ => None,
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

    // ========================================================================
    // ast_to_cel tests
    // ========================================================================

    fn span() -> Span {
        Span::default()
    }

    #[test]
    fn test_ast_to_cel_literal_bool() {
        let node = AstNode::Literal {
            value: LiteralValue::Bool(true),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "true");

        let node = AstNode::Literal {
            value: LiteralValue::Bool(false),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "false");
    }

    #[test]
    fn test_ast_to_cel_literal_int() {
        let node = AstNode::Literal {
            value: LiteralValue::Int(42),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "42");

        let node = AstNode::Literal {
            value: LiteralValue::Int(-100),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "-100");
    }

    #[test]
    fn test_ast_to_cel_literal_float() {
        let node = AstNode::Literal {
            value: LiteralValue::Float(3.14),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "3.14");
    }

    #[test]
    fn test_ast_to_cel_literal_string() {
        let node = AstNode::Literal {
            value: LiteralValue::String("hello".to_string()),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "\"hello\"");
    }

    #[test]
    fn test_ast_to_cel_literal_string_escape() {
        let node = AstNode::Literal {
            value: LiteralValue::String("line1\nline2".to_string()),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "\"line1\\nline2\"");
    }

    #[test]
    fn test_ast_to_cel_var() {
        let node = AstNode::Var {
            name: "user_id".to_string(),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "user_id");
    }

    #[test]
    fn test_ast_to_cel_field_access() {
        let node = AstNode::Field {
            object: Box::new(AstNode::Var {
                name: "user".to_string(),
                span: span(),
            }),
            field: "name".to_string(),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "user.name");
    }

    #[test]
    fn test_ast_to_cel_nested_field_access() {
        // user.profile.settings.theme
        let node = AstNode::Field {
            object: Box::new(AstNode::Field {
                object: Box::new(AstNode::Field {
                    object: Box::new(AstNode::Var {
                        name: "user".to_string(),
                        span: span(),
                    }),
                    field: "profile".to_string(),
                    span: span(),
                }),
                field: "settings".to_string(),
                span: span(),
            }),
            field: "theme".to_string(),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "user.profile.settings.theme");
    }

    #[test]
    fn test_ast_to_cel_function_call() {
        let node = AstNode::Call {
            function: "is_valid".to_string(),
            args: vec![AstNode::Var {
                name: "x".to_string(),
                span: span(),
            }],
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "is_valid(x)");
    }

    #[test]
    fn test_ast_to_cel_function_call_multiple_args() {
        let node = AstNode::Call {
            function: "max".to_string(),
            args: vec![
                AstNode::Var {
                    name: "a".to_string(),
                    span: span(),
                },
                AstNode::Var {
                    name: "b".to_string(),
                    span: span(),
                },
            ],
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "max(a, b)");
    }

    #[test]
    fn test_ast_to_cel_binary_comparison() {
        let node = AstNode::Binary {
            op: BinaryOp::Gt,
            left: Box::new(AstNode::Var {
                name: "x".to_string(),
                span: span(),
            }),
            right: Box::new(AstNode::Literal {
                value: LiteralValue::Int(10),
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "(x > 10)");
    }

    #[test]
    fn test_ast_to_cel_binary_logical_and() {
        // x > 10 && y < 20
        let node = AstNode::Binary {
            op: BinaryOp::And,
            left: Box::new(AstNode::Binary {
                op: BinaryOp::Gt,
                left: Box::new(AstNode::Var {
                    name: "x".to_string(),
                    span: span(),
                }),
                right: Box::new(AstNode::Literal {
                    value: LiteralValue::Int(10),
                    span: span(),
                }),
                span: span(),
            }),
            right: Box::new(AstNode::Binary {
                op: BinaryOp::Lt,
                left: Box::new(AstNode::Var {
                    name: "y".to_string(),
                    span: span(),
                }),
                right: Box::new(AstNode::Literal {
                    value: LiteralValue::Int(20),
                    span: span(),
                }),
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "((x > 10) && (y < 20))");
    }

    #[test]
    fn test_ast_to_cel_unary_not() {
        let node = AstNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(AstNode::Var {
                name: "enabled".to_string(),
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "!(enabled)");
    }

    #[test]
    fn test_ast_to_cel_index_access() {
        let node = AstNode::Index {
            object: Box::new(AstNode::Var {
                name: "items".to_string(),
                span: span(),
            }),
            index: Box::new(AstNode::Literal {
                value: LiteralValue::Int(0),
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "items[0]");
    }

    #[test]
    fn test_ast_to_cel_array() {
        let node = AstNode::Array {
            elements: vec![
                AstNode::Literal {
                    value: LiteralValue::Int(1),
                    span: span(),
                },
                AstNode::Literal {
                    value: LiteralValue::Int(2),
                    span: span(),
                },
                AstNode::Literal {
                    value: LiteralValue::Int(3),
                    span: span(),
                },
            ],
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "[1, 2, 3]");
    }

    #[test]
    fn test_ast_to_cel_field_method_chain() {
        // user.items.size()
        let node = AstNode::Call {
            function: "size".to_string(),
            args: vec![AstNode::Field {
                object: Box::new(AstNode::Var {
                    name: "user".to_string(),
                    span: span(),
                }),
                field: "items".to_string(),
                span: span(),
            }],
            span: span(),
        };
        assert_eq!(ast_to_cel(&node).unwrap(), "size(user.items)");
    }

    #[test]
    fn test_ast_to_cel_unknown_node_error() {
        let node = AstNode::Unknown {
            kind: "macro_invocation".to_string(),
            span: Span {
                start_line: 10,
                start_col: 0,
                end_line: 10,
                end_col: 20,
            },
        };
        let err = ast_to_cel(&node).unwrap_err();
        assert!(matches!(err, ConversionError::UnknownNode { .. }));
    }

    #[test]
    fn test_ast_to_cel_control_flow_error() {
        let node = AstNode::If {
            condition: Box::new(AstNode::Var {
                name: "x".to_string(),
                span: span(),
            }),
            then_branch: Box::new(AstNode::Literal {
                value: LiteralValue::Int(1),
                span: span(),
            }),
            else_branch: None,
            span: span(),
        };
        let err = ast_to_cel(&node).unwrap_err();
        assert!(matches!(err, ConversionError::ControlFlow(_)));
    }

    // ========================================================================
    // pattern_to_cel tests
    // ========================================================================

    #[test]
    fn test_pattern_to_cel_literal() {
        let pattern = Pattern::Literal(LiteralValue::Int(42));
        assert_eq!(
            pattern_to_cel(&pattern, "x"),
            Some("x == 42".to_string())
        );
    }

    #[test]
    fn test_pattern_to_cel_wildcard() {
        let pattern = Pattern::Wildcard;
        assert_eq!(pattern_to_cel(&pattern, "x"), None);
    }

    #[test]
    fn test_pattern_to_cel_constructor() {
        let pattern = Pattern::Constructor {
            name: "Active".to_string(),
            fields: vec![],
        };
        assert_eq!(
            pattern_to_cel(&pattern, "status"),
            Some("status == \"Active\"".to_string())
        );
    }

    #[test]
    fn test_pattern_to_cel_or() {
        let pattern = Pattern::Or(vec![
            Pattern::Literal(LiteralValue::Int(1)),
            Pattern::Literal(LiteralValue::Int(2)),
            Pattern::Literal(LiteralValue::Int(3)),
        ]);
        assert_eq!(
            pattern_to_cel(&pattern, "x"),
            Some("x in [1, 2, 3]".to_string())
        );
    }

    #[test]
    fn test_pattern_to_cel_or_constructors() {
        let pattern = Pattern::Or(vec![
            Pattern::Constructor {
                name: "Active".to_string(),
                fields: vec![],
            },
            Pattern::Constructor {
                name: "Pending".to_string(),
                fields: vec![],
            },
        ]);
        assert_eq!(
            pattern_to_cel(&pattern, "status"),
            Some("status in [\"Active\", \"Pending\"]".to_string())
        );
    }

    // ========================================================================
    // Original extraction tests
    // ========================================================================

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

    // ========================================================================
    // Data flow tracking tests
    // ========================================================================

    #[test]
    fn test_ast_to_cel_with_ctx_variable_substitution() {
        // Test that variables are inlined when their definition is tracked
        let ctx = ExtractionContext::new("");

        // Define x = a + b
        let def = AstNode::Binary {
            op: BinaryOp::Add,
            left: Box::new(AstNode::Var {
                name: "a".to_string(),
                span: span(),
            }),
            right: Box::new(AstNode::Var {
                name: "b".to_string(),
                span: span(),
            }),
            span: span(),
        };
        ctx.define_var("x", def);

        // Now convert x > 10 - should become (a + b) > 10
        let expr = AstNode::Binary {
            op: BinaryOp::Gt,
            left: Box::new(AstNode::Var {
                name: "x".to_string(),
                span: span(),
            }),
            right: Box::new(AstNode::Literal {
                value: LiteralValue::Int(10),
                span: span(),
            }),
            span: span(),
        };

        let result = ast_to_cel_with_ctx(&expr, &ctx).unwrap();
        // x gets inlined as (a + b)
        assert_eq!(result, "(((a + b)) > 10)");
    }

    #[test]
    fn test_ast_to_cel_with_ctx_no_substitution_for_unknown() {
        // Test that undefined variables are kept as-is
        let ctx = ExtractionContext::new("");

        let expr = AstNode::Var {
            name: "unknown_var".to_string(),
            span: span(),
        };

        let result = ast_to_cel_with_ctx(&expr, &ctx).unwrap();
        assert_eq!(result, "unknown_var");
    }

    #[test]
    fn test_ast_to_cel_with_ctx_chained_substitution() {
        // Test that chained definitions are resolved: y = x, x = 5
        let ctx = ExtractionContext::new("");

        // Define x = 5
        ctx.define_var(
            "x",
            AstNode::Literal {
                value: LiteralValue::Int(5),
                span: span(),
            },
        );

        // Define y = x
        ctx.define_var(
            "y",
            AstNode::Var {
                name: "x".to_string(),
                span: span(),
            },
        );

        // Now convert y - should become (5) (through x)
        let expr = AstNode::Var {
            name: "y".to_string(),
            span: span(),
        };

        let result = ast_to_cel_with_ctx(&expr, &ctx).unwrap();
        // y -> (x) -> ((5))
        assert_eq!(result, "((5))");
    }

    #[test]
    fn test_extract_with_let_binding() {
        // Test that let bindings are tracked during extraction
        let code = r#"
fn check(a: i32, b: i32) -> bool {
    let sum = a + b;
    if sum > 10 {
        true
    } else {
        false
    }
}
"#;
        let ast = parse_rust(code).unwrap();
        let report = extract_with_report(&ast, code);

        // Should have at least 1 rule
        assert!(
            !report.extracted.spec.rules.is_empty(),
            "Expected at least one rule"
        );

        // Check that the condition uses the inlined expression
        // The first rule should have a condition involving (a + b), not just "sum"
        let rule1 = &report.extracted.spec.rules[0];
        if let Some(when) = &rule1.when {
            let cel = when.to_cel();
            // The condition should include the inlined expression (a + b)
            // since we're tracking variable definitions now
            assert!(
                cel.contains("a") || cel.contains("+"),
                "Expected condition to contain inlined expression with 'a' or '+', got: {}",
                cel
            );
        }
    }

    #[test]
    fn test_extraction_context_var_tracking() {
        // Directly test that ExtractionContext tracks and resolves variables
        let ctx = ExtractionContext::new("");

        // Define: total = price + tax
        let price_var = AstNode::Var {
            name: "price".to_string(),
            span: span(),
        };
        let tax_var = AstNode::Var {
            name: "tax".to_string(),
            span: span(),
        };
        let def = AstNode::Binary {
            op: BinaryOp::Add,
            left: Box::new(price_var),
            right: Box::new(tax_var),
            span: span(),
        };
        ctx.define_var("total", def.clone());

        // Resolve should return the definition
        let resolved = ctx.resolve_var("total");
        assert!(resolved.is_some(), "Variable 'total' should be resolvable");

        // Unknown variable should return None
        let unknown = ctx.resolve_var("unknown");
        assert!(unknown.is_none(), "Unknown variable should return None");
    }

    // ========================================================================
    // Loop/Try extraction tests (Phase 5)
    // ========================================================================

    #[test]
    fn test_find_early_return_pattern_simple() {
        // Test the find_early_return_pattern helper directly
        let extractor = Extractor::new();
        let ctx = ExtractionContext::new("");

        // Create: if item.valid { return true }
        let body = AstNode::If {
            condition: Box::new(AstNode::Field {
                object: Box::new(AstNode::Var {
                    name: "item".to_string(),
                    span: span(),
                }),
                field: "valid".to_string(),
                span: span(),
            }),
            then_branch: Box::new(AstNode::Return {
                value: Some(Box::new(AstNode::Literal {
                    value: LiteralValue::Bool(true),
                    span: span(),
                })),
                span: span(),
            }),
            else_branch: None,
            span: span(),
        };

        let result = extractor.find_early_return_pattern(&body, "item", &ctx);
        assert!(result.is_some(), "Should find early return pattern");

        let (condition_cel, return_value) = result.unwrap();
        assert_eq!(condition_cel, "item.valid");
        assert_eq!(return_value, ConditionValue::Bool(true));
    }

    #[test]
    fn test_find_early_return_pattern_in_block() {
        // Test finding early return pattern when wrapped in a Block
        let extractor = Extractor::new();
        let ctx = ExtractionContext::new("");

        // Create block with: if item > 10 { return "found" }
        let body = AstNode::Block {
            statements: vec![AstNode::If {
                condition: Box::new(AstNode::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(AstNode::Var {
                        name: "item".to_string(),
                        span: span(),
                    }),
                    right: Box::new(AstNode::Literal {
                        value: LiteralValue::Int(10),
                        span: span(),
                    }),
                    span: span(),
                }),
                then_branch: Box::new(AstNode::Return {
                    value: Some(Box::new(AstNode::Literal {
                        value: LiteralValue::String("found".to_string()),
                        span: span(),
                    })),
                    span: span(),
                }),
                else_branch: None,
                span: span(),
            }],
            result: None,
            span: span(),
        };

        let result = extractor.find_early_return_pattern(&body, "item", &ctx);
        assert!(result.is_some(), "Should find early return pattern in block");

        let (condition_cel, return_value) = result.unwrap();
        assert_eq!(condition_cel, "(item > 10)");
        assert_eq!(
            return_value,
            ConditionValue::String("found".to_string())
        );
    }

    #[test]
    fn test_find_early_return_pattern_no_match() {
        // Test that non-matching patterns return None
        let extractor = Extractor::new();
        let ctx = ExtractionContext::new("");

        // Create a simple variable reference (not an if statement)
        let body = AstNode::Var {
            name: "x".to_string(),
            span: span(),
        };

        let result = extractor.find_early_return_pattern(&body, "item", &ctx);
        assert!(result.is_none(), "Should not find early return pattern in var");
    }

    #[test]
    fn test_extract_return_value_simple() {
        // Test extracting a return value from a simple return statement
        let extractor = Extractor::new();

        let node = AstNode::Return {
            value: Some(Box::new(AstNode::Literal {
                value: LiteralValue::Int(42),
                span: span(),
            })),
            span: span(),
        };

        let result = extractor.extract_return_value(&node);
        assert_eq!(result, Some(ConditionValue::Int(42)));
    }

    #[test]
    fn test_extract_return_value_from_block() {
        // Test extracting return value from inside a block
        let extractor = Extractor::new();

        let node = AstNode::Block {
            statements: vec![AstNode::Return {
                value: Some(Box::new(AstNode::Literal {
                    value: LiteralValue::Bool(false),
                    span: span(),
                })),
                span: span(),
            }],
            result: None,
            span: span(),
        };

        let result = extractor.extract_return_value(&node);
        assert_eq!(result, Some(ConditionValue::Bool(false)));
    }

    #[test]
    fn test_extract_return_value_literal() {
        // Test that bare literals are also extracted
        let extractor = Extractor::new();

        let node = AstNode::Literal {
            value: LiteralValue::String("result".to_string()),
            span: span(),
        };

        let result = extractor.extract_return_value(&node);
        assert_eq!(result, Some(ConditionValue::String("result".to_string())));
    }

    #[test]
    fn test_extract_return_value_none_for_complex() {
        // Test that complex expressions return None
        let extractor = Extractor::new();

        // A function call is not extractable as a return value
        let node = AstNode::Call {
            function: "compute".to_string(),
            args: vec![],
            span: span(),
        };

        let result = extractor.extract_return_value(&node);
        assert!(result.is_none(), "Complex expressions should return None");
    }

    #[test]
    fn test_foreach_skipped_without_early_return() {
        // Test that ForEach without early return pattern is skipped with diagnostic
        use crate::ast::{CodeAst, Function, Language, Parameter};

        // Create: for item in items { do_something(item) }
        let func = Function {
            name: "process_all".to_string(),
            params: vec![Parameter {
                name: "items".to_string(),
                typ: "Vec<Item>".to_string(),
            }],
            return_type: None,
            body: AstNode::ForEach {
                item: "item".to_string(),
                index: None,
                collection: Box::new(AstNode::Var {
                    name: "items".to_string(),
                    span: span(),
                }),
                body: Box::new(AstNode::Call {
                    function: "do_something".to_string(),
                    args: vec![AstNode::Var {
                        name: "item".to_string(),
                        span: span(),
                    }],
                    span: span(),
                }),
                span: span(),
            },
            span: span(),
        };

        let ast = CodeAst {
            language: Language::Rust,
            functions: vec![func],
            source_hash: String::new(),
        };

        let report = extract_with_report(&ast, "");

        // ForEach without early return should be skipped
        let has_foreach_skip = report.skipped_nodes.iter().any(|s| s.kind == "ForEach");
        assert!(
            has_foreach_skip,
            "ForEach without early return should be noted as skipped"
        );
    }

    #[test]
    fn test_while_loop_skipped() {
        // Test that while loops are properly skipped with diagnostic
        use crate::ast::{CodeAst, Function, Language, Parameter};

        // Create: while x > 0 { x -= 1 }
        let func = Function {
            name: "countdown".to_string(),
            params: vec![Parameter {
                name: "x".to_string(),
                typ: "i32".to_string(),
            }],
            return_type: None,
            body: AstNode::While {
                condition: Box::new(AstNode::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(AstNode::Var {
                        name: "x".to_string(),
                        span: span(),
                    }),
                    right: Box::new(AstNode::Literal {
                        value: LiteralValue::Int(0),
                        span: span(),
                    }),
                    span: span(),
                }),
                body: Box::new(AstNode::Assign {
                    target: Box::new(AstNode::Var {
                        name: "x".to_string(),
                        span: span(),
                    }),
                    value: Box::new(AstNode::Binary {
                        op: BinaryOp::Sub,
                        left: Box::new(AstNode::Var {
                            name: "x".to_string(),
                            span: span(),
                        }),
                        right: Box::new(AstNode::Literal {
                            value: LiteralValue::Int(1),
                            span: span(),
                        }),
                        span: span(),
                    }),
                    span: span(),
                }),
                span: span(),
            },
            span: span(),
        };

        let ast = CodeAst {
            language: Language::Rust,
            functions: vec![func],
            source_hash: String::new(),
        };

        let report = extract_with_report(&ast, "");

        // While should be skipped
        let has_while_skip = report.skipped_nodes.iter().any(|s| s.kind == "While");
        assert!(has_while_skip, "While loop should be noted as skipped");
    }

    #[test]
    fn test_try_catch_extraction() {
        // Test that Try/Catch blocks are extracted with _error condition
        use crate::ast::{CodeAst, Function, Language, Parameter};

        // Create: try { if x > 0 { return 1 } } catch { return -1 }
        // We need conditions in the try block to generate a rule
        let func = Function {
            name: "safe_call".to_string(),
            params: vec![Parameter {
                name: "x".to_string(),
                typ: "i32".to_string(),
            }],
            return_type: Some("i32".to_string()),
            body: AstNode::Try {
                try_block: Box::new(AstNode::If {
                    condition: Box::new(AstNode::Binary {
                        op: BinaryOp::Gt,
                        left: Box::new(AstNode::Var {
                            name: "x".to_string(),
                            span: span(),
                        }),
                        right: Box::new(AstNode::Literal {
                            value: LiteralValue::Int(0),
                            span: span(),
                        }),
                        span: span(),
                    }),
                    then_branch: Box::new(AstNode::Return {
                        value: Some(Box::new(AstNode::Literal {
                            value: LiteralValue::Int(1),
                            span: span(),
                        })),
                        span: span(),
                    }),
                    else_branch: None,
                    span: span(),
                }),
                catch_var: Some("e".to_string()),
                catch_block: Some(Box::new(AstNode::Return {
                    value: Some(Box::new(AstNode::Literal {
                        value: LiteralValue::Int(-1),
                        span: span(),
                    })),
                    span: span(),
                })),
                finally_block: None,
                span: span(),
            },
            span: span(),
        };

        let ast = CodeAst {
            language: Language::Rust,
            functions: vec![func],
            source_hash: String::new(),
        };

        let extracted = extract(&ast);

        // Should have at least 2 rules: one for try (if condition), one for catch (_error)
        assert!(
            extracted.spec.rules.len() >= 2,
            "Should extract rules from both try and catch blocks, got {} rules",
            extracted.spec.rules.len()
        );

        // Look for a rule with _error condition
        let has_error_rule = extracted.spec.rules.iter().any(|r| {
            if let Some(when) = &r.when {
                let cel: String = when.to_cel();
                cel.contains("_error")
            } else {
                false
            }
        });

        assert!(
            has_error_rule,
            "Should have a rule with _error condition for catch block"
        );
    }

    #[test]
    fn test_try_with_finally_extraction() {
        // Test that finally blocks are noted but don't affect conditions
        use crate::ast::{CodeAst, Function, Language, Parameter};

        // Create: try { if x > 0 { return 1 } } catch { return -1 } finally { cleanup() }
        // We need conditions in the try block to generate a rule
        let func = Function {
            name: "with_cleanup".to_string(),
            params: vec![Parameter {
                name: "x".to_string(),
                typ: "i32".to_string(),
            }],
            return_type: Some("i32".to_string()),
            body: AstNode::Try {
                try_block: Box::new(AstNode::If {
                    condition: Box::new(AstNode::Binary {
                        op: BinaryOp::Gt,
                        left: Box::new(AstNode::Var {
                            name: "x".to_string(),
                            span: span(),
                        }),
                        right: Box::new(AstNode::Literal {
                            value: LiteralValue::Int(0),
                            span: span(),
                        }),
                        span: span(),
                    }),
                    then_branch: Box::new(AstNode::Return {
                        value: Some(Box::new(AstNode::Literal {
                            value: LiteralValue::Int(1),
                            span: span(),
                        })),
                        span: span(),
                    }),
                    else_branch: None,
                    span: span(),
                }),
                catch_var: None,
                catch_block: Some(Box::new(AstNode::Return {
                    value: Some(Box::new(AstNode::Literal {
                        value: LiteralValue::Int(-1),
                        span: span(),
                    })),
                    span: span(),
                })),
                finally_block: Some(Box::new(AstNode::Call {
                    function: "cleanup".to_string(),
                    args: vec![],
                    span: span(),
                })),
                span: span(),
            },
            span: span(),
        };

        let ast = CodeAst {
            language: Language::Rust,
            functions: vec![func],
            source_hash: String::new(),
        };

        let report = extract_with_report(&ast, "");

        // Should extract at least one rule (from catch with _error condition)
        // The try block generates a rule only if there are conditions
        assert!(
            !report.extracted.spec.rules.is_empty(),
            "Should extract rules from try/catch"
        );

        // Finally should be noted as skipped
        let has_finally_skip = report.skipped_nodes.iter().any(|s| s.kind == "Finally");
        assert!(
            has_finally_skip,
            "Finally block should be noted as skipped"
        );
    }

    #[test]
    fn test_foreach_with_early_return_extraction() {
        // Test that ForEach loops with early return generate exists pattern
        use crate::ast::{CodeAst, Function, Language, Parameter};

        // Create: for item in items { if item.active { return true } }
        let func = Function {
            name: "has_active".to_string(),
            params: vec![Parameter {
                name: "items".to_string(),
                typ: "Vec<Item>".to_string(),
            }],
            return_type: Some("bool".to_string()),
            body: AstNode::ForEach {
                item: "item".to_string(),
                index: None,
                collection: Box::new(AstNode::Var {
                    name: "items".to_string(),
                    span: span(),
                }),
                body: Box::new(AstNode::If {
                    condition: Box::new(AstNode::Field {
                        object: Box::new(AstNode::Var {
                            name: "item".to_string(),
                            span: span(),
                        }),
                        field: "active".to_string(),
                        span: span(),
                    }),
                    then_branch: Box::new(AstNode::Return {
                        value: Some(Box::new(AstNode::Literal {
                            value: LiteralValue::Bool(true),
                            span: span(),
                        })),
                        span: span(),
                    }),
                    else_branch: None,
                    span: span(),
                }),
                span: span(),
            },
            span: span(),
        };

        let ast = CodeAst {
            language: Language::Rust,
            functions: vec![func],
            source_hash: String::new(),
        };

        let extracted = extract(&ast);

        // Should have at least one rule
        assert!(
            !extracted.spec.rules.is_empty(),
            "Should extract rules from ForEach exists pattern"
        );

        // Check that the first rule contains the exists pattern
        let rule = &extracted.spec.rules[0];
        if let Some(when) = &rule.when {
            let cel: String = when.to_cel();
            assert!(
                cel.contains("exists"),
                "Rule should contain exists pattern, got: {}",
                cel
            );
            assert!(
                cel.contains("items"),
                "Rule should reference collection 'items', got: {}",
                cel
            );
        } else {
            panic!("Rule should have when clause");
        }
    }

    // ========================================================================
    // Phase 6 tests - New node types (MacroCall, Ref, Cast, SyntaxError)
    // ========================================================================

    #[test]
    fn test_ast_to_cel_macro_call_unsupported() {
        // Macro calls cannot be converted to CEL
        let node = AstNode::MacroCall {
            name: "println".to_string(),
            args: "(\"hello\")".to_string(),
            span: span(),
        };

        let result = ast_to_cel(&node);
        assert!(result.is_err(), "Macro calls should not be convertible to CEL");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("macro"),
            "Error should mention macro: {}",
            err
        );
    }

    #[test]
    fn test_ast_to_cel_ref_dereference() {
        // Reference expression should dereference to inner value
        let node = AstNode::Ref {
            mutable: false,
            expr: Box::new(AstNode::Var {
                name: "x".to_string(),
                span: span(),
            }),
            span: span(),
        };

        let result = ast_to_cel(&node);
        assert!(result.is_ok(), "Ref should be convertible to CEL");
        assert_eq!(result.unwrap(), "x");
    }

    #[test]
    fn test_ast_to_cel_ref_mutable() {
        // Mutable reference should also dereference
        let node = AstNode::Ref {
            mutable: true,
            expr: Box::new(AstNode::Field {
                object: Box::new(AstNode::Var {
                    name: "obj".to_string(),
                    span: span(),
                }),
                field: "field".to_string(),
                span: span(),
            }),
            span: span(),
        };

        let result = ast_to_cel(&node);
        assert!(result.is_ok(), "Mutable ref should be convertible to CEL");
        assert_eq!(result.unwrap(), "obj.field");
    }

    #[test]
    fn test_ast_to_cel_cast() {
        // Cast should convert inner and note target type
        let node = AstNode::Cast {
            expr: Box::new(AstNode::Var {
                name: "x".to_string(),
                span: span(),
            }),
            target_type: "i32".to_string(),
            span: span(),
        };

        let result = ast_to_cel(&node);
        assert!(result.is_ok(), "Cast should be convertible to CEL");
        let cel = result.unwrap();
        assert!(cel.contains("x"), "Should contain variable name");
        assert!(cel.contains("i32"), "Should contain target type");
    }

    #[test]
    fn test_ast_to_cel_syntax_error() {
        // Syntax errors should produce conversion errors
        let node = AstNode::SyntaxError {
            message: "unexpected token".to_string(),
            source_text: "@@#$%".to_string(),
            span: span(),
        };

        let result = ast_to_cel(&node);
        assert!(result.is_err(), "Syntax errors should not be convertible");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Syntax error"),
            "Error should indicate syntax error: {}",
            err
        );
    }

    #[test]
    fn test_parse_rust_macro_invocation() {
        // Test that macro invocations are parsed correctly
        use crate::parse_rust;

        let code = r#"
fn test() {
    println!("Hello, world!");
}
"#;
        let ast = parse_rust(code).unwrap();
        assert!(!ast.functions.is_empty());

        // The body should contain a MacroCall
        let body = &ast.functions[0].body;
        let has_macro = contains_node_kind(body, "MacroCall");
        assert!(has_macro, "Should parse macro invocation as MacroCall");
    }

    #[test]
    fn test_parse_rust_reference() {
        // Test that reference expressions are parsed correctly
        use crate::parse_rust;

        let code = r#"
fn borrow(x: i32) -> i32 {
    let r = &x;
    *r
}
"#;
        let ast = parse_rust(code).unwrap();
        assert!(!ast.functions.is_empty());

        // Should contain a Ref node
        let body = &ast.functions[0].body;
        let has_ref = contains_node_kind(body, "Ref");
        assert!(has_ref, "Should parse reference expression as Ref");
    }

    #[test]
    fn test_parse_rust_type_cast() {
        // Test that type cast expressions are parsed correctly
        use crate::parse_rust;

        let code = r#"
fn cast(x: f64) -> i32 {
    x as i32
}
"#;
        let ast = parse_rust(code).unwrap();
        assert!(!ast.functions.is_empty());

        // Should contain a Cast node
        let body = &ast.functions[0].body;
        let has_cast = contains_node_kind(body, "Cast");
        assert!(has_cast, "Should parse type cast expression as Cast");
    }

    /// Helper function to check if an AST contains a specific node kind
    fn contains_node_kind(node: &AstNode, kind: &str) -> bool {
        match node {
            AstNode::MacroCall { .. } if kind == "MacroCall" => true,
            AstNode::Ref { expr, .. } => {
                kind == "Ref" || contains_node_kind(expr, kind)
            }
            AstNode::Cast { expr, .. } => {
                kind == "Cast" || contains_node_kind(expr, kind)
            }
            AstNode::SyntaxError { .. } if kind == "SyntaxError" => true,
            AstNode::Block { statements, result, .. } => {
                statements.iter().any(|s| contains_node_kind(s, kind))
                    || result.as_ref().map(|r| contains_node_kind(r, kind)).unwrap_or(false)
            }
            AstNode::Return { value, .. } => {
                value.as_ref().map(|v| contains_node_kind(v, kind)).unwrap_or(false)
            }
            AstNode::If { condition, then_branch, else_branch, .. } => {
                contains_node_kind(condition, kind)
                    || contains_node_kind(then_branch, kind)
                    || else_branch.as_ref().map(|e| contains_node_kind(e, kind)).unwrap_or(false)
            }
            AstNode::Binary { left, right, .. } => {
                contains_node_kind(left, kind) || contains_node_kind(right, kind)
            }
            AstNode::Unary { operand, .. } => contains_node_kind(operand, kind),
            AstNode::Field { object, .. } => contains_node_kind(object, kind),
            AstNode::Call { args, .. } => args.iter().any(|a| contains_node_kind(a, kind)),
            AstNode::Index { object, index, .. } => {
                contains_node_kind(object, kind) || contains_node_kind(index, kind)
            }
            AstNode::Let { value, .. } => contains_node_kind(value, kind),
            _ => false,
        }
    }
}
