//! Go test generation

use crate::spec::*;
use chrono::Utc;

use super::{to_pascal_case, TestConfig};

pub fn generate(spec: &Spec, _config: &TestConfig) -> String {
    let mut out = String::new();
    let func_name = to_pascal_case(&spec.id);

    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", spec.id));
    out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("package main\n\n");
    out.push_str("import \"testing\"\n\n");

    for rule in &spec.rules {
        let test_name = format!("Test{}_{}", func_name, to_pascal_case(&rule.id));
        let expected = go_value(&rule.then);

        out.push_str(&format!("func {}(t *testing.T) {{\n", test_name));
        out.push_str(&format!("\t// {}: {} → {}\n", rule.id, rule.as_cel().unwrap_or_default(), rule.then));
        out.push_str(&format!("\tresult := {}(/* input */)\n", func_name));
        out.push_str(&format!("\tif result != {} {{\n", expected));
        out.push_str(&format!("\t\tt.Errorf(\"Expected {}, got %v\", result)\n", expected));
        out.push_str("\t}\n");
        out.push_str("}\n\n");
    }

    out
}

fn go_value(output: &Output) -> String {
    match output {
        Output::Single(v) => go_condition_value(v),
        Output::Named(_) => "nil".into(),
    }
}

fn go_condition_value(v: &ConditionValue) -> String {
    match v {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => i.to_string(),
        ConditionValue::Float(f) => f.to_string(),
        ConditionValue::String(s) => format!("\"{}\"", s),
        ConditionValue::Null => "nil".into(),
        _ => "nil".into(),
    }
}
