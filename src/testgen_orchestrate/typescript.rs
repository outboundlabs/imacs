//! TypeScript orchestrator test generation

use crate::orchestrate::*;
use crate::spec::Spec;
use std::collections::HashMap;

pub fn generate_integration_tests(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "// Integration tests for orchestrator: {}\n\nimport {{ {} }} from './{}'\nimport {{ describe, it, expect }} from 'vitest'\n\ndescribe('{}', () => {{\n",
        orch.id, orch.id, orch.id, orch.id
    ));

    out.push_str(&format!(
        "  it('completes happy path', async () => {{\n    const input = {{\n      // TODO: Fill in valid input\n    }}\n    const result = await {}(input)\n    expect(result).toBeDefined()\n  }})\n\n",
        orch.id
    ));

    // Gate tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&format!(
                "  it('gate {} rejects invalid input', async () => {{\n    const input = {{}}\n    await expect({}(input)).rejects.toThrow()\n  }})\n\n",
                gate.id, orch.id
            ));
        }
    }

    out.push_str("})\n");
    out
}

pub fn generate_contract_tests(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    format!(
        "// Contract tests for orchestrator: {}\n\nimport {{ describe, it }} from 'vitest'\n\ndescribe('{} contracts', () => {{\n  it('spec interfaces are compatible', () => {{}})\n}})\n",
        orch.id, orch.id
    )
}
