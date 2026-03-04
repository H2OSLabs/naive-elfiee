/// Naming convention conversion utilities for Extension generation.
///
/// These functions handle conversion between different naming conventions:
/// - snake_case (Rust variables, file names)
/// - PascalCase (Rust types, struct names)
/// - camelCase (JavaScript/TypeScript)

/// Convert a string to snake_case.
///
/// # Examples
/// ```
/// use elfiee_ext_gen::utils::naming::to_snake_case;
///
/// assert_eq!(to_snake_case("TodoAddItem"), "todo_add_item");
/// assert_eq!(to_snake_case("HTTPRequest"), "http_request");
/// assert_eq!(to_snake_case("already_snake"), "already_snake");
/// ```
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut prev_was_lowercase = false;
    let mut prev_was_underscore = false;

    while let Some(ch) = chars.next() {
        if ch == '_' || ch == '-' {
            if !result.is_empty() && !prev_was_underscore {
                result.push('_');
                prev_was_underscore = true;
            }
            prev_was_lowercase = false;
            continue;
        }

        if ch.is_uppercase() {
            // Insert underscore before uppercase if:
            // 1. Not at start AND
            // 2. Previous was lowercase OR
            // 3. Next char is lowercase (for handling HTTPRequest -> http_request)
            if !result.is_empty() && !prev_was_underscore {
                if prev_was_lowercase || chars.peek().map_or(false, |c| c.is_lowercase()) {
                    result.push('_');
                }
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_was_lowercase = false;
            prev_was_underscore = false;
        } else {
            result.push(ch);
            prev_was_lowercase = ch.is_lowercase();
            prev_was_underscore = false;
        }
    }

    result
}

/// Convert a string to PascalCase.
///
/// # Examples
/// ```
/// use elfiee_ext_gen::utils::naming::to_pascal_case;
///
/// assert_eq!(to_pascal_case("todo_add_item"), "TodoAddItem");
/// assert_eq!(to_pascal_case("http_request"), "HttpRequest");
/// assert_eq!(to_pascal_case("AlreadyPascal"), "AlreadyPascal");
/// ```
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch.is_whitespace() {
            capitalize_next = true;
            continue;
        }

        if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert a string to camelCase.
///
/// # Note
/// Currently unused in generator - reserved for potential frontend code generation.
///
/// # Examples
/// ```
/// use elfiee_ext_gen::utils::naming::to_camel_case;
///
/// assert_eq!(to_camel_case("todo_add_item"), "todoAddItem");
/// assert_eq!(to_camel_case("http_request"), "httpRequest");
/// ```
#[allow(dead_code)]
pub fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    if pascal.is_empty() {
        return pascal;
    }

    // Convert first character to lowercase
    let mut chars = pascal.chars();
    let first = chars.next().unwrap();
    let rest: String = chars.collect();

    format!("{}{}", first.to_lowercase(), rest)
}

/// Convert a capability ID to a Capability struct name.
///
/// # Note
/// Currently unused - generator uses `to_pascal_case` directly to avoid duplication.
/// Reserved for potential use in capability registration code generation.
///
/// # Examples
/// ```
/// use elfiee_ext_gen::utils::naming::capability_to_struct_name;
///
/// assert_eq!(
///     capability_to_struct_name("todo.add_item"),
///     "TodoAddItemCapability"
/// );
/// assert_eq!(
///     capability_to_struct_name("markdown.write"),
///     "MarkdownWriteCapability"
/// );
/// ```
#[allow(dead_code)]
pub fn capability_to_struct_name(cap_id: &str) -> String {
    // Split by '.' and convert each part to PascalCase
    let parts: Vec<&str> = cap_id.split('.').collect();
    let pascal_parts: Vec<String> = parts.iter().map(|part| to_pascal_case(part)).collect();

    format!("{}Capability", pascal_parts.join(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Tests for to_snake_case
    // ========================================

    #[test]
    fn test_to_snake_case_from_pascal() {
        assert_eq!(to_snake_case("TodoAddItem"), "todo_add_item");
    }

    #[test]
    fn test_to_snake_case_consecutive_caps() {
        assert_eq!(to_snake_case("HTTPRequest"), "http_request");
        assert_eq!(to_snake_case("XMLParser"), "xml_parser");
    }

    #[test]
    fn test_to_snake_case_already_snake() {
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_snake_case_mixed() {
        assert_eq!(to_snake_case("getHTTPResponse"), "get_http_response");
    }

    #[test]
    fn test_to_snake_case_numbers() {
        assert_eq!(to_snake_case("Base64Encoder"), "base64_encoder");
    }

    // ========================================
    // Tests for to_pascal_case
    // ========================================

    #[test]
    fn test_to_pascal_case_from_snake() {
        assert_eq!(to_pascal_case("todo_add_item"), "TodoAddItem");
    }

    #[test]
    fn test_to_pascal_case_from_kebab() {
        assert_eq!(to_pascal_case("todo-add-item"), "TodoAddItem");
    }

    #[test]
    fn test_to_pascal_case_already_pascal() {
        assert_eq!(to_pascal_case("AlreadyPascal"), "AlreadyPascal");
    }

    #[test]
    fn test_to_pascal_case_with_numbers() {
        assert_eq!(to_pascal_case("base64_encoder"), "Base64Encoder");
    }

    #[test]
    fn test_to_pascal_case_single_word() {
        assert_eq!(to_pascal_case("http"), "Http");
    }

    // ========================================
    // Tests for to_camel_case
    // ========================================

    #[test]
    fn test_to_camel_case_from_snake() {
        assert_eq!(to_camel_case("todo_add_item"), "todoAddItem");
    }

    #[test]
    fn test_to_camel_case_from_pascal() {
        assert_eq!(to_camel_case("TodoAddItem"), "todoAddItem");
    }

    #[test]
    fn test_to_camel_case_single_word() {
        assert_eq!(to_camel_case("http"), "http");
    }

    // ========================================
    // Tests for capability_to_struct_name
    // ========================================

    #[test]
    fn test_capability_to_struct_name_simple() {
        assert_eq!(
            capability_to_struct_name("todo.add_item"),
            "TodoAddItemCapability"
        );
    }

    #[test]
    fn test_capability_to_struct_name_single_word() {
        assert_eq!(
            capability_to_struct_name("markdown.write"),
            "MarkdownWriteCapability"
        );
    }

    #[test]
    fn test_capability_to_struct_name_multiple_words() {
        assert_eq!(
            capability_to_struct_name("diagram.render_svg"),
            "DiagramRenderSvgCapability"
        );
    }

    #[test]
    fn test_capability_to_struct_name_core() {
        assert_eq!(
            capability_to_struct_name("core.create"),
            "CoreCreateCapability"
        );
    }
}
