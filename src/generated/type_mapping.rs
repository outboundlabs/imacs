// GENERATED FROM: type_mapping.yaml
// SPEC HASH: sha256:0ee4f2f0c417737b
// GENERATED: 2026-01-06T05:05:49.619105313+00:00
// DO NOT EDIT - regenerate from spec

#[allow(
    unused_parens,
    unused_variables,
    clippy::bool_comparison,
    clippy::if_same_then_else
)]
pub fn type_mapping(var_type: String, target: String) -> String {
    if ((var_type == "Bool") && (target == "Rust")) {
        // bool_rust
        "bool".to_string()
    } else if ((var_type == "Bool") && (target == "TypeScript")) {
        // bool_ts
        "boolean".to_string()
    } else if ((var_type == "Bool") && (target == "Python")) {
        // bool_py
        "bool".to_string()
    } else if ((var_type == "Int") && (target == "Rust")) {
        // int_rust
        "i64".to_string()
    } else if ((var_type == "Int") && (target == "TypeScript")) {
        // int_ts
        "number".to_string()
    } else if ((var_type == "Int") && (target == "Python")) {
        // int_py
        "int".to_string()
    } else if ((var_type == "Float") && (target == "Rust")) {
        // float_rust
        "f64".to_string()
    } else if ((var_type == "Float") && (target == "TypeScript")) {
        // float_ts
        "number".to_string()
    } else if ((var_type == "Float") && (target == "Python")) {
        // float_py
        "float".to_string()
    } else if ((var_type == "String") && (target == "Rust")) {
        // string_rust
        "String".to_string()
    } else if ((var_type == "String") && (target == "TypeScript")) {
        // string_ts
        "string".to_string()
    } else if ((var_type == "String") && (target == "Python")) {
        // string_py
        "str".to_string()
    } else if ((var_type == "Object") && (target == "Rust")) {
        // object_rust
        "serde_json::Value".to_string()
    } else if ((var_type == "Object") && (target == "TypeScript")) {
        // object_ts
        "Record<string, unknown>".to_string()
    } else if ((var_type == "Object") && (target == "Python")) {
        // object_py
        "dict".to_string()
    } else if ((var_type == "Bool") && (target == "CSharp")) {
        // bool_csharp
        "bool".to_string()
    } else if ((var_type == "Int") && (target == "CSharp")) {
        // int_csharp
        "long".to_string()
    } else if ((var_type == "Float") && (target == "CSharp")) {
        // float_csharp
        "double".to_string()
    } else if ((var_type == "String") && (target == "CSharp")) {
        // string_csharp
        "string".to_string()
    } else if ((var_type == "Object") && (target == "CSharp")) {
        // object_csharp
        "Dictionary<string, object>".to_string()
    } else if ((var_type == "Bool") && (target == "Java")) {
        // bool_java
        "boolean".to_string()
    } else if ((var_type == "Int") && (target == "Java")) {
        // int_java
        "long".to_string()
    } else if ((var_type == "Float") && (target == "Java")) {
        // float_java
        "double".to_string()
    } else if ((var_type == "String") && (target == "Java")) {
        // string_java
        "String".to_string()
    } else if ((var_type == "Object") && (target == "Java")) {
        // object_java
        "Map<String, Object>".to_string()
    } else if ((var_type == "Bool") && (target == "Go")) {
        // bool_go
        "bool".to_string()
    } else if ((var_type == "Int") && (target == "Go")) {
        // int_go
        "int64".to_string()
    } else if ((var_type == "Float") && (target == "Go")) {
        // float_go
        "float64".to_string()
    } else if ((var_type == "String") && (target == "Go")) {
        // string_go
        "string".to_string()
    } else if ((var_type == "Object") && (target == "Go")) {
        // object_go
        "map[string]interface{}".to_string()
    } else {
        unreachable!("No rule matched")
    }
}
