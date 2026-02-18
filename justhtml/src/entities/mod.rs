//! HTML5 character entity decoding.
//!
//! Implements HTML5 character reference (entity) decoding per WHATWG spec §13.2.5.
//! Supports both named entities (&amp;, &nbsp;) and numeric references (&#60;, &#x3C;).
//!
//! Corresponds to Python's `entities.py`.

mod entity_data;

use std::collections::HashMap;
use std::sync::OnceLock;

use phf::phf_map;
use phf::phf_set;

// ─── Numeric replacements (HTML5 §13.2.5.73) ────────────────────────────────

/// HTML5 numeric character reference replacements for Windows-1252 range.
static NUMERIC_REPLACEMENTS: phf::Map<u32, char> = phf_map! {
    0x00u32 => '\u{FFFD}',  // NULL -> REPLACEMENT CHARACTER
    0x80u32 => '\u{20AC}',  // EURO SIGN
    0x82u32 => '\u{201A}',  // SINGLE LOW-9 QUOTATION MARK
    0x83u32 => '\u{0192}',  // LATIN SMALL LETTER F WITH HOOK
    0x84u32 => '\u{201E}',  // DOUBLE LOW-9 QUOTATION MARK
    0x85u32 => '\u{2026}',  // HORIZONTAL ELLIPSIS
    0x86u32 => '\u{2020}',  // DAGGER
    0x87u32 => '\u{2021}',  // DOUBLE DAGGER
    0x88u32 => '\u{02C6}',  // MODIFIER LETTER CIRCUMFLEX ACCENT
    0x89u32 => '\u{2030}',  // PER MILLE SIGN
    0x8Au32 => '\u{0160}',  // LATIN CAPITAL LETTER S WITH CARON
    0x8Bu32 => '\u{2039}',  // SINGLE LEFT-POINTING ANGLE QUOTATION MARK
    0x8Cu32 => '\u{0152}',  // LATIN CAPITAL LIGATURE OE
    0x8Eu32 => '\u{017D}',  // LATIN CAPITAL LETTER Z WITH CARON
    0x91u32 => '\u{2018}',  // LEFT SINGLE QUOTATION MARK
    0x92u32 => '\u{2019}',  // RIGHT SINGLE QUOTATION MARK
    0x93u32 => '\u{201C}',  // LEFT DOUBLE QUOTATION MARK
    0x94u32 => '\u{201D}',  // RIGHT DOUBLE QUOTATION MARK
    0x95u32 => '\u{2022}',  // BULLET
    0x96u32 => '\u{2013}',  // EN DASH
    0x97u32 => '\u{2014}',  // EM DASH
    0x98u32 => '\u{02DC}',  // SMALL TILDE
    0x99u32 => '\u{2122}',  // TRADE MARK SIGN
    0x9Au32 => '\u{0161}',  // LATIN SMALL LETTER S WITH CARON
    0x9Bu32 => '\u{203A}',  // SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
    0x9Cu32 => '\u{0153}',  // LATIN SMALL LIGATURE OE
    0x9Eu32 => '\u{017E}',  // LATIN SMALL LETTER Z WITH CARON
    0x9Fu32 => '\u{0178}',  // LATIN CAPITAL LETTER Y WITH DIAERESIS
};

// ─── Legacy entities (work without semicolons) ──────────────────────────────

