//! Rust test generation

use crate::spec::*;
use chrono::Utc;

use super::{can_enumerate, generate_combinations, has_numeric_conditions, TestConfig};

pub fn generate(spec: &Spec, config: &TestConfig) -> String {
    RustTestGen { config }.generate(spec)
}

struct RustTestGen<'a> {
    config: &'a TestConfig,
}

impl<'a> RustTestGen<'a> {
    fn generate(&self, spec: &Spec) -> String {
        let mut out = String::new();

        // Header
        out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", spec.id));
        out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
        out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
        out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

        out.push_str("#[cfg(test)]\n");
        out.push_str(&format!("mod {}_tests {{\n", spec.id));
        out.push_str("    use super::*;\n\n");

        // Rule tests
        out.push_str("    // ═══════════════════════════════════════════════════════════════\n");
        out.push_str("    // Rule tests (one per rule)\n");
        out.push_str("    // ═══════════════════════════════════════════════════════════════\n\n");

        for rule in &spec.rules {
            if rule.when.is_some() && rule.conditions.is_none() {
                continue;
            }

            let test_name = format!("test_{}", rule.id.to_lowercase());
            let inputs = self.generate_inputs(spec, rule);
            let expected = self.rust_value_for_spec(&rule.then, spec);

            out.push_str("    #[test]\n");
            out.push_str(&format!("    fn {}() {{\n", test_name));
            out.push_str(&format!(
                "        // {}: {} → {}\n",
                rule.id,
                rule.as_cel().unwrap_or_default(),
                rule.then
            ));
            out.push_str(&format!(
                "        assert_eq!({}({}), {});\n",
                spec.id, inputs, expected
            ));
            out.push_str("    }\n\n");
        }

        // Exhaustive tests
        if self.config.exhaustive && can_enumerate(spec) {
            out.push_str(
                "    // ═══════════════════════════════════════════════════════════════\n",
            );
            out.push_str("    // Exhaustive tests (all input combinations)\n");
            out.push_str(
                "    // ═══════════════════════════════════════════════════════════════\n\n",
            );

            out.push_str("    #[test]\n");
            out.push_str("    fn test_exhaustive() {\n");

            let combinations = generate_combinations(spec);
            for (inputs, rule_id, expected) in combinations {
                let input_str = inputs
                    .iter()
                    .map(|v| self.to_rust_value(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "        assert_eq!({}({}), {});  // {}\n",
                    spec.id, input_str, expected, rule_id
                ));
            }

            out.push_str("    }\n\n");
        }

        // Boundary tests
        if self.config.boundary && has_numeric_conditions(spec) {
            out.push_str(
                "    // ═══════════════════════════════════════════════════════════════\n",
            );
            out.push_str("    // Boundary tests\n");
            out.push_str(
                "    // ═══════════════════════════════════════════════════════════════\n\n",
            );

            let boundary_tests = self.generate_boundary_tests(spec);
            for (name, tests) in boundary_tests {
                out.push_str("    #[test]\n");
                out.push_str(&format!("    fn test_boundary_{}() {{\n", name));
                for (inputs, expected, comment) in tests {
                    out.push_str(&format!(
                        "        assert_eq!({}({}), {});  // {}\n",
                        spec.id, inputs, expected, comment
                    ));
                }
                out.push_str("    }\n\n");
            }
        }

        // Property tests
        if self.config.property {
            out.push_str(
                "    // ═══════════════════════════════════════════════════════════════\n",
            );
            out.push_str("    // Property tests\n");
            out.push_str(
                "    // ═══════════════════════════════════════════════════════════════\n\n",
            );

            out.push_str("    #[cfg(feature = \"proptest\")]\n");
            out.push_str("    mod property_tests {\n");
            out.push_str("        use super::*;\n");
            out.push_str("        use proptest::prelude::*;\n\n");

            let valid_outputs = self.collect_outputs(spec);
            out.push_str("        proptest! {\n");
            out.push_str("            #[test]\n");
            out.push_str(&format!(
                "            fn prop_always_valid_output({}) {{\n",
                self.proptest_args(spec)
            ));
            out.push_str(&format!(
                "                let result = {}({});\n",
                spec.id,
                self.function_args(spec)
            ));
            out.push_str(&format!(
                "                prop_assert!([{}].contains(&result));\n",
                valid_outputs.join(", ")
            ));
            out.push_str("            }\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
        }

        out.push_str("}\n");
        out
    }

    fn generate_inputs(&self, spec: &Spec, rule: &Rule) -> String {
        let inputs: Vec<String> = spec
            .inputs
            .iter()
            .map(|input| {
                rule.conditions
                    .as_ref()
                    .and_then(|c| c.iter().find(|cond| cond.var == input.name))
                    .map(|c| self.rust_condition_value(&c.value))
                    .unwrap_or_else(|| self.default_value(&input.typ))
            })
            .collect();
        inputs.join(", ")
    }

    fn to_rust_value(&self, v: &str) -> String {
        if v == "true" || v == "false" {
            v.to_string()
        } else if v.starts_with('"') {
            format!("{}.to_string()", v)
        } else {
            v.to_string()
        }
    }

    fn rust_value_for_spec(&self, output: &Output, spec: &Spec) -> String {
        let map: Option<&std::collections::HashMap<String, ConditionValue>> = match output {
            Output::Named(m) => Some(m),
            Output::Single(ConditionValue::Map(m)) => Some(m),
            Output::Single(v) => return self.rust_condition_value(v),
        };

        let map = match map {
            Some(m) => m,
            None => return "()".into(),
        };

        if spec.outputs.len() > 1 {
            let values: Vec<_> = spec
                .outputs
                .iter()
                .map(|out_var| {
                    map.get(&out_var.name)
                        .map(|v| self.rust_condition_value(v))
                        .unwrap_or_else(|| "Default::default()".into())
                })
                .collect();
            format!("({})", values.join(", "))
        } else if let Some(first_output) = spec.outputs.first() {
            map.get(&first_output.name)
                .map(|v| self.rust_condition_value(v))
                .unwrap_or_else(|| "Default::default()".into())
        } else {
            "()".into()
        }
    }

    fn rust_condition_value(&self, v: &ConditionValue) -> String {
        match v {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Int(i) => i.to_string(),
            ConditionValue::Float(f) => format!("{:?}", f),
            ConditionValue::String(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{}\".to_string()", escaped)
            }
            ConditionValue::Null => "None".into(),
            _ => "()".into(),
        }
    }

