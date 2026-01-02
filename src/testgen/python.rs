//! Python test generation (pytest)

use crate::spec::*;
use chrono::Utc;

use super::{can_enumerate, generate_combinations, to_pascal_case, TestConfig};

pub fn generate(spec: &Spec, config: &TestConfig) -> String {
    PyTestGen { config }.generate(spec)
}

struct PyTestGen<'a> {
    config: &'a TestConfig,
}

impl<'a> PyTestGen<'a> {
    fn generate(&self, spec: &Spec) -> String {
        let mut out = String::new();

        out.push_str(&format!("# GENERATED TESTS FROM: {}.yaml\n", spec.id));
        out.push_str(&format!("# SPEC HASH: {}\n", spec.hash()));
        out.push_str(&format!("# GENERATED: {}\n", Utc::now().to_rfc3339()));
        out.push_str("# DO NOT EDIT — regenerate from spec\n\n");

        out.push_str("import pytest\n");
        out.push_str(&format!("from {} import {}\n\n", spec.id, spec.id));

        out.push_str(&format!("class Test{}Rules:\n", to_pascal_case(&spec.id)));
        out.push_str("    \"\"\"One test per rule\"\"\"\n\n");

        for rule in &spec.rules {
            let test_name = format!("test_{}", rule.id.to_lowercase());
            let inputs = self.generate_inputs(spec, rule);
            let expected = self.python_value(&rule.then);

            out.push_str(&format!("    def {}(self):\n", test_name));
            out.push_str(&format!("        # {}: {} → {}\n", rule.id, rule.as_cel().unwrap_or_default(), rule.then));
            out.push_str(&format!("        assert {}({}) == {}\n\n", spec.id, inputs, expected));
        }

        if self.config.exhaustive && can_enumerate(spec) {
            out.push_str(&format!("\nclass Test{}Exhaustive:\n", to_pascal_case(&spec.id)));
            out.push_str("    \"\"\"All input combinations\"\"\"\n\n");

            let combinations = generate_combinations(spec);
            let params: Vec<_> = combinations.iter().map(|(inputs, rule_id, expected)| {
                let py_inputs: Vec<_> = inputs.iter().map(|v| self.to_python_value(v)).collect();
                format!("({}, {}, \"{}\")", py_inputs.join(", "), self.to_python_value(expected), rule_id)
            }).collect();

            let input_names = spec.inputs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(",");
            out.push_str(&format!("    @pytest.mark.parametrize(\"{},expected,rule\", [\n", input_names));
            for param in params {
                out.push_str(&format!("        {},\n", param));
            }
            out.push_str("    ])\n");
            out.push_str(&format!("    def test_combination(self, {}, expected, rule):\n",
                spec.inputs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(", ")));
            out.push_str(&format!("        assert {}({}) == expected\n", spec.id,
                spec.inputs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(", ")));
        }

        out
    }

    fn generate_inputs(&self, spec: &Spec, rule: &Rule) -> String {
        spec.inputs.iter().map(|input| {
            rule.conditions.as_ref()
                .and_then(|c| c.iter().find(|cond| cond.var == input.name))
                .map(|c| self.python_condition_value(&c.value))
                .unwrap_or_else(|| self.default_value(&input.typ))
        }).collect::<Vec<_>>().join(", ")
    }

    fn to_python_value(&self, v: &str) -> String {
        match v {
            "true" => "True".into(),
            "false" => "False".into(),
            "null" => "None".into(),
            s => s.to_string(),
        }
    }

    fn python_value(&self, output: &Output) -> String {
        match output {
            Output::Single(v) => self.python_condition_value(v),
            Output::Named(_) => "{}".into(),
        }
    }

    fn python_condition_value(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(true) => "True".into(),
            ConditionValue::Bool(false) => "False".into(),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => f.to_string(),
            ConditionValue::String(s) => format!("\"{}\"", s),
            ConditionValue::Null => "None".into(),
            _ => "None".into(),
        }
    }

    fn default_value(&self, typ: &VarType) -> String {
        match typ {
            VarType::Bool => "False".into(),
            VarType::Int => "0".into(),
            VarType::Float => "0.0".into(),
            VarType::String => "\"\"".into(),
            _ => "None".into(),
        }
    }
}
