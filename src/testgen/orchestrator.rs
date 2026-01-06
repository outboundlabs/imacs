//! Orchestrator test generation
//!
//! Generates atomic declarative behavioral tests for orchestrators:
//! - Happy path tests (all gates pass, steps execute)
//! - Gate failure tests (each gate checked individually)
//! - Step execution verification

use crate::cel::Target;
use crate::orchestrate::{ChainStep, Orchestrator};
use crate::spec::VarType;
use chrono::Utc;

use super::{to_camel_case, to_pascal_case};

/// Generate orchestrator tests for target language
pub fn generate_orchestrator_tests(orch: &Orchestrator, target: Target) -> String {
    match target {
        Target::CSharp => generate_csharp(orch),
        Target::Rust => generate_rust(orch),
        Target::TypeScript => generate_typescript(orch),
        Target::Python => generate_python(orch),
        Target::Go => generate_go(orch),
        Target::Java => generate_java(orch),
    }
}

// ============================================================================
// C# Test Generation (xUnit + FluentAssertions)
// ============================================================================

fn generate_csharp(orch: &Orchestrator) -> String {
    let mut out = String::new();
    let class_name = to_pascal_case(&orch.id);

    // Header
    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", orch.id));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    // Usings
    out.push_str("using FluentAssertions;\n");
    out.push_str("using Xunit;\n\n");

    // Test class
    out.push_str(&format!("public class {}Tests\n{{\n", class_name));

    // Generate happy path test
    out.push_str(&generate_csharp_happy_path(orch));

    // Generate gate failure tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&generate_csharp_gate_failure_test(
                orch,
                &gate.id,
                &gate.condition,
            ));
        }
    }

    // Generate step execution tests
    for step in &orch.chain {
        if let ChainStep::Call(call) = step {
            out.push_str(&generate_csharp_step_execution_test(
                orch, &call.id, &call.spec,
            ));
        }
    }

    out.push_str("}\n");
    out
}

fn generate_csharp_happy_path(orch: &Orchestrator) -> String {
    let class_name = to_pascal_case(&orch.id);
    let mut out = String::new();

    out.push_str("    /// <summary>\n");
    out.push_str("    /// Happy path: all gates pass, orchestrator completes successfully\n");
    out.push_str("    /// </summary>\n");
    out.push_str("    [Fact]\n");
    out.push_str("    public void Execute_WithValidInputs_ShouldSucceed()\n");
    out.push_str("    {\n");
    out.push_str("        // Arrange\n");
    out.push_str(&format!("        var input = new {}Input\n", class_name));
    out.push_str("        {\n");

    for input in &orch.inputs {
        let value = csharp_sample_value(&input.var_type);
        out.push_str(&format!(
            "            {} = {},\n",
            to_pascal_case(&input.name),
            value
        ));
    }

    out.push_str("        };\n\n");
    out.push_str("        // Act\n");
    out.push_str(&format!(
        "        var act = () => {}Orchestrator.Execute(input);\n\n",
        class_name
    ));
    out.push_str("        // Assert\n");
    out.push_str("        act.Should().NotThrow();\n");
    out.push_str("    }\n\n");

    out
}

fn generate_csharp_gate_failure_test(
    orch: &Orchestrator,
    gate_id: &str,
    condition: &str,
) -> String {
    let class_name = to_pascal_case(&orch.id);
    let test_name = to_pascal_case(gate_id);
    let mut out = String::new();

    out.push_str("    /// <summary>\n");
    out.push_str(&format!(
        "    /// Gate '{}' should reject when condition '{}' is false\n",
        gate_id, condition
    ));
    out.push_str("    /// </summary>\n");
    out.push_str("    [Fact]\n");
    out.push_str(&format!(
        "    public void Execute_WhenGate{}_Fails_ShouldThrow()\n",
        test_name
    ));
    out.push_str("    {\n");
    out.push_str("        // Arrange - inputs designed to fail this gate\n");
    out.push_str(&format!("        var input = new {}Input\n", class_name));
    out.push_str("        {\n");

    for input in &orch.inputs {
        let value = csharp_default_value(&input.var_type);
        out.push_str(&format!(
            "            {} = {},\n",
            to_pascal_case(&input.name),
            value
        ));
    }

    out.push_str("        };\n\n");
    out.push_str("        // Act\n");
    out.push_str(&format!(
        "        var act = () => {}Orchestrator.Execute(input);\n\n",
        class_name
    ));
    out.push_str("        // Assert\n");
    out.push_str(&format!(
        "        act.Should().Throw<{}Exception>()\n",
        class_name
    ));
    out.push_str(&format!(
        "            .Where(e => e.Step == \"{}\")\n",
        gate_id
    ));
    out.push_str("            .Where(e => e.ErrorType == \"gate_failed\");\n");
    out.push_str("    }\n\n");

    out
}

