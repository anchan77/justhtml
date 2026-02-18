//! HTML5 tokenizer state machine.
//!
//! Implements the complete HTML5 tokenization algorithm per WHATWG spec §13.2.5.
//!
//! Corresponds to Python's `tokenizer.py`.

mod states;

use std::collections::HashMap;

use crate::errors::generate_error_message;
use crate::tokens::{
    CharacterTokens, CommentToken, Doctype, DoctypeToken, EOFToken, ParseError, Tag, TagKind,
    Token, TokenSinkResult,
};

/// Trait that receives tokens from the tokenizer.
pub trait TokenSink {
    /// Process a token. Return `Plaintext` to switch the tokenizer to plaintext mode.
    fn process_token(&mut self, token: Token) -> TokenSinkResult;

    /// Process character data (may be called multiple times with adjacent text).
    fn process_characters(&mut self, data: &str) -> TokenSinkResult;
}

/// Tokenizer configuration options.
#[derive(Debug, Clone)]
pub struct TokenizerOpts {
    /// Whether to emit exact error positions (slower but more precise).
    pub exact_errors: bool,
    /// Whether to discard BOM at start of input.
    pub discard_bom: bool,
    /// Initial tokenizer state override.
    pub initial_state: Option<TokenizerState>,
    /// If set, assume this tag is the "last start tag" for RAWTEXT/RCDATA.
    pub initial_rawtext_tag: Option<String>,
    /// Whether to coerce output for XML compatibility.
    pub xml_coercion: bool,
}

impl Default for TokenizerOpts {
    fn default() -> Self {
        Self {
            exact_errors: false,
            discard_bom: true,
            initial_state: None,
            initial_rawtext_tag: None,
            xml_coercion: false,
        }
    }
}

/// The 61 states of the HTML5 tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenizerState {
    Data = 0,
    TagOpen = 1,
    EndTagOpen = 2,
    TagName = 3,
    BeforeAttributeName = 4,
    AttributeName = 5,
    AfterAttributeName = 6,
    BeforeAttributeValue = 7,
    AttributeValueDouble = 8,
    AttributeValueSingle = 9,
    AttributeValueUnquoted = 10,
    AfterAttributeValueQuoted = 11,
    SelfClosingStartTag = 12,
    MarkupDeclarationOpen = 13,
    CommentStart = 14,
    CommentStartDash = 15,
    Comment = 16,
    CommentEndDash = 17,
    CommentEnd = 18,
    CommentEndBang = 19,
    BogusComment = 20,
    Doctype = 21,
    BeforeDoctypeName = 22,
    DoctypeName = 23,
    AfterDoctypeName = 24,
    BogusDoctype = 25,
    AfterDoctypePublicKeyword = 26,
    BeforeDoctypePublicIdentifier = 27,
    DoctypePublicIdentifierDouble = 28,
    DoctypePublicIdentifierSingle = 29,
    AfterDoctypePublicIdentifier = 30,
    BetweenDoctypePublicAndSystem = 31,
    AfterDoctypeSystemKeyword = 32,
    BeforeDoctypeSystemIdentifier = 33,
    DoctypeSystemIdentifierDouble = 34,
    DoctypeSystemIdentifierSingle = 35,
    AfterDoctypeSystemIdentifier = 36,
    CdataSection = 37,
    CdataSectionBracket = 38,
    CdataSectionEnd = 39,
    Rcdata = 40,
    RcdataLessThanSign = 41,
    RcdataEndTagOpen = 42,
    RcdataEndTagName = 43,
    Rawtext = 44,
    RawtextLessThanSign = 45,
    RawtextEndTagOpen = 46,
    RawtextEndTagName = 47,
    Plaintext = 48,
    ScriptData = 49,
    ScriptDataLessThanSign = 50,
    ScriptDataEndTagOpen = 51,
    ScriptDataEndTagName = 52,
    ScriptDataEscapeStart = 53,
    ScriptDataEscapeStartDash = 54,
    ScriptDataEscaped = 55,
    ScriptDataEscapedDash = 56,
    ScriptDataEscapedDashDash = 57,
    ScriptDataEscapedLessThanSign = 58,
    ScriptDataEscapedEndTagOpen = 59,
    ScriptDataEscapedEndTagName = 60,
    ScriptDataDoubleEscapeStart = 61,
    ScriptDataDoubleEscaped = 62,
    ScriptDataDoubleEscapedDash = 63,
    ScriptDataDoubleEscapedDashDash = 64,
    ScriptDataDoubleEscapedLessThanSign = 65,
    ScriptDataDoubleEscapeEnd = 66,
    // Character reference states handled inline
    CharacterReference = 67,
    NamedCharacterReference = 68,
    NumericCharacterReference = 69,
}

