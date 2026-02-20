//! HTML5 tokenizer state machine.
//!
//! Implements the complete HTML5 tokenization algorithm per WHATWG spec §13.2.5.
//!
//! Corresponds to Python's `tokenizer.py`.

mod states;

use std::collections::HashMap;

use crate::entities::decode_entities_in_text;
use crate::errors::generate_error_message;
use crate::tokens::{
    CommentToken, Doctype, DoctypeToken, EOFToken, ParseError, Tag, TagKind,
    Token, TokenSinkResult,
};

/// Trait that receives tokens from the tokenizer.
pub trait TokenSink {
    /// Process a token. Return `Plaintext` to switch the tokenizer to plaintext mode.
    fn process_token(&mut self, token: Token) -> TokenSinkResult;

    /// Process character data (may be called multiple times with adjacent text).
    fn process_characters(&mut self, data: &str) -> TokenSinkResult;

    /// Called by the tokenizer before emitting each token to provide position
    /// info and the full source buffer. The default implementation does nothing.
    fn set_token_position(&mut self, _line: usize, _column: usize, _buffer: &str) {}
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

/// Tags that may switch the tokenizer to RAWTEXT/RCDATA/ScriptData/Plaintext mode.
/// This mirrors the Python `_RAWTEXT_SWITCH_TAGS` set used to decide if we need a
/// rawtext check.
const RAWTEXT_SWITCH_TAGS: &[&str] = &[
    "script", "style", "xmp", "iframe", "noembed", "noframes", "textarea", "title",
];

/// RCDATA elements (subset of RAWTEXT_SWITCH_TAGS that use RCDATA mode).
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
    /// Tracks whether the last `get_char()` returned `None` (EOF).
    /// Needed so that `reconsume_current()` after EOF correctly returns `None`
    /// instead of the last consumed character.
    at_eof: bool,

    // State
    state: TokenizerState,

    // Current token being built
    current_tag_kind: TagKind,
    current_tag_name: String,
    current_tag_attrs: Vec<(String, Option<String>)>,
    current_attr_name: String,
    current_attr_value: String,
    current_attr_has_value: bool,
    current_attr_value_has_amp: bool,
    current_tag_self_closing: bool,
    current_comment: String,
    current_doctype_name: Option<String>,
    current_doctype_public: Option<String>,
    current_doctype_system: Option<String>,
    current_doctype_force_quirks: bool,

    // Text accumulation
    text_buffer: String,
    temp_buffer: String,