fn generate_csharp_step_execution_test(
    orch: &Orchestrator,
    step_id: &str,
    spec_id: &str,
) -> String {
    let class_name = to_pascal_case(&orch.id);
    let test_name = to_pascal_case(step_id);
    let spec_name = to_pascal_case(spec_id);
    let mut out = String::new();

    out.push_str("    /// <summary>\n");
    out.push_str(&format!(
        "    /// Step '{}' should call spec '{}'\n",
        step_id, spec_id
    ));
    out.push_str("    /// </summary>\n");
    out.push_str("    [Fact]\n");
    out.push_str(&format!(
        "    public void Execute_Step{}_{}_ShouldBeInvoked()\n",
        test_name, spec_name
    ));
    out.push_str("    {\n");
    out.push_str("        // Arrange\n");
    out.push_str(&format!("        var input = new {}Input\n", class_name));
    out.push_str("        {\n");

    for input in &orch.inputs {
        let value = csharp_sample_value(&input.var_type);
        out.push_str(&format!(
            "            {} = {},\n",
            to_pascal_case(&input.name),
            value
        ));
    }

    out.push_str("        };\n\n");
    out.push_str("        // Act\n");
    out.push_str(&format!(
        "        var result = {}Orchestrator.Execute(input);\n\n",
        class_name
    ));
    out.push_str("        // Assert - step was executed (context populated)\n");
    out.push_str("        result.Should().NotBeNull();\n");
    out.push_str("    }\n\n");

    out
}

fn csharp_sample_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "true".into(),
        VarType::Int => "100".into(),
        VarType::Float => "10.0".into(),
        VarType::String => "\"test\"".into(),
        VarType::Enum(variants) => variants
            .first()
            .map(|v| format!("\"{}\"", v))
            .unwrap_or("\"\"".into()),
        _ => "default".into(),
    }
}

fn csharp_default_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "false".into(),
        VarType::Int => "0".into(),
        VarType::Float => "0.0".into(),
        VarType::String => "\"\"".into(),
        _ => "default".into(),
    }
}

// ============================================================================
// Rust Test Generation
// ============================================================================

fn generate_rust(orch: &Orchestrator) -> String {
    let mut out = String::new();
    let fn_name = &orch.id;
    let struct_name = to_pascal_case(&orch.id);

    // Header
    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", orch.id));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("#[cfg(test)]\n");
    out.push_str(&format!("mod {}_tests {{\n", orch.id));
    out.push_str("    use super::*;\n\n");

    // Happy path test
    out.push_str("    /// Happy path: all gates pass\n");
    out.push_str("    #[test]\n");
    out.push_str("    fn test_happy_path() {\n");
    out.push_str(&format!("        let input = {}Input {{\n", struct_name));
    for input in &orch.inputs {
        let value = rust_sample_value(&input.var_type);
        out.push_str(&format!("            {}: {},\n", input.name, value));
    }
    out.push_str("        };\n\n");
    out.push_str(&format!("        let result = {}(input);\n", fn_name));
    out.push_str("        assert!(result.is_ok());\n");
    out.push_str("    }\n\n");

    // Gate failure tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&format!(
                "    /// Gate '{}' should reject invalid inputs\n",
                gate.id
            ));
            out.push_str("    #[test]\n");
            out.push_str(&format!("    fn test_gate_{}_fails() {{\n", gate.id));
            out.push_str(&format!("        let input = {}Input {{\n", struct_name));
            for input in &orch.inputs {
                let value = rust_default_value(&input.var_type);
                out.push_str(&format!("            {}: {},\n", input.name, value));
            }
            out.push_str("        };\n\n");
            out.push_str(&format!("        let result = {}(input);\n", fn_name));
            out.push_str("        assert!(result.is_err());\n");
            out.push_str("        let err = result.unwrap_err();\n");
            out.push_str(&format!("        assert_eq!(err.step, \"{}\");\n", gate.id));
            out.push_str("        assert_eq!(err.error_type, \"gate_failed\");\n");
            out.push_str("    }\n\n");
        }
    }

    out.push_str("}\n");
    out
}

