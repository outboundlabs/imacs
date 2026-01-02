//! Test generation for orchestrators
//!
//! Generates:
//! 1. Unit tests for each spec (existing imacs::testgen)
//! 2. Integration tests for the orchestrator flow
//! 3. Contract tests between specs

mod rust;
mod typescript;
mod python;
mod csharp;
mod java;
mod go;

use crate::cel::Target;
use crate::orchestrate::*;
use crate::spec::Spec;
use crate::testgen::generate_tests;
use std::collections::HashMap;

/// Generate all tests for an orchestrator
pub fn generate_orchestrator_tests(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    target: Target,
) -> OrchestratorTests {
    OrchestratorTests {
        unit_tests: generate_unit_tests(orch, specs, target),
        integration_tests: generate_integration_tests(orch, specs, target),
        contract_tests: generate_contract_tests(orch, specs, target),
    }
}

#[derive(Debug, Clone)]
pub struct OrchestratorTests {
    pub unit_tests: HashMap<String, String>,
    pub integration_tests: String,
    pub contract_tests: String,
}

fn generate_unit_tests(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    target: Target,
) -> HashMap<String, String> {
    let mut tests = HashMap::new();
    for spec_id in orch.referenced_specs() {
        if let Some(spec) = specs.get(&spec_id) {
            tests.insert(spec_id, generate_tests(spec, target));
        }
    }
    tests
}

fn generate_integration_tests(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    target: Target,
) -> String {
    match target {
        Target::Rust => rust::generate_integration_tests(orch, specs),
        Target::TypeScript => typescript::generate_integration_tests(orch, specs),
        Target::Python => python::generate_integration_tests(orch, specs),
        Target::CSharp => csharp::generate_integration_tests(orch, specs),
        Target::Java => java::generate_integration_tests(orch, specs),
        Target::Go => go::generate_integration_tests(orch, specs),
    }
}

fn generate_contract_tests(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    target: Target,
) -> String {
    match target {
        Target::Rust => rust::generate_contract_tests(orch, specs),
        Target::TypeScript => typescript::generate_contract_tests(orch, specs),
        Target::Python => python::generate_contract_tests(orch, specs),
        Target::CSharp => csharp::generate_contract_tests(orch, specs),
        Target::Java => java::generate_contract_tests(orch, specs),
        Target::Go => go::generate_contract_tests(orch, specs),
    }
}

// ============================================================================
// Common utilities
// ============================================================================

pub(crate) fn to_pascal(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

pub(crate) fn find_connections(steps: &[ChainStep]) -> Vec<(String, String, String)> {
    let mut connections = Vec::new();
    let mut previous_spec: Option<String> = None;

    for step in steps {
        if let ChainStep::Call(call) = step {
            if let Some(ref prev) = previous_spec {
                for (_, expr) in &call.inputs {
                    if expr.starts_with(prev) || expr.contains(&format!("{}.", prev)) {
                        connections.push((prev.clone(), call.spec.clone(), expr.clone()));
                    }
                }
            }
            previous_spec = Some(call.id.clone());
        }
    }

    connections
}

// ============================================================================
// Orchestrator verification
// ============================================================================

pub fn verify_orchestrator(
    orch: &Orchestrator,
    specs: &HashMap<String, Spec>,
    implementations: &HashMap<String, String>,
    target: Target,
) -> OrchestratorVerification {
    let mut result = OrchestratorVerification {
        orchestrator_id: orch.id.clone(),
        spec_results: HashMap::new(),
        all_passed: true,
        errors: Vec::new(),
    };

    for spec_id in orch.referenced_specs() {
        let spec = match specs.get(&spec_id) {
            Some(s) => s,
            None => {
                result.errors.push(format!("Missing spec: {}", spec_id));
                result.all_passed = false;
                continue;
            }
        };

        let code = match implementations.get(&spec_id) {
            Some(c) => c,
            None => {
                result.errors.push(format!("Missing implementation: {}", spec_id));
                result.all_passed = false;
                continue;
            }
        };

        let ast = match target {
            Target::Rust => match crate::parse::parse_rust(code) {
                Ok(ast) => ast,
                Err(e) => {
                    result.errors.push(format!("Parse error for {}: {}", spec_id, e));
                    result.all_passed = false;
                    continue;
                }
            },
            _ => continue,
        };

        let verification = crate::verify::verify(spec, &ast);
        if !verification.passed {
            result.all_passed = false;
        }
        result.spec_results.insert(spec_id, verification);
    }

    let validation_errors = orch.validate(specs);
    for error in validation_errors {
        result.errors.push(format!("{}", error));
        result.all_passed = false;
    }

    result
}

#[derive(Debug, Clone)]
pub struct OrchestratorVerification {
    pub orchestrator_id: String,
    pub spec_results: HashMap<String, crate::verify::VerificationResult>,
    pub all_passed: bool,
    pub errors: Vec<String>,
}

impl OrchestratorVerification {
    pub fn to_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("# Orchestrator Verification: {}\n\n", self.orchestrator_id));

        if self.all_passed {
            report.push_str("ALL CHECKS PASSED\n\n");
        } else {
            report.push_str("VERIFICATION FAILED\n\n");
        }

        report.push_str("## Spec Verification\n\n");
        report.push_str("| Spec | Status | Coverage |\n|------|--------|----------|\n");

        for (spec_id, result) in &self.spec_results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            report.push_str(&format!("| {} | {} | {:.0}% |\n", spec_id, status, result.coverage.percentage));
        }

        if !self.errors.is_empty() {
            report.push_str("\n## Errors\n\n");
            for error in &self.errors {
                report.push_str(&format!("- {}\n", error));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_integration_tests() {
        let orch = Orchestrator::from_yaml(r#"
id: test_flow
inputs:
  - name: value
    type: int
outputs:
  - name: result
    type: int
uses:
  - step_one
chain:
  - step: call
    id: first
    spec: step_one
    inputs:
      x: "value"
  - step: gate
    id: check
    condition: "first.result > 0"
"#).unwrap();

        let specs = HashMap::new();
        let tests = generate_integration_tests(&orch, &specs, Target::Rust);
        assert!(tests.contains("test_test_flow_happy_path"));
    }
}
