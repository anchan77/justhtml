//! Tokenizer state handler implementations.
//!
//! Each state is dispatched from `dispatch_state` and returns `true` if the tokenizer is done (EOF).

#![allow(unused_variables)]

use crate::tokens::TagKind;

use super::{TokenSink, Tokenizer, TokenizerState};

/// Dispatch to the appropriate state handler.
pub fn dispatch_state(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.state {
        TokenizerState::Data => state_data(tok, sink),
        TokenizerState::TagOpen => state_tag_open(tok, sink),
        TokenizerState::EndTagOpen => state_end_tag_open(tok, sink),
        TokenizerState::TagName => state_tag_name(tok, sink),
        TokenizerState::BeforeAttributeName => state_before_attribute_name(tok, sink),
        TokenizerState::AttributeName => state_attribute_name(tok, sink),
        TokenizerState::AfterAttributeName => state_after_attribute_name(tok, sink),
        TokenizerState::BeforeAttributeValue => state_before_attribute_value(tok, sink),
        TokenizerState::AttributeValueDouble => state_attribute_value_double(tok, sink),
        TokenizerState::AttributeValueSingle => state_attribute_value_single(tok, sink),
        TokenizerState::AttributeValueUnquoted => state_attribute_value_unquoted(tok, sink),
        TokenizerState::AfterAttributeValueQuoted => state_after_attribute_value_quoted(tok, sink),
        TokenizerState::SelfClosingStartTag => state_self_closing_start_tag(tok, sink),
        TokenizerState::MarkupDeclarationOpen => state_markup_declaration_open(tok, sink),
        TokenizerState::CommentStart => state_comment_start(tok, sink),
        TokenizerState::CommentStartDash => state_comment_start_dash(tok, sink),
        TokenizerState::Comment => state_comment(tok, sink),
        TokenizerState::CommentEndDash => state_comment_end_dash(tok, sink),
        TokenizerState::CommentEnd => state_comment_end(tok, sink),
        TokenizerState::CommentEndBang => state_comment_end_bang(tok, sink),
        TokenizerState::BogusComment => state_bogus_comment(tok, sink),
        TokenizerState::Doctype => state_doctype(tok, sink),
        TokenizerState::BeforeDoctypeName => state_before_doctype_name(tok, sink),
        TokenizerState::DoctypeName => state_doctype_name(tok, sink),
        TokenizerState::AfterDoctypeName => state_after_doctype_name(tok, sink),
        TokenizerState::BogusDoctype => state_bogus_doctype(tok, sink),
        TokenizerState::AfterDoctypePublicKeyword => state_after_doctype_public_keyword(tok, sink),
        TokenizerState::BeforeDoctypePublicIdentifier => state_before_doctype_public_identifier(tok, sink),
        TokenizerState::DoctypePublicIdentifierDouble => state_doctype_public_identifier_double(tok, sink),
        TokenizerState::DoctypePublicIdentifierSingle => state_doctype_public_identifier_single(tok, sink),
        TokenizerState::AfterDoctypePublicIdentifier => state_after_doctype_public_identifier(tok, sink),
        TokenizerState::BetweenDoctypePublicAndSystem => state_between_doctype_public_and_system(tok, sink),
        TokenizerState::AfterDoctypeSystemKeyword => state_after_doctype_system_keyword(tok, sink),
        TokenizerState::BeforeDoctypeSystemIdentifier => state_before_doctype_system_identifier(tok, sink),
        TokenizerState::DoctypeSystemIdentifierDouble => state_doctype_system_identifier_double(tok, sink),
        TokenizerState::DoctypeSystemIdentifierSingle => state_doctype_system_identifier_single(tok, sink),
        TokenizerState::AfterDoctypeSystemIdentifier => state_after_doctype_system_identifier(tok, sink),
        TokenizerState::CdataSection => state_cdata_section(tok, sink),
        TokenizerState::CdataSectionBracket => state_cdata_section_bracket(tok, sink),
        TokenizerState::CdataSectionEnd => state_cdata_section_end(tok, sink),
        TokenizerState::Rcdata => state_rcdata(tok, sink),
        TokenizerState::RcdataLessThanSign => state_rcdata_less_than_sign(tok, sink),
        TokenizerState::RcdataEndTagOpen => state_rcdata_end_tag_open(tok, sink),
        TokenizerState::RcdataEndTagName => state_rcdata_end_tag_name(tok, sink),
        TokenizerState::Rawtext => state_rawtext(tok, sink),
        TokenizerState::RawtextLessThanSign => state_rawtext_less_than_sign(tok, sink),
        TokenizerState::RawtextEndTagOpen => state_rawtext_end_tag_open(tok, sink),
        TokenizerState::RawtextEndTagName => state_rawtext_end_tag_name(tok, sink),
        TokenizerState::Plaintext => state_plaintext(tok, sink),
        TokenizerState::ScriptData => state_script_data(tok, sink),
        TokenizerState::ScriptDataLessThanSign => state_script_data_less_than_sign(tok, sink),
        TokenizerState::ScriptDataEndTagOpen => state_script_data_end_tag_open(tok, sink),
        TokenizerState::ScriptDataEndTagName => state_script_data_end_tag_name(tok, sink),
        TokenizerState::ScriptDataEscapeStart => state_script_data_escape_start(tok, sink),
        TokenizerState::ScriptDataEscapeStartDash => state_script_data_escape_start_dash(tok, sink),
        TokenizerState::ScriptDataEscaped => state_script_data_escaped(tok, sink),
        TokenizerState::ScriptDataEscapedDash => state_script_data_escaped_dash(tok, sink),
        TokenizerState::ScriptDataEscapedDashDash => state_script_data_escaped_dash_dash(tok, sink),
        TokenizerState::ScriptDataEscapedLessThanSign => state_script_data_escaped_less_than_sign(tok, sink),
        TokenizerState::ScriptDataEscapedEndTagOpen => state_script_data_escaped_end_tag_open(tok, sink),
        TokenizerState::ScriptDataEscapedEndTagName => state_script_data_escaped_end_tag_name(tok, sink),
        TokenizerState::ScriptDataDoubleEscapeStart => state_script_data_double_escape_start(tok, sink),
        TokenizerState::ScriptDataDoubleEscaped => state_script_data_double_escaped(tok, sink),
        TokenizerState::ScriptDataDoubleEscapedDash => state_script_data_double_escaped_dash(tok, sink),
        TokenizerState::ScriptDataDoubleEscapedDashDash => state_script_data_double_escaped_dash_dash(tok, sink),
        TokenizerState::ScriptDataDoubleEscapedLessThanSign => state_script_data_double_escaped_less_than_sign(tok, sink),
        TokenizerState::ScriptDataDoubleEscapeEnd => state_script_data_double_escape_end(tok, sink),
        // Character reference states are handled inline
        _ => false,
    }
}