/// Tags that switch the tokenizer to RAWTEXT mode.
const RAWTEXT_SWITCH_TAGS: &[&str] = &[
    "style", "script", "xmp", "iframe", "noembed", "noframes", "noscript", "plaintext",
];

/// RCDATA elements.
const RCDATA_ELEMENTS: &[&str] = &["title", "textarea"];

/// Characters that terminate an unquoted attribute value.
#[allow(dead_code)]
const ATTR_VALUE_UNQUOTED_TERMINATORS: &[char] = &[
    '\t', '\n', '\x0C', ' ', '>', '"', '\'', '`', '=', '<',
];

/// The main HTML5 tokenizer.
pub struct Tokenizer {
    // Input
    pub buffer: String,
    pos: usize,
    length: usize,
    reconsume: bool,

    // State
    state: TokenizerState,

    // Current token being built
    current_tag_kind: TagKind,
    current_tag_name: String,
    current_tag_attrs: Vec<(String, Option<String>)>,
    current_attr_name: String,
    current_attr_value: String,
    current_attr_has_value: bool,
    current_tag_self_closing: bool,
    current_comment: String,
    current_doctype_name: Option<String>,
    current_doctype_public: Option<String>,
    current_doctype_system: Option<String>,
    current_doctype_force_quirks: bool,

    // Text accumulation
    text_buffer: String,
    temp_buffer: String,

    // Tag matching
    last_start_tag_name: Option<String>,
    rawtext_tag_name: Option<String>,

    // Options
    pub opts: TokenizerOpts,
    pub collect_errors: bool,

    // Position tracking
    pub last_token_line: usize,
    pub last_token_column: usize,
    text_start_pos: usize,
    newline_positions: Vec<usize>,

    // Error collection
    pub errors: Vec<ParseError>,

    // Sink management
    is_done: bool,
}

impl Tokenizer {
    /// Create a new tokenizer.
    pub fn new(opts: Option<TokenizerOpts>, collect_errors: bool) -> Self {
        let opts = opts.unwrap_or_default();
        let initial_state = opts.initial_state.unwrap_or(TokenizerState::Data);
        let rawtext_tag = opts.initial_rawtext_tag.clone();

        Self {
            buffer: String::new(),
            pos: 0,
            length: 0,
            reconsume: false,
            state: initial_state,
            current_tag_kind: TagKind::Start,
            current_tag_name: String::new(),
            current_tag_attrs: Vec::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            current_attr_has_value: false,
            current_tag_self_closing: false,
            current_comment: String::new(),
            current_doctype_name: None,
            current_doctype_public: None,
            current_doctype_system: None,
            current_doctype_force_quirks: false,
            text_buffer: String::new(),
            temp_buffer: String::new(),
            last_start_tag_name: rawtext_tag.clone(),
            rawtext_tag_name: rawtext_tag,
            opts,
            collect_errors,
            last_token_line: 1,
            last_token_column: 0,
            text_start_pos: 0,
            newline_positions: Vec::new(),
            errors: Vec::new(),
            is_done: false,
        }
    }

    /// Initialize with HTML input string.
    pub fn initialize(&mut self, html: &str) {
        let mut input = html.to_string();

        // Normalize newlines: \r\n -> \n, \r -> \n
        input = input.replace("\r\n", "\n").replace('\r', "\n");

        // Discard BOM
        if self.opts.discard_bom && input.starts_with('\u{FEFF}') {
            input = input['\u{FEFF}'.len_utf8()..].to_string();
        }

        // Build newline position index
        self.newline_positions.clear();
        for (i, ch) in input.char_indices() {
            if ch == '\n' {
                self.newline_positions.push(i);
            }
        }

        self.buffer = input;
        self.length = self.buffer.len();
        self.pos = 0;
        self.reconsume = false;
        self.text_start_pos = 0;
    }

    /// Run the tokenizer to completion, sending tokens to the sink.
    pub fn run(&mut self, html: &str, sink: &mut dyn TokenSink) {
        self.initialize(html);

        loop {
            if self.step(sink) {
                break;
            }
        }
    }

    /// Run a single step of the tokenizer. Returns true at EOF.
    pub fn step(&mut self, sink: &mut dyn TokenSink) -> bool {
        if self.is_done {
            return true;
        }

        let result = states::dispatch_state(self, sink);

        if self.is_done {
            return true;
        }

        result
    }

    // ─── Character access helpers ────────────────────────────────────────

