//! Test generation — create tests from specs and orchestrators
//!
//! Generates comprehensive test suites from decision tables:
//! - One test per rule (basic coverage)
//! - Exhaustive tests (all input combinations)
//! - Boundary tests (edge cases for numeric conditions)
//! - Property tests (fuzzing)
//!
//! For orchestrators:
//! - Happy path tests (all gates pass)
//! - Gate failure tests (each gate individually)
//! - Step execution tests

mod csharp;
mod go;
mod java;
pub mod orchestrator;
mod python;
mod rust;
mod typescript;

use crate::cel::Target;
use crate::spec::*;

// Re-export language modules
pub use csharp::generate as generate_csharp;
pub use go::generate as generate_go;
pub use java::generate as generate_java;
pub use python::generate as generate_python;
pub use rust::generate as generate_rust;
pub use typescript::generate as generate_typescript;

/// Generate tests from spec
pub fn generate_tests(spec: &Spec, target: Target) -> String {
    TestGenerator::new(target).generate(spec)
}

/// Test generator
pub struct TestGenerator {
    target: Target,
    config: TestConfig,
}

/// Test generation configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Target language
    pub target: Target,
    /// Test mode
    pub mode: TestMode,
    /// Include exhaustive tests
    pub exhaustive: bool,
    /// Include boundary tests
    pub boundary: bool,
    /// Include property tests
    pub property: bool,
    /// Test framework
    pub framework: TestFramework,
}

/// Test generation mode
#[derive(Debug, Clone, Copy)]
pub enum TestMode {
    /// One test per rule
    PerRule,
    /// All tests in one function (table-driven)
    TableDriven,
}

/// Test framework
#[derive(Debug, Clone, Copy)]
pub enum TestFramework {
    /// Rust: built-in #[test]
    RustBuiltin,
    /// TypeScript: Vitest
    Vitest,
    /// TypeScript: Jest
    Jest,
    /// Python: pytest
    Pytest,
    /// C#: xUnit
    XUnit,
    /// Java: JUnit
    JUnit,
    /// Go: testing
    GoTest,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            target: Target::Rust,
            mode: TestMode::PerRule,
            exhaustive: true,
            boundary: true,
            property: true,
            framework: TestFramework::RustBuiltin,
        }
    }
}

impl TestGenerator {
    pub fn new(target: Target) -> Self {
        let framework = match target {
            Target::Rust => TestFramework::RustBuiltin,
            Target::TypeScript => TestFramework::Vitest,
            Target::Python => TestFramework::Pytest,
            Target::CSharp => TestFramework::XUnit,
            Target::Java => TestFramework::JUnit,
            Target::Go => TestFramework::GoTest,
        };

        Self {
            target,
            config: TestConfig {
                target,
                framework,
                ..Default::default()
            },
        }
    }

    pub fn with_config(config: TestConfig) -> Self {
        Self {
            target: config.target,
            config,
        }
    }

    /// Generate test file
    pub fn generate(&self, spec: &Spec) -> String {
        match self.target {
            Target::Rust => rust::generate(spec, &self.config),
            Target::TypeScript => typescript::generate(spec, &self.config),
            Target::Python => python::generate(spec, &self.config),
            Target::CSharp => csharp::generate(spec, &self.config),
            Target::Java => java::generate(spec, &self.config),
            Target::Go => go::generate(spec, &self.config),
        }
    }
}

// ============================================================================
// Common utilities
// ============================================================================

/// Extract test input values from a rule (handles both `conditions` and `when` CEL)
///
/// Returns a map of variable name -> value string
pub(crate) fn extract_test_values(
    rule: &Rule,
    inputs: &[Variable],
) -> std::collections::HashMap<String, String> {
    use crate::cel::CelCompiler;

    let mut values = std::collections::HashMap::new();

    // First try structured conditions
    if let Some(conditions) = &rule.conditions {
        for cond in conditions {
            let value_str = match &cond.value {
                ConditionValue::Bool(b) => b.to_string(),
                ConditionValue::Int(i) => i.to_string(),
                ConditionValue::Float(f) => f.to_string(),
                ConditionValue::String(s) => format!("\"{}\"", s),
                ConditionValue::Null => "null".into(),
                _ => continue,
            };
            values.insert(cond.var.clone(), value_str);
        }
    }

    // Then try parsing CEL expression using AST
    if let Some(cel_expr) = rule.as_cel() {
        if let Ok(ast) = CelCompiler::parse(&cel_expr) {
            extract_values_from_cel_ast(&ast, &mut values);
        }
    }

    // Fill in defaults for any inputs not covered
    for input in inputs {
        if !values.contains_key(&input.name) {
            let default = match &input.typ {
                VarType::Bool => "false".into(),
                VarType::Int => "0".into(),
                VarType::Float => "0.0".into(),
                VarType::String => "\"\"".into(),
                VarType::Enum(variants) => variants
                    .first()
                    .map(|v| format!("\"{}\"", v))
                    .unwrap_or("\"\"".into()),
                _ => "null".into(),
            };
            values.insert(input.name.clone(), default);
        }
    }

    values
}

