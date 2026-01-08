//! Rust test generation

use crate::spec::*;
use chrono::Utc;

use super::{
    can_enumerate, extract_test_values, generate_combinations, has_numeric_conditions, TestConfig,
};

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
        out.push_str("    #[allow(unused_imports)]\n");
        out.push_str("    use super::*;\n\n");

        // Rule tests
        out.push_str("    // ═══════════════════════════════════════════════════════════════\n");
        out.push_str("    // Rule tests (one per rule)\n");
        out.push_str("    // ═══════════════════════════════════════════════════════════════\n\n");

        for rule in &spec.rules {
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
            out.push_str("        #[allow(unused_imports)]\n");
            out.push_str("        use super::*;\n");
            out.push_str("        use proptest::prelude::*;\n\n");

            let valid_outputs = self.collect_outputs(spec);

            out.push_str("        proptest! {\n");
            out.push_str("            #[test]\n");
            // Build the function signature without format! to avoid brace interpretation
            out.push_str("            fn prop_always_valid_output(");
            out.push_str(&self.proptest_args(spec));
            out.push_str(") {\n");

            // Build the function call
            out.push_str("                let result = ");
            out.push_str(&spec.id);
            out.push('(');
            out.push_str(&self.function_args(spec));
            out.push_str(");\n");

            // Build the assertion - use a match or if-else to avoid format string issues
            if valid_outputs.len() == 1 {
                out.push_str("                prop_assert!(result == ");
                // Write the output value directly, escaping quotes if needed
                let output_val = &valid_outputs[0];
                if output_val.starts_with('"') {
                    // It's already a string literal, write it as-is
                    out.push_str(output_val);
                } else {
                    out.push_str(output_val);
                }
                out.push_str(");\n");
            } else {
                // Build vec![] with individual string literals
                out.push_str("                let valid_outputs = vec![");
                for (idx, output) in valid_outputs.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(", ");
                    }
                    // Output is already a Rust expression, write it directly
                    out.push_str(output);
                }
                out.push_str("];\n");
                out.push_str("                prop_assert!(valid_outputs.contains(&result));\n");
            }
            out.push_str("            }\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
        }

        out.push_str("}\n");
        out
    }

    fn generate_inputs(&self, spec: &Spec, rule: &Rule) -> String {
        let values = extract_test_values(rule, &spec.inputs);
        let inputs: Vec<String> = spec
            .inputs
            .iter()
            .map(|input| {
                let value = values
                    .get(&input.name)
                    .cloned()
                    .unwrap_or_else(|| self.default_value(&input.typ));
                // Convert to Rust syntax if needed
                self.to_rust_value(&value)
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
            Output::Expression(expr) => return expr.clone(),
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
                    VarType::Bool => "any::<bool>()".into(),
                    VarType::Int => "any::<i64>()".into(),
                    VarType::Float => "any::<f64>()".into(),
                    VarType::String => "any::<String>()".into(),
                    VarType::Enum(_) => "any::<String>()".into(),
                    _ => "any::<String>()".into(),
                };
                format!("{} in {}", i.name, strategy)
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
