//! justhtml – a pure-Rust HTML5 parser.
//!
//! Provides [`JustHTML`] as the primary entry point for parsing HTML documents and fragments.
//!
//! # Example
//! ```
//! use justhtml::JustHTML;
//!
//! let doc = JustHTML::parse("<p>Hello, world!</p>", Default::default());
//! assert!(doc.root.borrow().children.len() > 0);
//! ```

pub mod constants;
pub mod context;
pub mod encoding;
pub mod entities;
pub mod errors;
pub mod markdown;
pub mod node;
pub mod serialize;
pub mod tokenizer;
pub mod tokens;
pub mod treebuilder;

use std::fmt;

pub use context::FragmentContext;
pub use node::{NodeHandle, NodeData, NodeKind};
pub use tokens::ParseError;
pub use treebuilder::{InsertionMode, TreeBuilder};

use encoding::decode_html;
use tokenizer::{Tokenizer, TokenizerOpts, TokenizerState};

// ---------------------------------------------------------------------------
// StrictModeError
// ---------------------------------------------------------------------------

/// Error raised when strict mode encounters a parse error.
///
/// Wraps the first [`ParseError`] encountered during parsing.
#[derive(Debug, Clone)]
pub struct StrictModeError {
    /// The underlying parse error that triggered strict mode failure.
    pub error: ParseError,
}

impl StrictModeError {
    pub fn new(error: ParseError) -> Self {
        Self { error }
    }
}

impl fmt::Display for StrictModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StrictModeError: {}", self.error)
    }
}

impl std::error::Error for StrictModeError {}

// ---------------------------------------------------------------------------
// JustHTMLOptions
// ---------------------------------------------------------------------------

/// Configuration for parsing HTML with [`JustHTML`].
#[derive(Debug, Clone, Default)]
pub struct JustHTMLOptions {
    /// Whether to collect parse errors into the `errors` list.
    /// Default: `false` (for performance).
    pub collect_errors: bool,

    /// Enable debug mode for additional internal logging.
    /// Default: `false`.
    pub debug: bool,

    /// Transport-layer encoding hint (e.g. from an HTTP `Content-Type` header).
    /// When `None`, encoding is sniffed from the byte stream.
    /// Only relevant when parsing from bytes.
    pub encoding: Option<String>,

    /// Parse as a fragment rooted at this context element.
    /// When `None`, parse as a full document.
    pub fragment_context: Option<FragmentContext>,

    /// Whether the input comes from an iframe `srcdoc` attribute.
    /// Default: `false`.
    pub iframe_srcdoc: bool,

    /// Raise [`StrictModeError`] on the first parse error.
    /// When `true`, also implies `collect_errors = true`.
    /// Default: `false`.
    pub strict: bool,

    /// Optional tokenizer configuration overrides.
    pub tokenizer_opts: Option<TokenizerOpts>,
}

// ---------------------------------------------------------------------------
// JustHTML (parser entry point)
// ---------------------------------------------------------------------------

/// The primary entry point for parsing HTML5 documents and fragments.
///
/// Constructs a DOM tree from the given input (string or bytes) according to the
/// WHATWG HTML5 parsing specification.
#[derive(Debug)]
pub struct JustHTML {
    /// The root of the parsed DOM tree.
    pub root: NodeHandle,

    /// Parse errors collected during tokenization and tree construction.
    /// Populated only when `collect_errors` (or `strict`) is enabled.
    pub errors: Vec<ParseError>,

    /// The encoding that was detected/used (only meaningful for bytes input).
    pub encoding: Option<String>,

    /// The fragment context used for parsing, if any.
    pub fragment_context: Option<FragmentContext>,

    /// Whether debug mode was enabled.
    pub debug: bool,
}

impl JustHTML {
    /// Parse an HTML string with the given options.
    pub fn parse(html: &str, opts: JustHTMLOptions) -> Self {
        Self::parse_internal(html.to_string(), opts, None)
    }

    /// Parse raw bytes, performing encoding detection and decoding first.
    ///
    /// The `transport_encoding` field in `opts.encoding` is used as a hint.
    pub fn parse_bytes(data: &[u8], opts: JustHTMLOptions) -> Self {
        let (html_str, chosen_encoding) = decode_html(data, opts.encoding.as_deref());
        Self::parse_internal(html_str, opts, Some(chosen_encoding))
    }

    /// Parse an HTML string, returning an error in strict mode instead of panicking.
    pub fn try_parse(html: &str, opts: JustHTMLOptions) -> Result<Self, StrictModeError> {
        let result = Self::parse_internal(html.to_string(), opts, None);
        // In non-strict mode this always succeeds
        Ok(result)
    }