    fn default_value(&self, typ: &VarType) -> String {
        match typ {
            VarType::Bool => "false".into(),
            VarType::Int => "0".into(),
            VarType::Float => "0.0".into(),
            VarType::String => "\"\"".into(),
            _ => "Default::default()".into(),
        }
    }

    #[allow(clippy::type_complexity)]
    fn generate_boundary_tests(&self, spec: &Spec) -> Vec<(String, Vec<(String, String, String)>)> {
        let mut results = Vec::new();

        for rule in &spec.rules {
            if let Some(conditions) = &rule.conditions {
                for cond in conditions {
                    if let ConditionValue::Int(threshold) = &cond.value {
                        match cond.op {
                            ConditionOp::Gt => {
                                results.push((
                                    format!("{}_{}", cond.var, threshold),
                                    vec![
                                        (
                                            threshold.to_string(),
                                            "/*below*/".into(),
                                            format!("at threshold {}", threshold),
                                        ),
                                        (
                                            (threshold + 1).to_string(),
                                            self.rust_condition_value(&match &rule.then {
                                                Output::Single(v) => v.clone(),
                                                _ => ConditionValue::Int(0),
                                            }),
                                            format!("above threshold {}", threshold),
                                        ),
                                    ],
                                ));
                            }
                            ConditionOp::Lt => {
                                results.push((
                                    format!("{}_{}", cond.var, threshold),
                                    vec![
                                        (
                                            (threshold - 1).to_string(),
                                            self.rust_condition_value(&match &rule.then {
                                                Output::Single(v) => v.clone(),
                                                _ => ConditionValue::Int(0),
                                            }),
                                            format!("below threshold {}", threshold),
                                        ),
                                        (
                                            threshold.to_string(),
                                            "/*at or above*/".into(),
                                            format!("at threshold {}", threshold),
                                        ),
                                    ],
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        results
    }

    fn collect_outputs(&self, spec: &Spec) -> Vec<String> {
        let mut outputs: Vec<String> = spec
            .rules
            .iter()
            .map(|r| self.rust_value_for_spec(&r.then, spec))
            .collect();

        if let Some(default) = &spec.default {
            outputs.push(self.rust_value_for_spec(default, spec));
        }

        outputs.sort();
        outputs.dedup();
        outputs
    }

    fn proptest_args(&self, spec: &Spec) -> String {
        spec.inputs
            .iter()
            .map(|i| {
                let strategy: String = match &i.typ {
                    VarType::Bool => "bool".into(),
                    VarType::Int => "i64".into(),
                    VarType::Float => "f64".into(),
                    VarType::String => "String".into(),
                    _ => "bool".into(),
                };
                format!("{} in any::<{}>()", i.name, strategy)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn function_args(&self, spec: &Spec) -> String {
        spec.inputs
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
