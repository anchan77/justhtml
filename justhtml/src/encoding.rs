//! HTML encoding sniffing and decoding.
//!
//! Implements the HTML encoding sniffing behavior needed for the html5lib-tests
//! encoding fixtures.
//!
//! Inputs are bytes and an optional transport-supplied encoding label.
//! Outputs are a decoded Unicode string and the chosen encoding name.
//!
//! Corresponds to Python's `encoding.py`.

use encoding_rs::{UTF_8, WINDOWS_1252, ISO_8859_2, EUC_JP, UTF_16LE, UTF_16BE};

const ASCII_WHITESPACE: [u8; 5] = [0x09, 0x0A, 0x0C, 0x0D, 0x20];

fn is_ascii_whitespace(b: u8) -> bool {
    ASCII_WHITESPACE.contains(&b)
}

fn ascii_lower(b: u8) -> u8 {
    if (0x41..=0x5A).contains(&b) {
        b | 0x20
    } else {
        b
    }
}

fn is_ascii_alpha(b: u8) -> bool {
    let lower = ascii_lower(b);
    (0x61..=0x7A).contains(&lower)
}

fn skip_ascii_whitespace(data: &[u8], mut i: usize) -> usize {
    while i < data.len() && is_ascii_whitespace(data[i]) {
        i += 1;
    }
    i
}

/// Strip ASCII whitespace from both ends of a byte slice.
pub fn strip_ascii_whitespace(value: Option<&[u8]>) -> Option<Vec<u8>> {
    let value = value?;
    let mut start = 0;
    let mut end = value.len();
    while start < end && is_ascii_whitespace(value[start]) {
        start += 1;
    }
    while end > start && is_ascii_whitespace(value[end - 1]) {
        end -= 1;
    }
    Some(value[start..end].to_vec())
}

/// Normalize an encoding label to a canonical name.
///
/// Returns `None` if the label is not recognized or is empty.
pub fn normalize_encoding_label(label: Option<&str>) -> Option<String> {
    let label = label?;
    let s = label.trim();
    if s.is_empty() {
        return None;
    }

    let s = s.to_ascii_lowercase();

    // Security: never allow utf-7
    if matches!(s.as_str(), "utf-7" | "utf7" | "x-utf-7") {
        return Some("windows-1252".to_string());
    }

    if matches!(s.as_str(), "utf-8" | "utf8") {
        return Some("utf-8".to_string());
    }

    // HTML treats latin-1 labels as windows-1252
    if matches!(
        s.as_str(),
        "iso-8859-1" | "iso8859-1" | "latin1" | "latin-1" | "l1" | "cp819" | "ibm819"
    ) {
        return Some("windows-1252".to_string());
    }

    if matches!(
        s.as_str(),
        "windows-1252" | "windows1252" | "cp1252" | "x-cp1252"
    ) {
        return Some("windows-1252".to_string());
    }

    if matches!(s.as_str(), "iso-8859-2" | "iso8859-2" | "latin2" | "latin-2") {
        return Some("iso-8859-2".to_string());
    }

    if matches!(s.as_str(), "euc-jp" | "eucjp") {
        return Some("euc-jp".to_string());
    }

    if matches!(s.as_str(), "utf-16" | "utf16") {
        return Some("utf-16".to_string());
    }
    if matches!(s.as_str(), "utf-16le" | "utf16le") {
        return Some("utf-16le".to_string());
    }
    if matches!(s.as_str(), "utf-16be" | "utf16be") {
        return Some("utf-16be".to_string());
    }

    None
}

fn normalize_meta_declared_encoding(label: &[u8]) -> Option<String> {
    let label_str = std::str::from_utf8(label).ok().or_else(|| {
        // Try ASCII decoding by ignoring non-ASCII
        None
    });

    let label_str = match label_str {
        Some(s) => s,
        None => {
            // Fallback: convert bytes to string ignoring non-ASCII
            let ascii: String = label.iter().filter(|&&b| b < 128).map(|&b| b as char).collect();
            return normalize_encoding_label(Some(&ascii)).and_then(|enc| {
                normalize_utf16_to_utf8(&enc)
            });
        }
    };

    let enc = normalize_encoding_label(Some(label_str))?;
    normalize_utf16_to_utf8(&enc)
}