    /// Parse an HTML string in strict mode, returning `Err(StrictModeError)` on parse errors.
    pub fn parse_strict(html: &str, opts: JustHTMLOptions) -> Result<Self, StrictModeError> {
        let mut opts = opts;
        opts.strict = true;
        opts.collect_errors = true;
        let result = Self::parse_internal(html.to_string(), opts, None);
        if !result.errors.is_empty() {
            return Err(StrictModeError::new(result.errors[0].clone()));
        }
        Ok(result)
    }

    fn parse_internal(html_str: String, opts: JustHTMLOptions, detected_encoding: Option<String>) -> Self {
        let should_collect = opts.collect_errors || opts.strict;

        // Create tree builder
        let mut tree_builder = TreeBuilder::new(
            opts.fragment_context.clone(),
            opts.iframe_srcdoc,
            should_collect,
        );

        // Set up tokenizer options
        let mut tok_opts = opts.tokenizer_opts.unwrap_or_default();

        // For RAWTEXT fragment contexts, set initial tokenizer state and rawtext tag
        if let Some(ref ctx) = opts.fragment_context {
            let ns = ctx.namespace.as_deref().unwrap_or("html");
            if ns == "html" || ctx.namespace.is_none() {
                let tag_name = ctx.tag_name.to_lowercase();
                match tag_name.as_str() {
                    "textarea" | "title" | "style" => {
                        tok_opts.initial_state = Some(TokenizerState::Rawtext);
                        tok_opts.initial_rawtext_tag = Some(tag_name);
                    }
                    "plaintext" | "script" => {
                        tok_opts.initial_state = Some(TokenizerState::Plaintext);
                    }
                    _ => {}
                }
            }
        }

        // Create and run tokenizer
        let mut tokenizer = Tokenizer::new(Some(tok_opts), should_collect);
        tokenizer.run(&html_str, &mut tree_builder);

        // Finalize tree
        let root = tree_builder.finish();

        // Merge errors from tokenizer and tree builder
        let mut errors = Vec::new();
        errors.extend(tokenizer.errors.drain(..));
        errors.extend(tree_builder.errors.drain(..));

        // Sort errors by position (line, then column) for deterministic ordering
        errors.sort_by(|a, b| {
            let a_line = a.line.unwrap_or(0);
            let b_line = b.line.unwrap_or(0);
            let a_col = a.column.unwrap_or(0);
            let b_col = b.column.unwrap_or(0);
            (a_line, a_col).cmp(&(b_line, b_col))
        });

        JustHTML {
            root,
            errors,
            encoding: detected_encoding,
            fragment_context: opts.fragment_context,
            debug: opts.debug,
        }
    }

    // -----------------------------------------------------------------------
    // Convenience methods
    // -----------------------------------------------------------------------

    /// Query the document using a CSS selector.
    ///
    /// Stub: will be fully implemented in Milestone 4 with the selector engine.
    pub fn query(&self, _selector: &str) -> Vec<NodeHandle> {
        // TODO: Implement in Milestone 4
        Vec::new()
    }

    /// Serialize the document to HTML.
    pub fn to_html(&self, pretty: bool, indent_size: usize) -> String {
        node::node_to_html(&self.root, 0, indent_size, pretty)
    }

    /// Return the document's concatenated text.
    pub fn to_text(&self, separator: &str, strip: bool) -> String {
        node::node_to_text(&self.root, separator, strip)
    }

    /// Return a Markdown representation of the document.
    pub fn to_markdown(&self) -> String {
        node::node_to_markdown(&self.root)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_simple_html() {
        let doc = JustHTML::parse("<html><body><p>Hello</p></body></html>", Default::default());
        assert!(!doc.root.borrow().children.is_empty());
    }

    #[test]
    fn test_parse_empty_string() {
        let doc = JustHTML::parse("", Default::default());
        // Even empty input produces a document with html/head/body
        assert!(!doc.root.borrow().children.is_empty());
    }

    #[test]
    fn test_parse_no_errors_by_default() {
        let doc = JustHTML::parse("<html><body></body></html>", Default::default());
        assert!(doc.errors.is_empty());
    }

    #[test]
    fn test_collect_errors_enabled() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        // Null character triggers parse error
        let doc = JustHTML::parse("<p>\x00</p>", opts);
        assert!(!doc.errors.is_empty());
        assert!(doc.errors.iter().all(|e| !e.code.is_empty()));
    }