/// Extract variable=value mappings from CEL AST for test generation
fn extract_values_from_cel_ast(
    expr: &crate::cel::CelExpr,
    values: &mut std::collections::HashMap<String, String>,
) {
    use cel_parser::ast::{operators, Expr};

    // In cel-parser 0.10, Expression is IdedExpr with expr field
    match &expr.expr {
        // Handle: var == 'value' or 'value' == var
        Expr::Call(call) if call.func_name == operators::EQUALS && call.args.len() == 2 => {
            let left = &call.args[0];
            let right = &call.args[1];
            // Check left=ident, right=literal
            if let Expr::Ident(var) = &left.expr {
                if let Some(val) = extract_literal_value(right) {
                    values.insert(var.to_string(), val);
                }
            }
            // Check right=ident, left=literal
            if let Expr::Ident(var) = &right.expr {
                if let Some(val) = extract_literal_value(left) {
                    values.insert(var.to_string(), val);
                }
            }
        }

        // Handle: !var (negation means false)
        Expr::Call(call) if call.func_name == operators::LOGICAL_NOT => {
            if let Some(inner) = call.args.first() {
                if let Expr::Ident(var) = &inner.expr {
                    let var_str = var.to_string();
                    if var_str != "true" && var_str != "false" && var_str != "null" {
                        values.insert(var_str, "false".into());
                    }
                }
                // Recurse into inner expression
                extract_values_from_cel_ast(inner, values);
            }
        }

        // Handle: condition && condition
        Expr::Call(call) if call.func_name == operators::LOGICAL_AND && call.args.len() == 2 => {
            let left = &call.args[0];
            let right = &call.args[1];
            // Check for standalone identifiers (means true)
            if let Expr::Ident(var) = &left.expr {
                let var_str = var.to_string();
                if var_str != "true"
                    && var_str != "false"
                    && var_str != "null"
                    && !values.contains_key(&var_str)
                {
                    values.insert(var_str, "true".into());
                }
            }
            if let Expr::Ident(var) = &right.expr {
                let var_str = var.to_string();
                if var_str != "true"
                    && var_str != "false"
                    && var_str != "null"
                    && !values.contains_key(&var_str)
                {
                    values.insert(var_str, "true".into());
                }
            }
            extract_values_from_cel_ast(left, values);
            extract_values_from_cel_ast(right, values);
        }

        // Handle: condition || condition
        Expr::Call(call) if call.func_name == operators::LOGICAL_OR && call.args.len() == 2 => {
            extract_values_from_cel_ast(&call.args[0], values);
            extract_values_from_cel_ast(&call.args[1], values);
        }

        // Standalone identifier (means truthy/true for boolean context)
        Expr::Ident(var) => {
            let var_str = var.to_string();
            if var_str != "true"
                && var_str != "false"
                && var_str != "null"
                && !values.contains_key(&var_str)
            {
                values.insert(var_str, "true".into());
            }
        }

        // Recurse into other expression types
        // Note: Ternary expressions may be represented as Call expressions in the new API
        _ => {}
    }
}

/// Extract a literal value from CEL AST as a string
fn extract_literal_value(expr: &crate::cel::CelExpr) -> Option<String> {
    use cel_parser::reference::Val;

    // In cel-parser 0.10, Expression is IdedExpr with expr field
    match &expr.expr {
        cel_parser::ast::Expr::Literal(val) => match val {
            Val::String(s) => Some(format!("\"{}\"", s)),
            Val::Int(i) => Some(i.to_string()),
            Val::UInt(u) => Some(u.to_string()),
            Val::Double(f) => Some(f.to_string()),
            Val::Boolean(b) => Some(b.to_string()),
            Val::Null => Some("null".into()),
            _ => None,
        },
        _ => None,
    }
}

// Re-export from shared util module
pub(crate) use crate::util::{to_camel_case, to_pascal_case};