    /// Get the next character, advancing position.
    pub(crate) fn get_char(&mut self) -> Option<char> {
        if self.reconsume {
            self.reconsume = false;
            if self.pos > 0 {
                let prev_pos = self.prev_char_pos();
                return self.buffer[prev_pos..].chars().next();
            }
            return None;
        }

        if self.pos >= self.length {
            return None;
        }

        let ch = self.buffer[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Peek at the next character without consuming it.
    #[allow(dead_code)]
    pub(crate) fn peek_char(&self, offset: usize) -> Option<char> {
        let target = self.pos + offset;
        if target >= self.length {
            return None;
        }
        self.buffer[target..].chars().next()
    }

    /// Mark the current character for reconsumption.
    pub(crate) fn reconsume_current(&mut self) {
        self.reconsume = true;
    }

    /// Get byte position of the character before current pos.
    fn prev_char_pos(&self) -> usize {
        let s = &self.buffer[..self.pos];
        s.char_indices().next_back().map(|(i, _)| i).unwrap_or(0)
    }

    // ─── Text accumulation ───────────────────────────────────────────────

    /// Append text to the output buffer.
    pub(crate) fn append_text(&mut self, text: &str) {
        if self.text_buffer.is_empty() {
            self.text_start_pos = self.pos.saturating_sub(text.len());
        }
        self.text_buffer.push_str(text);
    }

    /// Append a single character to the text buffer.
    pub(crate) fn append_text_char(&mut self, ch: char) {
        if self.text_buffer.is_empty() {
            self.text_start_pos = self.pos.saturating_sub(ch.len_utf8());
        }
        self.text_buffer.push(ch);
    }

    /// Flush accumulated text to the sink.
    pub(crate) fn flush_text(&mut self, sink: &mut dyn TokenSink) -> TokenSinkResult {
        if self.text_buffer.is_empty() {
            return TokenSinkResult::Continue;
        }

        let text = std::mem::take(&mut self.text_buffer);

        // Apply XML coercion if needed
        let text = if self.opts.xml_coercion {
            coerce_text_for_xml(&text)
        } else {
            text
        };

        let result = sink.process_token(Token::Characters(CharacterTokens::new(text)));
        result
    }

    // ─── Attribute handling ──────────────────────────────────────────────

    /// Finish the current attribute and add it to the tag.
    pub(crate) fn finish_attribute(&mut self) {
        if self.current_attr_name.is_empty() {
            return;
        }

        let name = std::mem::take(&mut self.current_attr_name);
        let value = if self.current_attr_has_value {
            Some(std::mem::take(&mut self.current_attr_value))
        } else {
            self.current_attr_value.clear();
            None
        };

        // Check for duplicate
        let is_dup = self.current_tag_attrs.iter().any(|(n, _)| n == &name);
        if is_dup {
            if self.collect_errors {
                self.emit_error("duplicate-attribute");
            }
        } else {
            self.current_tag_attrs.push((name, value));
        }

        self.current_attr_has_value = false;
    }

    /// Append a character to the current attribute value.
    pub(crate) fn append_attr_value_char(&mut self, ch: char) {
        self.current_attr_has_value = true;
        self.current_attr_value.push(ch);
    }

    /// Append a string to the current attribute value.
    pub(crate) fn append_attr_value_str(&mut self, s: &str) {
        self.current_attr_has_value = true;
        self.current_attr_value.push_str(s);
    }

    // ─── Token emission ──────────────────────────────────────────────────

    /// Emit the current tag token.
    /// Returns true if the tokenizer should switch to raw text mode.
    pub(crate) fn emit_current_tag(&mut self, sink: &mut dyn TokenSink) -> bool {
        self.finish_attribute();
        self.record_token_position();

        let name = std::mem::take(&mut self.current_tag_name);
        let kind = self.current_tag_kind;
        let self_closing = self.current_tag_self_closing;

        let attrs: HashMap<String, Option<String>> = std::mem::take(&mut self.current_tag_attrs)
            .into_iter()
            .collect();

        let tag = Tag::new(kind, name.clone(), attrs, self_closing);

        // Reset tag state
        self.current_tag_self_closing = false;

        if kind == TagKind::Start {
            self.last_start_tag_name = Some(name.clone());
        }

        // Flush text before emitting tag
        self.flush_text(sink);

        let result = sink.process_token(Token::Tag(tag));

        // Check if we should switch to rawtext/rcdata mode
        if kind == TagKind::Start {
            if result == TokenSinkResult::Plaintext {
                self.state = TokenizerState::Plaintext;
                return true;
            }

            if RCDATA_ELEMENTS.contains(&name.as_str()) {
                self.state = TokenizerState::Rcdata;
                self.rawtext_tag_name = Some(name);
                return true;
            }

            if name == "script" {
                self.state = TokenizerState::ScriptData;
                self.rawtext_tag_name = Some(name);
                return true;
            }

            if RAWTEXT_SWITCH_TAGS.contains(&name.as_str()) && name != "script" && name != "plaintext" {
                self.state = TokenizerState::Rawtext;
                self.rawtext_tag_name = Some(name);
                return true;
            }

            if name == "plaintext" {
                self.state = TokenizerState::Plaintext;
                return true;
            }
        }

        false
    }

    /// Emit a comment token.
    pub(crate) fn emit_comment(&mut self, sink: &mut dyn TokenSink) {
        self.record_token_position();
        self.flush_text(sink);

        let data = std::mem::take(&mut self.current_comment);

        // Apply XML coercion if needed
        let data = if self.opts.xml_coercion {
            coerce_comment_for_xml(&data)
        } else {
            data
        };

        sink.process_token(Token::Comment(CommentToken::new(data)));
    }

    /// Emit a doctype token.
    pub(crate) fn emit_doctype(&mut self, sink: &mut dyn TokenSink) {
        self.record_token_position();
        self.flush_text(sink);

        let doctype = Doctype {
            name: self.current_doctype_name.take(),
            public_id: self.current_doctype_public.take(),
            system_id: self.current_doctype_system.take(),
            force_quirks: self.current_doctype_force_quirks,
        };
        self.current_doctype_force_quirks = false;

        sink.process_token(Token::Doctype(DoctypeToken::new(doctype)));
    }

    /// Emit EOF token.
    pub(crate) fn emit_eof(&mut self, sink: &mut dyn TokenSink) {
        self.flush_text(sink);
        sink.process_token(Token::EOF(EOFToken));
        self.is_done = true;
    }

    // ─── Error handling ──────────────────────────────────────────────────

    /// Record a parse error.
    pub(crate) fn emit_error(&mut self, code: &str) {
        if !self.collect_errors {
            return;
        }

        let (line, column) = self.get_current_position();
        let message = generate_error_message(code, None);

        self.errors.push(ParseError::new(
            code,
            Some(line),
            Some(column),
            if message != code { Some(&message) } else { None },
            Some(self.buffer.clone()),
            None,
        ));
    }

    // ─── Position tracking ───────────────────────────────────────────────

    /// Record the current position as the token position.
    pub(crate) fn record_token_position(&mut self) {
        let (line, col) = self.get_current_position();
        self.last_token_line = line;
        self.last_token_column = col;
    }

    /// Get the (line, column) for the current position.
    pub(crate) fn get_current_position(&self) -> (usize, usize) {
        let pos = if self.pos > 0 { self.pos - 1 } else { 0 };
        self.get_position_at(pos)
    }

    /// Get (line, column) for a given byte position.
    pub(crate) fn get_position_at(&self, pos: usize) -> (usize, usize) {
        if self.newline_positions.is_empty() {
            return (1, pos + 1);
        }

        // Binary search for the line
        let line_idx = match self.newline_positions.binary_search(&pos) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };

        let line = line_idx + 1;
        let col = if line_idx == 0 {
            pos + 1
        } else {
            pos - self.newline_positions[line_idx - 1]
        };

        (line, col)
    }

    // ─── Consume helpers ─────────────────────────────────────────────────

    /// Try to consume a case-insensitive string. Returns true if consumed.
    pub(crate) fn consume_case_insensitive(&mut self, literal: &str) -> bool {
        let remaining = &self.buffer[self.pos..];
        if remaining.len() < literal.len() {
            return false;
        }

        let chunk = &remaining[..literal.len()];
        if chunk.eq_ignore_ascii_case(literal) {
            self.pos += literal.len();
            true
        } else {
            false
        }
    }

    /// Try to consume an exact string. Returns true if consumed.
    pub(crate) fn consume_exact(&mut self, literal: &str) -> bool {
        let remaining = &self.buffer[self.pos..];
        if remaining.starts_with(literal) {
            self.pos += literal.len();
            true
        } else {
            false
        }
    }

    /// Check if the temp buffer matches the last start tag name (case-insensitive).
    pub(crate) fn temp_buffer_matches_last_start_tag(&self) -> bool {
        match &self.last_start_tag_name {
            Some(name) => self.temp_buffer.eq_ignore_ascii_case(name),
            None => false,
        }
    }

    /// Check if current position is at an appropriate end tag.
    pub(crate) fn is_appropriate_end_tag(&self) -> bool {
        self.temp_buffer_matches_last_start_tag()
    }
}

// ─── XML coercion helpers ────────────────────────────────────────────────────

/// Replace characters invalid in XML with the replacement character.
fn coerce_text_for_xml(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        let cp = ch as u32;
        if cp == 0x09 || cp == 0x0A || cp == 0x0D || (0x20..=0xD7FF).contains(&cp)
            || (0xE000..=0xFFFD).contains(&cp) || (0x10000..=0x10FFFF).contains(&cp)
        {
            result.push(ch);
        } else {
            result.push('\u{FFFD}');
        }
    }
    result
}

