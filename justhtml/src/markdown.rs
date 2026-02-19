//! Markdown rendering for JustHTML DOM nodes.
//!
//! Converts HTML DOM trees to GitHub Flavored Markdown.
//! Extracted from Python's `node.py` per the destination architecture spec.

use crate::node::{NodeHandle, NodeKind};
use crate::serialize;

// ---------------------------------------------------------------------------
// Markdown escape helpers
// ---------------------------------------------------------------------------

/// Escape characters that have special meaning in Markdown.
/// Escapes: `\`, `` ` ``, `*`, `_`, `[`, `]`
pub fn markdown_escape_text(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(s.len() + s.len() / 4);
    for ch in s.chars() {
        if matches!(ch, '\\' | '`' | '*' | '_' | '[' | ']') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Create a Markdown code span with proper backtick fencing.
/// Uses a fence longer than any run of backticks inside the content.
/// Adds spaces if the content starts or ends with a backtick.
pub fn markdown_code_span(s: Option<&str>) -> String {
    let s = s.unwrap_or("");

    // Find the longest run of backticks in the content
    let mut longest = 0usize;
    let mut run = 0usize;
    for ch in s.chars() {
        if ch == '`' {
            run += 1;
            if run > longest {
                longest = run;
            }
        } else {
            run = 0;
        }
    }

    let fence: String = "`".repeat(longest + 1);

    // CommonMark requires a space if content starts/ends with backticks.
    let needs_space = s.starts_with('`') || s.ends_with('`');
    if needs_space {
        format!("{} {} {}", fence, s, fence)
    } else {
        format!("{}{}{}", fence, s, fence)
    }
}

// ---------------------------------------------------------------------------
// Block elements constant
// ---------------------------------------------------------------------------

/// Elements treated as block-level for markdown rendering.
fn is_markdown_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "main"
            | "nav"
            | "aside"
            | "blockquote"
            | "pre"
            | "ul"
            | "ol"
            | "li"
            | "hr"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "table"
    )
}

// ---------------------------------------------------------------------------
// MarkdownBuilder
// ---------------------------------------------------------------------------

/// A buffer-based builder for constructing Markdown output.
/// Tracks newline state and pending whitespace for proper formatting.
pub struct MarkdownBuilder {
    pub buf: Vec<String>,
    pub newline_count: usize,
    pub pending_space: bool,
}

impl MarkdownBuilder {
    pub fn new() -> Self {
        MarkdownBuilder {
            buf: Vec::new(),
            newline_count: 0,
            pending_space: false,
        }
    }

    /// Strip trailing spaces/tabs from the last segment in the buffer.
    fn rstrip_last_segment(&mut self) {
        if let Some(last) = self.buf.last_mut() {
            let trimmed = last.trim_end_matches(|c: char| c == ' ' || c == '\t');
            if trimmed.len() != last.len() {
                *last = trimmed.to_string();
            }
        }
    }

    /// Append newlines to the output.
    pub fn newline(&mut self, count: usize) {
        for _ in 0..count {
            self.pending_space = false;
            self.rstrip_last_segment();
            self.buf.push("\n".to_string());
            if self.newline_count < 2 {
                self.newline_count += 1;
            }
        }
    }

    /// Ensure at least `count` consecutive trailing newlines.
    pub fn ensure_newlines(&mut self, count: usize) {
        while self.newline_count < count {
            self.newline(1);
        }
    }

    /// Append raw markdown content (no whitespace normalization).
    pub fn raw(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        // If we've collapsed whitespace and the next output is raw (e.g. "**"),
        // we still need to emit a single separating space.
        if self.pending_space {
            let first = s.chars().next().unwrap();
            if !matches!(first, ' ' | '\t' | '\n' | '\r' | '\x0C')
                && !self.buf.is_empty()
                && self.newline_count == 0
            {
                self.buf.push(" ".to_string());
            }
            self.pending_space = false;
        }

        self.buf.push(s.to_string());

        if s.contains('\n') {
            // Count trailing newlines (cap at 2 for blank-line semantics).
            let mut trailing = 0usize;
            for ch in s.chars().rev() {
                if ch == '\n' {
                    trailing += 1;
                } else {
                    break;
                }
            }
            self.newline_count = trailing.min(2);
            if trailing > 0 {
                self.pending_space = false;
            }
        } else {
            self.newline_count = 0;
        }
    }