    // Original (unfolded) tag name for RCDATA/RAWTEXT end tag recovery
    #[allow(dead_code)]
    original_tag_name: String,

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
            at_eof: false,
            state: initial_state,
            current_tag_kind: TagKind::Start,
            current_tag_name: String::new(),
            current_tag_attrs: Vec::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            current_attr_has_value: false,
            current_attr_value_has_amp: false,
            current_tag_self_closing: false,
            current_comment: String::new(),
            current_doctype_name: None,
            current_doctype_public: None,
            current_doctype_system: None,
            current_doctype_force_quirks: false,
            text_buffer: String::new(),
            temp_buffer: String::new(),
            original_tag_name: String::new(),
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
        self.at_eof = false;
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
            // If the previous get_char() returned None (EOF), reconsuming
            // must also return None to avoid an infinite loop.
            if self.at_eof {
                return None;
            }
            if self.pos > 0 {
                let prev_pos = self.prev_char_pos();
                return self.buffer[prev_pos..].chars().next();
            }
            return None;
        }

        if self.pos >= self.length {
            self.at_eof = true;
            return None;
        }

        self.at_eof = false;
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
    pub(crate) fn prev_char_pos(&self) -> usize {
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
    /// Mirrors Python's `_flush_text`: decodes character references for DATA/RCDATA
    /// but NOT for RAWTEXT, PLAINTEXT, or CDATA states.
    pub(crate) fn flush_text(&mut self, sink: &mut dyn TokenSink) -> TokenSinkResult {
        if self.text_buffer.is_empty() {
            return TokenSinkResult::Continue;
        }

        let mut data = std::mem::take(&mut self.text_buffer);

        // Decode character references based on current state (matching Python _flush_text).
        // RAWTEXT (>=44), PLAINTEXT (>=48), CDATA (37-39) states do NOT decode.
        let state_val = self.state as u8;
        let is_rawtext_or_above = state_val >= TokenizerState::Rawtext as u8;
        let is_cdata = (TokenizerState::CdataSection as u8..=TokenizerState::CdataSectionEnd as u8)
            .contains(&state_val);

        if !is_rawtext_or_above && !is_cdata {
            if data.contains('&') {
                data = decode_entities_in_text(&data, false);
            }
        }

        // Apply XML coercion if needed
        let data = if self.opts.xml_coercion {
            coerce_text_for_xml(&data)
        } else {
            data
        };

        // Record position
        if self.collect_errors {
            self.record_token_position();
        }

        sink.set_token_position(self.last_token_line, self.last_token_column, &self.buffer);
        sink.process_characters(&data)
    }

    // ─── Attribute handling ──────────────────────────────────────────────

    /// Finish the current attribute and add it to the tag.
    /// Mirrors Python's `_finish_attribute`: decodes entities in the value if
    /// `current_attr_value_has_amp` is true.
    pub(crate) fn finish_attribute(&mut self) {
        if self.current_attr_name.is_empty() {
            return;
        }

        let name = std::mem::take(&mut self.current_attr_name);

        // Check for duplicate first
        let is_dup = self.current_tag_attrs.iter().any(|(n, _)| n == &name);
        if is_dup {
            if self.collect_errors {
                self.emit_error("duplicate-attribute");
            }
            self.current_attr_value.clear();
            self.current_attr_has_value = false;
            self.current_attr_value_has_amp = false;
            return;
        }

        let value = if self.current_attr_has_value {
            let mut val = std::mem::take(&mut self.current_attr_value);
            // Decode entities in attribute value (deferred from parsing time)
            if self.current_attr_value_has_amp {
                val = decode_entities_in_text(&val, true);
            }
            Some(val)
        } else {
            self.current_attr_value.clear();
            None
        };

        self.current_tag_attrs.push((name, value));
        self.current_attr_has_value = false;
        self.current_attr_value_has_amp = false;
    }

    /// Append a character to the current attribute value.
    pub(crate) fn append_attr_value_char(&mut self, ch: char) {
        self.current_attr_has_value = true;
        self.current_attr_value.push(ch);
    }

    /// Append a string to the current attribute value.
    #[allow(dead_code)]
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

        sink.set_token_position(self.last_token_line, self.last_token_column, &self.buffer);
        let result = sink.process_token(Token::Tag(tag));

        // Check if we should switch to rawtext/rcdata mode.
        // Python checks `self.sink.open_elements` for namespace, but here we
        // just check whether the name is in the rawtext switch set. The tree builder
        // (Milestone 2) will refine this with namespace awareness.
        let mut switched_to_rawtext = false;

        if kind == TagKind::Start {
            if result == TokenSinkResult::Plaintext {
                self.state = TokenizerState::Plaintext;
                switched_to_rawtext = true;
            } else {
                let needs_rawtext_check = RAWTEXT_SWITCH_TAGS.contains(&name.as_str()) || name == "plaintext";
                if needs_rawtext_check {
                    if RCDATA_ELEMENTS.contains(&name.as_str()) {
                        self.state = TokenizerState::Rcdata;
                        self.rawtext_tag_name = Some(name);
                        switched_to_rawtext = true;
                    } else if name == "script" {
                        // Script uses "RAWTEXT" mode in the Python implementation
                        // (which includes special escape handling via `_state_rawtext` that
                        // checks rawtext_tag_name == "script")
                        self.state = TokenizerState::Rawtext;
                        self.rawtext_tag_name = Some(name);
                        switched_to_rawtext = true;
                    } else if name == "plaintext" {
                        self.state = TokenizerState::Plaintext;
                        switched_to_rawtext = true;
                    } else {
                        // style, xmp, iframe, noembed, noframes
                        self.state = TokenizerState::Rawtext;
                        self.rawtext_tag_name = Some(name);
                        switched_to_rawtext = true;
                    }
                }
            }
        }

        switched_to_rawtext
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

        sink.set_token_position(self.last_token_line, self.last_token_column, &self.buffer);
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

        sink.set_token_position(self.last_token_line, self.last_token_column, &self.buffer);
        sink.process_token(Token::Doctype(DoctypeToken::new(doctype)));
    }

    /// Emit EOF token.
    pub(crate) fn emit_eof(&mut self, sink: &mut dyn TokenSink) {
        self.flush_text(sink);
        sink.set_token_position(self.last_token_line, self.last_token_column, &self.buffer);
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
        eprintln!("DOCTYPE test tokens: {:?}", sink.tokens);
        assert!(sink.tokens.iter().any(|t| t == "doctype:html"),
            "Expected 'doctype:html' but got: {:?}", sink.tokens);
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
        let text_tokens: Vec<&String> = sink.tokens.iter().filter(|t| t.starts_with("chars:")).collect();
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

    // ─── Extended tokenizer tests ────────────────────────────────────────────

    /// A richer test sink that captures full token details for assertions.
    struct DetailedSink {
        events: Vec<TokenEvent>,
    }

    #[derive(Debug, Clone)]
    enum TokenEvent {
        StartTag { name: String, attrs: HashMap<String, Option<String>>, self_closing: bool },
        EndTag { name: String },
        Characters(String),
        Comment(String),
        Doctype { name: Option<String>, public_id: Option<String>, system_id: Option<String>, force_quirks: bool },
        Eof,
    }

    impl DetailedSink {
        fn new() -> Self {
            Self { events: Vec::new() }
        }
        fn start_tags(&self) -> Vec<&str> {
            self.events.iter().filter_map(|e| match e {
                TokenEvent::StartTag { name, .. } => Some(name.as_str()),
                _ => None,
            }).collect()
        }
        fn end_tags(&self) -> Vec<&str> {
            self.events.iter().filter_map(|e| match e {
                TokenEvent::EndTag { name, .. } => Some(name.as_str()),
                _ => None,
            }).collect()
        }
        fn all_text(&self) -> String {
            self.events.iter().filter_map(|e| match e {
                TokenEvent::Characters(s) => Some(s.as_str()),
                _ => None,
            }).collect::<Vec<_>>().join("")
        }
        fn comments(&self) -> Vec<&str> {
            self.events.iter().filter_map(|e| match e {
                TokenEvent::Comment(data) => Some(data.as_str()),
                _ => None,
            }).collect()
        }
        fn doctypes(&self) -> Vec<&TokenEvent> {
            self.events.iter().filter(|e| matches!(e, TokenEvent::Doctype { .. })).collect()
        }
    }

    impl TokenSink for DetailedSink {
        fn process_token(&mut self, token: Token) -> TokenSinkResult {
            match token {
                Token::Tag(tag) => {
                    if tag.kind == TagKind::Start {
                        self.events.push(TokenEvent::StartTag {
                            name: tag.name.clone(),
                            attrs: tag.attrs.clone(),
                            self_closing: tag.self_closing,
                        });
                    } else {
                        self.events.push(TokenEvent::EndTag { name: tag.name.clone() });
                    }
                }
                Token::Characters(chars) => {
                    self.events.push(TokenEvent::Characters(chars.data.clone()));
                }
                Token::Comment(c) => {
                    self.events.push(TokenEvent::Comment(c.data.clone()));
                }
                Token::Doctype(dt) => {
                    self.events.push(TokenEvent::Doctype {
                        name: dt.doctype.name.clone(),
                        public_id: dt.doctype.public_id.clone(),
                        system_id: dt.doctype.system_id.clone(),
                        force_quirks: dt.doctype.force_quirks,
                    });
                }
                Token::EOF(_) => {
                    self.events.push(TokenEvent::Eof);
                }
            }
            TokenSinkResult::Continue
        }

        fn process_characters(&mut self, data: &str) -> TokenSinkResult {
            self.events.push(TokenEvent::Characters(data.to_string()));
            TokenSinkResult::Continue
        }
    }

    fn tokenize(html: &str) -> DetailedSink {
        let mut tok = Tokenizer::new(None, false);
        let mut sink = DetailedSink::new();
        tok.run(html, &mut sink);
        sink
    }

    fn tokenize_with_errors(html: &str) -> (DetailedSink, Vec<ParseError>) {
        let mut tok = Tokenizer::new(None, true);
        let mut sink = DetailedSink::new();
        tok.run(html, &mut sink);
        (sink, tok.errors)
    }

    // ─── DOCTYPE tests ───────────────────────────────────────────────────────

    #[test]
    fn test_doctype_html5() {
        let sink = tokenize("<!DOCTYPE html>");
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        match &dts[0] {
            TokenEvent::Doctype { name, public_id, system_id, force_quirks } => {
                assert_eq!(name.as_deref(), Some("html"));
                assert!(public_id.is_none());
                assert!(system_id.is_none());
                assert!(!force_quirks);
            }
            _ => panic!("expected doctype"),
        }
    }

    #[test]
    fn test_doctype_case_insensitive() {
        let sink = tokenize("<!doctype HTML>");
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        match &dts[0] {
            TokenEvent::Doctype { name, .. } => {
                assert_eq!(name.as_deref(), Some("html"));
            }
            _ => panic!("expected doctype"),
        }
    }

    #[test]
    fn test_doctype_with_public_id() {
        let sink = tokenize(
            r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN">"#,
        );
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        match &dts[0] {
            TokenEvent::Doctype { name, public_id, .. } => {
                assert_eq!(name.as_deref(), Some("html"));
                assert_eq!(public_id.as_deref(), Some("-//W3C//DTD HTML 4.01//EN"));
            }
            _ => panic!("expected doctype"),
        }
    }

    #[test]
    fn test_doctype_with_public_and_system_id() {
        let sink = tokenize(
            r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">"#,
        );
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        match &dts[0] {
            TokenEvent::Doctype { name, public_id, system_id, .. } => {
                assert_eq!(name.as_deref(), Some("html"));
                assert_eq!(public_id.as_deref(), Some("-//W3C//DTD HTML 4.01//EN"));
                assert_eq!(system_id.as_deref(), Some("http://www.w3.org/TR/html4/strict.dtd"));
            }
            _ => panic!("expected doctype"),
        }
    }

    #[test]
    fn test_doctype_system_only() {
        let sink = tokenize(
            r#"<!DOCTYPE html SYSTEM "about:legacy-compat">"#,
        );
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        match &dts[0] {
            TokenEvent::Doctype { name, public_id, system_id, .. } => {
                assert_eq!(name.as_deref(), Some("html"));
                assert!(public_id.is_none());
                assert_eq!(system_id.as_deref(), Some("about:legacy-compat"));
            }
            _ => panic!("expected doctype"),
        }
    }

    #[test]
    fn test_doctype_force_quirks_missing_name() {
        let (sink, errors) = tokenize_with_errors("<!DOCTYPE >");
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        match &dts[0] {
            TokenEvent::Doctype { force_quirks, .. } => {
                assert!(*force_quirks);
            }
            _ => panic!("expected doctype"),
        }
        assert!(!errors.is_empty());
    }

    // ─── Comment tests ───────────────────────────────────────────────────────

    #[test]
    fn test_comment_basic() {
        let sink = tokenize("<!-- hello -->");
        assert_eq!(sink.comments(), vec![" hello "]);
    }

    #[test]
    fn test_comment_empty() {
        let sink = tokenize("<!---->");
        assert_eq!(sink.comments(), vec![""]);
    }

    #[test]
    fn test_comment_with_dashes() {
        let sink = tokenize("<!-- a -- b -->");
        let comments = sink.comments();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0], " a -- b ");
    }

    #[test]
    fn test_comment_abrupt_closing() {
        let (sink, errors) = tokenize_with_errors("<!-->");
        assert_eq!(sink.comments(), vec![""]);
        assert!(errors.iter().any(|e| e.code == "abrupt-closing-of-empty-comment"));
    }

    #[test]
    fn test_multiple_comments() {
        let sink = tokenize("<!-- a -->text<!-- b -->");
        assert_eq!(sink.comments(), vec![" a ", " b "]);
        assert_eq!(sink.all_text(), "text");
    }

    // ─── Tag tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_nested_tags() {
        let sink = tokenize("<div><p>text</p></div>");
        assert_eq!(sink.start_tags(), vec!["div", "p"]);
        assert_eq!(sink.end_tags(), vec!["p", "div"]);
        assert_eq!(sink.all_text(), "text");
    }

    #[test]
    fn test_tag_case_normalization() {
        let sink = tokenize("<DIV><P>text</P></DIV>");
        assert_eq!(sink.start_tags(), vec!["div", "p"]);
        assert_eq!(sink.end_tags(), vec!["p", "div"]);
    }

    #[test]
    fn test_self_closing_tag_details() {
        let sink = tokenize("<br/><hr /><img/>");
        let self_closings: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { name, self_closing, .. } => Some((name.as_str(), *self_closing)),
            _ => None,
        }).collect();
        assert_eq!(self_closings.len(), 3);
        for (name, sc) in &self_closings {
            assert!(sc, "Expected self_closing=true for {}", name);
        }
    }

    #[test]
    fn test_tag_with_double_quoted_attributes() {
        let sink = tokenize(r#"<div class="main" id="content">"#);
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { name, attrs, .. } => Some((name.clone(), attrs.clone())),
            _ => None,
        }).collect();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].0, "div");
        assert_eq!(tags[0].1.get("class"), Some(&Some("main".to_string())));
        assert_eq!(tags[0].1.get("id"), Some(&Some("content".to_string())));
    }

    #[test]
    fn test_tag_with_single_quoted_attributes() {
        let sink = tokenize("<div class='main' id='content'>");
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert_eq!(tags[0].get("class"), Some(&Some("main".to_string())));
        assert_eq!(tags[0].get("id"), Some(&Some("content".to_string())));
    }

    #[test]
    fn test_tag_with_unquoted_attributes() {
        let sink = tokenize("<div class=main>");
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert_eq!(tags[0].get("class"), Some(&Some("main".to_string())));
    }

    #[test]
    fn test_tag_with_boolean_attribute() {
        let sink = tokenize("<input disabled>");
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert_eq!(tags.len(), 1);
        // Boolean attribute: present but no value
        assert_eq!(tags[0].get("disabled"), Some(&None));
    }

    #[test]
    fn test_tag_with_empty_attribute_value() {
        let sink = tokenize(r#"<input value="">"#);
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert_eq!(tags[0].get("value"), Some(&Some("".to_string())));
    }

    #[test]
    fn test_attribute_case_normalization() {
        let sink = tokenize(r#"<div CLASS="main">"#);
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert!(tags[0].contains_key("class"), "attribute name should be lowercased");
    }

    #[test]
    fn test_duplicate_attributes() {
        let (sink, errors) = tokenize_with_errors(r#"<div class="a" class="b">"#);
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        // First value wins per HTML5 spec
        assert_eq!(tags[0].get("class"), Some(&Some("a".to_string())));
        assert!(errors.iter().any(|e| e.code == "duplicate-attribute"));
    }

    #[test]
    fn test_multiple_attributes() {
        let sink = tokenize(r#"<a href="/" target="_blank" rel="noopener">"#);
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert_eq!(tags[0].get("href"), Some(&Some("/".to_string())));
        assert_eq!(tags[0].get("target"), Some(&Some("_blank".to_string())));
        assert_eq!(tags[0].get("rel"), Some(&Some("noopener".to_string())));
    }

    // ─── Entity decoding tests ───────────────────────────────────────────────

    #[test]
    fn test_entity_in_text_named() {
        let sink = tokenize("a &amp; b");
        assert_eq!(sink.all_text(), "a & b");
    }

    #[test]
    fn test_entity_in_text_numeric() {
        let sink = tokenize("a &#60; b");
        assert_eq!(sink.all_text(), "a < b");
    }

    #[test]
    fn test_entity_in_text_hex() {
        let sink = tokenize("a &#x3C; b");
        assert_eq!(sink.all_text(), "a < b");
    }

    #[test]
    fn test_entity_in_attribute_value() {
        let sink = tokenize(r#"<a href="?a=1&amp;b=2">"#);
        let tags: Vec<_> = sink.events.iter().filter_map(|e| match e {
            TokenEvent::StartTag { attrs, .. } => Some(attrs.clone()),
            _ => None,
        }).collect();
        assert_eq!(tags[0].get("href"), Some(&Some("?a=1&b=2".to_string())));
    }

    #[test]
    fn test_multiple_entities_in_text() {
        let sink = tokenize("&lt;div&gt;");
        assert_eq!(sink.all_text(), "<div>");
    }

    // ─── Character / text tests ──────────────────────────────────────────────

    #[test]
    fn test_plain_text() {
        let sink = tokenize("hello world");
        assert_eq!(sink.all_text(), "hello world");
    }

    #[test]
    fn test_text_with_special_chars() {
        let sink = tokenize("hello < world");
        // '<' followed by a space triggers an error and emits text
        // The '<' itself gets emitted as text because it's not a valid tag opener
        assert!(sink.all_text().contains("hello"));
    }

    #[test]
    fn test_null_replacement_in_data() {
        let (sink, errors) = tokenize_with_errors("a\x00b");
        assert_eq!(sink.all_text(), "a\u{FFFD}b");
        assert!(errors.iter().any(|e| e.code == "unexpected-null-character"));
    }

    #[test]
    fn test_bom_stripping() {
        let sink = tokenize("\u{FEFF}hello");
        assert_eq!(sink.all_text(), "hello");
    }

    #[test]
    fn test_empty_input() {
        let sink = tokenize("");
        assert!(sink.events.iter().any(|e| matches!(e, TokenEvent::Eof)));
    }

    // ─── RCDATA / RAWTEXT mode tests ─────────────────────────────────────────

    #[test]
    fn test_title_rcdata_mode() {
        let sink = tokenize("<title>hello <b>world</b></title>");
        assert_eq!(sink.start_tags(), vec!["title"]);
        assert_eq!(sink.end_tags(), vec!["title"]);
        // <b> and </b> should NOT be parsed as tags inside <title>
        let text = sink.all_text();
        assert!(text.contains("<b>world</b>"), "RCDATA should preserve inner tags as text, got: {:?}", text);
    }

    #[test]
    fn test_textarea_rcdata_mode() {
        let sink = tokenize("<textarea><div>not a tag</div></textarea>");
        assert_eq!(sink.start_tags(), vec!["textarea"]);
        assert_eq!(sink.end_tags(), vec!["textarea"]);
        let text = sink.all_text();
        assert!(text.contains("<div>not a tag</div>"),
            "RCDATA should preserve inner tags as text, got: {:?}", text);
    }

    #[test]
    fn test_style_rawtext_mode() {
        let sink = tokenize("<style>.class { color: red; }</style>");
        assert_eq!(sink.start_tags(), vec!["style"]);
        assert_eq!(sink.end_tags(), vec!["style"]);
        let text = sink.all_text();
        assert!(text.contains(".class { color: red; }"), "RAWTEXT should preserve CSS as text");
    }

    #[test]
    fn test_script_rawtext_mode() {
        // Simple script without < in content
        let sink = tokenize("<script>var x = 1;</script>");
        assert_eq!(sink.start_tags(), vec!["script"]);
        assert_eq!(sink.end_tags(), vec!["script"]);
        let text = sink.all_text();
        assert!(text.contains("var x = 1;"), "Script content should be in text, got: {:?}", text);
    }

    #[test]
    fn test_script_rawtext_with_less_than() {
        // Script with < in content
        let sink = tokenize("<script>var x = 1 < 2;</script>");
        assert_eq!(sink.start_tags(), vec!["script"]);
        assert_eq!(sink.end_tags(), vec!["script"]);
        let text = sink.all_text();
        assert!(text.contains("var x = 1"), "Script content should be preserved");
    }

    #[test]
    fn test_xmp_rawtext_mode() {
        // xmp uses rawtext mode; inner tags should be preserved as text
        let sink = tokenize("<xmp><b>bold</b></xmp>");
        assert_eq!(sink.start_tags(), vec!["xmp"]);
        assert_eq!(sink.end_tags(), vec!["xmp"]);
        let text = sink.all_text();
        assert!(text.contains("<b>bold</b>"), "RAWTEXT should preserve inner HTML as text, got: {:?}", text);
    }

    // ─── Error reporting tests ───────────────────────────────────────────────

    #[test]
    fn test_eof_in_tag() {
        let (_, errors) = tokenize_with_errors("<div");
        assert!(errors.iter().any(|e| e.code == "eof-in-tag"),
            "Expected 'eof-in-tag' error, got: {:?}", errors);
    }

    #[test]
    fn test_eof_before_tag_name() {
        let (_, errors) = tokenize_with_errors("<");
        assert!(errors.iter().any(|e| e.code == "eof-before-tag-name"),
            "Expected 'eof-before-tag-name', got: {:?}", errors);
    }

    #[test]
    fn test_eof_in_comment() {
        let (_, errors) = tokenize_with_errors("<!-- unclosed comment");
        assert!(errors.iter().any(|e| e.code == "eof-in-comment"),
            "Expected 'eof-in-comment', got: {:?}", errors);
    }

    #[test]
    fn test_eof_in_doctype() {
        let (_, errors) = tokenize_with_errors("<!DOCTYPE");
        assert!(errors.iter().any(|e| e.code == "eof-in-doctype"),
            "Expected 'eof-in-doctype', got: {:?}", errors);
    }

    #[test]
    fn test_invalid_first_character_of_tag() {
        let (sink, errors) = tokenize_with_errors("<1abc>");
        assert!(errors.iter().any(|e| e.code == "invalid-first-character-of-tag-name"),
            "Expected 'invalid-first-character-of-tag-name', got: {:?}", errors);
        // The '<' and '1abc>' should be emitted as text
        let text = sink.all_text();
        assert!(text.contains("<"), "Should emit '<' as text");
    }

    #[test]
    fn test_unexpected_question_mark() {
        let (_, errors) = tokenize_with_errors("<?xml version='1.0'?>");
        assert!(errors.iter().any(|e| e.code == "unexpected-question-mark-instead-of-tag-name"),
            "Expected question mark error, got: {:?}", errors);
    }

    // ─── Position tracking tests ─────────────────────────────────────────────

    #[test]
    fn test_error_position_single_line() {
        let (_, errors) = tokenize_with_errors("hello\x00world");
        assert!(!errors.is_empty());
        let err = &errors[0];
        assert_eq!(err.line, Some(1));
        // Column should be somewhere after 'hello'
        assert!(err.column.unwrap() > 1, "Column should be > 1, got: {:?}", err.column);
    }

    #[test]
    fn test_error_position_multi_line() {
        let (_, errors) = tokenize_with_errors("line1\nline2\n\x00");
        assert!(!errors.is_empty());
        let err = &errors[0];
        assert_eq!(err.line, Some(3), "Error should be on line 3, got: {:?}", err.line);
    }

    // ─── Complete HTML document tests ────────────────────────────────────────

    #[test]
    fn test_complete_document() {
        let html = "<!DOCTYPE html><html><head><title>Test</title></head><body><p>Hello</p></body></html>";
        let sink = tokenize(html);
        let dts = sink.doctypes();
        assert_eq!(dts.len(), 1);
        assert_eq!(sink.start_tags(), vec!["html", "head", "title", "body", "p"]);
        assert_eq!(sink.end_tags(), vec!["title", "head", "p", "body", "html"]);
    }

    #[test]
    fn test_document_with_entities_and_comments() {
        let html = "<!DOCTYPE html><html><body><!-- nav --><p>Hello &amp; world</p></body></html>";
        let sink = tokenize(html);
        assert_eq!(sink.comments(), vec![" nav "]);
        assert!(sink.all_text().contains("Hello & world"));
    }

    #[test]
    fn test_mixed_content_complex() {
        let html = r#"<div id="a"><span class="b">text1</span><br/><span>text2</span></div>"#;
        let sink = tokenize(html);
        assert_eq!(sink.start_tags(), vec!["div", "span", "br", "span"]);
        assert_eq!(sink.end_tags(), vec!["span", "span", "div"]);
        assert!(sink.all_text().contains("text1"));
        assert!(sink.all_text().contains("text2"));
    }

    // ─── Initial state override tests ────────────────────────────────────────

    #[test]
    fn test_initial_state_rcdata() {
        let opts = TokenizerOpts {
            initial_state: Some(TokenizerState::Rcdata),
            initial_rawtext_tag: Some("title".to_string()),
            ..Default::default()
        };
        let mut tok = Tokenizer::new(Some(opts), false);
        let mut sink = DetailedSink::new();
        tok.run("hello <b>not tag</b></title>", &mut sink);
        // In RCDATA mode, <b> is not parsed as a tag
        assert!(sink.all_text().contains("<b>not tag</b>"));
        assert_eq!(sink.end_tags(), vec!["title"]);
    }

    #[test]
    fn test_initial_state_rawtext() {
        let opts = TokenizerOpts {
            initial_state: Some(TokenizerState::Rawtext),
            initial_rawtext_tag: Some("style".to_string()),
            ..Default::default()
        };
        let mut tok = Tokenizer::new(Some(opts), false);
        let mut sink = DetailedSink::new();
        tok.run("body { margin: 0; }</style>", &mut sink);
        assert!(sink.all_text().contains("body { margin: 0; }"));
        assert_eq!(sink.end_tags(), vec!["style"]);
    }

    // ─── XML coercion tests ──────────────────────────────────────────────────

    #[test]
    fn test_xml_coercion_in_tokenizer() {
        let opts = TokenizerOpts {
            xml_coercion: true,
            ..Default::default()
        };
        let mut tok = Tokenizer::new(Some(opts), false);
        let mut sink = DetailedSink::new();
        tok.run("text\x01more", &mut sink);
        let text = sink.all_text();
        assert!(text.contains('\u{FFFD}'), "XML coercion should replace invalid XML chars");
    }

    #[test]
    fn test_xml_coercion_comment_in_tokenizer() {
        let opts = TokenizerOpts {
            xml_coercion: true,
            ..Default::default()
        };
        let mut tok = Tokenizer::new(Some(opts), false);
        let mut sink = DetailedSink::new();
        tok.run("<!-- a--b -->", &mut sink);
        let comments = sink.comments();
        assert!(!comments.is_empty());
        // XML coercion replaces "--" with "- -" in comments
        assert!(comments[0].contains("- -"), "Expected '- -' but got: {:?}", comments[0]);
    }

    // ─── CDATA section tests ─────────────────────────────────────────────────

    #[test]
    fn test_cdata_section() {
        let (sink, errors) = tokenize_with_errors("<![CDATA[hello world]]>");
        // CDATA in HTML content generates an error
        assert!(errors.iter().any(|e| e.code == "cdata-in-html-content"));
        assert!(sink.all_text().contains("hello world"));
    }

    #[test]
    fn test_cdata_with_angle_brackets() {
        let (sink, _) = tokenize_with_errors("<![CDATA[<div>not a tag</div>]]>");
        assert!(sink.all_text().contains("<div>not a tag</div>"));
    }

    // ─── Bogus comment tests ─────────────────────────────────────────────────

    #[test]
    fn test_bogus_comment_from_processing_instruction() {
        let (sink, _) = tokenize_with_errors("<?xml version='1.0'?>");
        // PI is treated as a bogus comment
        assert!(!sink.comments().is_empty());
    }

    // ─── Edge cases ──────────────────────────────────────────────────────────

    #[test]
    fn test_adjacent_tags_no_whitespace() {
        let sink = tokenize("<a><b><c></c></b></a>");
        assert_eq!(sink.start_tags(), vec!["a", "b", "c"]);
        assert_eq!(sink.end_tags(), vec!["c", "b", "a"]);
    }

    #[test]
    fn test_text_between_tags() {
        let sink = tokenize("before<p>inside</p>after");
        let text = sink.all_text();
        assert!(text.contains("before"));
        assert!(text.contains("inside"));
        assert!(text.contains("after"));
    }

    #[test]
    fn test_empty_end_tag() {
        let (_, errors) = tokenize_with_errors("</>");
        assert!(errors.iter().any(|e| e.code == "empty-end-tag"),
            "Expected 'empty-end-tag', got: {:?}", errors);
    }

    #[test]
    fn test_missing_whitespace_between_attributes() {
        let (sink, errors) = tokenize_with_errors(r#"<div class="a"id="b">"#);
        assert_eq!(sink.start_tags(), vec!["div"]);
        assert!(errors.iter().any(|e| e.code == "missing-whitespace-between-attributes"),
            "Expected missing-whitespace error, got: {:?}", errors);
    }

    #[test]
    fn test_unexpected_solidus_in_tag() {
        let (_, errors) = tokenize_with_errors("<div / >");
        // The '/' in <div / > triggers unexpected-character-after-solidus-in-tag
        assert!(errors.iter().any(|e| e.code == "unexpected-character-after-solidus-in-tag"),
            "Expected solidus error, got: {:?}", errors);
    }

    #[test]
    fn test_only_text_no_tags() {
        let sink = tokenize("just plain text with no tags at all");
        assert!(sink.start_tags().is_empty());
        assert!(sink.end_tags().is_empty());
        assert_eq!(sink.all_text(), "just plain text with no tags at all");
    }

    #[test]
    fn test_deeply_nested() {
        let sink = tokenize("<a><b><c><d><e>deep</e></d></c></b></a>");
        assert_eq!(sink.start_tags(), vec!["a", "b", "c", "d", "e"]);
        assert_eq!(sink.end_tags(), vec!["e", "d", "c", "b", "a"]);
        assert_eq!(sink.all_text(), "deep");
    }
}