fn rust_sample_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "true".into(),
        VarType::Int => "100".into(),
        VarType::Float => "10.0".into(),
        VarType::String => "\"test\".to_string()".into(),
        VarType::Enum(variants) => variants
            .first()
            .map(|v| format!("\"{}\".to_string()", v))
            .unwrap_or("String::new()".into()),
        _ => "Default::default()".into(),
    }
}

fn rust_default_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "false".into(),
        VarType::Int => "0".into(),
        VarType::Float => "0.0".into(),
        VarType::String => "String::new()".into(),
        _ => "Default::default()".into(),
    }
}

// ============================================================================
// TypeScript Test Generation (Vitest)
// ============================================================================

fn generate_typescript(orch: &Orchestrator) -> String {
    let mut out = String::new();
    let fn_name = to_camel_case(&orch.id);

    // Header
    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", orch.id));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("import { describe, it, expect } from 'vitest';\n");
    out.push_str(&format!(
        "import {{ {}, {}Error }} from './{}';\n\n",
        fn_name,
        to_pascal_case(&orch.id),
        orch.id
    ));

    out.push_str(&format!("describe('{}', () => {{\n", fn_name));

    // Happy path
    out.push_str("  it('should succeed with valid inputs', async () => {\n");
    out.push_str("    const input = {\n");
    for input in &orch.inputs {
        let value = ts_sample_value(&input.var_type);
        out.push_str(&format!(
            "      {}: {},\n",
            to_camel_case(&input.name),
            value
        ));
    }
    out.push_str("    };\n\n");
    out.push_str(&format!(
        "    await expect({}(input)).resolves.toBeDefined();\n",
        fn_name
    ));
    out.push_str("  });\n\n");

    // Gate failure tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&format!(
                "  it('should throw when gate {} fails', async () => {{\n",
                gate.id
            ));
            out.push_str("    const input = {\n");
            for input in &orch.inputs {
                let value = ts_default_value(&input.var_type);
                out.push_str(&format!(
                    "      {}: {},\n",
                    to_camel_case(&input.name),
                    value
                ));
            }
            out.push_str("    };\n\n");
            out.push_str(&format!(
                "    await expect({}(input)).rejects.toThrow();\n",
                fn_name
            ));
            out.push_str("  });\n\n");
        }
    }

    out.push_str("});\n");
    out
}

fn ts_sample_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "true".into(),
        VarType::Int => "100".into(),
        VarType::Float => "10.0".into(),
        VarType::String => "'test'".into(),
        VarType::Enum(variants) => variants
            .first()
            .map(|v| format!("'{}'", v))
            .unwrap_or("''".into()),
        _ => "null".into(),
    }
}

fn ts_default_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "false".into(),
        VarType::Int => "0".into(),
        VarType::Float => "0.0".into(),
        VarType::String => "''".into(),
        _ => "null".into(),
    }
}

// ============================================================================
// Python Test Generation (pytest)
// ============================================================================

fn generate_python(orch: &Orchestrator) -> String {
    let mut out = String::new();
    let fn_name = &orch.id;
    let class_name = to_pascal_case(&orch.id);

    // Header
    out.push_str(&format!("# GENERATED TESTS FROM: {}.yaml\n", orch.id));
    out.push_str(&format!("# GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("# DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("import pytest\n");
    out.push_str(&format!(
        "from {} import {}, {}Input, {}Error\n\n",
        orch.id, fn_name, class_name, class_name
    ));

    // Happy path
    out.push_str(&format!("class Test{}:\n", class_name));
    out.push_str("    \"\"\"Tests for orchestrator happy path and gate failures\"\"\"\n\n");

    out.push_str("    def test_happy_path(self):\n");
    out.push_str("        \"\"\"All gates pass, orchestrator completes\"\"\"\n");
    out.push_str(&format!("        input_data = {}Input(\n", class_name));
    for input in &orch.inputs {
        let value = python_sample_value(&input.var_type);
        out.push_str(&format!("            {}={},\n", input.name, value));
    }
    out.push_str("        )\n\n");
    out.push_str(&format!("        result = {}(input_data)\n", fn_name));
    out.push_str("        assert result is not None\n\n");

    // Gate failure tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&format!("    def test_gate_{}_fails(self):\n", gate.id));
            out.push_str(&format!(
                "        \"\"\"Gate '{}' rejects invalid inputs\"\"\"\n",
                gate.id
            ));
            out.push_str(&format!("        input_data = {}Input(\n", class_name));
            for input in &orch.inputs {
                let value = python_default_value(&input.var_type);
                out.push_str(&format!("            {}={},\n", input.name, value));
            }
            out.push_str("        )\n\n");
            out.push_str(&format!(
                "        with pytest.raises({}Error) as exc:\n",
                class_name
            ));
            out.push_str(&format!("            {}(input_data)\n", fn_name));
            out.push_str(&format!(
                "        assert exc.value.step == \"{}\"\n",
                gate.id
            ));
            out.push_str("        assert exc.value.error_type == \"gate_failed\"\n\n");
        }
    }

    out
}

