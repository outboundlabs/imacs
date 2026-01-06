//! Python orchestrator test generation

use crate::orchestrate::*;
use crate::spec::Spec;
use std::collections::HashMap;

use super::to_pascal;

pub fn generate_integration_tests(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "# Integration tests for orchestrator: {}\n\nimport pytest\nfrom .{} import {}, {}Input\n\nclass Test{}:\n",
        orch.id, orch.id, orch.id, to_pascal(&orch.id), to_pascal(&orch.id)
    ));

    out.push_str(&format!(
        "    @pytest.mark.asyncio\n    async def test_happy_path(self):\n        input = {}Input()\n        result = await {}(input)\n        assert result is not None\n\n",
        to_pascal(&orch.id), orch.id
    ));

    // Gate tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&format!(
                "    @pytest.mark.asyncio\n    async def test_gate_{}_rejects(self):\n        input = {}Input()\n        with pytest.raises(ValueError):\n            await {}(input)\n\n",
                gate.id, to_pascal(&orch.id), orch.id
            ));
        }
    }

    out
}

pub fn generate_contract_tests(orch: &Orchestrator, _specs: &HashMap<String, Spec>) -> String {
    format!(
        "# Contract tests for orchestrator: {}\n\nimport pytest\n\nclass Test{}Contracts:\n    def test_spec_interfaces_compatible(self):\n        pass\n",
        orch.id, to_pascal(&orch.id)
    )
}
