//! TypeScript test generation (Vitest)

use crate::spec::*;
use chrono::Utc;

use super::{can_enumerate, extract_test_values, generate_combinations, to_camel_case, TestConfig};

pub fn generate(spec: &Spec, config: &TestConfig) -> String {
    TsTestGen { config }.generate(spec)
}

struct TsTestGen<'a> {
    config: &'a TestConfig,
}

impl<'a> TsTestGen<'a> {
    fn generate(&self, spec: &Spec) -> String {
        let mut out = String::new();
        let func_name = to_camel_case(&spec.id);

        out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", spec.id));
        out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
        out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
        out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

        out.push_str("import { describe, it, expect } from 'vitest';\n");
        out.push_str(&format!(
            "import {{ {} }} from './{}';\n\n",
            func_name, spec.id
        ));

        out.push_str(&format!("describe('{}', () => {{\n", func_name));

        // Rule tests
        out.push_str("  describe('rules', () => {\n");
        for rule in &spec.rules {
            let inputs = self.generate_input_object(spec, rule);
            let expected = self.ts_value(&rule.then);

            // Escape single quotes in test description for valid JS string
            let cel_desc = rule.as_cel().unwrap_or_default().replace('\'', "\\'");

            out.push_str(&format!(
                "    it('{}: {} → {}', () => {{\n",
                rule.id, cel_desc, rule.then
            ));
            out.push_str(&format!(
                "      expect({}({})).toBe({});\n",
                func_name, inputs, expected
            ));
            out.push_str("    });\n\n");
        }
        out.push_str("  });\n\n");

        // Exhaustive tests
        if self.config.exhaustive && can_enumerate(spec) {
            out.push_str("  describe('exhaustive', () => {\n");
            out.push_str("    const cases = [\n");

            for (inputs, rule_id, expected) in generate_combinations(spec) {
                let input_obj = self.format_input_object(spec, &inputs);
                out.push_str(&format!(
                    "      {{ input: {}, expected: {}, rule: '{}' }},\n",
                    input_obj, expected, rule_id
                ));
            }

            out.push_str("    ];\n\n");
            out.push_str("    it.each(cases)('$rule', ({ input, expected }) => {\n");
            out.push_str(&format!(
                "      expect({}(input)).toBe(expected);\n",
                func_name
            ));
            out.push_str("    });\n");
            out.push_str("  });\n");
        }

        out.push_str("});\n");
        out
    }

    fn generate_input_object(&self, spec: &Spec, rule: &Rule) -> String {
        let values = extract_test_values(rule, &spec.inputs);
        let fields: Vec<String> = spec
            .inputs
            .iter()
            .map(|input| {
                let value = values
                    .get(&input.name)
                    .cloned()
                    .unwrap_or_else(|| self.default_value(&input.typ));
                format!("{}: {}", to_camel_case(&input.name), value)
            })
            .collect();
        format!("{{ {} }}", fields.join(", "))
    }

    fn format_input_object(&self, spec: &Spec, values: &[String]) -> String {
        let fields: Vec<String> = spec
            .inputs
            .iter()
            .zip(values.iter())
            .map(|(input, value)| format!("{}: {}", to_camel_case(&input.name), value))
            .collect();
        format!("{{ {} }}", fields.join(", "))
    }

    fn ts_value(&self, output: &Output) -> String {
        match output {
            Output::Single(v) => self.ts_condition_value(v),
            Output::Named(_) => "{}".into(),
            Output::Expression(expr) => expr.clone(),
        }
    }

    fn ts_condition_value(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => f.to_string(),
            ConditionValue::String(s) => format!("\"{}\"", s),
            ConditionValue::Null => "null".into(),
            _ => "undefined".into(),
        }
    }

    fn default_value(&self, typ: &VarType) -> String {
        match typ {
            VarType::Bool => "false".into(),
            VarType::Int | VarType::Float => "0".into(),
            VarType::String => "\"\"".into(),
            _ => "undefined".into(),
        }
    }
}
