//! C# test generation (xUnit)

use crate::spec::*;
use chrono::Utc;

use super::{can_enumerate, generate_combinations, to_camel_case, to_pascal_case, TestConfig};

pub fn generate(spec: &Spec, config: &TestConfig) -> String {
    CSharpTestGen { config }.generate(spec)
}

struct CSharpTestGen<'a> {
    config: &'a TestConfig,
}

impl<'a> CSharpTestGen<'a> {
    fn generate(&self, spec: &Spec) -> String {
        let mut out = String::new();
        let class_name = to_pascal_case(&spec.id);

        out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", spec.id));
        out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
        out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
        out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

        out.push_str("using Xunit;\n\n");
        out.push_str(&format!("public class {}Tests\n{{\n", class_name));

        for rule in &spec.rules {
            let test_name = format!("Test_{}", to_pascal_case(&rule.id));
            let inputs = self.generate_input_object(spec, rule);
            let expected = self.csharp_value(&rule.then);

            out.push_str("    [Fact]\n");
            out.push_str(&format!("    public void {}()\n    {{\n", test_name));
            out.push_str(&format!(
                "        // {}: {} → {}\n",
                rule.id,
                rule.as_cel().unwrap_or_default(),
                rule.then
            ));
            out.push_str(&format!(
                "        Assert.Equal({}, {}.Evaluate({}));\n",
                expected, class_name, inputs
            ));
            out.push_str("    }\n\n");
        }

        if self.config.exhaustive && can_enumerate(spec) {
            out.push_str("    // Exhaustive tests\n    [Theory]\n");
            for (inputs, rule_id, expected) in generate_combinations(spec) {
                let input_vals = inputs
                    .iter()
                    .map(|i| self.to_csharp_value(i))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "    [InlineData({}, {})] // {}\n",
                    input_vals, expected, rule_id
                ));
            }

            let params = spec
                .inputs
                .iter()
                .map(|i| format!("{} {}", self.csharp_type(&i.typ), to_camel_case(&i.name)))
                .collect::<Vec<_>>()
                .join(", ");
            let expected_type = spec
                .outputs
                .first()
                .map(|o| self.csharp_type(&o.typ))
                .unwrap_or_else(|| "object".into());

            out.push_str(&format!(
                "    public void TestExhaustive({}, {} expected)\n    {{\n",
                params, expected_type
            ));
            out.push_str(&format!(
                "        var input = new {}Input {{ {} }};\n",
                class_name,
                spec.inputs
                    .iter()
                    .map(|i| format!("{} = {}", to_pascal_case(&i.name), to_camel_case(&i.name)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            out.push_str(&format!(
                "        Assert.Equal(expected, {}.Evaluate(input));\n    }}\n",
                class_name
            ));
        }

        out.push_str("}\n");
        out
    }

    fn generate_input_object(&self, spec: &Spec, rule: &Rule) -> String {
        let fields: Vec<String> = spec
            .inputs
            .iter()
            .map(|input| {
                let value = rule
                    .conditions
                    .as_ref()
                    .and_then(|c| c.iter().find(|cond| cond.var == input.name))
                    .map(|c| self.csharp_condition_value(&c.value))
                    .unwrap_or_else(|| self.default_value(&input.typ));
                format!("{} = {}", to_pascal_case(&input.name), value)
            })
            .collect();
        format!(
            "new {}Input {{ {} }}",
            to_pascal_case(&spec.id),
            fields.join(", ")
        )
    }

    fn to_csharp_value(&self, v: &str) -> String {
        v.to_string()
    }

    fn csharp_value(&self, output: &Output) -> String {
        match output {
            Output::Single(v) => self.csharp_condition_value(v),
            Output::Named(_) => "new { }".into(),
        }
    }

    fn csharp_condition_value(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Int(i) => format!("{}L", i),
            ConditionValue::Float(f) => format!("{}d", f),
            ConditionValue::String(s) => format!("\"{}\"", s),
            ConditionValue::Null => "null".into(),
            _ => "default".into(),
        }
    }

    fn csharp_type(&self, typ: &VarType) -> String {
        match typ {
            VarType::Bool => "bool".into(),
            VarType::Int => "long".into(),
            VarType::Float => "double".into(),
            VarType::String => "string".into(),
            _ => "object".into(),
        }
    }

    fn default_value(&self, typ: &VarType) -> String {
        match typ {
            VarType::Bool => "false".into(),
            VarType::Int => "0L".into(),
            VarType::Float => "0.0d".into(),
            VarType::String => "\"\"".into(),
            _ => "default".into(),
        }
    }
}