fn normalize_utf16_to_utf8(enc: &str) -> Option<String> {
    // Per HTML meta charset handling: ignore UTF-16/UTF-32 declarations and
    // treat them as UTF-8.
    match enc {
        "utf-16" | "utf-16le" | "utf-16be" | "utf-32" | "utf-32le" | "utf-32be" => {
            Some("utf-8".to_string())
        }
        _ => Some(enc.to_string()),
    }
}

/// Detect BOM (Byte Order Mark) at the start of data.
///
/// Returns (encoding_name, bytes_to_skip).
pub fn sniff_bom(data: &[u8]) -> (Option<String>, usize) {
    if data.len() >= 3 && &data[0..3] == b"\xef\xbb\xbf" {
        return (Some("utf-8".to_string()), 3);
    }
    if data.len() >= 2 && &data[0..2] == b"\xff\xfe" {
        return (Some("utf-16le".to_string()), 2);
    }
    if data.len() >= 2 && &data[0..2] == b"\xfe\xff" {
        return (Some("utf-16be".to_string()), 2);
    }
    (None, 0)
}

/// Extract charset from Content-Type style value.
pub fn extract_charset_from_content(content_bytes: &[u8]) -> Option<Vec<u8>> {
    if content_bytes.is_empty() {
        return None;
    }

    // Normalize whitespace to spaces for robust matching
    let mut normalized = Vec::with_capacity(content_bytes.len());
    for &ch in content_bytes {
        if is_ascii_whitespace(ch) {
            normalized.push(0x20); // space
        } else {
            normalized.push(ascii_lower(ch));
        }
    }

    let charset_bytes = b"charset";
    let idx = normalized
        .windows(charset_bytes.len())
        .position(|w| w == charset_bytes)?;

    let mut i = idx + charset_bytes.len();
    let n = normalized.len();

    // Skip whitespace
    while i < n && is_ascii_whitespace(normalized[i]) {
        i += 1;
    }
    if i >= n || normalized[i] != b'=' {
        return None;
    }
    i += 1;
    // Skip whitespace
    while i < n && is_ascii_whitespace(normalized[i]) {
        i += 1;
    }
    if i >= n {
        return None;
    }

    let quote: Option<u8> = if normalized[i] == b'"' || normalized[i] == b'\'' {
        let q = normalized[i];
        i += 1;
        Some(q)
    } else {
        None
    };

    let start = i;
    while i < n {
        let ch = normalized[i];
        if let Some(q) = quote {
            if ch == q {
                break;
            }
        } else if is_ascii_whitespace(ch) || ch == b';' {
            break;
        }
        i += 1;
    }

    // Unclosed quote
    if let Some(q) = quote {
        if i >= n || normalized[i] != q {
            return None;
        }
    }

    Some(normalized[start..i].to_vec())
}

