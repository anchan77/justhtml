//! Token data types for the HTML5 tokenizer.
//!
//! Corresponds to Python's `tokens.py`.

use std::collections::HashMap;
use std::fmt;

/// The kind of a tag token (start or end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Start = 0,
    End = 1,
}

/// An HTML tag token (start or end).
#[derive(Debug, Clone)]
pub struct Tag {
    pub kind: TagKind,
    pub name: String,
    pub attrs: HashMap<String, Option<String>>,
    pub self_closing: bool,
}

impl Tag {
    pub fn new(kind: TagKind, name: String, attrs: HashMap<String, Option<String>>, self_closing: bool) -> Self {
        Self { kind, name, attrs, self_closing }
    }

    pub fn new_start(name: &str) -> Self {
        Self {
            kind: TagKind::Start,
            name: name.to_string(),
            attrs: HashMap::new(),
            self_closing: false,
        }
    }

    pub fn new_end(name: &str) -> Self {
        Self {
            kind: TagKind::End,
            name: name.to_string(),
            attrs: HashMap::new(),
            self_closing: false,
        }
    }
}

/// Character data token.
#[derive(Debug, Clone)]
pub struct CharacterTokens {
    pub data: String,
}

impl CharacterTokens {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}

/// A comment token.
#[derive(Debug, Clone)]
pub struct CommentToken {
    pub data: String,
}

impl CommentToken {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}

/// DOCTYPE information.
#[derive(Debug, Clone)]
pub struct Doctype {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub force_quirks: bool,
}

impl Doctype {
    pub fn new() -> Self {
        Self {
            name: None,
            public_id: None,
            system_id: None,
            force_quirks: false,
        }
    }
}

impl Default for Doctype {
    fn default() -> Self {
        Self::new()
    }
}

/// A DOCTYPE token wrapping a [`Doctype`].
#[derive(Debug, Clone)]
pub struct DoctypeToken {
    pub doctype: Doctype,
}

impl DoctypeToken {
    pub fn new(doctype: Doctype) -> Self {
        Self { doctype }
    }
}

/// End-of-file token.
#[derive(Debug, Clone, Copy)]
pub struct EOFToken;

/// Result returned by `TokenSink::process_token`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSinkResult {
    Continue = 0,
    Plaintext = 1,
}

/// A unified token enum wrapping all possible token types.
#[derive(Debug)]
pub enum Token {
    Tag(Tag),
    Characters(CharacterTokens),
    Comment(CommentToken),
    Doctype(DoctypeToken),
    EOF(EOFToken),
}

/// Represents a parse error with location information.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub code: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
    pub source_html: Option<String>,
    pub end_column: Option<usize>,
}

impl ParseError {
    pub fn new(
        code: &str,
        line: Option<usize>,
        column: Option<usize>,
        message: Option<&str>,
        source_html: Option<String>,
        end_column: Option<usize>,
    ) -> Self {
        let msg = message.unwrap_or(code).to_string();
        Self {
            code: code.to_string(),
            line,
            column,
            message: msg,
            source_html,
            end_column,
        }
    }

    /// Convert to a formatted error string with source highlighting info.
    ///
    /// In Rust, we don't have Python's SyntaxError, so we provide formatting
    /// methods instead.
    pub fn format_with_source(&self) -> String {
        if self.line.is_none() || self.column.is_none() || self.source_html.is_none() {
            return self.message.clone();
        }

        let line_num = self.line.unwrap();
        let col = self.column.unwrap();
        let source = self.source_html.as_ref().unwrap();

        let lines: Vec<&str> = source.split('\n').collect();
        if line_num < 1 || line_num > lines.len() {
            return self.message.clone();
        }

        let error_line = lines[line_num - 1];
        format!(
            "  File \"<html>\", line {}\n    {}\n    {}^\n{}: {}",
            line_num,
            error_line,
            " ".repeat(col.saturating_sub(1)),
            "ParseError",
            self.message
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => {
                if self.message != self.code {
                    write!(f, "({},{}): {} - {}", line, column, self.code, self.message)
                } else {
                    write!(f, "({},{}): {}", line, column, self.code)
                }
            }
            _ => {
                if self.message != self.code {
                    write!(f, "{} - {}", self.code, self.message)
                } else {
                    write!(f, "{}", self.code)
                }
            }
        }
    }
}

impl PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code && self.line == other.line && self.column == other.column
    }
}

impl Eq for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_str_with_location() {
        let error = ParseError::new("test-error", Some(1), Some(5), None, None, None);
        assert_eq!(format!("{}", error), "(1,5): test-error");
    }

    #[test]
    fn test_parse_error_str_without_location() {
        let error = ParseError::new("test-error", None, None, None, None, None);
        assert_eq!(format!("{}", error), "test-error");
    }

    #[test]
    fn test_parse_error_with_message_no_location() {
        let error = ParseError::new("test-error", None, None, Some("This is a test error"), None, None);
        assert_eq!(format!("{}", error), "test-error - This is a test error");
    }

    #[test]
    fn test_parse_error_with_location_and_message() {
        let error = ParseError::new("test-error", Some(5), Some(10), Some("Detailed error"), None, None);
        assert_eq!(format!("{}", error), "(5,10): test-error - Detailed error");
    }

    #[test]
    fn test_parse_error_repr() {
        let error = ParseError::new("test-error", Some(1), Some(5), None, None, None);
        let repr = format!("{:?}", error);
        assert!(repr.contains("test-error"));
    }

    #[test]
    fn test_parse_error_equality() {
        let e1 = ParseError::new("error-code", Some(1), Some(5), None, None, None);
        let e2 = ParseError::new("error-code", Some(1), Some(5), None, None, None);
        let e3 = ParseError::new("other-error", Some(1), Some(5), None, None, None);
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn test_tag_construction() {
        let tag = Tag::new_start("div");
        assert_eq!(tag.kind, TagKind::Start);
        assert_eq!(tag.name, "div");
        assert!(tag.attrs.is_empty());
        assert!(!tag.self_closing);

        let end_tag = Tag::new_end("div");
        assert_eq!(end_tag.kind, TagKind::End);
    }

    #[test]
    fn test_token_sink_result_values() {
        assert_eq!(TokenSinkResult::Continue as u8, 0);
        assert_eq!(TokenSinkResult::Plaintext as u8, 1);
    }

    #[test]
    fn test_doctype_default() {
        let dt = Doctype::default();
        assert!(dt.name.is_none());
        assert!(dt.public_id.is_none());
        assert!(dt.system_id.is_none());
        assert!(!dt.force_quirks);
    }
}