// ─── DATA state ──────────────────────────────────────────────────────────────

fn state_data(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python _state_data: collect text up to '<' or '\0',
    // let flush_text handle entity decoding.
    // NULL characters trigger errors and are replaced with U+FFFD.
    loop {
        if tok.reconsume {
            tok.reconsume = false;
            // Reconsume: back up one character
            if tok.pos > 0 {
                let s = &tok.buffer[..tok.pos];
                if let Some((i, _)) = s.char_indices().next_back() {
                    tok.pos = i;
                }
            }
        }

        if tok.pos >= tok.length {
            tok.flush_text(sink);
            tok.emit_eof(sink);
            return true;
        }

        // Fast path: find next '<' or '\0' using find
        let remaining = &tok.buffer[tok.pos..];
        let next_special = remaining
            .find(|c: char| c == '<' || c == '\0')
            .map(|i| tok.pos + i)
            .unwrap_or(tok.length);

        // Collect text up to special char
        if next_special > tok.pos {
            let chunk = tok.buffer[tok.pos..next_special].to_string();
            tok.append_text(&chunk);
            tok.pos = next_special;
            if tok.pos >= tok.length {
                continue;
            }
        }

        let c = tok.buffer[tok.pos..].chars().next().unwrap();
        tok.pos += c.len_utf8();

        if c == '\0' {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            continue;
        }

        // At this point c is '<'
        tok.flush_text(sink);
        tok.state = TokenizerState::TagOpen;
        return false;
    }
}

// ─── TAG OPEN state ──────────────────────────────────────────────────────────

fn state_tag_open(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('!') => {
            tok.state = TokenizerState::MarkupDeclarationOpen;
            false
        }
        Some('/') => {
            tok.state = TokenizerState::EndTagOpen;
            false
        }
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_kind = TagKind::Start;
            tok.current_tag_name.clear();
            tok.current_tag_name.push(c.to_ascii_lowercase());
            tok.current_tag_attrs.clear();
            tok.current_tag_self_closing = false;
            tok.state = TokenizerState::TagName;
            false
        }
        Some('?') => {
            tok.emit_error("unexpected-question-mark-instead-of-tag-name");
            tok.current_comment.clear();
            tok.reconsume_current();
            tok.state = TokenizerState::BogusComment;
            false
        }
        None => {
            tok.emit_error("eof-before-tag-name");
            tok.append_text_char('<');
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("invalid-first-character-of-tag-name");
            tok.append_text_char('<');
            tok.reconsume_current();
            tok.state = TokenizerState::Data;
            false
        }
    }
}

// ─── END TAG OPEN state ─────────────────────────────────────────────────────

fn state_end_tag_open(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_kind = TagKind::End;
            tok.current_tag_name.clear();
            tok.current_tag_name.push(c.to_ascii_lowercase());
            tok.current_tag_attrs.clear();
            tok.current_tag_self_closing = false;
            tok.state = TokenizerState::TagName;
            false
        }
        Some('>') => {
            tok.emit_error("empty-end-tag");
            tok.state = TokenizerState::Data;
            false
        }
        None => {
            tok.emit_error("eof-before-tag-name");
            tok.append_text("</");
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("invalid-first-character-of-tag-name");
            tok.current_comment.clear();
            tok.reconsume_current();
            tok.state = TokenizerState::BogusComment;
            false
        }
    }
}

// ─── TAG NAME state ──────────────────────────────────────────────────────────

fn state_tag_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::BeforeAttributeName;
            false
        }
        Some('/') => {
            tok.state = TokenizerState::SelfClosingStartTag;
            false
        }
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_current_tag(sink);
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.current_tag_name.push('\u{FFFD}');
            false
        }
        Some(c) => {
            // Fast path: collect tag name characters
            tok.current_tag_name.push(c.to_ascii_lowercase());
            while tok.pos < tok.length {
                let next = tok.buffer[tok.pos..].chars().next().unwrap();
                match next {
                    '\t' | '\n' | '\x0C' | ' ' | '/' | '>' | '\0' => break,
                    _ => {
                        tok.current_tag_name.push(next.to_ascii_lowercase());
                        tok.pos += next.len_utf8();
                    }
                }
            }
            false
        }
        None => {
            tok.emit_error("eof-in-tag");
            tok.emit_eof(sink);
            true
        }
    }
}

// ─── BEFORE ATTRIBUTE NAME state ────────────────────────────────────────────

fn state_before_attribute_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false, // ignore
        Some('/') | Some('>') | None => {
            tok.reconsume_current();
            tok.state = TokenizerState::AfterAttributeName;
            false
        }
        Some('=') => {
            tok.emit_error("unexpected-equals-sign-before-attribute-name");
            tok.current_attr_name.clear();
            tok.current_attr_name.push('=');
            tok.current_attr_value.clear();
            tok.current_attr_has_value = false;
            tok.state = TokenizerState::AttributeName;
            false
        }
        Some(_c) => {
            tok.finish_attribute();
            tok.current_attr_name.clear();
            tok.current_attr_value.clear();
            tok.current_attr_has_value = false;
            tok.reconsume_current();
            tok.state = TokenizerState::AttributeName;
            false
        }
    }
}

// ─── ATTRIBUTE NAME state ───────────────────────────────────────────────────

fn state_attribute_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') | Some('/') | Some('>') | None => {
            tok.reconsume_current();
            tok.state = TokenizerState::AfterAttributeName;
            false
        }
        Some('=') => {
            tok.state = TokenizerState::BeforeAttributeValue;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.current_attr_name.push('\u{FFFD}');
            false
        }
        Some('"') | Some('\'') | Some('<') => {
            tok.emit_error("unexpected-character-in-attribute-name");
            tok.current_attr_name.push(tok.buffer[tok.pos - 1..].chars().next().unwrap().to_ascii_lowercase());
            false
        }
        Some(c) => {
            tok.current_attr_name.push(c.to_ascii_lowercase());
            // Fast path
            while tok.pos < tok.length {
                let next = tok.buffer[tok.pos..].chars().next().unwrap();
                match next {
                    '\t' | '\n' | '\x0C' | ' ' | '/' | '>' | '=' | '\0' | '"' | '\'' | '<' => break,
                    _ => {
                        tok.current_attr_name.push(next.to_ascii_lowercase());
                        tok.pos += next.len_utf8();
                    }
                }
            }
            false
        }
    }
}