/// Legacy named character references that can be used without semicolons.
/// Per HTML5 spec, these are primarily ISO-8859-1 (Latin-1) entities from HTML4.
static LEGACY_ENTITIES: phf::Set<&'static str> = phf_set! {
    "AElig", "AMP", "Aacute", "Acirc", "Agrave", "Aring", "Atilde", "Auml",
    "COPY", "Ccedil", "ETH", "Eacute", "Ecirc", "Egrave", "Euml",
    "GT", "Iacute", "Icirc", "Igrave", "Iuml", "LT",
    "Ntilde", "Oacute", "Ocirc", "Ograve", "Oslash", "Otilde", "Ouml",
    "QUOT", "REG", "THORN", "Uacute", "Ucirc", "Ugrave", "Uuml", "Yacute",
    "aacute", "acirc", "acute", "aelig", "agrave", "amp", "aring", "atilde", "auml",
    "brvbar", "ccedil", "cedil", "cent", "copy", "curren",
    "deg", "divide",
    "eacute", "ecirc", "egrave", "eth", "euml",
    "frac12", "frac14", "frac34",
    "gt",
    "iacute", "icirc", "iexcl", "igrave", "iquest", "iuml",
    "laquo", "lt",
    "macr", "micro", "middot",
    "nbsp", "not", "ntilde",
    "oacute", "ocirc", "ograve", "ordf", "ordm", "oslash", "otilde", "ouml",
    "para", "plusmn", "pound",
    "quot", "raquo", "reg",
    "sect", "shy", "sup1", "sup2", "sup3", "szlig",
    "thorn", "times",
    "uacute", "ucirc", "ugrave", "uml", "uuml",
    "yacute", "yen", "yuml",
};

// ─── Named entity lookup (lazily initialized HashMap) ────────────────────────

fn named_entities() -> &'static HashMap<&'static str, &'static str> {
    static INSTANCE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let data = entity_data::ENTITY_DATA;
        let mut map = HashMap::with_capacity(data.len());
        for &(name, value) in data {
            map.insert(name, value);
        }
        map
    })
}

/// Check if an entity name is a legacy entity.
pub fn is_legacy_entity(name: &str) -> bool {
    LEGACY_ENTITIES.contains(name)
}

/// Look up a named entity.
pub fn lookup_named_entity(name: &str) -> Option<&'static str> {
    named_entities().get(name).copied()
}

// ─── Numeric entity decoding ─────────────────────────────────────────────────

/// Decode a numeric character reference like &#60; or &#x3C;.
///
/// # Arguments
/// * `text` - The numeric part (without &# prefix or ; suffix)
/// * `is_hex` - Whether this is hexadecimal (&#x) or decimal (&#)
///
/// # Returns
/// The decoded character as a string
pub fn decode_numeric_entity(text: &str, is_hex: bool) -> String {
    let codepoint = if is_hex {
        u32::from_str_radix(text, 16).unwrap_or(0)
    } else {
        text.parse::<u32>().unwrap_or(0)
    };

    // Apply HTML5 replacements for certain ranges
    if let Some(&replacement) = NUMERIC_REPLACEMENTS.get(&codepoint) {
        return replacement.to_string();
    }

    // Invalid ranges per HTML5 spec
    if codepoint > 0x10FFFF {
        return "\u{FFFD}".to_string(); // REPLACEMENT CHARACTER
    }
    if (0xD800..=0xDFFF).contains(&codepoint) {
        // Surrogate range
        return "\u{FFFD}".to_string();
    }

    match char::from_u32(codepoint) {
        Some(c) => c.to_string(),
        None => "\u{FFFD}".to_string(),
    }
}

// ─── Entity decoding in text ─────────────────────────────────────────────────