fn python_sample_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "True".into(),
        VarType::Int => "100".into(),
        VarType::Float => "10.0".into(),
        VarType::String => "\"test\"".into(),
        VarType::Enum(variants) => variants
            .first()
            .map(|v| format!("\"{}\"", v))
            .unwrap_or("\"\"".into()),
        _ => "None".into(),
    }
}

fn python_default_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "False".into(),
        VarType::Int => "0".into(),
        VarType::Float => "0.0".into(),
        VarType::String => "\"\"".into(),
        _ => "None".into(),
    }
}

// ============================================================================
// Go Test Generation
// ============================================================================

fn generate_go(orch: &Orchestrator) -> String {
    let mut out = String::new();
    let fn_name = to_pascal_case(&orch.id);

    // Header
    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", orch.id));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("package main\n\n");
    out.push_str("import (\n");
    out.push_str("\t\"testing\"\n");
    out.push_str(")\n\n");

    // Happy path
    out.push_str(&format!(
        "func Test{}_HappyPath(t *testing.T) {{\n",
        fn_name
    ));
    out.push_str(&format!("\tinput := {}Input{{\n", fn_name));
    for input in &orch.inputs {
        let value = go_sample_value(&input.var_type);
        out.push_str(&format!(
            "\t\t{}: {},\n",
            to_pascal_case(&input.name),
            value
        ));
    }
    out.push_str("\t}\n\n");
    out.push_str(&format!("\t_, err := {}(input)\n", fn_name));
    out.push_str("\tif err != nil {\n");
    out.push_str("\t\tt.Errorf(\"expected success, got error: %v\", err)\n");
    out.push_str("\t}\n");
    out.push_str("}\n\n");

    // Gate failure tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str(&format!(
                "func Test{}_Gate_{}_Fails(t *testing.T) {{\n",
                fn_name,
                to_pascal_case(&gate.id)
            ));
            out.push_str(&format!("\tinput := {}Input{{\n", fn_name));
            for input in &orch.inputs {
                let value = go_default_value(&input.var_type);
                out.push_str(&format!(
                    "\t\t{}: {},\n",
                    to_pascal_case(&input.name),
                    value
                ));
            }
            out.push_str("\t}\n\n");
            out.push_str(&format!("\t_, err := {}(input)\n", fn_name));
            out.push_str("\tif err == nil {\n");
            out.push_str("\t\tt.Error(\"expected error, got success\")\n");
            out.push_str("\t}\n");
            out.push_str(&format!(
                "\tif orchErr, ok := err.({}Error); ok {{\n",
                fn_name
            ));
            out.push_str(&format!("\t\tif orchErr.Step != \"{}\" {{\n", gate.id));
            out.push_str(&format!(
                "\t\t\tt.Errorf(\"expected step '{}', got '%s'\", orchErr.Step)\n",
                gate.id
            ));
            out.push_str("\t\t}\n");
            out.push_str("\t}\n");
            out.push_str("}\n\n");
        }
    }

    out
}

fn go_sample_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "true".into(),
        VarType::Int => "100".into(),
        VarType::Float => "10.0".into(),
        VarType::String => "\"test\"".into(),
        VarType::Enum(variants) => variants
            .first()
            .map(|v| format!("\"{}\"", v))
            .unwrap_or("\"\"".into()),
        _ => "nil".into(),
    }
}

fn go_default_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "false".into(),
        VarType::Int => "0".into(),
        VarType::Float => "0.0".into(),
        VarType::String => "\"\"".into(),
        _ => "nil".into(),
    }
}

// ============================================================================
// Java Test Generation (JUnit 5)
// ============================================================================

