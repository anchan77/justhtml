//! HTML serialization utilities for JustHTML DOM nodes.
//!
//! Corresponds to Python's `serialize.py`.
//! Provides functions for converting DOM trees to HTML strings and
//! to the html5lib test format for test comparison.

use crate::constants::{
    FOREIGN_ATTRIBUTE_ADJUSTMENTS, NAMESPACE_URL_TO_PREFIX, SPECIAL_ELEMENTS, VOID_ELEMENTS,
    HTML_NAMESPACE,
};
use crate::node::{NodeData, NodeHandle, NodeKind};

// ---------------------------------------------------------------------------
// Text and attribute escaping helpers
// ---------------------------------------------------------------------------

/// Escape text content: replace `&`, `<`, `>` with their HTML entity equivalents.
pub fn escape_text(text: Option<&str>) -> String {
    match text {
        None => String::new(),
        Some(t) if t.is_empty() => String::new(),
        Some(t) => t
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;"),
    }
}

/// Choose the best quote character for an attribute value.
/// Prefers double quotes, but uses single quotes if the value contains `"` but not `'`.
pub fn choose_attr_quote(value: Option<&str>) -> char {
    match value {
        None => '"',
        Some(v) => {
            if v.contains('"') && !v.contains('\'') {
                '\''
            } else {
                '"'
            }
        }
    }
}

/// Escape an attribute value for the given quote character.
pub fn escape_attr_value(value: Option<&str>, quote_char: char) -> String {
    match value {
        None => String::new(),
        Some(v) => {
            let escaped = v.replace('&', "&amp;");
            if quote_char == '"' {
                escaped.replace('"', "&quot;")
            } else {
                escaped.replace('\'', "&#39;")
            }
        }
    }
}