// ─── AFTER ATTRIBUTE NAME state ──────────────────────────────────────────────

fn state_after_attribute_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false, // ignore
        Some('/') => {
            tok.state = TokenizerState::SelfClosingStartTag;
            false
        }
        Some('=') => {
            tok.state = TokenizerState::BeforeAttributeValue;
            false
        }
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_current_tag(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-tag");
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.finish_attribute();
            tok.current_attr_name.clear();
            tok.current_attr_value.clear();
            tok.current_attr_has_value = false;
            tok.reconsume_current();
            tok.state = TokenizerState::AttributeName;
            false
        }
    }
}

// ─── BEFORE ATTRIBUTE VALUE state ───────────────────────────────────────────

fn state_before_attribute_value(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false, // ignore
        Some('"') => {
            tok.current_attr_has_value = true;
            tok.state = TokenizerState::AttributeValueDouble;
            false
        }
        Some('\'') => {
            tok.current_attr_has_value = true;
            tok.state = TokenizerState::AttributeValueSingle;
            false
        }
        Some('>') => {
            tok.emit_error("missing-attribute-value");
            tok.state = TokenizerState::Data;
            tok.emit_current_tag(sink);
            false
        }
        _ => {
            tok.current_attr_has_value = true;
            tok.reconsume_current();
            tok.state = TokenizerState::AttributeValueUnquoted;
            false
        }
    }
}

// ─── ATTRIBUTE VALUE (double-quoted) state ──────────────────────────────────

fn state_attribute_value_double(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python: accumulate value including raw '&' chars.
    // Entity decoding is deferred to finish_attribute.
    loop {
        let pos = tok.pos;
        if pos < tok.length {
            // Fast path: find next terminator (", &, \0)
            let remaining = &tok.buffer[pos..];
            let next_quote = remaining.find('"').map(|i| pos + i).unwrap_or(tok.length);
            let chunk = &tok.buffer[pos..next_quote];
            let end = if chunk.contains('&') || chunk.contains('\0') {
                let mut e = next_quote;
                if let Some(i) = chunk.find('&') { e = e.min(pos + i); }
                if let Some(i) = chunk.find('\0') { e = e.min(pos + i); }
                e
            } else {
                next_quote
            };

            if end > pos {
                tok.current_attr_value.push_str(&tok.buffer[pos..end]);
                tok.current_attr_has_value = true;
                tok.pos = end;
            }
        }

        if tok.pos >= tok.length {
            tok.emit_error("eof-in-tag");
            tok.emit_eof(sink);
            return true;
        }

        let c = tok.buffer[tok.pos..].chars().next().unwrap();
        tok.pos += c.len_utf8();

        match c {
            '"' => {
                tok.state = TokenizerState::AfterAttributeValueQuoted;
                return false;
            }
            '&' => {
                tok.append_attr_value_char('&');
                tok.current_attr_value_has_amp = true;
            }
            '\0' => {
                tok.emit_error("unexpected-null-character");
                tok.append_attr_value_char('\u{FFFD}');
            }
            _ => unreachable!(),
        }
    }
}

// ─── ATTRIBUTE VALUE (single-quoted) state ──────────────────────────────────

fn state_attribute_value_single(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python: accumulate value including raw '&' chars.
    loop {
        let pos = tok.pos;
        if pos < tok.length {
            let remaining = &tok.buffer[pos..];
            let next_quote = remaining.find('\'').map(|i| pos + i).unwrap_or(tok.length);
            let chunk = &tok.buffer[pos..next_quote];
            let end = if chunk.contains('&') || chunk.contains('\0') {
                let mut e = next_quote;
                if let Some(i) = chunk.find('&') { e = e.min(pos + i); }
                if let Some(i) = chunk.find('\0') { e = e.min(pos + i); }
                e
            } else {
                next_quote
            };

            if end > pos {
                tok.current_attr_value.push_str(&tok.buffer[pos..end]);
                tok.current_attr_has_value = true;
                tok.pos = end;
            }
        }

        if tok.pos >= tok.length {
            tok.emit_error("eof-in-tag");
            tok.emit_eof(sink);
            return true;
        }

        let c = tok.buffer[tok.pos..].chars().next().unwrap();
        tok.pos += c.len_utf8();

        match c {
            '\'' => {
                tok.state = TokenizerState::AfterAttributeValueQuoted;
                return false;
            }
            '&' => {
                tok.append_attr_value_char('&');
                tok.current_attr_value_has_amp = true;
            }
            '\0' => {
                tok.emit_error("unexpected-null-character");
                tok.append_attr_value_char('\u{FFFD}');
            }
            _ => unreachable!(),
        }
    }
}

// ─── ATTRIBUTE VALUE (unquoted) state ───────────────────────────────────────

fn state_attribute_value_unquoted(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python: accumulate value including raw '&' chars.
    loop {
        // Fast path: skip non-terminator chars
        if !tok.reconsume {
            let pos = tok.pos;
            if pos < tok.length {
                // Find next terminator
                let remaining = &tok.buffer[pos..];
                let mut end = tok.length;
                for (i, ch) in remaining.char_indices() {
                    match ch {
                        '\t' | '\n' | '\x0C' | ' ' | '>' | '&' | '"' | '\'' | '<' | '=' | '`' | '\0' => {
                            end = pos + i;
                            break;
                        }
                        _ => {}
                    }
                }
                if end > pos {
                    tok.current_attr_value.push_str(&tok.buffer[pos..end]);
                    tok.current_attr_has_value = true;
                    tok.pos = end;
                }
            }
        }

        match tok.get_char() {
            None => {
                tok.emit_error("eof-in-tag");
                tok.emit_eof(sink);
                return true;
            }
            Some(c) => match c {
                '\t' | '\n' | '\x0C' | ' ' => {
                    tok.finish_attribute();
                    tok.state = TokenizerState::BeforeAttributeName;
                    return false;
                }
                '>' => {
                    tok.finish_attribute();
                    tok.emit_current_tag(sink);
                    if tok.state == TokenizerState::Data || tok.state == TokenizerState::Rcdata
                        || tok.state == TokenizerState::Rawtext || tok.state == TokenizerState::Plaintext
                        || tok.state == TokenizerState::ScriptData {
                        // Already switched
                    } else {
                        tok.state = TokenizerState::Data;
                    }
                    return false;
                }
                '&' => {
                    tok.append_attr_value_char('&');
                    tok.current_attr_value_has_amp = true;
                }
                '"' | '\'' | '<' | '=' | '`' => {
                    tok.emit_error("unexpected-character-in-unquoted-attribute-value");
                    tok.append_attr_value_char(c);
                }
                '\0' => {
                    tok.emit_error("unexpected-null-character");
                    tok.append_attr_value_char('\u{FFFD}');
                }
                _ => {
                    tok.append_attr_value_char(c);
                }
            }
        }
    }
}