    /// Append text with optional whitespace normalization.
    pub fn text(&mut self, s: &str, preserve_whitespace: bool) {
        if s.is_empty() {
            return;
        }

        if preserve_whitespace {
            self.raw(s);
            return;
        }

        for ch in s.chars() {
            if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0C') {
                self.pending_space = true;
                continue;
            }

            if self.pending_space {
                if !self.buf.is_empty() && self.newline_count == 0 {
                    self.buf.push(" ".to_string());
                }
                self.pending_space = false;
            }

            self.buf.push(ch.to_string());
            self.newline_count = 0;
        }
    }

    /// Finalize and return the Markdown string.
    pub fn finish(&self) -> String {
        let out: String = self.buf.join("");
        out.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\n').to_string()
    }
}

// ---------------------------------------------------------------------------
// Text collection for markdown (needed for code blocks, inline code)
// ---------------------------------------------------------------------------

/// Collect all text from a node's descendants without stripping, joining with no separator.
/// Used for code blocks and inline code rendering.
fn collect_text_for_markdown(node: &NodeHandle) -> String {
    let data = node.borrow();
    match data.kind {
        NodeKind::Text => data.data.as_deref().unwrap_or("").to_string(),
        _ => {
            let mut result = String::new();
            for child in &data.children {
                result.push_str(&collect_text_for_markdown(child));
            }
            if let Some(ref tc) = data.template_content {
                result.push_str(&collect_text_for_markdown(tc));
            }
            result
        }
    }
}

// ---------------------------------------------------------------------------
// Main markdown walker
// ---------------------------------------------------------------------------

/// Convert a DOM node to Markdown.
/// Entry point for markdown rendering.
pub fn to_markdown(node: &NodeHandle) -> String {
    let mut builder = MarkdownBuilder::new();
    to_markdown_walk(node, &mut builder, false, 0);
    builder.finish()
}

