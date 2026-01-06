//! Shared utility functions
//!
//! Common utilities used across multiple modules to avoid duplication.

/// Convert snake_case to PascalCase
///
/// # Examples
/// ```
/// use imacs::util::to_pascal_case;
/// assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
/// assert_eq!(to_pascal_case("foo"), "Foo");
/// ```
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

/// Convert snake_case to camelCase
///
/// # Examples
/// ```
/// use imacs::util::to_camel_case;
/// assert_eq!(to_camel_case("hello_world"), "helloWorld");
/// assert_eq!(to_camel_case("foo"), "foo");
/// ```
pub fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Convert PascalCase or camelCase to snake_case
///
/// # Examples
/// ```
/// use imacs::util::to_snake_case;
/// assert_eq!(to_snake_case("HelloWorld"), "hello_world");
/// assert_eq!(to_snake_case("fooBar"), "foo_bar");
/// ```
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert to UPPER_SNAKE_CASE
///
/// # Examples
/// ```
/// use imacs::util::to_upper_snake_case;
/// assert_eq!(to_upper_snake_case("HelloWorld"), "HELLO_WORLD");
/// ```
pub fn to_upper_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("foo"), "Foo");
        assert_eq!(to_pascal_case("access_level"), "AccessLevel");
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
        assert_eq!(to_camel_case("foo"), "foo");
        assert_eq!(to_camel_case("access_level"), "accessLevel");
        assert_eq!(to_camel_case(""), "");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("fooBar"), "foo_bar");
        assert_eq!(to_snake_case("AccessLevel"), "access_level");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_upper_snake_case() {
        assert_eq!(to_upper_snake_case("HelloWorld"), "HELLO_WORLD");
        assert_eq!(to_upper_snake_case("fooBar"), "FOO_BAR");
    }
}