// ─── AFTER ATTRIBUTE VALUE (quoted) state ───────────────────────────────────

fn state_after_attribute_value_quoted(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::BeforeAttributeName;
            false
        }
        Some('/') => {
            tok.state = TokenizerState::SelfClosingStartTag;
            false
        }
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_current_tag(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-tag");
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-whitespace-between-attributes");
            tok.reconsume_current();
            tok.state = TokenizerState::BeforeAttributeName;
            false
        }
    }
}

// ─── SELF-CLOSING START TAG state ───────────────────────────────────────────

fn state_self_closing_start_tag(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('>') => {
            tok.current_tag_self_closing = true;
            tok.state = TokenizerState::Data;
            tok.emit_current_tag(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-tag");
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("unexpected-character-after-solidus-in-tag");
            tok.reconsume_current();
            tok.state = TokenizerState::BeforeAttributeName;
            false
        }
    }
}

// ─── MARKUP DECLARATION OPEN state ──────────────────────────────────────────

fn state_markup_declaration_open(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    if tok.consume_exact("--") {
        tok.current_comment.clear();
        tok.state = TokenizerState::CommentStart;
        return false;
    }

    if tok.consume_case_insensitive("DOCTYPE") {
        tok.state = TokenizerState::Doctype;
        return false;
    }

    if tok.consume_exact("[CDATA[") {
        // CDATA is only valid in foreign content; always emit error from tokenizer
        tok.emit_error("cdata-in-html-content");
        tok.state = TokenizerState::CdataSection;
        return false;
    }

    tok.emit_error("incorrectly-opened-comment");
    tok.current_comment.clear();
    tok.state = TokenizerState::BogusComment;
    false
}

// ─── COMMENT states ──────────────────────────────────────────────────────────

fn state_comment_start(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.state = TokenizerState::CommentStartDash;
            false
        }
        Some('>') => {
            tok.emit_error("abrupt-closing-of-empty-comment");
            tok.state = TokenizerState::Data;
            tok.emit_comment(sink);
            false
        }
        _ => {
            tok.reconsume_current();
            tok.state = TokenizerState::Comment;
            false
        }
    }
}

fn state_comment_start_dash(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.state = TokenizerState::CommentEnd;
            false
        }
        Some('>') => {
            tok.emit_error("abrupt-closing-of-empty-comment");
            tok.state = TokenizerState::Data;
            tok.emit_comment(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-comment");
            tok.emit_comment(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.current_comment.push('-');
            tok.reconsume_current();
            tok.state = TokenizerState::Comment;
            false
        }
    }
}

fn state_comment(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('<') => {
            tok.current_comment.push('<');
            false
        }
        Some('-') => {
            tok.state = TokenizerState::CommentEndDash;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.current_comment.push('\u{FFFD}');
            false
        }
        Some(c) => {
            tok.current_comment.push(c);
            // Fast path
            while tok.pos < tok.length {
                let next = tok.buffer[tok.pos..].chars().next().unwrap();
                match next {
                    '<' | '-' | '\0' => break,
                    _ => {
                        tok.current_comment.push(next);
                        tok.pos += next.len_utf8();
                    }
                }
            }
            false
        }
        None => {
            tok.emit_error("eof-in-comment");
            tok.emit_comment(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_comment_end_dash(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.state = TokenizerState::CommentEnd;
            false
        }
        None => {
            tok.emit_error("eof-in-comment");
            tok.emit_comment(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.current_comment.push('-');
            tok.reconsume_current();
            tok.state = TokenizerState::Comment;
            false
        }
    }
}

fn state_comment_end(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_comment(sink);
            false
        }
        Some('!') => {
            tok.state = TokenizerState::CommentEndBang;
            false
        }
        Some('-') => {
            tok.current_comment.push('-');
            false
        }
        None => {
            tok.emit_error("eof-in-comment");
            tok.emit_comment(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.current_comment.push_str("--");
            tok.reconsume_current();
            tok.state = TokenizerState::Comment;
            false
        }
    }
}

fn state_comment_end_bang(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.current_comment.push_str("--!");
            tok.state = TokenizerState::CommentEndDash;
            false
        }
        Some('>') => {
            tok.emit_error("incorrectly-closed-comment");
            tok.state = TokenizerState::Data;
            tok.emit_comment(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-comment");
            tok.emit_comment(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.current_comment.push_str("--!");
            tok.reconsume_current();
            tok.state = TokenizerState::Comment;
            false
        }
    }
}

fn state_bogus_comment(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_comment(sink);
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.current_comment.push('\u{FFFD}');
            false
        }
        Some(c) => {
            tok.current_comment.push(c);
            false
        }
        None => {
            tok.emit_comment(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

// ─── DOCTYPE states ──────────────────────────────────────────────────────────

fn state_doctype(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::BeforeDoctypeName;
            false
        }
        Some('>') => {
            tok.reconsume_current();
            tok.state = TokenizerState::BeforeDoctypeName;
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-whitespace-before-doctype-name");
            tok.reconsume_current();
            tok.state = TokenizerState::BeforeDoctypeName;
            false
        }
    }
}

fn state_before_doctype_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false, // ignore
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.current_doctype_name = Some("\u{FFFD}".to_string());
            tok.current_doctype_public = None;
            tok.current_doctype_system = None;
            tok.current_doctype_force_quirks = false;
            tok.state = TokenizerState::DoctypeName;
            false
        }
        Some('>') => {
            tok.emit_error("expected-doctype-name-but-got-right-bracket");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some(c) => {
            tok.current_doctype_name = Some(c.to_ascii_lowercase().to_string());
            tok.current_doctype_public = None;
            tok.current_doctype_system = None;
            tok.current_doctype_force_quirks = false;
            tok.state = TokenizerState::DoctypeName;
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_doctype_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::AfterDoctypeName;
            false
        }
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            if let Some(ref mut name) = tok.current_doctype_name {
                name.push('\u{FFFD}');
            }
            false
        }
        Some(c) => {
            if let Some(ref mut name) = tok.current_doctype_name {
                name.push(c.to_ascii_lowercase());
            }
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_after_doctype_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false, // ignore
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            // Check for PUBLIC or SYSTEM
            // Move pos back to before the character we just consumed so
            // consume_case_insensitive reads from the correct position.
            tok.pos = tok.prev_char_pos();
            tok.reconsume = false;
            if tok.consume_case_insensitive("PUBLIC") {
                tok.state = TokenizerState::AfterDoctypePublicKeyword;
            } else if tok.consume_case_insensitive("SYSTEM") {
                tok.state = TokenizerState::AfterDoctypeSystemKeyword;
            } else {
                tok.get_char(); // consume the character
                tok.emit_error("expected-doctype-name-but-got-right-bracket");
                tok.current_doctype_force_quirks = true;
                tok.state = TokenizerState::BogusDoctype;
            }
            false
        }
    }
}