/// Decode all HTML entities in text.
///
/// Handles named entities (&amp; &lt; &gt; &quot; &nbsp; etc.),
/// decimal numeric (&#60; &#160; etc.), and hex numeric (&#x3C; &#xA0; etc.).
///
/// # Arguments
/// * `text` - Input text potentially containing entities
/// * `in_attribute` - Whether this is attribute value (stricter rules for legacy entities)
///
/// # Returns
/// Text with entities decoded
pub fn decode_entities_in_text(text: &str, in_attribute: bool) -> String {
    let bytes = text.as_bytes();
    let length = bytes.len();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < length {
        // Find next ampersand
        let next_amp = match text[i..].find('&') {
            Some(pos) => i + pos,
            None => {
                result.push_str(&text[i..]);
                break;
            }
        };

        if next_amp > i {
            result.push_str(&text[i..next_amp]);
        }

        i = next_amp;
        let j_start = i + 1;

        // Check for numeric entity
        if j_start < length && bytes[j_start] == b'#' {
            let mut j = j_start + 1;
            let is_hex;

            if j < length && (bytes[j] == b'x' || bytes[j] == b'X') {
                is_hex = true;
                j += 1;
            } else {
                is_hex = false;
            }

            // Collect digits
            let digit_start = j;
            if is_hex {
                while j < length && bytes[j].is_ascii_hexdigit() {
                    j += 1;
                }
            } else {
                while j < length && bytes[j].is_ascii_digit() {
                    j += 1;
                }
            }

            let has_semicolon = j < length && bytes[j] == b';';
            let digit_text = &text[digit_start..j];

            if !digit_text.is_empty() {
                result.push_str(&decode_numeric_entity(digit_text, is_hex));
                i = if has_semicolon { j + 1 } else { j };
                continue;
            }

            // Invalid numeric entity, keep as-is
            let end = if has_semicolon { j + 1 } else { j };
            result.push_str(&text[i..end]);
            i = end;
            continue;
        }

        // Named entity - collect alphanumeric characters
        let mut j = j_start;
        while j < length && (bytes[j].is_ascii_alphanumeric()) {
            j += 1;
        }

        let entity_name = &text[j_start..j];
        let has_semicolon = j < length && bytes[j] == b';';

        if entity_name.is_empty() {
            result.push('&');
            i += 1;
            continue;
        }

        // Try exact match first (with semicolon expected)
        if has_semicolon {
            if let Some(replacement) = lookup_named_entity(entity_name) {
                result.push_str(replacement);
                i = j + 1;
                continue;
            }
        }

        // If semicolon present but no exact match, allow legacy prefix match in text
        if has_semicolon && !in_attribute {
            if let Some((matched, match_len)) = find_legacy_prefix_match(entity_name) {
                result.push_str(matched);
                i = i + 1 + match_len;
                continue;
            }
        }

        // Try without semicolon for legacy compatibility
        if LEGACY_ENTITIES.contains(entity_name) {
            if let Some(replacement) = lookup_named_entity(entity_name) {
                // Legacy entities without semicolon have strict rules in attributes:
                // don't decode if followed by alphanumeric or '='
                // Per HTML5 spec §13.2.5.72
                let next_char = if j < length { Some(bytes[j]) } else { None };
                if in_attribute {
                    if let Some(nc) = next_char {
                        if nc.is_ascii_alphanumeric() || nc == b'=' {
                            result.push('&');
                            i += 1;
                            continue;
                        }
                    }
                }

                result.push_str(replacement);
                i = j;
                continue;
            }
        }

        // Try longest prefix match for legacy entities without semicolon
        if let Some((matched, match_len)) = find_legacy_prefix_match(entity_name) {
            let end_pos = i + 1 + match_len;
            if in_attribute {
                // In attributes with prefix match, don't decode if followed by alphanumeric or =
                result.push('&');
                i += 1;
                continue;
            }

            result.push_str(matched);
            i = end_pos;
            continue;
        }

        // No match found
        if has_semicolon {
            result.push_str(&text[i..j + 1]);
            i = j + 1;
        } else {
            result.push('&');
            i += 1;
        }
    }

    result
}