/// Replace `--` sequences in comments for XML compatibility.
fn coerce_comment_for_xml(text: &str) -> String {
    text.replace("--", "- -")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test sink that collects tokens.
    struct CollectorSink {
        tokens: Vec<String>,
    }

    impl CollectorSink {
        fn new() -> Self {
            Self { tokens: Vec::new() }
        }
    }

    impl TokenSink for CollectorSink {
        fn process_token(&mut self, token: Token) -> TokenSinkResult {
            match token {
                Token::Tag(ref tag) => {
                    let kind = if tag.kind == TagKind::Start { "start" } else { "end" };
                    self.tokens.push(format!("<{} {}>", kind, tag.name));
                }
                Token::Characters(ref chars) => {
                    self.tokens.push(format!("text:{}", chars.data));
                }
                Token::Comment(ref comment) => {
                    self.tokens.push(format!("comment:{}", comment.data));
                }
                Token::Doctype(ref dt) => {
                    self.tokens.push(format!(
                        "doctype:{}",
                        dt.doctype.name.as_deref().unwrap_or("")
                    ));
                }
                Token::EOF(_) => {
                    self.tokens.push("EOF".to_string());
                }
            }
            TokenSinkResult::Continue
        }

        fn process_characters(&mut self, data: &str) -> TokenSinkResult {
            self.tokens.push(format!("chars:{}", data));
            TokenSinkResult::Continue
        }
    }

    #[test]
    fn test_tokenizer_creation() {
        let tok = Tokenizer::new(None, false);
        assert_eq!(tok.state, TokenizerState::Data);
        assert!(!tok.collect_errors);
    }

    #[test]
    fn test_tokenizer_simple_tag() {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = CollectorSink::new();
        tok.run("<div>hello</div>", &mut sink);
        assert!(sink.tokens.iter().any(|t| t == "<start div>"));
        assert!(sink.tokens.iter().any(|t| t == "<end div>"));
        assert!(sink.tokens.iter().any(|t| t.contains("hello")));
    }

    #[test]
    fn test_tokenizer_comment() {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = CollectorSink::new();
        tok.run("<!-- test comment -->", &mut sink);
        assert!(sink.tokens.iter().any(|t| t == "comment: test comment "));
    }

    #[test]
    fn test_tokenizer_doctype() {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = CollectorSink::new();
        tok.run("<!DOCTYPE html>", &mut sink);
        assert!(sink.tokens.iter().any(|t| t == "doctype:html"));
    }

    #[test]
    fn test_tokenizer_self_closing() {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = CollectorSink::new();
        tok.run("<br/>", &mut sink);
        assert!(sink.tokens.iter().any(|t| t == "<start br>"));
    }

    #[test]
    fn test_tokenizer_attributes() {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = CollectorSink::new();
        tok.run("<div class=\"test\" id='foo'>", &mut sink);
        assert!(sink.tokens.iter().any(|t| t == "<start div>"));
    }

    #[test]
    fn test_tokenizer_error_collection() {
        let mut tok = Tokenizer::new(None, true);
        let mut sink = CollectorSink::new();
        tok.run("<p>\x00</p>", &mut sink);
        assert!(!tok.errors.is_empty());
    }

    #[test]
    fn test_tokenizer_newline_normalization() {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = CollectorSink::new();
        tok.run("a\r\nb\rc", &mut sink);
        // After normalization, \r\n -> \n and \r -> \n
        let text_tokens: Vec<&String> = sink.tokens.iter().filter(|t| t.starts_with("text:")).collect();
        // Should have normalized text
        assert!(text_tokens.iter().any(|t| t.contains("a\nb\nc")));
    }

    #[test]
    fn test_xml_coercion_text() {
        assert_eq!(coerce_text_for_xml("hello"), "hello");
        assert_eq!(coerce_text_for_xml("a\x01b"), "a\u{FFFD}b");
    }

    #[test]
    fn test_xml_coercion_comment() {
        assert_eq!(coerce_comment_for_xml("no dashes"), "no dashes");
        assert_eq!(coerce_comment_for_xml("a--b"), "a- -b");
    }
}