fn state_after_doctype_public_keyword(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::BeforeDoctypePublicIdentifier;
            false
        }
        Some('"') => {
            tok.emit_error("missing-whitespace-before-doctype-public-identifier");
            tok.current_doctype_public = Some(String::new());
            tok.state = TokenizerState::DoctypePublicIdentifierDouble;
            false
        }
        Some('\'') => {
            tok.emit_error("missing-whitespace-before-doctype-public-identifier");
            tok.current_doctype_public = Some(String::new());
            tok.state = TokenizerState::DoctypePublicIdentifierSingle;
            false
        }
        Some('>') => {
            tok.emit_error("missing-doctype-public-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-quote-before-doctype-public-identifier");
            tok.current_doctype_force_quirks = true;
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_before_doctype_public_identifier(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false,
        Some('"') => {
            tok.current_doctype_public = Some(String::new());
            tok.state = TokenizerState::DoctypePublicIdentifierDouble;
            false
        }
        Some('\'') => {
            tok.current_doctype_public = Some(String::new());
            tok.state = TokenizerState::DoctypePublicIdentifierSingle;
            false
        }
        Some('>') => {
            tok.emit_error("missing-doctype-public-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-quote-before-doctype-public-identifier");
            tok.current_doctype_force_quirks = true;
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_doctype_public_identifier_double(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('"') => { tok.state = TokenizerState::AfterDoctypePublicIdentifier; false }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            if let Some(ref mut id) = tok.current_doctype_public { id.push('\u{FFFD}'); }
            false
        }
        Some('>') => {
            tok.emit_error("abrupt-doctype-public-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some(c) => {
            if let Some(ref mut id) = tok.current_doctype_public { id.push(c); }
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_doctype_public_identifier_single(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\'') => { tok.state = TokenizerState::AfterDoctypePublicIdentifier; false }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            if let Some(ref mut id) = tok.current_doctype_public { id.push('\u{FFFD}'); }
            false
        }
        Some('>') => {
            tok.emit_error("abrupt-doctype-public-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some(c) => {
            if let Some(ref mut id) = tok.current_doctype_public { id.push(c); }
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_after_doctype_public_identifier(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::BetweenDoctypePublicAndSystem;
            false
        }
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some('"') => {
            tok.emit_error("missing-whitespace-between-doctype-public-and-system-identifiers");
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierDouble;
            false
        }
        Some('\'') => {
            tok.emit_error("missing-whitespace-between-doctype-public-and-system-identifiers");
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierSingle;
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-quote-before-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_between_doctype_public_and_system(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false,
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some('"') => {
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierDouble;
            false
        }
        Some('\'') => {
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierSingle;
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-quote-before-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_after_doctype_system_keyword(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
            tok.state = TokenizerState::BeforeDoctypeSystemIdentifier;
            false
        }
        Some('"') => {
            tok.emit_error("missing-whitespace-before-doctype-system-identifier");
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierDouble;
            false
        }
        Some('\'') => {
            tok.emit_error("missing-whitespace-before-doctype-system-identifier");
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierSingle;
            false
        }
        Some('>') => {
            tok.emit_error("missing-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-quote-before-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_before_doctype_system_identifier(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false,
        Some('"') => {
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierDouble;
            false
        }
        Some('\'') => {
            tok.current_doctype_system = Some(String::new());
            tok.state = TokenizerState::DoctypeSystemIdentifierSingle;
            false
        }
        Some('>') => {
            tok.emit_error("missing-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("missing-quote-before-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_doctype_system_identifier_double(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('"') => { tok.state = TokenizerState::AfterDoctypeSystemIdentifier; false }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            if let Some(ref mut id) = tok.current_doctype_system { id.push('\u{FFFD}'); }
            false
        }
        Some('>') => {
            tok.emit_error("abrupt-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some(c) => {
            if let Some(ref mut id) = tok.current_doctype_system { id.push(c); }
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_doctype_system_identifier_single(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\'') => { tok.state = TokenizerState::AfterDoctypeSystemIdentifier; false }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            if let Some(ref mut id) = tok.current_doctype_system { id.push('\u{FFFD}'); }
            false
        }
        Some('>') => {
            tok.emit_error("abrupt-doctype-system-identifier");
            tok.current_doctype_force_quirks = true;
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some(c) => {
            if let Some(ref mut id) = tok.current_doctype_system { id.push(c); }
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_after_doctype_system_identifier(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => false,
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        None => {
            tok.emit_error("eof-in-doctype");
            tok.current_doctype_force_quirks = true;
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
        Some(_) => {
            tok.emit_error("unexpected-character-after-doctype-system-identifier");
            tok.reconsume_current();
            tok.state = TokenizerState::BogusDoctype;
            false
        }
    }
}

fn state_bogus_doctype(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('>') => {
            tok.state = TokenizerState::Data;
            tok.emit_doctype(sink);
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            false
        }
        Some(_) => false,
        None => {
            tok.emit_doctype(sink);
            tok.emit_eof(sink);
            true
        }
    }
}

// ─── CDATA states ────────────────────────────────────────────────────────────

fn state_cdata_section(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(']') => {
            tok.state = TokenizerState::CdataSectionBracket;
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            false
        }
        None => {
            tok.emit_error("eof-in-cdata");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_cdata_section_bracket(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(']') => {
            tok.state = TokenizerState::CdataSectionEnd;
            false
        }
        _ => {
            tok.append_text_char(']');
            tok.reconsume_current();
            tok.state = TokenizerState::CdataSection;
            false
        }
    }
}

fn state_cdata_section_end(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(']') => {
            tok.append_text_char(']');
            false
        }
        Some('>') => {
            tok.state = TokenizerState::Data;
            false
        }
        _ => {
            tok.append_text("]]");
            tok.reconsume_current();
            tok.state = TokenizerState::CdataSection;
            false
        }
    }
}

// ─── RCDATA states ───────────────────────────────────────────────────────────

fn state_rcdata(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python _state_rcdata: collect text (including '&' for deferred entity decoding).
    // NULL replaced with U+FFFD, '<' transitions to RCDATA_LESS_THAN_SIGN.
    loop {
        if tok.reconsume {
            tok.reconsume = false;
            if tok.pos > 0 {
                let s = &tok.buffer[..tok.pos];
                if let Some((i, _)) = s.char_indices().next_back() {
                    tok.pos = i;
                }
            }
        }

        if tok.pos >= tok.length {
            tok.flush_text(sink);
            tok.emit_eof(sink);
            return true;
        }

        // Find the nearest special character (<, &, \0)
        let remaining = &tok.buffer[tok.pos..];
        let mut next_special = tok.length;
        if let Some(i) = remaining.find('<') {
            next_special = next_special.min(tok.pos + i);
        }
        if let Some(i) = remaining.find('&') {
            next_special = next_special.min(tok.pos + i);
        }
        if let Some(i) = remaining.find('\0') {
            next_special = next_special.min(tok.pos + i);
        }

        // Consume text up to the special character
        if next_special > tok.pos {
            let chunk = tok.buffer[tok.pos..next_special].to_string();
            tok.append_text(&chunk);
            tok.pos = next_special;
        }

        if tok.pos >= tok.length {
            continue;
        }

        let c = tok.buffer[tok.pos..].chars().next().unwrap();
        tok.pos += c.len_utf8();

        match c {
            '\0' => {
                tok.emit_error("unexpected-null-character");
                tok.append_text_char('\u{FFFD}');
            }
            '&' => {
                // Accumulate ampersand; entity decoding is deferred to flush_text
                tok.append_text_char('&');
            }
            '<' => {
                tok.state = TokenizerState::RcdataLessThanSign;
                return false;
            }
            _ => unreachable!(),
        }
    }
}

fn state_rcdata_less_than_sign(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('/') => {
            tok.temp_buffer.clear();
            tok.state = TokenizerState::RcdataEndTagOpen;
            false
        }
        _ => {
            tok.append_text_char('<');
            tok.reconsume_current();
            tok.state = TokenizerState::Rcdata;
            false
        }
    }
}

fn state_rcdata_end_tag_open(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_name.clear();
            tok.current_tag_name.push(c.to_ascii_lowercase());
            tok.original_tag_name.clear();
            tok.original_tag_name.push(c);
            tok.state = TokenizerState::RcdataEndTagName;
            false
        }
        _ => {
            tok.append_text("</");
            tok.reconsume_current();
            tok.state = TokenizerState::Rcdata;
            false
        }
    }
}

fn state_rcdata_end_tag_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python: accumulate tag name, compare against rawtext_tag_name
    loop {
        match tok.get_char() {
            Some(c) if c.is_ascii_alphabetic() => {
                tok.current_tag_name.push(c.to_ascii_lowercase());
                tok.original_tag_name.push(c);
            }
            other => {
                let tag_name = tok.current_tag_name.clone();
                let matches = tok.rawtext_tag_name.as_deref() == Some(tag_name.as_str());
                if matches {
                    match other {
                        Some('>') => {
                            let attrs = std::collections::HashMap::new();
                            let tag = crate::tokens::Tag::new(TagKind::End, tag_name, attrs, false);
                            tok.flush_text(sink);
                            sink.process_token(crate::tokens::Token::Tag(tag));
                            tok.state = TokenizerState::Data;
                            tok.rawtext_tag_name = None;
                            tok.original_tag_name.clear();
                            return false;
                        }
                        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                            tok.current_tag_kind = TagKind::End;
                            tok.current_tag_attrs.clear();
                            tok.state = TokenizerState::BeforeAttributeName;
                            return false;
                        }
                        Some('/') => {
                            tok.flush_text(sink);
                            tok.current_tag_kind = TagKind::End;
                            tok.current_tag_attrs.clear();
                            tok.state = TokenizerState::SelfClosingStartTag;
                            return false;
                        }
                        _ => {}
                    }
                }
                // Not a matching end tag - emit as text (original case preserved)
                if other.is_none() {
                    tok.append_text("</");
                    let orig = tok.original_tag_name.clone();
                    tok.append_text(&orig);
                    tok.current_tag_name.clear();
                    tok.original_tag_name.clear();
                    tok.flush_text(sink);
                    tok.emit_eof(sink);
                    return true;
                }
                tok.append_text("</");
                let orig = tok.original_tag_name.clone();
                tok.append_text(&orig);
                tok.current_tag_name.clear();
                tok.original_tag_name.clear();
                tok.reconsume_current();
                tok.state = TokenizerState::Rcdata;
                return false;
            }
        }
    }
}

// ─── RAWTEXT states ──────────────────────────────────────────────────────────

fn state_rawtext(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python _state_rawtext with script escape detection.
    // When rawtext_tag_name == "script" and we see "<!--", transition to ScriptDataEscaped.
    loop {
        // Handle reconsume flag (set when returning from sub-states like RawtextLessThanSign)
        if tok.reconsume {
            tok.reconsume = false;
            if tok.pos > 0 {
                let s = &tok.buffer[..tok.pos];
                if let Some((i, _)) = s.char_indices().next_back() {
                    tok.pos = i;
                }
            }
        }

        if tok.pos >= tok.length {
            tok.flush_text(sink);
            tok.emit_eof(sink);
            return true;
        }

        // Find next '<' or '\0'
        let remaining = &tok.buffer[tok.pos..];
        let lt_pos = remaining.find('<').map(|i| tok.pos + i);
        let null_pos = remaining.find('\0').map(|i| tok.pos + i);

        // Determine which comes first
        let next_special = match (lt_pos, null_pos) {
            (Some(l), Some(n)) => Some(l.min(n)),
            (Some(l), None) => Some(l),
            (None, Some(n)) => Some(n),
            (None, None) => None,
        };

        match next_special {
            None => {
                // No special chars - consume rest as text
                let chunk = tok.buffer[tok.pos..].to_string();
                tok.append_text(&chunk);
                tok.pos = tok.length;
                // Loop back to EOF handling
            }
            Some(sp) => {
                // Consume text up to special char
                if sp > tok.pos {
                    let chunk = tok.buffer[tok.pos..sp].to_string();
                    tok.append_text(&chunk);
                    tok.pos = sp;
                }

                let c = tok.buffer[tok.pos..].chars().next().unwrap();
                tok.pos += c.len_utf8();

                if c == '\0' {
                    tok.emit_error("unexpected-null-character");
                    tok.append_text_char('\u{FFFD}');
                    // Continue loop
                } else {
                    // c == '<'
                    // Script escape detection
                    if tok.rawtext_tag_name.as_deref() == Some("script") {
                        if tok.pos + 2 < tok.length {
                            let peek = &tok.buffer[tok.pos..tok.pos + 3];
                            if peek == "!--" {
                                tok.append_text("<!--");
                                tok.pos += 3;
                                tok.state = TokenizerState::ScriptDataEscaped;
                                return false;
                            }
                        }
                    }
                    tok.state = TokenizerState::RawtextLessThanSign;
                    return false;
                }
            }
        }
    }
}

fn state_rawtext_less_than_sign(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('/') => {
            tok.temp_buffer.clear();
            tok.state = TokenizerState::RawtextEndTagOpen;
            false
        }
        _ => {
            tok.append_text_char('<');
            tok.reconsume_current();
            tok.state = TokenizerState::Rawtext;
            false
        }
    }
}

fn state_rawtext_end_tag_open(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_name.clear();
            tok.current_tag_name.push(c.to_ascii_lowercase());
            tok.original_tag_name.clear();
            tok.original_tag_name.push(c);
            tok.state = TokenizerState::RawtextEndTagName;
            false
        }
        _ => {
            tok.append_text("</");
            tok.reconsume_current();
            tok.state = TokenizerState::Rawtext;
            false
        }
    }
}

fn state_rawtext_end_tag_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python: accumulate tag name, compare against rawtext_tag_name
    loop {
        match tok.get_char() {
            Some(c) if c.is_ascii_alphabetic() => {
                tok.current_tag_name.push(c.to_ascii_lowercase());
                tok.original_tag_name.push(c);
            }
            other => {
                let tag_name = tok.current_tag_name.clone();
                let matches = tok.rawtext_tag_name.as_deref() == Some(tag_name.as_str());
                if matches {
                    match other {
                        Some('>') => {
                            let attrs = std::collections::HashMap::new();
                            let tag = crate::tokens::Tag::new(TagKind::End, tag_name, attrs, false);
                            tok.flush_text(sink);
                            sink.process_token(crate::tokens::Token::Tag(tag));
                            tok.state = TokenizerState::Data;
                            tok.rawtext_tag_name = None;
                            tok.original_tag_name.clear();
                            return false;
                        }
                        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                            tok.current_tag_kind = TagKind::End;
                            tok.current_tag_attrs.clear();
                            tok.state = TokenizerState::BeforeAttributeName;
                            return false;
                        }
                        Some('/') => {
                            tok.flush_text(sink);
                            tok.current_tag_kind = TagKind::End;
                            tok.current_tag_attrs.clear();
                            tok.state = TokenizerState::SelfClosingStartTag;
                            return false;
                        }
                        _ => {}
                    }
                }
                // Not a matching end tag - emit as text (original case preserved)
                if other.is_none() {
                    tok.append_text("</");
                    let orig = tok.original_tag_name.clone();
                    tok.append_text(&orig);
                    tok.current_tag_name.clear();
                    tok.original_tag_name.clear();
                    tok.flush_text(sink);
                    tok.emit_eof(sink);
                    return true;
                }
                tok.append_text("</");
                let orig = tok.original_tag_name.clone();
                tok.append_text(&orig);
                tok.current_tag_name.clear();
                tok.original_tag_name.clear();
                tok.reconsume_current();
                tok.state = TokenizerState::Rawtext;
                return false;
            }
        }
    }
}

// ─── PLAINTEXT state ─────────────────────────────────────────────────────────

fn state_plaintext(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            // Consume everything until NULL or EOF
            let rest = &tok.buffer[tok.pos..];
            if let Some(null_pos) = rest.find('\0') {
                tok.append_text(&tok.buffer[tok.pos..tok.pos + null_pos].to_string());
                tok.pos += null_pos;
            } else {
                tok.append_text(&tok.buffer[tok.pos..].to_string());
                tok.pos = tok.length;
            }
            false
        }
        None => {
            tok.emit_eof(sink);
            true
        }
    }
}

// ─── SCRIPT DATA states ─────────────────────────────────────────────────────

fn state_script_data(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('<') => {
            tok.state = TokenizerState::ScriptDataLessThanSign;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            while tok.pos < tok.length {
                let next = tok.buffer[tok.pos..].chars().next().unwrap();
                match next {
                    '<' | '\0' => break,
                    _ => {
                        tok.append_text_char(next);
                        tok.pos += next.len_utf8();
                    }
                }
            }
            false
        }
        None => {
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_less_than_sign(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('/') => {
            tok.temp_buffer.clear();
            tok.state = TokenizerState::ScriptDataEndTagOpen;
            false
        }
        Some('!') => {
            tok.append_text("<!");
            tok.state = TokenizerState::ScriptDataEscapeStart;
            false
        }
        _ => {
            tok.append_text_char('<');
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptData;
            false
        }
    }
}

fn state_script_data_end_tag_open(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_kind = TagKind::End;
            tok.current_tag_name.clear();
            tok.current_tag_attrs.clear();
            tok.current_tag_self_closing = false;
            tok.temp_buffer.clear();
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataEndTagName;
            false
        }
        _ => {
            tok.append_text("</");
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptData;
            false
        }
    }
}

fn state_script_data_end_tag_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('\t') | Some('\n') | Some('\x0C') | Some(' ') if tok.is_appropriate_end_tag() => {
            tok.current_tag_name = tok.temp_buffer.clone();
            tok.state = TokenizerState::BeforeAttributeName;
            false
        }
        Some('/') if tok.is_appropriate_end_tag() => {
            tok.current_tag_name = tok.temp_buffer.clone();
            tok.state = TokenizerState::SelfClosingStartTag;
            false
        }
        Some('>') if tok.is_appropriate_end_tag() => {
            tok.current_tag_name = tok.temp_buffer.clone();
            tok.state = TokenizerState::Data;
            tok.emit_current_tag(sink);
            false
        }
        Some(c) if c.is_ascii_alphabetic() => {
            tok.temp_buffer.push(c.to_ascii_lowercase());
            false
        }
        _ => {
            tok.append_text("</");
            tok.append_text(&tok.temp_buffer.clone());
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptData;
            false
        }
    }
}