/// Prescan the first part of an HTML document for meta charset declarations.
pub fn prescan_for_meta_charset(data: &[u8]) -> Option<String> {
    let max_non_comment = 1024;
    let max_total_scan = 65536;
    let n = data.len();
    let mut i = 0;
    let mut non_comment = 0;

    while i < n && i < max_total_scan && non_comment < max_non_comment {
        if data[i] != b'<' {
            i += 1;
            non_comment += 1;
            continue;
        }

        // Comment: <!--
        if i + 3 < n && &data[i + 1..i + 4] == b"!--" {
            let end = find_bytes(data, b"-->", i + 4);
            if end.is_none() {
                return None;
            }
            i = end.unwrap() + 3;
            continue;
        }

        // Tag open
        let j = i + 1;
        if j < n && data[j] == b'/' {
            // Skip end tag
            let mut k = i;
            let mut quote: Option<u8> = None;
            while k < n && k < max_total_scan && non_comment < max_non_comment {
                let ch = data[k];
                if quote.is_none() {
                    if ch == b'"' || ch == b'\'' {
                        quote = Some(ch);
                    } else if ch == b'>' {
                        k += 1;
                        non_comment += 1;
                        break;
                    }
                } else if Some(ch) == quote {
                    quote = None;
                }
                k += 1;
                non_comment += 1;
            }
            i = k;
            continue;
        }

        if j >= n || !is_ascii_alpha(data[j]) {
            i += 1;
            non_comment += 1;
            continue;
        }

        let name_start = j;
        let mut jj = j;
        while jj < n && is_ascii_alpha(data[jj]) {
            jj += 1;
        }

        let tag_name: Vec<u8> = data[name_start..jj].iter().map(|&b| ascii_lower(b)).collect();
        if tag_name != b"meta" {
            // Skip the rest of this tag
            let mut k = i;
            let mut quote: Option<u8> = None;
            while k < n && k < max_total_scan && non_comment < max_non_comment {
                let ch = data[k];
                if quote.is_none() {
                    if ch == b'"' || ch == b'\'' {
                        quote = Some(ch);
                    } else if ch == b'>' {
                        k += 1;
                        non_comment += 1;
                        break;
                    }
                } else if Some(ch) == quote {
                    quote = None;
                }
                k += 1;
                non_comment += 1;
            }
            i = k;
            continue;
        }

        // Parse meta tag attributes
        let mut charset: Option<Vec<u8>> = None;
        let mut http_equiv: Option<Vec<u8>> = None;
        let mut content: Option<Vec<u8>> = None;

        let mut k = jj;
        let mut saw_gt = false;
        let start_i = i;

        while k < n && k < max_total_scan {
            let ch = data[k];
            if ch == b'>' {
                saw_gt = true;
                k += 1;
                break;
            }

            if ch == b'<' {
                break;
            }

            if is_ascii_whitespace(ch) || ch == b'/' {
                k += 1;
                continue;
            }

            // Attribute name
            let attr_start = k;
            while k < n {
                let ch2 = data[k];
                if is_ascii_whitespace(ch2) || ch2 == b'=' || ch2 == b'>' || ch2 == b'/' || ch2 == b'<' {
                    break;
                }
                k += 1;
            }
            let attr_name: Vec<u8> = data[attr_start..k].iter().map(|&b| ascii_lower(b)).collect();
            k = skip_ascii_whitespace(data, k);

            let mut value: Option<Vec<u8>> = None;
            if k < n && data[k] == b'=' {
                k += 1;
                k = skip_ascii_whitespace(data, k);
                if k >= n {
                    break;
                }

                let quote: Option<u8> = if data[k] == b'"' || data[k] == b'\'' {
                    let q = data[k];
                    k += 1;
                    Some(q)
                } else {
                    None
                };

                if let Some(q) = quote {
                    let val_start = k;
                    let end_quote = data[k..].iter().position(|&b| b == q);
                    if end_quote.is_none() {
                        // Unclosed quote: ignore this meta
                        i += 1;
                        non_comment += 1;
                        charset = None;
                        http_equiv = None;
                        content = None;
                        saw_gt = false;
                        break;
                    }
                    let end_pos = k + end_quote.unwrap();
                    value = Some(data[val_start..end_pos].to_vec());
                    k = end_pos + 1;
                } else {
                    let val_start = k;
                    while k < n {
                        let ch2 = data[k];
                        if is_ascii_whitespace(ch2) || ch2 == b'>' || ch2 == b'<' {
                            break;
                        }
                        k += 1;
                    }
                    value = Some(data[val_start..k].to_vec());
                }
            }

            if attr_name == b"charset" {
                charset = strip_ascii_whitespace(value.as_deref());
            } else if attr_name == b"http-equiv" {
                http_equiv = value;
            } else if attr_name == b"content" {
                content = value;
            }
        }

        if saw_gt {
            if let Some(ref cs) = charset {
                if !cs.is_empty() {
                    if let Some(enc) = normalize_meta_declared_encoding(cs) {
                        return Some(enc);
                    }
                }
            }

            if let (Some(ref he), Some(ref ct)) = (&http_equiv, &content) {
                let he_lower: Vec<u8> = he.iter().map(|&b| ascii_lower(b)).collect();
                if he_lower == b"content-type" {
                    if let Some(extracted) = extract_charset_from_content(ct) {
                        if let Some(enc) = normalize_meta_declared_encoding(&extracted) {
                            return Some(enc);
                        }
                    }
                }
            }

            // Continue scanning after this tag
            i = k;
            let consumed = i - start_i;
            non_comment += consumed;
        } else {
            i += 1;
            non_comment += 1;
        }
    }

    None
}

