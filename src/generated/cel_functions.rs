// GENERATED FROM: cel_functions.yaml
// SPEC HASH: sha256:ac37fe0ca12854eb
// GENERATED: 2026-01-02T13:32:39.752042327+00:00
// DO NOT EDIT — regenerate from spec

pub fn cel_functions(func: String, target: String) -> String {
    if (func == "size") && (target == "Rust") {
        // size_rust
        "{0}.len()".to_string()
    } else if (func == "size") && (target == "TypeScript") {
        // size_ts
        "{0}.length".to_string()
    } else if (func == "size") && (target == "Python") {
        // size_py
        "len({0})".to_string()
    } else if (func == "has") && (target == "Rust") {
        // has_rust
        "{0}.is_some()".to_string()
    } else if (func == "has") && (target == "TypeScript") {
        // has_ts
        "({0} !== undefined)".to_string()
    } else if (func == "has") && (target == "Python") {
        // has_py
        "({0} is not None)".to_string()
    } else if (func == "contains") && (target == "Rust") {
        // contains_rust
        "{0}.contains({1})".to_string()
    } else if (func == "contains") && (target == "TypeScript") {
        // contains_ts
        "{0}.includes({1})".to_string()
    } else if (func == "contains") && (target == "Python") {
        // contains_py
        "({1} in {0})".to_string()
    } else if (func == "startsWith") && (target == "Rust") {
        // startswith_rust
        "{0}.starts_with({1})".to_string()
    } else if (func == "startsWith") && (target == "TypeScript") {
        // startswith_ts
        "{0}.startsWith({1})".to_string()
    } else if (func == "startsWith") && (target == "Python") {
        // startswith_py
        "{0}.startswith({1})".to_string()
    } else if (func == "endsWith") && (target == "Rust") {
        // endswith_rust
        "{0}.ends_with({1})".to_string()
    } else if (func == "endsWith") && (target == "TypeScript") {
        // endswith_ts
        "{0}.endsWith({1})".to_string()
    } else if (func == "endsWith") && (target == "Python") {
        // endswith_py
        "{0}.endswith({1})".to_string()
    } else if (func == "matches") && (target == "Rust") {
        // matches_rust
        "Regex::new({1}).unwrap().is_match({0})".to_string()
    } else if (func == "matches") && (target == "TypeScript") {
        // matches_ts
        "{0}.match({1})".to_string()
    } else if (func == "matches") && (target == "Python") {
        // matches_py
        "re.match({1}, {0})".to_string()
    } else if (func == "int") && (target == "Rust") {
        // int_rust
        "{0} as i64".to_string()
    } else if (func == "int") && (target == "TypeScript") {
        // int_ts
        "parseInt({0})".to_string()
    } else if (func == "int") && (target == "Python") {
        // int_py
        "int({0})".to_string()
    } else if (func == "float") && (target == "Rust") {
        // float_rust
        "{0} as f64".to_string()
    } else if (func == "float") && (target == "TypeScript") {
        // float_ts
        "parseFloat({0})".to_string()
    } else if (func == "float") && (target == "Python") {
        // float_py
        "float({0})".to_string()
    } else if (func == "string") && (target == "Rust") {
        // string_rust
        "{0}.to_string()".to_string()
    } else if (func == "string") && (target == "TypeScript") {
        // string_ts
        "String({0})".to_string()
    } else if (func == "string") && (target == "Python") {
        // string_py
        "str({0})".to_string()
    } else if (func == "size") && (target == "CSharp") {
        // size_csharp
        "{0}.Count".to_string()
    } else if (func == "has") && (target == "CSharp") {
        // has_csharp
        "({0} != null)".to_string()
    } else if (func == "contains") && (target == "CSharp") {
        // contains_csharp
        "{0}.Contains({1})".to_string()
    } else if (func == "startsWith") && (target == "CSharp") {
        // startswith_csharp
        "{0}.StartsWith({1})".to_string()
    } else if (func == "endsWith") && (target == "CSharp") {
        // endswith_csharp
        "{0}.EndsWith({1})".to_string()
    } else if (func == "matches") && (target == "CSharp") {
        // matches_csharp
        "Regex.IsMatch({0}, {1})".to_string()
    } else if (func == "int") && (target == "CSharp") {
        // int_csharp
        "(long){0}".to_string()
    } else if (func == "float") && (target == "CSharp") {
        // float_csharp
        "(double){0}".to_string()
    } else if (func == "string") && (target == "CSharp") {
        // string_csharp
        "{0}.ToString()".to_string()
    } else if (func == "size") && (target == "Java") {
        // size_java
        "{0}.size()".to_string()
    } else if (func == "has") && (target == "Java") {
        // has_java
        "({0} != null)".to_string()
    } else if (func == "contains") && (target == "Java") {
        // contains_java
        "{0}.contains({1})".to_string()
    } else if (func == "startsWith") && (target == "Java") {
        // startswith_java
        "{0}.startsWith({1})".to_string()
    } else if (func == "endsWith") && (target == "Java") {
        // endswith_java
        "{0}.endsWith({1})".to_string()
    } else if (func == "matches") && (target == "Java") {
        // matches_java
        "{0}.matches({1})".to_string()
    } else if (func == "int") && (target == "Java") {
        // int_java
        "(long){0}".to_string()
    } else if (func == "float") && (target == "Java") {
        // float_java
        "(double){0}".to_string()
    } else if (func == "string") && (target == "Java") {
        // string_java
        "{0}.toString()".to_string()
    } else if (func == "size") && (target == "Go") {
        // size_go
        "len({0})".to_string()
    } else if (func == "has") && (target == "Go") {
        // has_go
        "({0} != nil)".to_string()
    } else if (func == "contains") && (target == "Go") {
        // contains_go
        "strings.Contains({0}, {1})".to_string()
    } else if (func == "startsWith") && (target == "Go") {
        // startswith_go
        "strings.HasPrefix({0}, {1})".to_string()
    } else if (func == "endsWith") && (target == "Go") {
        // endswith_go
        "strings.HasSuffix({0}, {1})".to_string()
    } else if (func == "matches") && (target == "Go") {
        // matches_go
        "regexp.MatchString({1}, {0})".to_string()
    } else if (func == "int") && (target == "Go") {
        // int_go
        "int64({0})".to_string()
    } else if (func == "float") && (target == "Go") {
        // float_go
        "float64({0})".to_string()
    } else if (func == "string") && (target == "Go") {
        // string_go
        "fmt.Sprintf(\"%v\", {0})".to_string()
    } else {
        unreachable!("No rule matched")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GENERATED TESTS FROM: cel_functions.yaml
    // SPEC HASH: sha256:ac37fe0ca12854eb
    // GENERATED: 2026-01-02T13:32:40.298754621+00:00
    // DO NOT EDIT — regenerate from spec

    #[cfg(test)]
    mod cel_functions_tests {
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
                fn prop_always_valid_output(func in any::<bool>(), target in any::<bool>()) {
                    let result = cel_functions(func, target);
                    prop_assert!(["(double){0}".to_string(), "(long){0}".to_string(), "({0} != nil)".to_string(), "({0} != null)".to_string(), "({0} !== undefined)".to_string(), "({0} is not None)".to_string(), "({1} in {0})".to_string(), "Regex.IsMatch({0}, {1})".to_string(), "Regex::new({1}).unwrap().is_match({0})".to_string(), "String({0})".to_string(), "float({0})".to_string(), "float64({0})".to_string(), "fmt.Sprintf(\"%v\", {0})".to_string(), "int({0})".to_string(), "int64({0})".to_string(), "len({0})".to_string(), "parseFloat({0})".to_string(), "parseInt({0})".to_string(), "re.match({1}, {0})".to_string(), "regexp.MatchString({1}, {0})".to_string(), "str({0})".to_string(), "strings.Contains({0}, {1})".to_string(), "strings.HasPrefix({0}, {1})".to_string(), "strings.HasSuffix({0}, {1})".to_string(), "{0} as f64".to_string(), "{0} as i64".to_string(), "{0}.Contains({1})".to_string(), "{0}.Count".to_string(), "{0}.EndsWith({1})".to_string(), "{0}.StartsWith({1})".to_string(), "{0}.ToString()".to_string(), "{0}.contains({1})".to_string(), "{0}.endsWith({1})".to_string(), "{0}.ends_with({1})".to_string(), "{0}.endswith({1})".to_string(), "{0}.includes({1})".to_string(), "{0}.is_some()".to_string(), "{0}.len()".to_string(), "{0}.length".to_string(), "{0}.match({1})".to_string(), "{0}.matches({1})".to_string(), "{0}.size()".to_string(), "{0}.startsWith({1})".to_string(), "{0}.starts_with({1})".to_string(), "{0}.startswith({1})".to_string(), "{0}.toString()".to_string(), "{0}.to_string()".to_string()].contains(&result));
                }
            }
        }
    }
}