/// Check if spec inputs can be enumerated (small finite domain)
pub(crate) fn can_enumerate(spec: &Spec) -> bool {
    if spec.default.is_none() || spec.outputs.len() > 1 {
        return false;
    }

    let total: usize = spec
        .inputs
        .iter()
        .map(|i| match &i.typ {
            VarType::Bool => 2,
            VarType::Enum(v) => v.len(),
            _ => 100,
        })
        .product();
    total <= 64
}

/// Check if spec has numeric conditions (for boundary tests)
pub(crate) fn has_numeric_conditions(spec: &Spec) -> bool {
    spec.rules.iter().any(|r| {
        r.conditions
            .as_ref()
            .map(|c| {
                c.iter().any(|cond| {
                    matches!(
                        cond.op,
                        ConditionOp::Lt | ConditionOp::Le | ConditionOp::Gt | ConditionOp::Ge
                    )
                })
            })
            .unwrap_or(false)
    })
}

/// Generate all input combinations for exhaustive testing
pub(crate) fn generate_combinations(spec: &Spec) -> Vec<(Vec<String>, String, String)> {
    let value_sets: Vec<Vec<String>> = spec
        .inputs
        .iter()
        .map(|i| match &i.typ {
            VarType::Bool => vec!["false".into(), "true".into()],
            VarType::Enum(v) => v.iter().map(|s| format!("\"{}\"", s)).collect(),
            VarType::String => vec!["\"\"".into()],
            VarType::Int => vec!["0".into()],
            VarType::Float => vec!["0.0".into()],
            _ => vec!["null".into()],
        })
        .collect();

    let mut results = Vec::new();
    let mut indices = vec![0; spec.inputs.len()];

    loop {
        let inputs: Vec<String> = indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| value_sets[i][idx].clone())
            .collect();

        let (rule_id, expected) = find_matching_rule(spec, &inputs);
        results.push((inputs, rule_id, expected));

        let mut carry = true;
        for i in (0..indices.len()).rev() {
            if carry {
                indices[i] += 1;
                if indices[i] >= value_sets[i].len() {
                    indices[i] = 0;
                } else {
                    carry = false;
                }
            }
        }

        if carry {
            break;
        }
    }

    results
}

fn find_matching_rule(spec: &Spec, inputs: &[String]) -> (String, String) {
    for rule in &spec.rules {
        if let Some(conditions) = &rule.conditions {
            let matches = conditions.iter().all(|cond| {
                let idx = spec.inputs.iter().position(|i| i.name == cond.var);
                if let Some(idx) = idx {
                    let input_val = &inputs[idx];
                    let cond_val = condition_value_str(&cond.value);
                    input_val == &cond_val
                } else {
                    false
                }
            });

            if matches {
                return (rule.id.clone(), output_value_str(&rule.then));
            }
        }
    }

    spec.default
        .as_ref()
        .map(|d| ("default".into(), output_value_str(d)))
        .unwrap_or_else(|| ("unknown".into(), "null".into()))
}

fn condition_value_str(v: &ConditionValue) -> String {
    match v {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => i.to_string(),
        ConditionValue::Float(f) => f.to_string(),
        ConditionValue::String(s) => format!("\"{}\"", s),
        ConditionValue::Null => "null".into(),
        _ => "null".into(),
    }
}

fn output_value_str(output: &Output) -> String {
    match output {
        Output::Single(v) => condition_value_str(v),
        Output::Named(_) => "{}".into(),
        Output::Expression(expr) => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> Spec {
        Spec::from_yaml(
            r#"
id: check_status
inputs:
  - name: rate_exceeded
    type: bool
  - name: locked
    type: bool
outputs:
  - name: status
    type: int
rules:
  - id: R1
    conditions:
      - var: rate_exceeded
        value: true
    then: 429
  - id: R2
    conditions:
      - var: rate_exceeded
        value: false
      - var: locked
        value: true
    then: 423
  - id: R3
    conditions:
      - var: rate_exceeded
        value: false
      - var: locked
        value: false
    then: 200
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_generate_rust() {
        let spec = sample_spec();
        let tests = generate_tests(&spec, Target::Rust);

        assert!(tests.contains("#[test]"));
        assert!(tests.contains("test_r1"));
    }

    #[test]
    fn test_generate_typescript() {
        let spec = sample_spec();
        let tests = generate_tests(&spec, Target::TypeScript);

        assert!(tests.contains("describe"));
        assert!(tests.contains("expect"));
    }

    #[test]
    fn test_generate_python() {
        let spec = sample_spec();
        let tests = generate_tests(&spec, Target::Python);

        assert!(tests.contains("def test_"));
        assert!(tests.contains("assert"));
    }
}