/// Recursive walker that converts an HTML DOM tree to Markdown.
pub fn to_markdown_walk(
    node: &NodeHandle,
    builder: &mut MarkdownBuilder,
    preserve_whitespace: bool,
    list_depth: usize,
) {
    let data = node.borrow();
    let name = data.name.as_str();

    // Text node
    if data.kind == NodeKind::Text {
        if preserve_whitespace {
            builder.raw(data.data.as_deref().unwrap_or(""));
        } else {
            builder.text(
                &markdown_escape_text(data.data.as_deref().unwrap_or("")),
                false,
            );
        }
        return;
    }

    // br
    if name == "br" {
        builder.newline(1);
        return;
    }

    // Comments/doctype don't contribute
    if data.kind == NodeKind::Comment || data.kind == NodeKind::Doctype || name == "!doctype" {
        return;
    }

    // Document containers contribute via descendants
    if name.starts_with('#') {
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        for child in &children {
            to_markdown_walk(child, builder, preserve_whitespace, list_depth);
        }
        return;
    }

    let tag = name.to_lowercase();

    // Preserve <img> and <table> as HTML
    if tag == "img" {
        drop(data);
        builder.raw(&serialize::to_html(node, 0, 2, false));
        return;
    }

    if tag == "table" {
        let has_content = !builder.buf.is_empty();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        builder.raw(&serialize::to_html(node, 0, 2, false));
        builder.ensure_newlines(2);
        return;
    }

    // Headings
    if matches!(tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
        let has_content = !builder.buf.is_empty();
        let level: usize = tag[1..].parse().unwrap();
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        builder.raw(&"#".repeat(level));
        builder.raw(" ");
        for child in &children {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.ensure_newlines(2);
        return;
    }

    // Horizontal rule
    if tag == "hr" {
        let has_content = !builder.buf.is_empty();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        builder.raw("---");
        builder.ensure_newlines(2);
        return;
    }

    // Code blocks
    if tag == "pre" {
        let has_content = !builder.buf.is_empty();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        let code = collect_text_for_markdown(node);
        builder.raw("```");
        builder.newline(1);
        if !code.is_empty() {
            builder.raw(code.trim_end_matches('\n'));
            builder.newline(1);
        }
        builder.raw("```");
        builder.ensure_newlines(2);
        return;
    }

    // Inline code
    if tag == "code" && !preserve_whitespace {
        drop(data);
        let code = collect_text_for_markdown(node);
        builder.raw(&markdown_code_span(Some(&code)));
        return;
    }

    // Paragraph-like blocks
    if tag == "p" {
        let has_content = !builder.buf.is_empty();
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        for child in &children {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.ensure_newlines(2);
        return;
    }

    // Blockquotes
    if tag == "blockquote" {
        let has_content = !builder.buf.is_empty();
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        let mut inner = MarkdownBuilder::new();
        for child in &children {
            to_markdown_walk(child, &mut inner, false, list_depth);
        }
        let text = inner.finish();
        if !text.is_empty() {
            let lines: Vec<&str> = text.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    builder.newline(1);
                }
                builder.raw("> ");
                builder.raw(line);
            }
        }
        builder.ensure_newlines(2);
        return;
    }

    // Lists
    if tag == "ul" || tag == "ol" {
        let has_content = !builder.buf.is_empty();
        let ordered = tag == "ol";
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.ensure_newlines(if has_content { 2 } else { 0 });
        let mut idx = 1usize;
        for child in &children {
            let child_name = child.borrow().name.to_lowercase();
            if child_name != "li" {
                continue;
            }
            if idx > 1 {
                builder.newline(1);
            }
            let indent_str = "  ".repeat(list_depth);
            let marker = if ordered {
                format!("{}. ", idx)
            } else {
                "- ".to_string()
            };
            builder.raw(&indent_str);
            builder.raw(&marker);
            // Render list item children inline-ish
            let li_children: Vec<NodeHandle> = child.borrow().children.iter().map(|c| c.clone()).collect();
            for li_child in &li_children {
                to_markdown_walk(li_child, builder, false, list_depth + 1);
            }
            idx += 1;
        }
        builder.ensure_newlines(2);
        return;
    }

    // Emphasis/strong
    if tag == "em" || tag == "i" {
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.raw("*");
        for child in &children {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.raw("*");
        return;
    }

    if tag == "strong" || tag == "b" {
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.raw("**");
        for child in &children {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.raw("**");
        return;
    }

    // Links
    if tag == "a" {
        let href = data
            .attrs
            .get("href")
            .and_then(|v| v.as_deref())
            .unwrap_or("")
            .to_string();
        let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
        drop(data);
        builder.raw("[");
        for child in &children {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.raw("]");
        if !href.is_empty() {
            builder.raw("(");
            builder.raw(&href);
            builder.raw(")");
        }
        return;
    }

    // Containers / unknown tags: recurse into children
    let next_preserve =
        preserve_whitespace || matches!(tag.as_str(), "textarea" | "script" | "style");
    let children: Vec<NodeHandle> = data.children.iter().map(|c| c.clone()).collect();
    let template_content = data.template_content.clone();
    drop(data);

    for child in &children {
        to_markdown_walk(child, builder, next_preserve, list_depth);
    }

    if let Some(ref tc) = template_content {
        to_markdown_walk(tc, builder, next_preserve, list_depth);
    }

    // Add spacing after block containers to keep output readable.
    if is_markdown_block_element(&tag) {
        builder.ensure_newlines(2);
    }
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
    // Markdown escape and code span tests
    // -------------------------------------------------------------------

    #[test]
    fn test_markdown_escape_text_basic() {
        assert_eq!(markdown_escape_text("a*b"), "a\\*b");
        assert_eq!(markdown_escape_text(""), "");
        assert_eq!(markdown_escape_text("hello"), "hello");
        assert_eq!(
            markdown_escape_text("\\`*_[]"),
            "\\\\\\`\\*\\_\\[\\]"
        );
    }

    #[test]
    fn test_markdown_code_span_edge_cases() {
        assert_eq!(markdown_code_span(None), "``");
        assert_eq!(markdown_code_span(Some("`x")), "`` `x ``");
        assert_eq!(markdown_code_span(Some("x`")), "`` x` ``");
        assert_eq!(markdown_code_span(Some("a`b`")), "`` a`b` ``");
    }

    #[test]
    fn test_markdown_code_span_no_backticks() {
        assert_eq!(markdown_code_span(Some("hello")), "`hello`");
    }

    // -------------------------------------------------------------------
    // MarkdownBuilder tests
    // -------------------------------------------------------------------

    #[test]
    fn test_markdown_builder_text_preserve_whitespace_branch() {
        let mut b = MarkdownBuilder::new();
        b.text("x\n", true);
        assert_eq!(b.finish(), "x");
    }

    #[test]
    fn test_markdown_builder_text_leading_whitespace_does_not_add_space() {
        let mut b = MarkdownBuilder::new();
        b.text("   a", false);
        assert_eq!(b.finish(), "a");
    }

    #[test]
    fn test_markdown_builder_raw_inserts_pending_space() {
        let mut b = MarkdownBuilder::new();
        b.text("a ", false);
        b.raw("**");
        b.raw("b");
        assert_eq!(b.finish(), "a **b");
    }

    #[test]
    fn test_markdown_builder_raw_does_not_insert_space_before_newline() {
        let mut b = MarkdownBuilder::new();
        b.text("a ", false);
        b.raw("\n");
        assert_eq!(b.finish(), "a");
    }

    // -------------------------------------------------------------------
    // Full markdown rendering tests
    // -------------------------------------------------------------------

    #[test]
    fn test_to_markdown_headings_paragraphs_and_inline() {
        // Build: <h1>Title</h1><p>Hello <b>world</b> <em>ok</em> <a href='https://e.com'>link</a> a*b</p>
        let doc = new_document();
        let html_node = html_elem("html");
        append_child(&doc, &html_node);
        let body = html_elem("body");
        append_child(&html_node, &body);

        let h1 = html_elem("h1");
        append_child(&h1, &new_text("Title"));
        append_child(&body, &h1);

        let p = html_elem("p");
        append_child(&p, &new_text("Hello "));
        let b = html_elem("b");
        append_child(&b, &new_text("world"));
        append_child(&p, &b);
        append_child(&p, &new_text(" "));
        let em = html_elem("em");
        append_child(&em, &new_text("ok"));
        append_child(&p, &em);
        append_child(&p, &new_text(" "));
        let a = html_elem_with_attrs("a", vec![("href", "https://e.com")]);
        append_child(&a, &new_text("link"));
        append_child(&p, &a);
        append_child(&p, &new_text(" a*b"));
        append_child(&body, &p);

        let md = to_markdown(&doc);
        assert!(md.starts_with("# Title\n\n"));
        assert!(md.contains("Hello **world** *ok* [link](https://e.com) a\\*b"));
    }

    #[test]
    fn test_to_markdown_code_inline_and_block() {
        // Build: <pre>code`here\n</pre><p>inline <code>a`b</code></p>
        let doc = new_document();
        let html_node = html_elem("html");
        append_child(&doc, &html_node);
        let body = html_elem("body");
        append_child(&html_node, &body);

        let pre = html_elem("pre");
        append_child(&pre, &new_text("code`here\n"));
        append_child(&body, &pre);

        let p = html_elem("p");
        append_child(&p, &new_text("inline "));
        let code = html_elem("code");
        append_child(&code, &new_text("a`b"));
        append_child(&p, &code);
        append_child(&body, &p);

        let md = to_markdown(&doc);
        assert!(md.contains("```\ncode`here\n```"));
        assert!(md.contains("inline ``a`b``"));
    }

    #[test]
    fn test_to_markdown_blockquote_and_br() {
        // Build: <blockquote><p>Q<br>R</p></blockquote>
        let bq = html_elem("blockquote");
        let p = html_elem("p");
        append_child(&p, &new_text("Q"));
        let br = html_elem("br");
        append_child(&p, &br);
        append_child(&p, &new_text("R"));
        append_child(&bq, &p);

        assert_eq!(to_markdown(&bq), "> Q\n> R");
    }

    #[test]
    fn test_to_markdown_lists() {
        // Build: <ul><li>One</li><li>Two</li></ul><ol><li>A</li><li>B</li></ol>
        let wrapper = new_document_fragment();

        let ul = html_elem("ul");
        let li1 = html_elem("li");
        append_child(&li1, &new_text("One"));
        append_child(&ul, &li1);
        let li2 = html_elem("li");
        append_child(&li2, &new_text("Two"));
        append_child(&ul, &li2);
        append_child(&wrapper, &ul);

        let ol = html_elem("ol");
        let li_a = html_elem("li");
        append_child(&li_a, &new_text("A"));
        append_child(&ol, &li_a);
        let li_b = html_elem("li");
        append_child(&li_b, &new_text("B"));
        append_child(&ol, &li_b);
        append_child(&wrapper, &ol);

        let md = to_markdown(&wrapper);
        assert!(md.contains("- One\n- Two"));
        assert!(md.contains("1. A\n2. B"));
    }

    #[test]
    fn test_to_markdown_tables_and_images_are_html() {
        // <p>Hi<img src=x alt=y>there</p><table><tr><td>A</td></tr></table>
        let wrapper = new_document_fragment();
        let p = html_elem("p");
        append_child(&p, &new_text("Hi"));
        let img = html_elem_with_attrs("img", vec![("src", "x"), ("alt", "y")]);
        append_child(&p, &img);
        append_child(&p, &new_text("there"));
        append_child(&wrapper, &p);

        let table = html_elem("table");
        let tr = html_elem("tr");
        let td = html_elem("td");
        append_child(&td, &new_text("A"));
        append_child(&tr, &td);
        append_child(&table, &tr);
        append_child(&wrapper, &table);

        let md = to_markdown(&wrapper);
        assert!(md.contains("<img"));
        assert!(md.contains("<table"));
        assert!(md.contains("<td>A</td>"));
        assert!(md.contains("</table>"));
    }

    #[test]
    fn test_to_markdown_ignores_comment_and_doctype() {
        let root = html_elem("div");
        append_child(&root, &new_comment("nope"));
        let dt = new_doctype(DoctypeData {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
        });
        append_child(&root, &dt);
        append_child(&root, &new_text("ok"));
        assert_eq!(to_markdown(&root), "ok");
    }

    #[test]
    fn test_to_markdown_preserves_script_whitespace() {
        let root = html_elem("div");
        let script = html_elem("script");
        append_child(&script, &new_text("var x = 1;\nvar y = 2;\n"));
        append_child(&root, &script);
        assert_eq!(to_markdown(&root), "var x = 1;\nvar y = 2;");
    }

    #[test]
    fn test_to_markdown_textnode_method() {
        let t = new_text("a*b");
        assert_eq!(to_markdown(&t), "a\\*b");
    }

    #[test]
    fn test_to_markdown_empty_textnode() {
        let t = new_text("");
        assert_eq!(to_markdown(&t), "");
    }

    #[test]
    fn test_to_markdown_br_on_empty_buffer_and_multiple_newlines() {
        // Exercises newline logic when buffer is empty and when newline_count is already >= 2.
        let wrapper = new_document_fragment();
        append_child(&wrapper, &html_elem("br"));
        append_child(&wrapper, &html_elem("br"));
        append_child(&wrapper, &html_elem("br"));
        assert_eq!(to_markdown(&wrapper), "");
    }

    #[test]
    fn test_to_markdown_empty_blocks_and_hr() {
        // <hr><h2></h2><p></p><pre></pre><blockquote></blockquote>
        let wrapper = new_document_fragment();
        append_child(&wrapper, &html_elem("hr"));
        append_child(&wrapper, &html_elem("h2"));
        append_child(&wrapper, &html_elem("p"));
        append_child(&wrapper, &html_elem("pre"));
        append_child(&wrapper, &html_elem("blockquote"));
        let md = to_markdown(&wrapper);
        assert!(md.contains("---"));
        assert!(md.contains("##"));
        assert!(md.contains("```\n```"));
    }

    #[test]
    fn test_to_markdown_list_skips_non_li_children() {
        // <ul>\n<li>One</li>\n</ul>
        let ul = html_elem("ul");
        append_child(&ul, &new_text("\n"));
        let li = html_elem("li");
        append_child(&li, &new_text("One"));
        append_child(&ul, &li);
        append_child(&ul, &new_text("\n"));
        assert_eq!(to_markdown(&ul), "- One");
    }

    #[test]
    fn test_to_markdown_link_without_href() {
        let p = html_elem("p");
        let a = html_elem("a");
        append_child(&a, &new_text("text"));
        append_child(&p, &a);
        assert_eq!(to_markdown(&p), "[text]");
    }

    #[test]
    fn test_to_markdown_pre_rstrips_trailing_spaces_before_newline() {
        let pre = html_elem("pre");
        append_child(&pre, &new_text("X   \n"));
        assert_eq!(to_markdown(&pre), "```\nX\n```");
    }

    #[test]
    fn test_to_markdown_document_container_direct() {
        let doc = new_document();
        let p = html_elem("p");
        append_child(&doc, &p);
        assert_eq!(to_markdown(&doc), "");
    }

    #[test]
    fn test_to_markdown_raw_with_internal_newline_no_trailing_newline() {
        let root = html_elem("div");
        let style = html_elem("style");
        append_child(&style, &new_text("a {\n  b: c; }"));
        append_child(&root, &style);
        assert!(to_markdown(&root).contains("a {\n  b: c; }"));
    }

    #[test]
    fn test_to_markdown_unknown_container_walks_children() {
        let span = html_elem("span");
        append_child(&span, &new_text("Hi"));
        assert_eq!(to_markdown(&span), "Hi");
    }

    #[test]
    fn test_markdown_walk_document_children_loop() {
        let mut b = MarkdownBuilder::new();
        let doc = new_document();
        append_child(&doc, &new_text("Hi"));
        to_markdown_walk(&doc, &mut b, false, 0);
        assert_eq!(b.finish(), "Hi");
    }

    #[test]
    fn test_markdown_walk_document_without_children() {
        let doc = new_document();
        assert_eq!(to_markdown(&doc), "");
    }

    #[test]
    fn test_to_markdown_includes_template_content() {
        let template = new_template(HTML_NAMESPACE, HashMap::new());
        {
            let data = template.borrow();
            let tc = data.template_content.as_ref().unwrap();
            append_child(tc, &new_text("T"));
        }
        assert_eq!(to_markdown(&template), "T");
    }

    #[test]
    fn test_markdown_walk_unknown_tag_children_loop() {
        let mut b = MarkdownBuilder::new();
        let span = html_elem("span");
        append_child(&span, &new_text("Hi"));
        to_markdown_walk(&span, &mut b, false, 0);
        assert_eq!(b.finish(), "Hi");
    }

    #[test]
    fn test_to_markdown_horizontal_rule() {
        let hr = html_elem("hr");
        assert_eq!(to_markdown(&hr), "---");
    }
}