fn state_script_data_escape_start(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            tok.state = TokenizerState::ScriptDataEscapeStartDash;
            false
        }
        _ => {
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptData;
            false
        }
    }
}

fn state_script_data_escape_start_dash(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            tok.state = TokenizerState::ScriptDataEscapedDashDash;
            false
        }
        _ => {
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptData;
            false
        }
    }
}

fn state_script_data_escaped(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            tok.state = TokenizerState::ScriptDataEscapedDash;
            false
        }
        Some('<') => {
            tok.state = TokenizerState::ScriptDataEscapedLessThanSign;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            false
        }
        None => {
            tok.emit_error("eof-in-script-html-comment-like-text");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_escaped_dash(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            tok.state = TokenizerState::ScriptDataEscapedDashDash;
            false
        }
        Some('<') => {
            tok.state = TokenizerState::ScriptDataEscapedLessThanSign;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
        None => {
            tok.emit_error("eof-in-script-html-comment-like-text");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_escaped_dash_dash(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            false
        }
        Some('<') => {
            tok.append_text_char('<');
            tok.state = TokenizerState::ScriptDataEscapedLessThanSign;
            false
        }
        Some('>') => {
            tok.append_text_char('>');
            tok.state = TokenizerState::Rawtext;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
        None => {
            tok.emit_error("eof-in-script-html-comment-like-text");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_escaped_less_than_sign(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('/') => {
            tok.temp_buffer.clear();
            tok.state = TokenizerState::ScriptDataEscapedEndTagOpen;
            false
        }
        Some(c) if c.is_ascii_alphabetic() => {
            tok.temp_buffer.clear();
            tok.append_text_char('<');
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataDoubleEscapeStart;
            false
        }
        _ => {
            tok.append_text_char('<');
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
    }
}

fn state_script_data_escaped_end_tag_open(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_name.clear();
            tok.original_tag_name.clear();
            tok.temp_buffer.clear();
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataEscapedEndTagName;
            false
        }
        _ => {
            tok.append_text("</");
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
    }
}

fn state_script_data_escaped_end_tag_name(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    // Mirrors Python _state_script_data_escaped_end_tag_name
    match tok.get_char() {
        Some(c) if c.is_ascii_alphabetic() => {
            tok.current_tag_name.push(c.to_ascii_lowercase());
            tok.original_tag_name.push(c);
            tok.temp_buffer.push(c);
            false
        }
        other => {
            let tag_name = tok.current_tag_name.clone();
            let is_appropriate = tok.rawtext_tag_name.as_deref() == Some(tag_name.as_str());

            if is_appropriate {
                match other {
                    Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                        tok.current_tag_kind = TagKind::End;
                        tok.current_tag_attrs.clear();
                        tok.state = TokenizerState::BeforeAttributeName;
                        return false;
                    }
                    Some('/') => {
                        tok.flush_text(sink);
                        tok.current_tag_kind = TagKind::End;
                        tok.current_tag_attrs.clear();
                        tok.state = TokenizerState::SelfClosingStartTag;
                        return false;
                    }
                    Some('>') => {
                        tok.flush_text(sink);
                        let attrs = std::collections::HashMap::new();
                        let tag = crate::tokens::Tag::new(TagKind::End, tag_name, attrs, false);
                        sink.process_token(crate::tokens::Token::Tag(tag));
                        tok.state = TokenizerState::Data;
                        tok.rawtext_tag_name = None;
                        tok.current_tag_name.clear();
                        tok.original_tag_name.clear();
                        return false;
                    }
                    _ => {}
                }
            }
            // Not appropriate - emit as text
            tok.append_text("</");
            let temp = tok.temp_buffer.clone();
            tok.append_text(&temp);
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
    }
}

fn state_script_data_double_escape_start(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c @ ('\t' | '\n' | '\x0C' | ' ' | '/' | '>')) => {
            tok.append_text_char(c);
            if tok.temp_buffer == "script" {
                tok.state = TokenizerState::ScriptDataDoubleEscaped;
            } else {
                tok.state = TokenizerState::ScriptDataEscaped;
            }
            false
        }
        Some(c) if c.is_ascii_alphabetic() => {
            tok.temp_buffer.push(c.to_ascii_lowercase());
            tok.append_text_char(c);
            false
        }
        _ => {
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataEscaped;
            false
        }
    }
}

fn state_script_data_double_escaped(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            tok.state = TokenizerState::ScriptDataDoubleEscapedDash;
            false
        }
        Some('<') => {
            tok.append_text_char('<');
            tok.state = TokenizerState::ScriptDataDoubleEscapedLessThanSign;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            false
        }
        None => {
            tok.emit_error("eof-in-script-html-comment-like-text");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_double_escaped_dash(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            tok.state = TokenizerState::ScriptDataDoubleEscapedDashDash;
            false
        }
        Some('<') => {
            tok.append_text_char('<');
            tok.state = TokenizerState::ScriptDataDoubleEscapedLessThanSign;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            tok.state = TokenizerState::ScriptDataDoubleEscaped;
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            tok.state = TokenizerState::ScriptDataDoubleEscaped;
            false
        }
        None => {
            tok.emit_error("eof-in-script-html-comment-like-text");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_double_escaped_dash_dash(tok: &mut Tokenizer, sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('-') => {
            tok.append_text_char('-');
            false
        }
        Some('<') => {
            tok.append_text_char('<');
            tok.state = TokenizerState::ScriptDataDoubleEscapedLessThanSign;
            false
        }
        Some('>') => {
            tok.append_text_char('>');
            tok.state = TokenizerState::Rawtext;
            false
        }
        Some('\0') => {
            tok.emit_error("unexpected-null-character");
            tok.append_text_char('\u{FFFD}');
            tok.state = TokenizerState::ScriptDataDoubleEscaped;
            false
        }
        Some(c) => {
            tok.append_text_char(c);
            tok.state = TokenizerState::ScriptDataDoubleEscaped;
            false
        }
        None => {
            tok.emit_error("eof-in-script-html-comment-like-text");
            tok.emit_eof(sink);
            true
        }
    }
}

fn state_script_data_double_escaped_less_than_sign(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some('/') => {
            tok.temp_buffer.clear();
            tok.append_text_char('/');
            tok.state = TokenizerState::ScriptDataDoubleEscapeEnd;
            false
        }
        Some(c) if c.is_ascii_alphabetic() => {
            tok.temp_buffer.clear();
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataDoubleEscapeStart;
            false
        }
        _ => {
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataDoubleEscaped;
            false
        }
    }
}

fn state_script_data_double_escape_end(tok: &mut Tokenizer, _sink: &mut dyn TokenSink) -> bool {
    match tok.get_char() {
        Some(c @ ('\t' | '\n' | '\x0C' | ' ' | '/' | '>')) => {
            tok.append_text_char(c);
            if tok.temp_buffer == "script" {
                tok.state = TokenizerState::ScriptDataEscaped;
            } else {
                tok.state = TokenizerState::ScriptDataDoubleEscaped;
            }
            false
        }
        Some(c) if c.is_ascii_alphabetic() => {
            tok.temp_buffer.push(c.to_ascii_lowercase());
            tok.append_text_char(c);
            false
        }
        _ => {
            tok.reconsume_current();
            tok.state = TokenizerState::ScriptDataDoubleEscaped;
            false
        }
    }
}

