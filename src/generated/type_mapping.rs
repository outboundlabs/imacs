// GENERATED FROM: type_mapping.yaml
// SPEC HASH: sha256:2a1a69e313b24d0e
// GENERATED: 2026-01-02T13:32:40.967334266+00:00
// DO NOT EDIT — regenerate from spec

pub fn type_mapping(var_type: String, target: String) -> String {
    if (var_type == "Bool") && (target == "Rust") {
        // bool_rust
        "bool".to_string()
    } else if (var_type == "Bool") && (target == "TypeScript") {
        // bool_ts
        "boolean".to_string()
    } else if (var_type == "Bool") && (target == "Python") {
        // bool_py
        "bool".to_string()
    } else if (var_type == "Int") && (target == "Rust") {
        // int_rust
        "i64".to_string()
    } else if (var_type == "Int") && (target == "TypeScript") {
        // int_ts
        "number".to_string()
    } else if (var_type == "Int") && (target == "Python") {
        // int_py
        "int".to_string()
    } else if (var_type == "Float") && (target == "Rust") {
        // float_rust
        "f64".to_string()
    } else if (var_type == "Float") && (target == "TypeScript") {
        // float_ts
        "number".to_string()
    } else if (var_type == "Float") && (target == "Python") {
        // float_py
        "float".to_string()
    } else if (var_type == "String") && (target == "Rust") {
        // string_rust
        "String".to_string()
    } else if (var_type == "String") && (target == "TypeScript") {
        // string_ts
        "string".to_string()
    } else if (var_type == "String") && (target == "Python") {
        // string_py
        "str".to_string()
    } else if (var_type == "Object") && (target == "Rust") {
        // object_rust
        "serde_json::Value".to_string()
    } else if (var_type == "Object") && (target == "TypeScript") {
        // object_ts
        "Record<string, unknown>".to_string()
    } else if (var_type == "Object") && (target == "Python") {
        // object_py
        "dict".to_string()
    } else if (var_type == "Bool") && (target == "CSharp") {
        // bool_csharp
        "bool".to_string()
    } else if (var_type == "Int") && (target == "CSharp") {
        // int_csharp
        "long".to_string()
    } else if (var_type == "Float") && (target == "CSharp") {
        // float_csharp
        "double".to_string()
    } else if (var_type == "String") && (target == "CSharp") {
        // string_csharp
        "string".to_string()
    } else if (var_type == "Object") && (target == "CSharp") {
        // object_csharp
        "Dictionary<string, object>".to_string()
    } else if (var_type == "Bool") && (target == "Java") {
        // bool_java
        "boolean".to_string()
    } else if (var_type == "Int") && (target == "Java") {
        // int_java
        "long".to_string()
    } else if (var_type == "Float") && (target == "Java") {
        // float_java
        "double".to_string()
    } else if (var_type == "String") && (target == "Java") {
        // string_java
        "String".to_string()
    } else if (var_type == "Object") && (target == "Java") {
        // object_java
        "Map<String, Object>".to_string()
    } else if (var_type == "Bool") && (target == "Go") {
        // bool_go
        "bool".to_string()
    } else if (var_type == "Int") && (target == "Go") {
        // int_go
        "int64".to_string()
    } else if (var_type == "Float") && (target == "Go") {
        // float_go
        "float64".to_string()
    } else if (var_type == "String") && (target == "Go") {
        // string_go
        "string".to_string()
    } else if (var_type == "Object") && (target == "Go") {
        // object_go
        "map[string]interface{}".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: type_mapping.yaml
    // SPEC HASH: sha256:2a1a69e313b24d0e
    // GENERATED: 2026-01-02T13:32:41.294017121+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod type_mapping_tests {
        use super::*;

        // ═══════════════════════════════════════════════════════════════
        // Rule tests (one per rule)
        // ═══════════════════════════════════════════════════════════════

        // ═══════════════════════════════════════════════════════════════
        // Property tests
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "proptest")]
        mod property_tests {
            use super::*;
            use proptest::prelude::*;

            proptest! {
                #[test]
                fn prop_always_valid_output(var_type in any::<bool>(), target in any::<bool>()) {
                    let result = type_mapping(var_type, target);
                    prop_assert!(["Dictionary<string, object>".to_string(), "Map<String, Object>".to_string(), "Record<string, unknown>".to_string(), "String".to_string(), "bool".to_string(), "boolean".to_string(), "dict".to_string(), "double".to_string(), "f64".to_string(), "float".to_string(), "float64".to_string(), "i64".to_string(), "int".to_string(), "int64".to_string(), "long".to_string(), "map[string]interface{}".to_string(), "number".to_string(), "serde_json::Value".to_string(), "str".to_string(), "string".to_string()].contains(&result));
                }
            }
        }
    }
}