/// Check if an attribute value can be safely rendered without quotes.
pub fn can_unquote_attr_value(value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(v) if v.is_empty() => true,
        Some(v) => {
            for ch in v.chars() {
                if ch == '>' || ch == '"' || ch == '\'' || ch == '=' {
                    return false;
                }
                if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\x0C' || ch == '\r' {
                    return false;
                }
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Tag serialization
// ---------------------------------------------------------------------------

/// Serialize an opening HTML tag with its attributes.
pub fn serialize_start_tag(name: &str, attrs: &[(String, Option<String>)]) -> String {
    let mut parts = String::new();
    parts.push('<');
    parts.push_str(name);

    for (key, value) in attrs {
        match value {
            None => {
                parts.push(' ');
                parts.push_str(key);
            }
            Some(v) if v.is_empty() => {
                parts.push(' ');
                parts.push_str(key);
            }
            Some(v) => {
                if can_unquote_attr_value(Some(v)) {
                    let escaped = v.replace('&', "&amp;");
                    parts.push(' ');
                    parts.push_str(key);
                    parts.push('=');
                    parts.push_str(&escaped);
                } else {
                    let quote = choose_attr_quote(Some(v));
                    let escaped = escape_attr_value(Some(v), quote);
                    parts.push(' ');
                    parts.push_str(key);
                    parts.push('=');
                    parts.push(quote);
                    parts.push_str(&escaped);
                    parts.push(quote);
                }
            }
        }
    }

    parts.push('>');
    parts
}

/// Serialize a closing HTML tag.
pub fn serialize_end_tag(name: &str) -> String {
    format!("</{}>", name)
}

// ---------------------------------------------------------------------------
// HTML serialization
// ---------------------------------------------------------------------------

/// Preformatted elements where whitespace must be preserved.
fn is_preformatted(name: &str) -> bool {
    name == "pre" || name == "textarea"
}

/// Check if a node is a whitespace-only text node.
fn is_whitespace_text_node(node: &NodeData) -> bool {
    node.kind == NodeKind::Text && node.data.as_deref().unwrap_or("").trim().is_empty()
}

/// Determine whether children should be pretty-printed with indentation.
/// Returns false if there are comments, non-whitespace text, or non-special (inline) elements.
fn should_pretty_indent_children(children: &[NodeHandle]) -> bool {
    let mut has_comment = false;
    let mut has_non_whitespace_text = false;

    for child in children {
        let child_data = child.borrow();
        match child_data.kind {
            NodeKind::Comment => {
                has_comment = true;
                break;
            }
            NodeKind::Text => {
                if child_data.data.as_deref().unwrap_or("").trim().is_empty() {
                    continue;
                } else {
                    has_non_whitespace_text = true;
                    break;
                }
            }
            _ => {}
        }
    }

    if has_comment || has_non_whitespace_text {
        return false;
    }

    for child in children {
        let child_data = child.borrow();
        let name = &child_data.name;
        if name == "#text" || name == "#comment" {
            continue;
        }
        // Only indent safely when children are known "blockish" HTML elements.
        if !SPECIAL_ELEMENTS.contains(name.as_str()) {
            return false;
        }
    }
    true
}

/// Get the ordered list of attributes from a node as (key, value) pairs.
fn get_attrs_list(data: &NodeData) -> Vec<(String, Option<String>)> {
    data.attrs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Collect all text from a node's descendants (for all_text rendering).
/// This is a simple text collector that concatenates text nodes without separator.
fn collect_text_no_strip(node: &NodeHandle) -> String {
    let data = node.borrow();
    match data.kind {
        NodeKind::Text => data.data.as_deref().unwrap_or("").to_string(),
        _ => {
            let mut result = String::new();
            for child in &data.children {
                result.push_str(&collect_text_no_strip(child));
            }
            if let Some(ref tc) = data.template_content {
                result.push_str(&collect_text_no_strip(tc));
            }
            result
        }
    }
}

/// Convert a node to an HTML string.
///
/// * `node` - The DOM node to serialize.
/// * `indent` - Current indentation level.
/// * `indent_size` - Number of spaces per indent level.
/// * `pretty` - Whether to pretty-print with indentation.
pub fn to_html(node: &NodeHandle, indent: usize, indent_size: usize, pretty: bool) -> String {
    let data = node.borrow();
    if data.name == "#document" {
        let mut parts: Vec<String> = Vec::new();
        for child in &data.children {
            parts.push(node_to_html(child, indent, indent_size, pretty, false));
        }
        if pretty {
            parts.join("\n")
        } else {
            parts.join("")
        }
    } else {
        drop(data);
        node_to_html(node, indent, indent_size, pretty, false)
    }
}

/// Internal recursive helper to convert a node to HTML.
fn node_to_html(
    node: &NodeHandle,
    indent: usize,
    indent_size: usize,
    pretty: bool,
    in_pre: bool,
) -> String {
    let data = node.borrow();
    let name = &data.name;
    let prefix = if pretty && !in_pre {
        " ".repeat(indent * indent_size)
    } else {
        String::new()
    };
    let content_pre = in_pre || is_preformatted(name);
    let newline = if pretty && !content_pre { "\n" } else { "" };

    // Text node
    if data.kind == NodeKind::Text {
        let text = data.data.as_deref().unwrap_or("");
        if pretty && !in_pre {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return format!("{}{}", prefix, escape_text(Some(trimmed)));
            }
            return String::new();
        }
        if text.is_empty() {
            return String::new();
        }
        return escape_text(Some(text));
    }

    // Comment node
    if data.kind == NodeKind::Comment {
        return format!("{}<!--{}-->", prefix, data.data.as_deref().unwrap_or(""));
    }

    // Doctype
    if data.kind == NodeKind::Doctype || name == "!doctype" {
        return format!("{}<!DOCTYPE html>", prefix);
    }

    // Document fragment
    if name == "#document-fragment" {
        let mut parts: Vec<String> = Vec::new();
        for child in &data.children {
            let child_html = node_to_html(child, indent, indent_size, pretty, in_pre);
            if !child_html.is_empty() {
                parts.push(child_html);
            }
        }
        return if pretty {
            parts.join(newline)
        } else {
            parts.join("")
        };
    }

    // Element node
    let attrs = get_attrs_list(&data);
    let open_tag = serialize_start_tag(name, &attrs);

    // Void elements
    if VOID_ELEMENTS.contains(name.as_str()) {
        return format!("{}{}", prefix, open_tag);
    }

    let children = &data.children;

    // No children
    if children.is_empty() {
        return format!("{}{}{}", prefix, open_tag, serialize_end_tag(name));
    }

    // Check if all children are text-only (inline rendering)
    let all_text = children.iter().all(|c| c.borrow().kind == NodeKind::Text);

    if all_text && pretty && !content_pre {
        // Render text children inline
        let text = collect_text_no_strip(node);
        return format!(
            "{}{}{}{}",
            prefix,
            open_tag,
            escape_text(Some(&text)),
            serialize_end_tag(name)
        );
    }

    if pretty && content_pre {
        // Preformatted: render children without extra indentation
        let mut inner = String::new();
        for child in children {
            inner.push_str(&node_to_html(child, indent + 1, indent_size, pretty, true));
        }
        return format!("{}{}{}{}", prefix, open_tag, inner, serialize_end_tag(name));
    }

    if pretty && !content_pre && !should_pretty_indent_children(children) {
        // Inline/mixed content: render without indentation
        let mut inner = String::new();
        for child in children {
            inner.push_str(&node_to_html(child, 0, indent_size, false, content_pre));
        }
        return format!("{}{}{}{}", prefix, open_tag, inner, serialize_end_tag(name));
    }

    // Render with child indentation
    let mut parts: Vec<String> = vec![format!("{}{}", prefix, open_tag)];
    for child in children {
        if pretty && !content_pre && is_whitespace_text_node(&child.borrow()) {
            continue;
        }
        let child_html = node_to_html(child, indent + 1, indent_size, pretty, content_pre);
        if !child_html.is_empty() {
            parts.push(child_html);
        }
    }
    parts.push(format!("{}{}", prefix, serialize_end_tag(name)));
    if pretty {
        parts.join(newline)
    } else {
        parts.join("")
    }
}

// ---------------------------------------------------------------------------
// html5lib test format serialization
// ---------------------------------------------------------------------------

/// Convert a node to html5lib test format string.
///
/// This format is used by html5lib-tests for validating parser output.
/// Uses `| ` prefixes and specific indentation rules.
pub fn to_test_format(node: &NodeHandle, indent: usize) -> String {
    let data = node.borrow();
    if data.name == "#document" || data.name == "#document-fragment" {
        let parts: Vec<String> = data
            .children
            .iter()
            .map(|child| node_to_test_format(child, 0))
            .collect();
        return parts.join("\n");
    }
    drop(data);
    node_to_test_format(node, indent)
}

/// Internal helper to convert a node to test format.
fn node_to_test_format(node: &NodeHandle, indent: usize) -> String {
    let data = node.borrow();

    // Comment
    if data.kind == NodeKind::Comment {
        let comment = data.data.as_deref().unwrap_or("");
        return format!("| {}<!-- {} -->", " ".repeat(indent), comment);
    }

    // Doctype
    if data.kind == NodeKind::Doctype || data.name == "!doctype" {
        return doctype_to_test_format(&data);
    }

    // Text
    if data.kind == NodeKind::Text {
        let text = data.data.as_deref().unwrap_or("");
        return format!("| {}\"{}\"", " ".repeat(indent), text);
    }

    // Regular element
    let qualified = qualified_name(&data);
    let line = format!("| {}<{}>", " ".repeat(indent), qualified);
    let attribute_lines = attrs_to_test_format(&data, indent);

    // Template special handling (only HTML namespace templates have template_content)
    if data.kind == NodeKind::Template
        && is_html_namespace(&data.namespace)
        && data.template_content.is_some()
    {
        let mut sections: Vec<String> = vec![line];
        sections.extend(attribute_lines);
        let content_line = format!("| {}content", " ".repeat(indent + 2));
        sections.push(content_line);
        if let Some(ref tc) = data.template_content {
            let tc_data = tc.borrow();
            for child in &tc_data.children {
                sections.push(node_to_test_format(child, indent + 4));
            }
        }
        return sections.join("\n");
    }

    // Regular element with children
    let mut sections: Vec<String> = vec![line];
    sections.extend(attribute_lines);
    for child in &data.children {
        sections.push(node_to_test_format(child, indent + 2));
    }
    sections.join("\n")
}

/// Get the qualified name of a node (with namespace prefix if needed).
fn qualified_name(data: &NodeData) -> String {
    let ns = &data.namespace;
    if !ns.is_empty() && !is_html_namespace(ns) {
        // Map full URL to short prefix
        if let Some(&prefix) = NAMESPACE_URL_TO_PREFIX.get(ns.as_str()) {
            if prefix != "html" {
                return format!("{} {}", prefix, data.name);
            }
        } else {
            // Unknown namespace, use the namespace value directly
            return format!("{} {}", ns, data.name);
        }
    }
    data.name.clone()
}

/// Check if a namespace is the HTML namespace.
fn is_html_namespace(ns: &str) -> bool {
    ns.is_empty() || ns == "html" || ns == HTML_NAMESPACE
}

/// Format element attributes for test output.
fn attrs_to_test_format(data: &NodeData, indent: usize) -> Vec<String> {
    if data.attrs.is_empty() {
        return Vec::new();
    }

    let padding = " ".repeat(indent + 2);

    // Prepare display names for sorting
    let mut display_attrs: Vec<(String, String)> = Vec::new();
    let namespace = &data.namespace;

    for (attr_name, attr_value) in &data.attrs {
        let value = attr_value.as_deref().unwrap_or("");
        let mut display_name = attr_name.clone();

        if !namespace.is_empty() && !is_html_namespace(namespace) {
            let lower_name = attr_name.to_lowercase();
            if FOREIGN_ATTRIBUTE_ADJUSTMENTS.contains_key(lower_name.as_str()) {
                display_name = attr_name.replace(':', " ");
            }
        }

        display_attrs.push((display_name, value.to_string()));
    }

    // Sort by display name for canonical test output
    display_attrs.sort_by(|a, b| a.0.cmp(&b.0));

    display_attrs
        .iter()
        .map(|(name, value)| format!("| {}{}=\"{}\"", padding, name, value))
        .collect()
}

/// Format DOCTYPE node for test output.
fn doctype_to_test_format(data: &NodeData) -> String {
    let mut parts = vec!["| <!DOCTYPE".to_string()];

    if let Some(ref dt) = data.doctype_data {
        let name = dt.name.as_deref().unwrap_or("");
        if !name.is_empty() {
            parts.push(format!(" {}", name));
        } else {
            parts.push(" ".to_string());
        }

        if dt.public_id.is_some() || dt.system_id.is_some() {
            let pub_id = dt.public_id.as_deref().unwrap_or("");
            let sys_id = dt.system_id.as_deref().unwrap_or("");
            parts.push(format!(" \"{}\"", pub_id));
            parts.push(format!(" \"{}\"", sys_id));
        }
    } else {
        // No doctype_data, fallback
        parts.push(" ".to_string());
    }

    parts.push(">".to_string());
    parts.join("")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::HTML_NAMESPACE;
    use crate::node::{
        append_child, new_comment, new_doctype, new_document, new_document_fragment,
        new_element, new_template, new_text, DoctypeData,
    };
    use std::collections::HashMap;

    /// Helper to create an element with no attrs in the HTML namespace.
    fn html_elem(name: &str) -> NodeHandle {
        new_element(name, HTML_NAMESPACE, HashMap::new())
    }

    /// Helper to create an element with attrs in the HTML namespace.
    fn html_elem_with_attrs(name: &str, attrs: Vec<(&str, &str)>) -> NodeHandle {
        let mut map = HashMap::new();
        for (k, v) in attrs {
            map.insert(k.to_string(), Some(v.to_string()));
        }
        new_element(name, HTML_NAMESPACE, map)
    }

    // -------------------------------------------------------------------
    // Text escaping tests
    // -------------------------------------------------------------------

    #[test]
    fn test_text_escaping() {
        let frag = new_document_fragment();
        let div = html_elem("div");
        append_child(&frag, &div);
        let text = new_text("a<b&c");
        append_child(&div, &text);
        let output = to_html(&frag, 0, 2, false);
        assert_eq!(output, "<div>a&lt;b&amp;c</div>");
    }

    #[test]
    fn test_void_elements() {
        // Create a document with void elements
        let doc = new_document();
        let html = html_elem("html");
        append_child(&doc, &html);
        let body = html_elem("body");
        append_child(&html, &body);
        let br = html_elem("br");
        append_child(&body, &br);
        let hr = html_elem("hr");
        append_child(&body, &hr);
        let img = html_elem("img");
        append_child(&body, &img);

        let output = to_html(&doc, 0, 2, true);
        assert!(output.contains("<br>"));
        assert!(output.contains("<hr>"));
        assert!(output.contains("<img>"));
        assert!(!output.contains("</br>"));
    }

    #[test]
    fn test_comments() {
        let div = html_elem("div");
        let comment = new_comment(" hello world ");
        append_child(&div, &comment);
        let output = to_html(&div, 0, 2, true);
        assert!(output.contains("<!-- hello world -->"));
    }

    #[test]
    fn test_document_fragment() {
        let frag = new_document_fragment();
        let child = html_elem("div");
        append_child(&frag, &child);
        let output = to_html(&frag, 0, 2, true);
        assert!(output.contains("<div></div>"));
    }

    #[test]
    fn test_text_only_children() {
        let div = html_elem("div");
        let text = new_text("Text only");
        append_child(&div, &text);
        let output = to_html(&div, 0, 2, true);
        assert!(output.contains("<div>Text only</div>"));
    }

    #[test]
    fn test_mixed_children() {
        // <div>Text <span>Span</span></div>
        let div = html_elem("div");
        let text = new_text("Text ");
        append_child(&div, &text);
        let span = html_elem("span");
        let span_text = new_text("Span");
        append_child(&span, &span_text);
        append_child(&div, &span);

        let output = to_html(&div, 0, 2, true);
        assert_eq!(output, "<div>Text <span>Span</span></div>");
    }

    #[test]
    fn test_pretty_print_does_not_insert_spaces_in_inline_mixed_content() {
        // <code><span>BApplication</span>(<span><span>const </span><span>char* </span><span>signature</span></span>);</code>
        let code = html_elem("code");
        let span1 = html_elem("span");
        append_child(&span1, &new_text("BApplication"));
        append_child(&code, &span1);
        append_child(&code, &new_text("("));
        let span2 = html_elem("span");
        let span2a = html_elem("span");
        append_child(&span2a, &new_text("const "));
        append_child(&span2, &span2a);
        let span2b = html_elem("span");
        append_child(&span2b, &new_text("char* "));
        append_child(&span2, &span2b);
        let span2c = html_elem("span");
        append_child(&span2c, &new_text("signature"));
        append_child(&span2, &span2c);
        append_child(&code, &span2);
        append_child(&code, &new_text(");"));

        let pretty_html = to_html(&code, 0, 2, true);
        assert!(pretty_html.contains("</span>(<span"));
    }

    #[test]
    fn test_empty_attributes() {
        // <input disabled>
        let mut attrs = HashMap::new();
        attrs.insert("disabled".to_string(), None);
        let input = new_element("input", HTML_NAMESPACE, attrs);
        let output = to_html(&input, 0, 2, true);
        assert!(output.contains("<input disabled>"));
    }

    #[test]
    fn test_none_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("data-test".to_string(), None);
        let node = new_element("div", HTML_NAMESPACE, attrs);
        let output = to_html(&node, 0, 2, true);
        assert!(output.contains("<div data-test></div>"));
    }

    #[test]
    fn test_empty_string_attribute() {
        let mut attrs = HashMap::new();
        attrs.insert("data-val".to_string(), Some(String::new()));
        let node = new_element("div", HTML_NAMESPACE, attrs);
        let output = to_html(&node, 0, 2, true);
        assert!(output.contains("<div data-val></div>"));
    }

    #[test]
    fn test_serialize_start_tag_quotes() {
        // Prefer single quotes if the value contains a double quote but no single quote
        let tag = serialize_start_tag(
            "span",
            &[("title".to_string(), Some("foo\"bar".to_string()))],
        );
        assert_eq!(tag, "<span title='foo\"bar'>");

        // Otherwise use double quotes and escape embedded double quotes
        let tag = serialize_start_tag(
            "span",
            &[("title".to_string(), Some("foo'bar\"baz".to_string()))],
        );
        assert_eq!(tag, "<span title=\"foo'bar&quot;baz\">");

        // Unquoted when safe
        let tag = serialize_start_tag(
            "span",
            &[("title".to_string(), Some("foo".to_string()))],
        );
        assert_eq!(tag, "<span title=foo>");

        assert!(can_unquote_attr_value(Some("foo<bar")));
        assert!(!can_unquote_attr_value(Some("foo>bar")));
        assert!(!can_unquote_attr_value(Some("foo\"bar")));
        assert!(!can_unquote_attr_value(Some("foo bar")));
    }

    #[test]
    fn test_serialize_end_tag() {
        assert_eq!(serialize_end_tag("span"), "</span>");
    }

    #[test]
    fn test_serializer_private_helpers_none() {
        assert_eq!(escape_text(None), "");
        assert_eq!(choose_attr_quote(None), '"');
        assert_eq!(escape_attr_value(None, '"'), "");
        assert!(!can_unquote_attr_value(None));
    }

    #[test]
    fn test_mixed_content_whitespace() {
        // <div>   <p></p></div>
        let div = html_elem("div");
        append_child(&div, &new_text("   "));
        let p = html_elem("p");
        append_child(&div, &p);
        let output = to_html(&div, 0, 2, true);
        assert!(output.contains("<div>"));
        assert!(output.contains("<p></p>"));
    }

    #[test]
    fn test_pretty_indent_skips_whitespace_text_nodes() {
        let div = html_elem("div");
        append_child(&div, &new_text("\n  "));
        let p = html_elem("p");
        append_child(&div, &p);
        append_child(&div, &new_text("\n"));
        let output = to_html(&div, 0, 2, true);
        assert_eq!(output, "<div>\n  <p></p>\n</div>");
    }

    #[test]
    fn test_pretty_indent_children_does_not_indent_inline_elements() {
        let div = html_elem("div");
        let span = html_elem("span");
        append_child(&div, &span);
        let output = to_html(&div, 0, 2, true);
        assert_eq!(output, "<div><span></span></div>");
    }

    #[test]
    fn test_pretty_indent_children_does_not_indent_comments() {
        let div = html_elem("div");
        append_child(&div, &new_comment("x"));
        let p = html_elem("p");
        append_child(&div, &p);
        let output = to_html(&div, 0, 2, true);
        assert_eq!(output, "<div><!--x--><p></p></div>");
    }

    #[test]
    fn test_whitespace_in_fragment() {
        let frag = new_document_fragment();
        let text_node = new_text("   ");
        append_child(&frag, &text_node);
        let output = to_html(&frag, 0, 2, true);
        assert_eq!(output, "");
    }

    #[test]
    fn test_text_node_pretty_strips_and_renders() {
        let frag = new_document_fragment();
        append_child(&frag, &new_text("  hi  "));
        let output = to_html(&frag, 0, 2, true);
        assert_eq!(output, "hi");
    }

    #[test]
    fn test_empty_text_node_is_dropped_when_not_pretty() {
        let div = html_elem("div");
        append_child(&div, &new_text(""));
        let output = to_html(&div, 0, 2, false);
        assert_eq!(output, "<div></div>");
    }

    #[test]
    fn test_element_with_nested_children() {
        let div = html_elem("div");
        let span = html_elem("span");
        append_child(&span, &new_text("inner"));
        append_child(&div, &span);
        let output = to_html(&div, 0, 2, true);
        assert!(output.contains("<div>"));
        assert!(output.contains("<span>inner</span>"));
        assert!(output.contains("</div>"));
    }

    #[test]
    fn test_element_without_attributes() {
        let div = html_elem("div");
        append_child(&div, &new_text("hello"));
        let output = to_html(&div, 0, 2, true);
        assert_eq!(output, "<div>hello</div>");
    }

    #[test]
    fn test_to_test_format_single_element() {
        let node = html_elem("div");
        let output = to_test_format(&node, 0);
        assert_eq!(output, "| <div>");
    }

    #[test]
    fn test_to_test_format_template_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), Some("t1".to_string()));
        let template = new_template(HTML_NAMESPACE, attrs);
        let p = html_elem("p");
        {
            let data = template.borrow();
            let tc = data.template_content.as_ref().unwrap();
            append_child(tc, &p);
        }
        let output = to_test_format(&template, 0);
        assert!(output.contains("| <template>"));
        assert!(output.contains("|   id=\"t1\""));
        assert!(output.contains("|   content"));
        assert!(output.contains("|     <p>"));
    }

    #[test]
    fn test_basic_document() {
        // Build: <!DOCTYPE html><html><head><title>Test</title></head><body><p>Hello</p></body></html>
        let doc = new_document();
        let dt = new_doctype(DoctypeData {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
        });
        append_child(&doc, &dt);
        let html_node = html_elem("html");
        append_child(&doc, &html_node);
        let head = html_elem("head");
        append_child(&html_node, &head);
        let title = html_elem("title");
        append_child(&title, &new_text("Test"));
        append_child(&head, &title);
        let body = html_elem("body");
        append_child(&html_node, &body);
        let p = html_elem("p");
        append_child(&p, &new_text("Hello"));
        append_child(&body, &p);

        let output = to_html(&doc, 0, 2, true);
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("<title>Test</title>"));
        assert!(output.contains("<p>Hello</p>"));
    }

    #[test]
    fn test_attributes() {
        let node = html_elem_with_attrs(
            "div",
            vec![("id", "test"), ("class", "foo"), ("data-val", "x&y")],
        );
        let output = to_html(&node, 0, 2, true);
        // Check that id and class appear (could be unquoted or quoted)
        assert!(output.contains("id=test") || output.contains("id=\"test\""));
        assert!(output.contains("class=foo") || output.contains("class=\"foo\""));
        // Ampersand should be escaped
        assert!(
            output.contains("data-val=x&amp;y") || output.contains("data-val=\"x&amp;y\"")
        );
    }

    #[test]
    fn test_to_test_format_doctype() {
        let dt = new_doctype(DoctypeData {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
        });
        let output = to_test_format(&dt, 0);
        assert_eq!(output, "| <!DOCTYPE html>");
    }

    #[test]
    fn test_to_test_format_doctype_with_ids() {
        let dt = new_doctype(DoctypeData {
            name: Some("html".to_string()),
            public_id: Some("-//W3C//DTD HTML 4.01//EN".to_string()),
            system_id: Some("http://www.w3.org/TR/html4/strict.dtd".to_string()),
        });
        let output = to_test_format(&dt, 0);
        assert!(output.contains("| <!DOCTYPE html"));
        assert!(output.contains("\"-//W3C//DTD HTML 4.01//EN\""));
        assert!(output.contains("\"http://www.w3.org/TR/html4/strict.dtd\""));
    }

    #[test]
    fn test_to_test_format_text_node() {
        let text = new_text("Hello World");
        let output = to_test_format(&text, 0);
        assert_eq!(output, "| \"Hello World\"");
    }

    #[test]
    fn test_to_test_format_comment() {
        let comment = new_comment("test");
        let output = to_test_format(&comment, 0);
        assert_eq!(output, "| <!-- test -->");
    }

    #[test]
    fn test_to_test_format_document() {
        let doc = new_document();
        let div = html_elem("div");
        append_child(&div, &new_text("hello"));
        append_child(&doc, &div);
        let output = to_test_format(&doc, 0);
        assert!(output.contains("| <div>"));
        assert!(output.contains("|   \"hello\""));
    }

    #[test]
    fn test_to_test_format_svg_element() {
        use crate::constants::SVG_NAMESPACE;
        let node = new_element("rect", SVG_NAMESPACE, HashMap::new());
        let output = to_test_format(&node, 0);
        assert_eq!(output, "| <svg rect>");
    }
}
