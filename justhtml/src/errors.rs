//! Centralized error message definitions and helpers for HTML parsing errors.
//!
//! This module provides human-readable error messages for all parse error codes
//! emitted by both the tokenizer and tree builder during HTML parsing.
//!
//! Corresponds to Python's `errors.py`.

/// Generate human-readable error message from error code.
///
/// # Arguments
/// * `code` - The error code string (kebab-case format)
/// * `tag_name` - Optional tag name to include in the message for context
///
/// # Returns
/// Human-readable error message string
pub fn generate_error_message(code: &str, tag_name: Option<&str>) -> String {
    let tag = tag_name.unwrap_or("");

    match code {
        // ================================================================
        // TOKENIZER ERRORS
        // ================================================================
        // DOCTYPE errors
        "eof-in-doctype" => "Unexpected end of file in DOCTYPE declaration".to_string(),
        "eof-in-doctype-name" => "Unexpected end of file while reading DOCTYPE name".to_string(),
        "eof-in-doctype-public-identifier" => "Unexpected end of file in DOCTYPE public identifier".to_string(),
        "eof-in-doctype-system-identifier" => "Unexpected end of file in DOCTYPE system identifier".to_string(),
        "expected-doctype-name-but-got-right-bracket" => "Expected DOCTYPE name but got >".to_string(),
        "missing-whitespace-before-doctype-name" => "Missing whitespace after <!DOCTYPE".to_string(),
        "abrupt-doctype-public-identifier" => "DOCTYPE public identifier ended abruptly".to_string(),
        "abrupt-doctype-system-identifier" => "DOCTYPE system identifier ended abruptly".to_string(),
        "missing-quote-before-doctype-public-identifier" => "Missing quote before DOCTYPE public identifier".to_string(),
        "missing-quote-before-doctype-system-identifier" => "Missing quote before DOCTYPE system identifier".to_string(),
        "missing-doctype-public-identifier" => "Missing DOCTYPE public identifier".to_string(),
        "missing-doctype-system-identifier" => "Missing DOCTYPE system identifier".to_string(),
        "missing-whitespace-before-doctype-public-identifier" => "Missing whitespace before DOCTYPE public identifier".to_string(),
        "missing-whitespace-after-doctype-public-identifier" => "Missing whitespace after DOCTYPE public identifier".to_string(),
        "missing-whitespace-between-doctype-public-and-system-identifiers" => "Missing whitespace between DOCTYPE identifiers".to_string(),
        "missing-whitespace-after-doctype-name" => "Missing whitespace after DOCTYPE name".to_string(),
        "unexpected-character-after-doctype-public-keyword" => "Unexpected character after PUBLIC keyword".to_string(),
        "unexpected-character-after-doctype-system-keyword" => "Unexpected character after SYSTEM keyword".to_string(),
        "unexpected-character-after-doctype-public-identifier" => "Unexpected character after public identifier".to_string(),
        "unexpected-character-after-doctype-system-identifier" => "Unexpected character after system identifier".to_string(),
        // Comment errors
        "eof-in-comment" => "Unexpected end of file in comment".to_string(),
        "abrupt-closing-of-empty-comment" => "Comment ended abruptly with -->".to_string(),
        "incorrectly-closed-comment" => "Comment ended with --!> instead of -->".to_string(),
        // Tag errors
        "eof-in-tag" => "Unexpected end of file in tag".to_string(),
        "eof-before-tag-name" => "Unexpected end of file before tag name".to_string(),
        "empty-end-tag" => "Empty end tag </> is not allowed".to_string(),
        "invalid-first-character-of-tag-name" => "Invalid first character of tag name".to_string(),
        "unexpected-question-mark-instead-of-tag-name" => "Unexpected ? instead of tag name".to_string(),
        "unexpected-character-after-solidus-in-tag" => "Unexpected character after / in tag".to_string(),
        // Attribute errors
        "duplicate-attribute" => "Duplicate attribute name".to_string(),
        "missing-attribute-value" => "Missing attribute value".to_string(),
        "unexpected-character-in-attribute-name" => "Unexpected character in attribute name".to_string(),
        "unexpected-character-in-unquoted-attribute-value" => "Unexpected character in unquoted attribute value".to_string(),
        "missing-whitespace-between-attributes" => "Missing whitespace between attributes".to_string(),
        "unexpected-equals-sign-before-attribute-name" => "Unexpected = before attribute name".to_string(),
        // Script errors
        "eof-in-script-html-comment-like-text" => "Unexpected end of file in script with HTML-like comment".to_string(),
        "eof-in-script-in-script" => "Unexpected end of file in nested script tag".to_string(),
        // CDATA errors
        "eof-in-cdata" => "Unexpected end of file in CDATA section".to_string(),
        "cdata-in-html-content" => "CDATA section only allowed in SVG/MathML content".to_string(),
        // NULL character errors
        "unexpected-null-character" => "Unexpected NULL character (U+0000)".to_string(),
        // Markup declaration errors
        "incorrectly-opened-comment" => "Incorrectly opened comment".to_string(),
        // Character reference errors
        "control-character-reference" => "Invalid control character in character reference".to_string(),
        "illegal-codepoint-for-numeric-entity" => "Invalid codepoint in numeric character reference".to_string(),
        "missing-semicolon-after-character-reference" => "Missing semicolon after character reference".to_string(),
        "named-entity-without-semicolon" => "Named entity used without semicolon".to_string(),

        // ================================================================
        // TREE BUILDER ERRORS
        // ================================================================
        // DOCTYPE errors
        "unexpected-doctype" => "Unexpected DOCTYPE declaration".to_string(),
        "unknown-doctype" => "Unknown DOCTYPE (expected <!DOCTYPE html>)".to_string(),
        "expected-doctype-but-got-chars" => "Expected DOCTYPE but got text content".to_string(),
        "expected-doctype-but-got-eof" => "Expected DOCTYPE but reached end of file".to_string(),
        "expected-doctype-but-got-start-tag" => format!("Expected DOCTYPE but got <{}> tag", tag),
        "expected-doctype-but-got-end-tag" => format!("Expected DOCTYPE but got </{}> tag", tag),
        "unexpected-doctype-in-foreign-content" => "Unexpected DOCTYPE in SVG/MathML content".to_string(),
        // Unexpected tag errors
        "unexpected-start-tag" => format!("Unexpected <{}> start tag", tag),
        "unexpected-end-tag" => format!("Unexpected </{}> end tag", tag),
        "unexpected-end-tag-before-html" => format!("Unexpected </{}> end tag before <html>", tag),
        "unexpected-end-tag-before-head" => format!("Unexpected </{}> end tag before <head>", tag),
        "unexpected-end-tag-after-head" => format!("Unexpected </{}> end tag after <head>", tag),
        "unexpected-start-tag-ignored" => format!("<{}> start tag ignored in current context", tag),
        "unexpected-start-tag-implies-end-tag" => format!("<{}> start tag implicitly closes previous element", tag),
        // EOF errors
        "expected-closing-tag-but-got-eof" => format!("Expected </{}> closing tag but reached end of file", tag),
        "expected-named-closing-tag-but-got-eof" => format!("Expected </{}> closing tag but reached end of file", tag),
        // Invalid character errors
        "invalid-codepoint" => "Invalid character (U+0000 NULL or U+000C FORM FEED)".to_string(),
        "invalid-codepoint-before-head" => "Invalid character before <head>".to_string(),
        "invalid-codepoint-in-body" => "Invalid character in <body>".to_string(),
        "invalid-codepoint-in-table-text" => "Invalid character in table text".to_string(),
        "invalid-codepoint-in-select" => "Invalid character in <select>".to_string(),
        "invalid-codepoint-in-foreign-content" => "Invalid character in SVG/MathML content".to_string(),
        // Foster parenting / table errors
        "foster-parenting-character" => "Text content in table requires foster parenting".to_string(),
        "foster-parenting-start-tag" => "Start tag in table requires foster parenting".to_string(),
        "unexpected-start-tag-implies-table-voodoo" => format!("<{}> start tag in table triggers foster parenting", tag),
        "unexpected-end-tag-implies-table-voodoo" => format!("</{}> end tag in table triggers foster parenting", tag),
        "unexpected-cell-in-table-body" => "Unexpected table cell outside of table row".to_string(),
        "unexpected-form-in-table" => "Form element not allowed in table context".to_string(),
        "unexpected-hidden-input-in-table" => "Hidden input in table triggers foster parenting".to_string(),
        // Context-specific errors
        "unexpected-hidden-input-after-head" => "Unexpected hidden input after <head>".to_string(),
        "unexpected-token-in-frameset" => "Unexpected content in <frameset>".to_string(),
        "unexpected-token-after-frameset" => "Unexpected content after <frameset>".to_string(),
        "unexpected-token-after-after-frameset" => "Unexpected content after frameset closed".to_string(),
        "unexpected-token-after-body" => "Unexpected content after </body>".to_string(),
        "unexpected-char-after-body" => "Unexpected character after </body>".to_string(),
        "unexpected-characters-in-column-group" => "Text not allowed in <colgroup>".to_string(),
        "unexpected-characters-in-template-column-group" => "Text not allowed in template column group".to_string(),
        "unexpected-start-tag-in-column-group" => format!("<{}> start tag not allowed in <colgroup>", tag),
        "unexpected-start-tag-in-template-column-group" => format!("<{}> start tag not allowed in template column group", tag),
        "unexpected-start-tag-in-template-table-context" => format!("<{}> start tag not allowed in template table context", tag),
        "unexpected-start-tag-in-cell-fragment" => format!("<{}> start tag not allowed in cell fragment context", tag),
        // Foreign content errors
        "unexpected-html-element-in-foreign-content" => "HTML element breaks out of SVG/MathML content".to_string(),
        "unexpected-end-tag-in-foreign-content" => format!("Mismatched </{}> end tag in SVG/MathML content", tag),
        "unexpected-end-tag-in-fragment-context" => format!("</{}> end tag not allowed in fragment parsing context", tag),
        // Miscellaneous errors
        "end-tag-too-early" => format!("</{}> end tag closed early (unclosed children)", tag),
        "adoption-agency-1.3" => "Misnested tags require adoption agency algorithm".to_string(),
        "non-void-html-element-start-tag-with-trailing-solidus" => format!("<{}/> self-closing syntax on non-void element", tag),
        "image-start-tag" => format!("Deprecated <{}> tag (use <img> instead)", tag),

        // Default: return code as-is
        _ => code.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_tokenizer_error() {
        let msg = generate_error_message("eof-in-tag", None);
        assert_eq!(msg, "Unexpected end of file in tag");
    }

    #[test]
    fn test_error_with_tag_name() {
        let msg = generate_error_message("unexpected-start-tag", Some("div"));
        assert_eq!(msg, "Unexpected <div> start tag");
    }

    #[test]
    fn test_error_with_end_tag_name() {
        let msg = generate_error_message("unexpected-end-tag", Some("span"));
        assert_eq!(msg, "Unexpected </span> end tag");
    }

    #[test]
    fn test_unknown_error_code() {
        let msg = generate_error_message("some-unknown-error", None);
        assert_eq!(msg, "some-unknown-error");
    }

    #[test]
    fn test_null_character_error() {
        let msg = generate_error_message("unexpected-null-character", None);
        assert_eq!(msg, "Unexpected NULL character (U+0000)");
    }

    #[test]
    fn test_tag_name_not_used_when_not_needed() {
        // Even if tag_name is provided, messages that don't use it should work
        let msg = generate_error_message("eof-in-comment", Some("div"));
        assert_eq!(msg, "Unexpected end of file in comment");
    }

    #[test]
    fn test_eof_in_doctype() {
        let msg = generate_error_message("eof-in-doctype", None);
        assert!(msg.to_lowercase().contains("eof") || msg.to_lowercase().contains("end of file"),
            "eof-in-doctype message should mention EOF, got: {}", msg);
        assert!(msg.to_lowercase().contains("doctype"),
            "eof-in-doctype message should mention DOCTYPE, got: {}", msg);
    }

    #[test]
    fn test_duplicate_attribute() {
        let msg = generate_error_message("duplicate-attribute", None);
        assert!(msg.to_lowercase().contains("duplicate"),
            "duplicate-attribute message should mention 'duplicate', got: {}", msg);
        assert!(msg.to_lowercase().contains("attribute"),
            "duplicate-attribute message should mention 'attribute', got: {}", msg);
    }

    #[test]
    fn test_eof_before_tag_name() {
        let msg = generate_error_message("eof-before-tag-name", None);
        assert!(msg.to_lowercase().contains("eof") || msg.to_lowercase().contains("end of file"),
            "eof-before-tag-name message should mention EOF, got: {}", msg);
    }

    #[test]
    fn test_missing_whitespace_between_attributes() {
        let msg = generate_error_message("missing-whitespace-between-attributes", None);
        assert!(msg.to_lowercase().contains("whitespace") || msg.to_lowercase().contains("missing"),
            "missing-whitespace message should be descriptive, got: {}", msg);
    }

    #[test]
    fn test_unexpected_equals_sign_before_attribute_name() {
        let msg = generate_error_message("unexpected-equals-sign-before-attribute-name", None);
        assert!(msg.to_lowercase().contains("=") || msg.to_lowercase().contains("equals"),
            "unexpected-equals message should mention =, got: {}", msg);
    }

    #[test]
    fn test_various_error_codes_produce_different_messages() {
        // Ensure different error codes produce different messages
        let codes = vec![
            "eof-in-tag",
            "eof-in-comment",
            "eof-in-doctype",
            "unexpected-null-character",
            "duplicate-attribute",
        ];
        let messages: Vec<String> = codes.iter().map(|c| generate_error_message(c, None)).collect();
        // Each should be unique
        for i in 0..messages.len() {
            for j in i + 1..messages.len() {
                assert_ne!(messages[i], messages[j],
                    "Error codes '{}' and '{}' should have different messages",
                    codes[i], codes[j]);
            }
        }
    }

    #[test]
    fn test_tag_name_formatting_start_vs_end() {
        let start_msg = generate_error_message("unexpected-start-tag", Some("p"));
        let end_msg = generate_error_message("unexpected-end-tag", Some("p"));
        assert!(start_msg.contains("<p>"), "Start tag message should contain <p>");
        assert!(end_msg.contains("</p>"), "End tag message should contain </p>");
    }
}