/// Sniff the HTML encoding from byte data and optional transport encoding.
///
/// Returns (encoding_name, bytes_to_skip).
pub fn sniff_html_encoding(data: &[u8], transport_encoding: Option<&str>) -> (String, usize) {
    // Transport overrides everything
    if let Some(enc) = normalize_encoding_label(transport_encoding) {
        return (enc, 0);
    }

    let (bom_enc, bom_len) = sniff_bom(data);
    if let Some(enc) = bom_enc {
        return (enc, bom_len);
    }

    if let Some(meta_enc) = prescan_for_meta_charset(data) {
        return (meta_enc, 0);
    }

    ("windows-1252".to_string(), 0)
}

/// Decode an HTML byte stream using HTML encoding sniffing.
///
/// Returns (decoded_text, encoding_name).
pub fn decode_html(data: &[u8], transport_encoding: Option<&str>) -> (String, String) {
    let (enc, bom_len) = sniff_html_encoding(data, transport_encoding);

    let payload = if bom_len > 0 { &data[bom_len..] } else { data };

    match enc.as_str() {
        "windows-1252" => {
            let (decoded, _, _) = WINDOWS_1252.decode(payload);
            (decoded.into_owned(), "windows-1252".to_string())
        }
        "iso-8859-2" => {
            let (decoded, _, _) = ISO_8859_2.decode(payload);
            (decoded.into_owned(), "iso-8859-2".to_string())
        }
        "euc-jp" => {
            let (decoded, _, _) = EUC_JP.decode(payload);
            (decoded.into_owned(), "euc-jp".to_string())
        }
        "utf-16le" => {
            let (decoded, _, _) = UTF_16LE.decode(payload);
            (decoded.into_owned(), "utf-16le".to_string())
        }
        "utf-16be" => {
            let (decoded, _, _) = UTF_16BE.decode(payload);
            (decoded.into_owned(), "utf-16be".to_string())
        }
        "utf-16" => {
            // UTF-16 with BOM auto-detection
            // encoding_rs doesn't have a generic UTF-16, use the platform default
            // For data without BOM we default to little-endian
            if payload.len() >= 2 {
                if payload[0] == 0xFF && payload[1] == 0xFE {
                    let (decoded, _, _) = UTF_16LE.decode(&payload[2..]);
                    return (decoded.into_owned(), "utf-16".to_string());
                }
                if payload[0] == 0xFE && payload[1] == 0xFF {
                    let (decoded, _, _) = UTF_16BE.decode(&payload[2..]);
                    return (decoded.into_owned(), "utf-16".to_string());
                }
            }
            // Default: UTF-16LE
            let (decoded, _, _) = UTF_16LE.decode(payload);
            (decoded.into_owned(), "utf-16".to_string())
        }
        _ => {
            // Default: UTF-8
            let (decoded, _, _) = UTF_8.decode(payload);
            (decoded.into_owned(), "utf-8".to_string())
        }
    }
}

