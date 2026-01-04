//! Java test generation (JUnit)

use crate::spec::*;
use chrono::Utc;

use super::{to_pascal_case, TestConfig};

pub fn generate(spec: &Spec, _config: &TestConfig) -> String {
    let mut out = String::new();
    let class_name = to_pascal_case(&spec.id);

    out.push_str(&format!("// GENERATED TESTS FROM: {}.yaml\n", spec.id));
    out.push_str(&format!("// SPEC HASH: {}\n", spec.hash()));
    out.push_str(&format!("// GENERATED: {}\n", Utc::now().to_rfc3339()));
    out.push_str("// DO NOT EDIT — regenerate from spec\n\n");

    out.push_str("import org.junit.jupiter.api.Test;\n");
    out.push_str("import static org.junit.jupiter.api.Assertions.*;\n\n");

    out.push_str(&format!("public class {}Test {{\n", class_name));

    for rule in &spec.rules {
        let test_name = format!("test{}", to_pascal_case(&rule.id));
        let expected = java_value(&rule.then);

        out.push_str("    @Test\n");
        out.push_str(&format!("    public void {}() {{\n", test_name));
        out.push_str(&format!(
            "        // {}: {} → {}\n",
            rule.id,
            rule.as_cel().unwrap_or_default(),
            rule.then
        ));
        out.push_str(&format!(
            "        assertEquals({}, {}.evaluate(/* input */));\n",
            expected, class_name
        ));
        out.push_str("    }\n\n");
    }

    out.push_str("}\n");
    out
}

fn java_value(output: &Output) -> String {
    match output {
        Output::Single(v) => java_condition_value(v),
        Output::Named(_) => "new Object()".into(),
    }
}

fn java_condition_value(v: &ConditionValue) -> String {
    match v {
        ConditionValue::Bool(b) => b.to_string(),
        ConditionValue::Int(i) => format!("{}L", i),
        ConditionValue::Float(f) => format!("{}d", f),
        ConditionValue::String(s) => format!("\"{}\"", s),
        ConditionValue::Null => "null".into(),
        _ => "null".into(),
    }
}