/// Find the longest legacy entity prefix match for a name.
fn find_legacy_prefix_match(entity_name: &str) -> Option<(&'static str, usize)> {
    // Try from longest to shortest prefix
    for k in (1..=entity_name.len()).rev() {
        let prefix = &entity_name[..k];
        if LEGACY_ENTITIES.contains(prefix) {
            if let Some(replacement) = lookup_named_entity(prefix) {
                return Some((replacement, k));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_entity_lookup() {
        assert_eq!(lookup_named_entity("amp"), Some("&"));
        assert_eq!(lookup_named_entity("lt"), Some("<"));
        assert_eq!(lookup_named_entity("gt"), Some(">"));
        assert_eq!(lookup_named_entity("quot"), Some("\""));
        assert_eq!(lookup_named_entity("nbsp"), Some("\u{00A0}"));
        assert!(lookup_named_entity("nonexistent").is_none());
    }

    #[test]
    fn test_entity_count() {
        assert_eq!(entity_data::ENTITY_DATA.len(), 2125);
    }

    #[test]
    fn test_legacy_entity_check() {
        assert!(is_legacy_entity("amp"));
        assert!(is_legacy_entity("lt"));
        assert!(is_legacy_entity("gt"));
        assert!(is_legacy_entity("nbsp"));
        assert!(!is_legacy_entity("lambda")); // Not a legacy entity
    }

    #[test]
    fn test_numeric_entity_decimal() {
        assert_eq!(decode_numeric_entity("60", false), "<");
        assert_eq!(decode_numeric_entity("160", false), "\u{00A0}");
        assert_eq!(decode_numeric_entity("65", false), "A");
    }

    #[test]
    fn test_numeric_entity_hex() {
        assert_eq!(decode_numeric_entity("3C", true), "<");
        assert_eq!(decode_numeric_entity("A0", true), "\u{00A0}");
        assert_eq!(decode_numeric_entity("41", true), "A");
    }

    #[test]
    fn test_numeric_entity_replacements() {
        // NULL -> REPLACEMENT CHARACTER
        assert_eq!(decode_numeric_entity("0", false), "\u{FFFD}");
        // Windows-1252 range
        assert_eq!(decode_numeric_entity("128", false), "\u{20AC}"); // 0x80 -> EURO SIGN
        assert_eq!(decode_numeric_entity("80", true), "\u{20AC}");
    }

    #[test]
    fn test_numeric_entity_invalid() {
        // Surrogate range
        assert_eq!(decode_numeric_entity("D800", true), "\u{FFFD}");
        assert_eq!(decode_numeric_entity("DFFF", true), "\u{FFFD}");
        // Beyond Unicode range
        assert_eq!(decode_numeric_entity("110000", true), "\u{FFFD}");
    }

    #[test]
    fn test_decode_entities_named() {
        assert_eq!(decode_entities_in_text("&amp;", false), "&");
        assert_eq!(decode_entities_in_text("&lt;", false), "<");
        assert_eq!(decode_entities_in_text("&gt;", false), ">");
        assert_eq!(decode_entities_in_text("hello &amp; world", false), "hello & world");
    }

    #[test]
    fn test_decode_entities_numeric() {
        assert_eq!(decode_entities_in_text("&#60;", false), "<");
        assert_eq!(decode_entities_in_text("&#x3C;", false), "<");
        assert_eq!(decode_entities_in_text("&#X3C;", false), "<");
    }

    #[test]
    fn test_decode_entities_no_semicolon_legacy() {
        // Legacy entities work without semicolons
        assert_eq!(decode_entities_in_text("&amp text", false), "& text");
        assert_eq!(decode_entities_in_text("&lt text", false), "< text");
    }

    #[test]
    fn test_decode_entities_in_attribute_strict() {
        // In attributes, legacy entities without semicolon followed by alphanumeric are NOT decoded
        assert_eq!(decode_entities_in_text("&ampx", true), "&ampx");
        // But with semicolon they are decoded
        assert_eq!(decode_entities_in_text("&amp;x", true), "&x");
    }

    #[test]
    fn test_decode_entities_prefix_match() {
        // &notit -> &not is a legacy entity, should match
        assert_eq!(decode_entities_in_text("&notit;", false), "\u{00AC}it;");
    }

    #[test]
    fn test_decode_entities_bare_ampersand() {
        assert_eq!(decode_entities_in_text("& ", false), "& ");
        assert_eq!(decode_entities_in_text("&", false), "&");
    }

    #[test]
    fn test_decode_entities_invalid_numeric() {
        // Invalid numeric entity kept as-is
        assert_eq!(decode_entities_in_text("&#;", false), "&#;");
        assert_eq!(decode_entities_in_text("&#x;", false), "&#x;");
    }

    #[test]
    fn test_decode_entities_mixed() {
        assert_eq!(
            decode_entities_in_text("a &amp; b &#60; c &lt; d", false),
            "a & b < c < d"
        );
    }
}
