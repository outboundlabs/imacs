//! Orchestrator-aware suite analysis
//!
//! When analyzing a suite, if orchestrators are present, automatically:
//! 1. Discover all specs referenced by orchestrators
//! 2. Load and analyze those specs together
//! 3. Check for issues across the entire orchestrated workflow

use crate::completeness::suite::{analyze_suite, SuiteAnalysisResult};
use crate::orchestrate::Orchestrator;
use crate::spec::Spec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of analyzing an orchestrator and its referenced specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrchestratorSuiteResult {
    /// The orchestrator that was analyzed
    pub orchestrator_id: String,
    /// All specs referenced by this orchestrator
    pub referenced_spec_ids: Vec<String>,
    /// Specs that were found and analyzed
    pub found_specs: Vec<String>,
    /// Specs that were referenced but not found
    pub missing_specs: Vec<String>,
    /// Suite analysis result for all found specs
    pub suite_result: SuiteAnalysisResult,
    /// Input/output mapping issues
    pub mapping_issues: Vec<MappingIssue>,
}

/// An issue with input/output mapping between orchestrator and specs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MappingIssue {
    pub step_id: String,
    pub spec_id: String,
    pub issue_type: MappingIssueType,
    pub details: String,
}

/// Type of mapping issue
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum MappingIssueType {
    /// Required input not provided
    MissingInput,
    /// Output type mismatch
    OutputTypeMismatch,
    /// Input type mismatch
    InputTypeMismatch,
    /// Output not mapped (unused)
    UnusedOutput,
}

/// Analyze an orchestrator and all its referenced specs
pub fn analyze_orchestrator_suite(
    orchestrator: &Orchestrator,
    available_specs: &HashMap<String, Spec>,
    full: bool,
) -> OrchestratorSuiteResult {
    // 1. Get all referenced spec IDs
    let referenced_spec_ids = orchestrator.referenced_specs();

    // 2. Load available specs
    let mut found_specs = Vec::new();
    let mut missing_specs = Vec::new();
    let mut specs_to_analyze = Vec::new();

    for spec_id in &referenced_spec_ids {
        if let Some(spec) = available_specs.get(spec_id) {
            found_specs.push(spec_id.clone());
            specs_to_analyze.push((spec_id.clone(), spec.clone()));
        } else {
            missing_specs.push(spec_id.clone());
        }
    }

    // 3. Analyze the suite of referenced specs
    let suite_result = analyze_suite(&specs_to_analyze, full);

    // 4. Check input/output mappings
    let mapping_issues = check_mappings(orchestrator, available_specs);

    OrchestratorSuiteResult {
        orchestrator_id: orchestrator.id.clone(),
        referenced_spec_ids,
        found_specs,
        missing_specs,
        suite_result,
        mapping_issues,
    }
}

/// Check input/output mappings between orchestrator and specs
fn check_mappings(orchestrator: &Orchestrator, specs: &HashMap<String, Spec>) -> Vec<MappingIssue> {
    let mut issues = Vec::new();

    // Walk through the chain and check each CallStep
    check_chain_mappings(&orchestrator.chain, specs, &mut issues);

    issues
}

fn check_chain_mappings(
    steps: &[crate::orchestrate::ChainStep],
    specs: &HashMap<String, Spec>,
    issues: &mut Vec<MappingIssue>,
) {
    use crate::orchestrate::ChainStep;

    for step in steps {
        match step {
            ChainStep::Call(call) => {
                if let Some(spec) = specs.get(&call.spec) {
                    // Check all required inputs are provided
                    for input in &spec.inputs {
                        if !call.inputs.contains_key(&input.name) {
                            issues.push(MappingIssue {
                                step_id: call.id.clone(),
                                spec_id: call.spec.clone(),
                                issue_type: MappingIssueType::MissingInput,
                                details: format!(
                                    "Required input '{}' (type: {:?}) not provided",
                                    input.name, input.typ
                                ),
                            });
                        } else {
                            // TODO: Check type compatibility of provided input
                        }
                    }

                    // Check outputs are mapped (warn about unused outputs)
                    for output in &spec.outputs {
                        if !call.outputs.values().any(|v| v == &output.name) {
                            issues.push(MappingIssue {
                                step_id: call.id.clone(),
                                spec_id: call.spec.clone(),
                                issue_type: MappingIssueType::UnusedOutput,
                                details: format!(
                                    "Output '{}' (type: {:?}) is not mapped to any variable",
                                    output.name, output.typ
                                ),
                            });
                        }
                    }
                }
            }
            ChainStep::Parallel(par) => {
                check_chain_mappings(&par.steps, specs, issues);
            }
            ChainStep::Branch(branch) => {
                for steps in branch.cases.values() {
                    check_chain_mappings(steps, specs, issues);
                }
                if let Some(default) = &branch.default {
                    check_chain_mappings(default, specs, issues);
                }
            }
            ChainStep::Loop(loop_) => {
                check_chain_mappings(&loop_.steps, specs, issues);
            }
            ChainStep::ForEach(foreach) => {
                check_chain_mappings(&foreach.steps, specs, issues);
            }
            ChainStep::Try(try_) => {
                check_chain_mappings(&try_.try_steps, specs, issues);
                if let Some(catch) = &try_.catch {
                    check_chain_mappings(&catch.steps, specs, issues);
                }
                if let Some(finally) = &try_.finally {
                    check_chain_mappings(finally, specs, issues);
                }
            }
            _ => {}
        }
    }
}

