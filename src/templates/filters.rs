//! Custom MiniJinja filters for code generation
//!
//! These filters handle language-specific transformations like:
//! - Case conversion (PascalCase, camelCase, snake_case)
//! - Type mapping per language
//! - Indentation

use crate::util;
use minijinja::value::Value;
use minijinja::Environment;

/// Register all custom filters with the environment
pub fn register_filters(env: &mut Environment<'_>) {
    env.add_filter("pascal_case", filter_pascal_case);
    env.add_filter("camel_case", filter_camel_case);
    env.add_filter("snake_case", filter_snake_case);
    env.add_filter("upper_snake_case", filter_upper_snake_case);
    env.add_filter("indent", indent);
    env.add_filter("rust_type", rust_type);
    env.add_filter("typescript_type", typescript_type);
    env.add_filter("python_type", python_type);
    env.add_filter("go_type", go_type);
    env.add_filter("java_type", java_type);
    env.add_filter("csharp_type", csharp_type);
    env.add_filter("escape_string", escape_string);
    env.add_filter("items", items);
}

/// Convert a map to a list of [key, value] pairs for iteration
fn items(value: Value) -> Vec<Vec<Value>> {
    if let Ok(iter) = value.try_iter() {
        iter.filter_map(|key| value.get_item(&key).ok().map(|val| vec![key, val]))
            .collect()
    } else {
        vec![]
    }
}

// Filter wrappers that delegate to shared util functions
fn filter_pascal_case(value: &str) -> String {
    util::to_pascal_case(value)
}

fn filter_camel_case(value: &str) -> String {
    util::to_camel_case(value)
}

fn filter_snake_case(value: &str) -> String {
    util::to_snake_case(value)
}

fn filter_upper_snake_case(value: &str) -> String {
    util::to_upper_snake_case(value)
}

/// Add indentation to each line
fn indent(value: &str, spaces: usize) -> String {
    let indent_str = " ".repeat(spaces);
    value
        .lines()
        .map(|line| {
            if line.is_empty() {
                line.to_string()
            } else {
                format!("{}{}", indent_str, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Map IMACS type to Rust type
fn rust_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "bool".to_string(),
        "int" | "Int" => "i64".to_string(),
        "float" | "Float" => "f64".to_string(),
        "string" | "String" => "String".to_string(),
        "object" | "Object" => "serde_json::Value".to_string(),
        other => {
            // Handle List<T> -> Vec<T>
            if other.starts_with("List<") && other.ends_with('>') {
                let inner = &other[5..other.len() - 1];
                format!("Vec<{}>", rust_type(inner))
            } else {
                other.to_string()
            }
        }
    }
}

/// Map IMACS type to TypeScript type
fn typescript_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "boolean".to_string(),
        "int" | "Int" => "number".to_string(),
        "float" | "Float" => "number".to_string(),
        "string" | "String" => "string".to_string(),
        "object" | "Object" => "Record<string, unknown>".to_string(),
        other => {
            if other.starts_with("List<") && other.ends_with('>') {
                let inner = &other[5..other.len() - 1];
                format!("{}[]", typescript_type(inner))
            } else {
                other.to_string()
            }
        }
    }
}

/// Map IMACS type to Python type hint
fn python_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "bool".to_string(),
        "int" | "Int" => "int".to_string(),
        "float" | "Float" => "float".to_string(),
        "string" | "String" => "str".to_string(),
        "object" | "Object" => "dict[str, Any]".to_string(),
        other => {
            if other.starts_with("List<") && other.ends_with('>') {
                let inner = &other[5..other.len() - 1];
                format!("list[{}]", python_type(inner))
            } else {
                other.to_string()
            }
        }
    }
}

/// Map IMACS type to Go type
fn go_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "bool".to_string(),
        "int" | "Int" => "int64".to_string(),
        "float" | "Float" => "float64".to_string(),
        "string" | "String" => "string".to_string(),
        "object" | "Object" => "map[string]interface{}".to_string(),
        other => {
            if other.starts_with("List<") && other.ends_with('>') {
                let inner = &other[5..other.len() - 1];
                format!("[]{}", go_type(inner))
            } else {
                other.to_string()
            }
        }
    }
}

/// Map IMACS type to Java type
fn java_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "boolean".to_string(),
        "int" | "Int" => "long".to_string(),
        "float" | "Float" => "double".to_string(),
        "string" | "String" => "String".to_string(),
        "object" | "Object" => "Map<String, Object>".to_string(),
        other => {
            if other.starts_with("List<") && other.ends_with('>') {
                let inner = &other[5..other.len() - 1];
                format!("List<{}>", java_boxed_type(inner))
            } else {
                other.to_string()
            }
        }
    }
}

/// Get boxed Java type (for generics)
fn java_boxed_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "Boolean".to_string(),
        "int" | "Int" => "Long".to_string(),
        "float" | "Float" => "Double".to_string(),
        other => java_type(other),
    }
}

/// Map IMACS type to C# type
fn csharp_type(value: &str) -> String {
    match value {
        "bool" | "Bool" => "bool".to_string(),
        "int" | "Int" => "long".to_string(),
        "float" | "Float" => "double".to_string(),
        "string" | "String" => "string".to_string(),
        "object" | "Object" => "Dictionary<string, object>".to_string(),
        other => {
            if other.starts_with("List<") && other.ends_with('>') {
                let inner = &other[5..other.len() - 1];
                format!("List<{}>", csharp_type(inner))
            } else {
                other.to_string()
            }
        }
    }
}

/// Escape string for use in generated code
fn escape_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(util::to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(util::to_pascal_case("foo"), "Foo");
        assert_eq!(util::to_pascal_case("foo_bar_baz"), "FooBarBaz");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(util::to_camel_case("hello_world"), "helloWorld");
        assert_eq!(util::to_camel_case("foo"), "foo");
        assert_eq!(util::to_camel_case("FOO_BAR"), "fOOBAR");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(util::to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(util::to_snake_case("fooBar"), "foo_bar");
    }

    #[test]
    fn test_indent() {
        assert_eq!(indent("foo\nbar", 4), "    foo\n    bar");
        assert_eq!(indent("foo\n\nbar", 2), "  foo\n\n  bar");
    }

    #[test]
    fn test_rust_type() {
        assert_eq!(rust_type("bool"), "bool");
        assert_eq!(rust_type("int"), "i64");
        assert_eq!(rust_type("string"), "String");
        assert_eq!(rust_type("List<int>"), "Vec<i64>");
    }

    #[test]
    fn test_typescript_type() {
        assert_eq!(typescript_type("bool"), "boolean");
        assert_eq!(typescript_type("int"), "number");
        assert_eq!(typescript_type("List<string>"), "string[]");
    }

    #[test]
    fn test_go_type() {
        assert_eq!(go_type("bool"), "bool");
        assert_eq!(go_type("int"), "int64");
        assert_eq!(go_type("List<float>"), "[]float64");
    }

    #[test]
    fn test_java_type() {
        assert_eq!(java_type("bool"), "boolean");
        assert_eq!(java_type("int"), "long");
        assert_eq!(java_type("List<int>"), "List<Long>");
    }

    #[test]
    fn test_csharp_type() {
        assert_eq!(csharp_type("bool"), "bool");
        assert_eq!(csharp_type("int"), "long");
        assert_eq!(csharp_type("List<string>"), "List<string>");
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
    }
}