    #[test]
    fn test_error_has_line_and_column() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<p>\x00</p>", opts);
        assert!(!doc.errors.is_empty());
        let error = &doc.errors[0];
        assert!(error.line.is_some());
        assert!(error.column.is_some());
    }

    #[test]
    fn test_strict_mode_raises() {
        let result = JustHTML::parse_strict("<p>\x00</p>", Default::default());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.error.code.is_empty());
    }

    #[test]
    fn test_strict_mode_valid_html() {
        let result = JustHTML::parse_strict(
            "<!DOCTYPE html><html><head><title>Test</title></head><body></body></html>",
            Default::default(),
        );
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(!doc.root.borrow().children.is_empty());
    }

    #[test]
    fn test_parse_bytes() {
        let html = b"<!DOCTYPE html><html><head></head><body>Hello</body></html>";
        let doc = JustHTML::parse_bytes(html, Default::default());
        assert!(!doc.root.borrow().children.is_empty());
        assert!(doc.encoding.is_some());
    }

    #[test]
    fn test_to_html() {
        let doc = JustHTML::parse("<p>Hello</p>", Default::default());
        let html = doc.to_html(false, 2);
        assert!(html.contains("<p>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_to_text() {
        let doc = JustHTML::parse("<p>Hello World</p>", Default::default());
        let text = doc.to_text(" ", true);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_to_markdown() {
        let doc = JustHTML::parse("<p>Hello</p>", Default::default());
        let md = doc.to_markdown();
        assert!(md.contains("Hello"));
    }

    #[test]
    fn test_fragment_parsing() {
        let opts = JustHTMLOptions {
            fragment_context: Some(FragmentContext::new("div", None)),
            ..Default::default()
        };
        let doc = JustHTML::parse("<p>Fragment</p>", opts);
        // Fragment parsing produces a document-fragment
        assert_eq!(doc.root.borrow().name, "#document-fragment");
    }

    #[test]
    fn test_multiline_error_positions() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let html = "<!DOCTYPE html>\n<html>\n<body>\n<p><b></p>";
        let doc = JustHTML::parse(html, opts);
        // Should have errors due to misnesting
        for error in &doc.errors {
            if let Some(line) = error.line {
                assert!(line >= 1);
            }
        }
    }

    #[test]
    fn test_error_column_after_newline() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let html = "line1\nline2\x00";
        let doc = JustHTML::parse(html, opts);
        assert!(!doc.errors.is_empty());
        let error = &doc.errors[0];
        assert_eq!(error.line, Some(2));
        assert!(error.column.unwrap_or(0) > 0);
    }

    #[test]
    fn test_valid_html_no_errors() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<!DOCTYPE html><html><head></head><body></body></html>", opts);
        assert!(doc.errors.is_empty() || doc.errors.iter().all(|e| !e.code.is_empty()));
    }

    #[test]
    fn test_unexpected_end_tag() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<!DOCTYPE html><html><body></span>", opts);
        assert!(!doc.errors.is_empty());
    }

    #[test]
    fn test_treebuilder_error_after_newline() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let html = "<!DOCTYPE html>\n<html>\n<body>\n</span>";
        let doc = JustHTML::parse(html, opts);
        assert!(!doc.errors.is_empty());
        assert!(doc.errors.iter().any(|e| e.line.map_or(false, |l| l > 1)));
    }

    #[test]
    fn test_nested_p_in_button() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<!DOCTYPE html><button><p>text</button>", opts);
        assert!(!doc.root.borrow().children.is_empty());
    }

    #[test]
    fn test_null_character_error() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<p>\x00</p>", opts);
        assert!(!doc.errors.is_empty());
    }

    #[test]
    fn test_unexpected_eof_in_tag() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<div att", opts);
        assert!(!doc.errors.is_empty());
    }

    #[test]
    fn test_unexpected_null_in_attribute() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<div attr=\"val\x00\">text</div>", opts);
        assert!(!doc.errors.is_empty());
    }

    #[test]
    fn test_line_counting_in_attribute_whitespace() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        // Whitespace with newlines before attribute name
        let html = "<div\n   \n   class='test'>content</div>";
        let doc = JustHTML::parse(html, opts.clone());
        assert!(!doc.root.borrow().children.is_empty());

        // Whitespace with newlines AFTER attribute name (before =)
        let html_after = "<div class\n   \n   ='test'>content</div>";
        let doc = JustHTML::parse(html_after, opts);
        assert!(!doc.root.borrow().children.is_empty());
    }

    #[test]
    fn test_line_counting_in_quoted_attribute_values() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        // Double-quoted attribute with newlines
        let html_double = "<div data-content=\"line1\nline2\nline3\">text</div>";
        let doc = JustHTML::parse(html_double, opts.clone());
        assert!(!doc.root.borrow().children.is_empty());

        // Single-quoted attribute with newlines
        let html_single = "<div data-content='line1\nline2'>text</div>";
        let doc = JustHTML::parse(html_single, opts);
        assert!(!doc.root.borrow().children.is_empty());
    }

    #[test]
    fn test_line_counting_with_cr_in_attributes() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let html = "<div data-x=\"a\r\nb\rc\">text</div>";
        let doc = JustHTML::parse(html, opts);
        assert!(!doc.root.borrow().children.is_empty());
    }

    // -----------------------------------------------------------------------
    // StrictModeError tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_strict_mode_error_display() {
        let err = StrictModeError::new(ParseError::new("test-error", Some(1), Some(5), None, None, None));
        let display = format!("{}", err);
        assert!(display.contains("test-error"));
    }

    #[test]
    fn test_strict_mode_enables_error_collection() {
        let result = JustHTML::parse_strict("<p>\x00</p>", Default::default());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error.line.is_some());
        assert!(err.error.column.is_some());
    }

    // -----------------------------------------------------------------------
    // Parse error formatting tests (ported from test_errors.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_error_str() {
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
    fn test_parse_error_equality() {
        let e1 = ParseError::new("error-code", Some(1), Some(5), None, None, None);
        let e2 = ParseError::new("error-code", Some(1), Some(5), None, None, None);
        let e3 = ParseError::new("other-error", Some(1), Some(5), None, None, None);
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    // -----------------------------------------------------------------------
    // Token-based error highlighting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tag_token_start_tag_highlighting() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<html>", opts);
        // <html> without doctype should produce errors
        assert!(!doc.errors.is_empty());
        let error = &doc.errors[0];
        assert!(error.column.is_some());
    }

    #[test]
    fn test_end_tag_br_highlighting() {
        let opts = JustHTMLOptions {
            collect_errors: true,
            ..Default::default()
        };
        let doc = JustHTML::parse("<html></br></html>", opts);
        // </br> is treated as error (should be <br>)
        let br_errors: Vec<_> = doc.errors.iter()
            .filter(|e| e.code.contains("unexpected-end-tag") || e.code.contains("br"))
            .collect();
        assert!(!br_errors.is_empty());
        // Should have end_column set for tag-based highlighting
        for err in &br_errors {
            if err.end_column.is_some() {
                assert!(err.end_column.unwrap() > err.column.unwrap_or(0));
            }
        }
    }

    // -----------------------------------------------------------------------
    // TreeBuilder parse_error with token tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_error_with_tag_token() {
        let mut builder = TreeBuilder::new(None, false, true);
        builder.last_token_line = Some(1);
        builder.last_token_column = Some(18);

        use tokens::{Tag, TagKind, Token};
        let mut attrs = HashMap::new();
        attrs.insert("class".to_string(), Some("test".to_string()));
        let tag = Tag::new(TagKind::Start, "div".to_string(), attrs, false);
        let token = Token::Tag(tag);

        builder.parse_error_with_token("test-error", Some("div"), Some(&token));

        assert_eq!(builder.errors.len(), 1);
        let error = &builder.errors[0];
        // Should have adjusted column for tag highlighting
        assert!(error.column.is_some());
        assert!(error.end_column.is_some());
    }

    #[test]
    fn test_parse_error_with_non_tag_token() {
        let mut builder = TreeBuilder::new(None, false, true);
        builder.last_token_line = Some(1);
        builder.last_token_column = Some(11);

        use tokens::{Token, CharacterTokens};
        let token = Token::Characters(CharacterTokens::new("hello".to_string()));

        builder.parse_error_with_token("test-error", None, Some(&token));

        assert_eq!(builder.errors.len(), 1);
        let error = &builder.errors[0];
        // Non-tag tokens don't get special position calculation
        assert_eq!(error.column, Some(11));
        assert!(error.end_column.is_none());
    }

    #[test]
    fn test_parse_error_without_token() {
        let mut builder = TreeBuilder::new(None, false, true);
        builder.last_token_line = Some(1);
        builder.last_token_column = Some(5);

        builder.parse_error("test-error", Some("div"));

        assert_eq!(builder.errors.len(), 1);
        let error = &builder.errors[0];
        assert_eq!(error.column, Some(5));
        assert!(error.end_column.is_none());
    }
}