/// Analyze a directory that may contain both specs and orchestrators
pub fn analyze_directory_with_orchestrators(
    dir_path: &str,
    full: bool,
) -> Result<DirectorySuiteResult, String> {
    use std::fs;

    let mut specs = HashMap::new();
    let mut orchestrators = Vec::new();

    // Load all YAML files
    let dir = fs::read_dir(dir_path).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in dir {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

            // Try parsing as orchestrator first (has 'chain:' or 'uses:')
            if content.contains("\nchain:") || content.contains("\nuses:") {
                match Orchestrator::from_yaml(&content) {
                    Ok(orch) => {
                        let id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        orchestrators.push((id, orch));
                    }
                    Err(_) => {
                        // Try as spec
                        if let Ok(spec) = Spec::from_yaml(&content) {
                            specs.insert(spec.id.clone(), spec);
                        }
                    }
                }
            } else {
                // Try as spec
                if let Ok(spec) = Spec::from_yaml(&content) {
                    specs.insert(spec.id.clone(), spec);
                }
            }
        }
    }

    // Analyze each orchestrator and its referenced specs
    let mut orchestrator_results = Vec::new();
    for (_orch_id, orch) in &orchestrators {
        let result = analyze_orchestrator_suite(orch, &specs, full);
        orchestrator_results.push(result);
    }

    // Also analyze all specs together as a suite
    let all_specs: Vec<_> = specs
        .iter()
        .map(|(id, spec)| (id.clone(), spec.clone()))
        .collect();
    let suite_result = analyze_suite(&all_specs, full);

    Ok(DirectorySuiteResult {
        specs_found: specs.len(),
        orchestrators_found: orchestrators.len(),
        orchestrator_results,
        overall_suite_result: suite_result,
    })
}

/// Result of analyzing a directory
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DirectorySuiteResult {
    pub specs_found: usize,
    pub orchestrators_found: usize,
    pub orchestrator_results: Vec<OrchestratorSuiteResult>,
    pub overall_suite_result: SuiteAnalysisResult,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrate::{CallStep, ChainStep};
    use std::collections::HashMap;

    #[test]
    fn test_analyze_orchestrator_suite() {
        // Create an orchestrator that references two specs
        let mut orch = Orchestrator {
            id: "test_flow".into(),
            name: None,
            description: None,
            inputs: vec![],
            outputs: vec![],
            uses: vec!["spec_a".into(), "spec_b".into()],
            chain: vec![ChainStep::Call(CallStep {
                id: "step1".into(),
                spec: "spec_a".into(),
                inputs: HashMap::new(),
                outputs: HashMap::new(),
                condition: None,
                timeout: None,
                retry: None,
            })],
        };

        // Create the referenced specs
        let mut specs = HashMap::new();
        specs.insert(
            "spec_a".into(),
            Spec {
                id: "spec_a".into(),
                name: None,
                description: None,
                inputs: vec![],
                outputs: vec![],
                rules: vec![],
                default: None,
                meta: Default::default(),
            },
        );

        let result = analyze_orchestrator_suite(&orch, &specs, false);

        assert_eq!(result.orchestrator_id, "test_flow");
        assert_eq!(result.referenced_spec_ids.len(), 2);
        assert_eq!(result.found_specs.len(), 1); // Only spec_a is found
        assert_eq!(result.missing_specs.len(), 1); // spec_b is missing
    }
}
