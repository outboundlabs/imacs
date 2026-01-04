//! Test generation — create tests from specs
//!
//! Generates comprehensive test suites from decision tables:
//! - One test per rule (basic coverage)
//! - Exhaustive tests (all input combinations)
//! - Boundary tests (edge cases for numeric conditions)
//! - Property tests (fuzzing)

mod csharp;
mod go;
mod java;
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

pub(crate) fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

pub(crate) fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

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
