// GENERATED TESTS FROM: cel_functions.yaml
// SPEC HASH: sha256:712fd99981775811
// GENERATED: 2026-01-06T05:05:48.990091921+00:00
// DO NOT EDIT — regenerate from spec

#[cfg(test)]
mod cel_functions_tests {
    #[allow(unused_imports)]
    use super::*;

    // ═══════════════════════════════════════════════════════════════
    // Rule tests (one per rule)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_size_rust() {
        // size_rust: (func == 'size') && (target == 'Rust') → "{0}.len()"
        assert_eq!(cel_functions("size".to_string(), "Rust".to_string()), "{0}.len()".to_string());
    }

    #[test]
    fn test_size_ts() {
        // size_ts: (func == 'size') && (target == 'TypeScript') → "{0}.length"
        assert_eq!(cel_functions("size".to_string(), "TypeScript".to_string()), "{0}.length".to_string());
    }

    #[test]
    fn test_size_py() {
        // size_py: (func == 'size') && (target == 'Python') → "len({0})"
        assert_eq!(cel_functions("size".to_string(), "Python".to_string()), "len({0})".to_string());
    }

    #[test]
    fn test_has_rust() {
        // has_rust: (func == 'has') && (target == 'Rust') → "{0}.is_some()"
        assert_eq!(cel_functions("has".to_string(), "Rust".to_string()), "{0}.is_some()".to_string());
    }

    #[test]
    fn test_has_ts() {
        // has_ts: (func == 'has') && (target == 'TypeScript') → "({0} !== undefined)"
        assert_eq!(cel_functions("has".to_string(), "TypeScript".to_string()), "({0} !== undefined)".to_string());
    }

    #[test]
    fn test_has_py() {
        // has_py: (func == 'has') && (target == 'Python') → "({0} is not None)"
        assert_eq!(cel_functions("has".to_string(), "Python".to_string()), "({0} is not None)".to_string());
    }

    #[test]
    fn test_contains_rust() {
        // contains_rust: (func == 'contains') && (target == 'Rust') → "{0}.contains({1})"
        assert_eq!(cel_functions("contains".to_string(), "Rust".to_string()), "{0}.contains({1})".to_string());
    }

    #[test]
    fn test_contains_ts() {
        // contains_ts: (func == 'contains') && (target == 'TypeScript') → "{0}.includes({1})"
        assert_eq!(cel_functions("contains".to_string(), "TypeScript".to_string()), "{0}.includes({1})".to_string());
    }

    #[test]
    fn test_contains_py() {
        // contains_py: (func == 'contains') && (target == 'Python') → "({1} in {0})"
        assert_eq!(cel_functions("contains".to_string(), "Python".to_string()), "({1} in {0})".to_string());
    }

    #[test]
    fn test_startswith_rust() {
        // startswith_rust: (func == 'startsWith') && (target == 'Rust') → "{0}.starts_with({1})"
        assert_eq!(cel_functions("startsWith".to_string(), "Rust".to_string()), "{0}.starts_with({1})".to_string());
    }

    #[test]
    fn test_startswith_ts() {
        // startswith_ts: (func == 'startsWith') && (target == 'TypeScript') → "{0}.startsWith({1})"
        assert_eq!(cel_functions("startsWith".to_string(), "TypeScript".to_string()), "{0}.startsWith({1})".to_string());
    }

    #[test]
    fn test_startswith_py() {
        // startswith_py: (func == 'startsWith') && (target == 'Python') → "{0}.startswith({1})"
        assert_eq!(cel_functions("startsWith".to_string(), "Python".to_string()), "{0}.startswith({1})".to_string());
    }

    #[test]
    fn test_endswith_rust() {
        // endswith_rust: (func == 'endsWith') && (target == 'Rust') → "{0}.ends_with({1})"
        assert_eq!(cel_functions("endsWith".to_string(), "Rust".to_string()), "{0}.ends_with({1})".to_string());
    }

    #[test]
    fn test_endswith_ts() {
        // endswith_ts: (func == 'endsWith') && (target == 'TypeScript') → "{0}.endsWith({1})"
        assert_eq!(cel_functions("endsWith".to_string(), "TypeScript".to_string()), "{0}.endsWith({1})".to_string());
    }

    #[test]
    fn test_endswith_py() {
        // endswith_py: (func == 'endsWith') && (target == 'Python') → "{0}.endswith({1})"
        assert_eq!(cel_functions("endsWith".to_string(), "Python".to_string()), "{0}.endswith({1})".to_string());
    }

    #[test]
    fn test_matches_rust() {
        // matches_rust: (func == 'matches') && (target == 'Rust') → "Regex::new({1}).unwrap().is_match({0})"
        assert_eq!(cel_functions("matches".to_string(), "Rust".to_string()), "Regex::new({1}).unwrap().is_match({0})".to_string());
    }

    #[test]
    fn test_matches_ts() {
        // matches_ts: (func == 'matches') && (target == 'TypeScript') → "{0}.match({1})"
        assert_eq!(cel_functions("matches".to_string(), "TypeScript".to_string()), "{0}.match({1})".to_string());
    }

    #[test]
    fn test_matches_py() {
        // matches_py: (func == 'matches') && (target == 'Python') → "re.match({1}, {0})"
        assert_eq!(cel_functions("matches".to_string(), "Python".to_string()), "re.match({1}, {0})".to_string());
    }

    #[test]
    fn test_int_rust() {
        // int_rust: (func == 'int') && (target == 'Rust') → "{0} as i64"
        assert_eq!(cel_functions("int".to_string(), "Rust".to_string()), "{0} as i64".to_string());
    }

    #[test]
    fn test_int_ts() {
        // int_ts: (func == 'int') && (target == 'TypeScript') → "parseInt({0})"
        assert_eq!(cel_functions("int".to_string(), "TypeScript".to_string()), "parseInt({0})".to_string());
    }

    #[test]
    fn test_int_py() {
        // int_py: (func == 'int') && (target == 'Python') → "int({0})"
        assert_eq!(cel_functions("int".to_string(), "Python".to_string()), "int({0})".to_string());
    }

    #[test]
    fn test_float_rust() {
        // float_rust: (func == 'float') && (target == 'Rust') → "{0} as f64"
        assert_eq!(cel_functions("float".to_string(), "Rust".to_string()), "{0} as f64".to_string());
    }

    #[test]
    fn test_float_ts() {
        // float_ts: (func == 'float') && (target == 'TypeScript') → "parseFloat({0})"
        assert_eq!(cel_functions("float".to_string(), "TypeScript".to_string()), "parseFloat({0})".to_string());
    }

    #[test]
    fn test_float_py() {
        // float_py: (func == 'float') && (target == 'Python') → "float({0})"
        assert_eq!(cel_functions("float".to_string(), "Python".to_string()), "float({0})".to_string());
    }

    #[test]
    fn test_string_rust() {
        // string_rust: (func == 'string') && (target == 'Rust') → "{0}.to_string()"
        assert_eq!(cel_functions("string".to_string(), "Rust".to_string()), "{0}.to_string()".to_string());
    }

    #[test]
    fn test_string_ts() {
        // string_ts: (func == 'string') && (target == 'TypeScript') → "String({0})"
        assert_eq!(cel_functions("string".to_string(), "TypeScript".to_string()), "String({0})".to_string());
    }

    #[test]
    fn test_string_py() {
        // string_py: (func == 'string') && (target == 'Python') → "str({0})"
        assert_eq!(cel_functions("string".to_string(), "Python".to_string()), "str({0})".to_string());
    }

    #[test]
    fn test_size_csharp() {
        // size_csharp: (func == 'size') && (target == 'CSharp') → "{0}.Count"
        assert_eq!(cel_functions("size".to_string(), "CSharp".to_string()), "{0}.Count".to_string());
    }

    #[test]
    fn test_has_csharp() {
        // has_csharp: (func == 'has') && (target == 'CSharp') → "({0} != null)"
        assert_eq!(cel_functions("has".to_string(), "CSharp".to_string()), "({0} != null)".to_string());
    }

    #[test]
    fn test_contains_csharp() {
        // contains_csharp: (func == 'contains') && (target == 'CSharp') → "{0}.Contains({1})"
        assert_eq!(cel_functions("contains".to_string(), "CSharp".to_string()), "{0}.Contains({1})".to_string());
    }

    #[test]
    fn test_startswith_csharp() {
        // startswith_csharp: (func == 'startsWith') && (target == 'CSharp') → "{0}.StartsWith({1})"
        assert_eq!(cel_functions("startsWith".to_string(), "CSharp".to_string()), "{0}.StartsWith({1})".to_string());
    }

    #[test]
    fn test_endswith_csharp() {
        // endswith_csharp: (func == 'endsWith') && (target == 'CSharp') → "{0}.EndsWith({1})"
        assert_eq!(cel_functions("endsWith".to_string(), "CSharp".to_string()), "{0}.EndsWith({1})".to_string());
    }

    #[test]
    fn test_matches_csharp() {
        // matches_csharp: (func == 'matches') && (target == 'CSharp') → "Regex.IsMatch({0}, {1})"
        assert_eq!(cel_functions("matches".to_string(), "CSharp".to_string()), "Regex.IsMatch({0}, {1})".to_string());
    }

    #[test]
    fn test_int_csharp() {
        // int_csharp: (func == 'int') && (target == 'CSharp') → "(long){0}"
        assert_eq!(cel_functions("int".to_string(), "CSharp".to_string()), "(long){0}".to_string());
    }

    #[test]
    fn test_float_csharp() {
        // float_csharp: (func == 'float') && (target == 'CSharp') → "(double){0}"
        assert_eq!(cel_functions("float".to_string(), "CSharp".to_string()), "(double){0}".to_string());
    }

    #[test]
    fn test_string_csharp() {
        // string_csharp: (func == 'string') && (target == 'CSharp') → "{0}.ToString()"
        assert_eq!(cel_functions("string".to_string(), "CSharp".to_string()), "{0}.ToString()".to_string());
    }

    #[test]
    fn test_size_java() {
        // size_java: (func == 'size') && (target == 'Java') → "{0}.size()"
        assert_eq!(cel_functions("size".to_string(), "Java".to_string()), "{0}.size()".to_string());
    }

    #[test]
    fn test_has_java() {
        // has_java: (func == 'has') && (target == 'Java') → "({0} != null)"
        assert_eq!(cel_functions("has".to_string(), "Java".to_string()), "({0} != null)".to_string());
    }

    #[test]
    fn test_contains_java() {
        // contains_java: (func == 'contains') && (target == 'Java') → "{0}.contains({1})"
        assert_eq!(cel_functions("contains".to_string(), "Java".to_string()), "{0}.contains({1})".to_string());
    }

    #[test]
    fn test_startswith_java() {
        // startswith_java: (func == 'startsWith') && (target == 'Java') → "{0}.startsWith({1})"
        assert_eq!(cel_functions("startsWith".to_string(), "Java".to_string()), "{0}.startsWith({1})".to_string());
    }

    #[test]
    fn test_endswith_java() {
        // endswith_java: (func == 'endsWith') && (target == 'Java') → "{0}.endsWith({1})"
        assert_eq!(cel_functions("endsWith".to_string(), "Java".to_string()), "{0}.endsWith({1})".to_string());
    }

    #[test]
    fn test_matches_java() {
        // matches_java: (func == 'matches') && (target == 'Java') → "{0}.matches({1})"
        assert_eq!(cel_functions("matches".to_string(), "Java".to_string()), "{0}.matches({1})".to_string());
    }

    #[test]
    fn test_int_java() {
        // int_java: (func == 'int') && (target == 'Java') → "(long){0}"
        assert_eq!(cel_functions("int".to_string(), "Java".to_string()), "(long){0}".to_string());
    }

    #[test]
    fn test_float_java() {
        // float_java: (func == 'float') && (target == 'Java') → "(double){0}"
        assert_eq!(cel_functions("float".to_string(), "Java".to_string()), "(double){0}".to_string());
    }

    #[test]
    fn test_string_java() {
        // string_java: (func == 'string') && (target == 'Java') → "{0}.toString()"
        assert_eq!(cel_functions("string".to_string(), "Java".to_string()), "{0}.toString()".to_string());
    }

    #[test]
    fn test_size_go() {
        // size_go: (func == 'size') && (target == 'Go') → "len({0})"
        assert_eq!(cel_functions("size".to_string(), "Go".to_string()), "len({0})".to_string());
    }

    #[test]
    fn test_has_go() {
        // has_go: (func == 'has') && (target == 'Go') → "({0} != nil)"
        assert_eq!(cel_functions("has".to_string(), "Go".to_string()), "({0} != nil)".to_string());
    }

    #[test]
    fn test_contains_go() {
        // contains_go: (func == 'contains') && (target == 'Go') → "strings.Contains({0}, {1})"
        assert_eq!(cel_functions("contains".to_string(), "Go".to_string()), "strings.Contains({0}, {1})".to_string());
    }

    #[test]
    fn test_startswith_go() {
        // startswith_go: (func == 'startsWith') && (target == 'Go') → "strings.HasPrefix({0}, {1})"
        assert_eq!(cel_functions("startsWith".to_string(), "Go".to_string()), "strings.HasPrefix({0}, {1})".to_string());
    }

    #[test]
    fn test_endswith_go() {
        // endswith_go: (func == 'endsWith') && (target == 'Go') → "strings.HasSuffix({0}, {1})"
        assert_eq!(cel_functions("endsWith".to_string(), "Go".to_string()), "strings.HasSuffix({0}, {1})".to_string());
    }

    #[test]
    fn test_matches_go() {
        // matches_go: (func == 'matches') && (target == 'Go') → "regexp.MatchString({1}, {0})"
        assert_eq!(cel_functions("matches".to_string(), "Go".to_string()), "regexp.MatchString({1}, {0})".to_string());
    }

    #[test]
    fn test_int_go() {
        // int_go: (func == 'int') && (target == 'Go') → "int64({0})"
        assert_eq!(cel_functions("int".to_string(), "Go".to_string()), "int64({0})".to_string());
    }

    #[test]
    fn test_float_go() {
        // float_go: (func == 'float') && (target == 'Go') → "float64({0})"
        assert_eq!(cel_functions("float".to_string(), "Go".to_string()), "float64({0})".to_string());
    }

    #[test]
    fn test_string_go() {
        // string_go: (func == 'string') && (target == 'Go') → "fmt.Sprintf("%v", {0})"
        assert_eq!(cel_functions("string".to_string(), "Go".to_string()), "fmt.Sprintf(\"%v\", {0})".to_string());
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
            fn prop_always_valid_output(func in any::<String>(), target in any::<String>()) {
                let result = cel_functions(func, target);
                let valid_outputs = vec!["(double){0}".to_string(), "(long){0}".to_string(), "({0} != nil)".to_string(), "({0} != null)".to_string(), "({0} !== undefined)".to_string(), "({0} is not None)".to_string(), "({1} in {0})".to_string(), "Regex.IsMatch({0}, {1})".to_string(), "Regex::new({1}).unwrap().is_match({0})".to_string(), "String({0})".to_string(), "float({0})".to_string(), "float64({0})".to_string(), "fmt.Sprintf(\"%v\", {0})".to_string(), "int({0})".to_string(), "int64({0})".to_string(), "len({0})".to_string(), "parseFloat({0})".to_string(), "parseInt({0})".to_string(), "re.match({1}, {0})".to_string(), "regexp.MatchString({1}, {0})".to_string(), "str({0})".to_string(), "strings.Contains({0}, {1})".to_string(), "strings.HasPrefix({0}, {1})".to_string(), "strings.HasSuffix({0}, {1})".to_string(), "{0} as f64".to_string(), "{0} as i64".to_string(), "{0}.Contains({1})".to_string(), "{0}.Count".to_string(), "{0}.EndsWith({1})".to_string(), "{0}.StartsWith({1})".to_string(), "{0}.ToString()".to_string(), "{0}.contains({1})".to_string(), "{0}.endsWith({1})".to_string(), "{0}.ends_with({1})".to_string(), "{0}.endswith({1})".to_string(), "{0}.includes({1})".to_string(), "{0}.is_some()".to_string(), "{0}.len()".to_string(), "{0}.length".to_string(), "{0}.match({1})".to_string(), "{0}.matches({1})".to_string(), "{0}.size()".to_string(), "{0}.startsWith({1})".to_string(), "{0}.starts_with({1})".to_string(), "{0}.startswith({1})".to_string(), "{0}.toString()".to_string(), "{0}.to_string()".to_string()];
                prop_assert!(valid_outputs.contains(&result));
            }
        }
    }
}