fn generate_java(orch: &Orchestrator) -> String {
    let mut out = String::new();
    let class_name = to_pascal_case(&orch.id);

    // Header
    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", orch.id));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("import org.junit.jupiter.api.Test;\n");
    out.push_str("import static org.junit.jupiter.api.Assertions.*;\n\n");

    out.push_str(&format!("class {}Tests {{\n\n", class_name));

    // Happy path
    out.push_str("    @Test\n");
    out.push_str("    void execute_withValidInputs_shouldSucceed() {\n");
    out.push_str(&format!(
        "        var input = new {}Orchestrator.Input(\n",
        class_name
    ));
    let input_values: Vec<String> = orch
        .inputs
        .iter()
        .map(|i| java_sample_value(&i.var_type))
        .collect();
    out.push_str(&format!(
        "            {}\n",
        input_values.join(",\n            ")
    ));
    out.push_str("        );\n\n");
    out.push_str(&format!(
        "        assertDoesNotThrow(() -> {}Orchestrator.execute(input));\n",
        class_name
    ));
    out.push_str("    }\n\n");

    // Gate failure tests
    for step in &orch.chain {
        if let ChainStep::Gate(gate) = step {
            out.push_str("    @Test\n");
            out.push_str(&format!(
                "    void execute_whenGate{}_fails_shouldThrow() {{\n",
                to_pascal_case(&gate.id)
            ));
            out.push_str(&format!(
                "        var input = new {}Orchestrator.Input(\n",
                class_name
            ));
            let default_values: Vec<String> = orch
                .inputs
                .iter()
                .map(|i| java_default_value(&i.var_type))
                .collect();
            out.push_str(&format!(
                "            {}\n",
                default_values.join(",\n            ")
            ));
            out.push_str("        );\n\n");
            out.push_str(&format!(
                "        var ex = assertThrows({}Orchestrator.{}Exception.class, () -> {{\n",
                class_name, class_name
            ));
            out.push_str(&format!(
                "            {}Orchestrator.execute(input);\n",
                class_name
            ));
            out.push_str("        });\n");
            out.push_str(&format!(
                "        assertEquals(\"{}\", ex.step);\n",
                gate.id
            ));
            out.push_str("        assertEquals(\"gate_failed\", ex.type);\n");
            out.push_str("    }\n\n");
        }
    }

    out.push_str("}\n");
    out
}

fn java_sample_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "true".into(),
        VarType::Int => "100".into(),
        VarType::Float => "10.0".into(),
        VarType::String => "\"test\"".into(),
        VarType::Enum(variants) => variants
            .first()
            .map(|v| format!("\"{}\"", v))
            .unwrap_or("\"\"".into()),
        _ => "null".into(),
    }
}

fn java_default_value(typ: &VarType) -> String {
    match typ {
        VarType::Bool => "false".into(),
        VarType::Int => "0".into(),
        VarType::Float => "0.0".into(),
        VarType::String => "\"\"".into(),
        _ => "null".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_orchestrator() -> Orchestrator {
        Orchestrator::from_yaml(
            r#"
id: order_flow
inputs:
  - name: role
    type: string
  - name: verified
    type: bool
  - name: amount
    type: float
outputs:
  - name: approved
    type: bool
uses:
  - access_level
chain:
  - step: call
    id: check_access
    spec: access_level
    inputs:
      role: "role"
      verified: "verified"
  - step: gate
    id: require_access
    condition: "check_access.level >= 50"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_generate_csharp_orchestrator_tests() {
        let orch = sample_orchestrator();
        let tests = generate_csharp(&orch);

        assert!(tests.contains("FluentAssertions"));
        assert!(tests.contains("[Fact]"));
        assert!(tests.contains("Execute_WithValidInputs_ShouldSucceed"));
        assert!(tests.contains("Execute_WhenGateRequireAccess_Fails_ShouldThrow"));
        assert!(tests.contains(".Should().Throw"));
    }

    #[test]
    fn test_generate_rust_orchestrator_tests() {
        let orch = sample_orchestrator();
        let tests = generate_rust(&orch);

        assert!(tests.contains("#[test]"));
        assert!(tests.contains("test_happy_path"));
        assert!(tests.contains("test_gate_require_access_fails"));
    }

    #[test]
    fn test_generate_typescript_orchestrator_tests() {
        let orch = sample_orchestrator();
        let tests = generate_typescript(&orch);

        assert!(tests.contains("describe"));
        assert!(tests.contains("expect"));
        assert!(tests.contains("should succeed with valid inputs"));
    }

    #[test]
    fn test_generate_python_orchestrator_tests() {
        let orch = sample_orchestrator();
        let tests = generate_python(&orch);

        assert!(tests.contains("import pytest"));
        assert!(tests.contains("def test_happy_path"));
        assert!(tests.contains("def test_gate_require_access_fails"));
    }
}