/// Find a byte pattern in data starting from a position.
fn find_bytes(data: &[u8], pattern: &[u8], from: usize) -> Option<usize> {
    if from >= data.len() || pattern.is_empty() {
        return None;
    }
    data[from..]
        .windows(pattern.len())
        .position(|w| w == pattern)
        .map(|pos| from + pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_encoding_label_none() {
        assert!(normalize_encoding_label(None).is_none());
        assert!(normalize_encoding_label(Some("")).is_none());
        assert!(normalize_encoding_label(Some("   ")).is_none());
    }

    #[test]
    fn test_normalize_encoding_label_utf8() {
        assert_eq!(normalize_encoding_label(Some("UTF-8")), Some("utf-8".to_string()));
        assert_eq!(normalize_encoding_label(Some("utf8")), Some("utf-8".to_string()));
    }

    #[test]
    fn test_normalize_encoding_label_security() {
        assert_eq!(normalize_encoding_label(Some("utf7")), Some("windows-1252".to_string()));
    }

    #[test]
    fn test_normalize_encoding_label_latin1() {
        assert_eq!(normalize_encoding_label(Some("iso-8859-1")), Some("windows-1252".to_string()));
    }

    #[test]
    fn test_normalize_encoding_label_iso8859_2() {
        assert_eq!(normalize_encoding_label(Some("iso8859-2")), Some("iso-8859-2".to_string()));
    }

    #[test]
    fn test_normalize_encoding_label_unknown() {
        assert!(normalize_encoding_label(Some("koi8-r")).is_none());
    }

    #[test]
    fn test_sniff_transport_overrides() {
        let data = b"<meta charset=iso8859-2>";
        let (enc_name, bom_len) = sniff_html_encoding(data, Some("utf-8"));
        assert_eq!(enc_name, "utf-8");
        assert_eq!(bom_len, 0);
    }

    #[test]
    fn test_sniff_bom_utf16() {
        assert_eq!(sniff_html_encoding(b"\xff\xfeh\x00i\x00", None).0, "utf-16le");
        assert_eq!(sniff_html_encoding(b"\xfe\xff\x00h\x00i", None).0, "utf-16be");
    }

    #[test]
    fn test_extract_charset_from_content_empty() {
        assert!(extract_charset_from_content(b"").is_none());
    }

    #[test]
    fn test_extract_charset_from_content_uppercase() {
        assert_eq!(
            extract_charset_from_content(b"TEXT/HTML; CHARSET=UTF-8"),
            Some(b"utf-8".to_vec())
        );
    }

    #[test]
    fn test_extract_charset_from_content_no_charset() {
        assert!(extract_charset_from_content(b"text/html").is_none());
        assert!(extract_charset_from_content(b"charset").is_none());
        assert!(extract_charset_from_content(b"charset;").is_none());
    }

    #[test]
    fn test_extract_charset_from_content_various() {
        assert_eq!(
            extract_charset_from_content(b"text/html; charset=iso8859-2"),
            Some(b"iso8859-2".to_vec())
        );
        assert_eq!(
            extract_charset_from_content(b"text/html; charset='utf-8'"),
            Some(b"utf-8".to_vec())
        );
        assert_eq!(
            extract_charset_from_content(b"text/html; charset=\"utf-8\""),
            Some(b"utf-8".to_vec())
        );
    }

    #[test]
    fn test_extract_charset_unterminated_quote() {
        assert!(extract_charset_from_content(b"text/html; charset='utf-8").is_none());
    }

    #[test]
    fn test_prescan_edge_cases() {
        assert!(prescan_for_meta_charset(b"<!--").is_none());
        assert!(prescan_for_meta_charset(b"</a title='x'>").is_none());
        assert!(prescan_for_meta_charset(b"</a title=\"x\"").is_none());
        assert!(prescan_for_meta_charset(b"<p title=\"x><meta charset=iso8859-2>").is_none());
        assert!(prescan_for_meta_charset(b"<meta charset=\"utf-8").is_none());
        assert!(prescan_for_meta_charset(b"<meta charset").is_none());
        assert!(prescan_for_meta_charset(b"<meta charset=utf-8").is_none());
    }

    #[test]
    fn test_decode_html_branches() {
        let (text, name) = decode_html(b"\x80", None);
        assert_eq!(text, "\u{20ac}");
        assert_eq!(name, "windows-1252");

        let (text, name) = decode_html(b"abc", Some("iso-8859-2"));
        assert_eq!(text, "abc");
        assert_eq!(name, "iso-8859-2");

        let (text, name) = decode_html(b"abc", Some("euc-jp"));
        assert_eq!(text, "abc");
        assert_eq!(name, "euc-jp");

        let (text, name) = decode_html(b"\xff\xfeh\x00i\x00", None);
        assert_eq!(text, "hi");
        assert_eq!(name, "utf-16le");

        let (text, name) = decode_html(b"\xfe\xff\x00h\x00i", None);
        assert_eq!(text, "hi");
        assert_eq!(name, "utf-16be");

        let (text, name) = decode_html(b"\xff\xfeh\x00i\x00", Some("utf-16"));
        assert_eq!(text, "hi");
        assert_eq!(name, "utf-16");

        let (text, name) = decode_html(b"hi", Some("utf-8"));
        assert_eq!(text, "hi");
        assert_eq!(name, "utf-8");
    }

    #[test]
    fn test_internal_helpers() {
        assert!(strip_ascii_whitespace(None).is_none());

        assert!(extract_charset_from_content(b"charset   utf-8").is_none());
        assert_eq!(
            extract_charset_from_content(b"charset   =utf-8"),
            Some(b"utf-8".to_vec())
        );
        assert_eq!(
            extract_charset_from_content(b"charset=   utf-8"),
            Some(b"utf-8".to_vec())
        );
        assert!(extract_charset_from_content(b"charset=   ").is_none());
    }

    #[test]
    fn test_prescan_http_equiv_bogus_charset() {
        assert!(prescan_for_meta_charset(
            b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=bogus\">"
        )
        .is_none());
    }
}
