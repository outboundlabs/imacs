// GENERATED TESTS FROM: type_mapping.yaml
// SPEC HASH: sha256:0ee4f2f0c417737b
// GENERATED: 2026-01-06T05:05:49.619287921+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod type_mapping_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_bool_rust() {
        // bool_rust: (var_type == 'Bool') && (target == 'Rust') → "bool"
        assert_eq!(type_mapping("Bool".to_string(), "Rust".to_string()), "bool".to_string());
    }

    #[test]
    fn test_bool_ts() {
        // bool_ts: (var_type == 'Bool') && (target == 'TypeScript') → "boolean"
        assert_eq!(type_mapping("Bool".to_string(), "TypeScript".to_string()), "boolean".to_string());
    }

    #[test]
    fn test_bool_py() {
        // bool_py: (var_type == 'Bool') && (target == 'Python') → "bool"
        assert_eq!(type_mapping("Bool".to_string(), "Python".to_string()), "bool".to_string());
    }

    #[test]
    fn test_int_rust() {
        // int_rust: (var_type == 'Int') && (target == 'Rust') → "i64"
        assert_eq!(type_mapping("Int".to_string(), "Rust".to_string()), "i64".to_string());
    }

    #[test]
    fn test_int_ts() {
        // int_ts: (var_type == 'Int') && (target == 'TypeScript') → "number"
        assert_eq!(type_mapping("Int".to_string(), "TypeScript".to_string()), "number".to_string());
    }

    #[test]
    fn test_int_py() {
        // int_py: (var_type == 'Int') && (target == 'Python') → "int"
        assert_eq!(type_mapping("Int".to_string(), "Python".to_string()), "int".to_string());
    }

    #[test]
    fn test_float_rust() {
        // float_rust: (var_type == 'Float') && (target == 'Rust') → "f64"
        assert_eq!(type_mapping("Float".to_string(), "Rust".to_string()), "f64".to_string());
    }

    #[test]
    fn test_float_ts() {
        // float_ts: (var_type == 'Float') && (target == 'TypeScript') → "number"
        assert_eq!(type_mapping("Float".to_string(), "TypeScript".to_string()), "number".to_string());
    }

    #[test]
    fn test_float_py() {
        // float_py: (var_type == 'Float') && (target == 'Python') → "float"
        assert_eq!(type_mapping("Float".to_string(), "Python".to_string()), "float".to_string());
    }

    #[test]
    fn test_string_rust() {
        // string_rust: (var_type == 'String') && (target == 'Rust') → "String"
        assert_eq!(type_mapping("String".to_string(), "Rust".to_string()), "String".to_string());
    }

    #[test]
    fn test_string_ts() {
        // string_ts: (var_type == 'String') && (target == 'TypeScript') → "string"
        assert_eq!(type_mapping("String".to_string(), "TypeScript".to_string()), "string".to_string());
    }

    #[test]
    fn test_string_py() {
        // string_py: (var_type == 'String') && (target == 'Python') → "str"
        assert_eq!(type_mapping("String".to_string(), "Python".to_string()), "str".to_string());
    }

    #[test]
    fn test_object_rust() {
        // object_rust: (var_type == 'Object') && (target == 'Rust') → "serde_json::Value"
        assert_eq!(type_mapping("Object".to_string(), "Rust".to_string()), "serde_json::Value".to_string());
    }

    #[test]
    fn test_object_ts() {
        // object_ts: (var_type == 'Object') && (target == 'TypeScript') → "Record<string, unknown>"
        assert_eq!(type_mapping("Object".to_string(), "TypeScript".to_string()), "Record<string, unknown>".to_string());
    }

    #[test]
    fn test_object_py() {
        // object_py: (var_type == 'Object') && (target == 'Python') → "dict"
        assert_eq!(type_mapping("Object".to_string(), "Python".to_string()), "dict".to_string());
    }

    #[test]
    fn test_bool_csharp() {
        // bool_csharp: (var_type == 'Bool') && (target == 'CSharp') → "bool"
        assert_eq!(type_mapping("Bool".to_string(), "CSharp".to_string()), "bool".to_string());
    }

    #[test]
    fn test_int_csharp() {
        // int_csharp: (var_type == 'Int') && (target == 'CSharp') → "long"
        assert_eq!(type_mapping("Int".to_string(), "CSharp".to_string()), "long".to_string());
    }

    #[test]
    fn test_float_csharp() {
        // float_csharp: (var_type == 'Float') && (target == 'CSharp') → "double"
        assert_eq!(type_mapping("Float".to_string(), "CSharp".to_string()), "double".to_string());
    }

    #[test]
    fn test_string_csharp() {
        // string_csharp: (var_type == 'String') && (target == 'CSharp') → "string"
        assert_eq!(type_mapping("String".to_string(), "CSharp".to_string()), "string".to_string());
    }

    #[test]
    fn test_object_csharp() {
        // object_csharp: (var_type == 'Object') && (target == 'CSharp') → "Dictionary<string, object>"
        assert_eq!(type_mapping("Object".to_string(), "CSharp".to_string()), "Dictionary<string, object>".to_string());
    }

    #[test]
    fn test_bool_java() {
        // bool_java: (var_type == 'Bool') && (target == 'Java') → "boolean"
        assert_eq!(type_mapping("Bool".to_string(), "Java".to_string()), "boolean".to_string());
    }

    #[test]
    fn test_int_java() {
        // int_java: (var_type == 'Int') && (target == 'Java') → "long"
        assert_eq!(type_mapping("Int".to_string(), "Java".to_string()), "long".to_string());
    }

    #[test]
    fn test_float_java() {
        // float_java: (var_type == 'Float') && (target == 'Java') → "double"
        assert_eq!(type_mapping("Float".to_string(), "Java".to_string()), "double".to_string());
    }

    #[test]
    fn test_string_java() {
        // string_java: (var_type == 'String') && (target == 'Java') → "String"
        assert_eq!(type_mapping("String".to_string(), "Java".to_string()), "String".to_string());
    }

    #[test]
    fn test_object_java() {
        // object_java: (var_type == 'Object') && (target == 'Java') → "Map<String, Object>"
        assert_eq!(type_mapping("Object".to_string(), "Java".to_string()), "Map<String, Object>".to_string());
    }

    #[test]
    fn test_bool_go() {
        // bool_go: (var_type == 'Bool') && (target == 'Go') → "bool"
        assert_eq!(type_mapping("Bool".to_string(), "Go".to_string()), "bool".to_string());
    }

    #[test]
    fn test_int_go() {
        // int_go: (var_type == 'Int') && (target == 'Go') → "int64"
        assert_eq!(type_mapping("Int".to_string(), "Go".to_string()), "int64".to_string());
    }

    #[test]
    fn test_float_go() {
        // float_go: (var_type == 'Float') && (target == 'Go') → "float64"
        assert_eq!(type_mapping("Float".to_string(), "Go".to_string()), "float64".to_string());
    }

    #[test]
    fn test_string_go() {
        // string_go: (var_type == 'String') && (target == 'Go') → "string"
        assert_eq!(type_mapping("String".to_string(), "Go".to_string()), "string".to_string());
    }

    #[test]
    fn test_object_go() {
        // object_go: (var_type == 'Object') && (target == 'Go') → "map[string]interface{}"
        assert_eq!(type_mapping("Object".to_string(), "Go".to_string()), "map[string]interface{}".to_string());
    }

    // ═══════════════════════════════════════════════════════════════
    // Property tests
    // ═══════════════════════════════════════════════════════════════

    #[cfg(feature = "proptest")]
    mod property_tests {
        #[allow(unused_imports)]
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_always_valid_output(var_type in any::<String>(), target in any::<String>()) {
                let result = type_mapping(var_type, target);
                let valid_outputs = vec!["Dictionary<string, object>".to_string(), "Map<String, Object>".to_string(), "Record<string, unknown>".to_string(), "String".to_string(), "bool".to_string(), "boolean".to_string(), "dict".to_string(), "double".to_string(), "f64".to_string(), "float".to_string(), "float64".to_string(), "i64".to_string(), "int".to_string(), "int64".to_string(), "long".to_string(), "map[string]interface{}".to_string(), "number".to_string(), "serde_json::Value".to_string(), "str".to_string(), "string".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
