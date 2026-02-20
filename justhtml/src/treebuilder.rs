//! HTML5 tree construction algorithm.
//!
//! Implements the WHATWG HTML5 tree builder per spec §13.2.6.
//! Corresponds to Python's `treebuilder.py`, `treebuilder_modes.py`, and `treebuilder_utils.py`.

use std::collections::HashMap;
use std::rc::Rc;

use crate::constants::*;
use crate::context::FragmentContext;
use crate::errors::generate_error_message;
use crate::node::*;
use crate::tokenizer::TokenSink;
use crate::tokens::*;

// ---------------------------------------------------------------------------
// Insertion modes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InsertionMode {
    Initial = 0,
    BeforeHtml = 1,
    BeforeHead = 2,
    InHead = 3,
    InHeadNoscript = 4,
    AfterHead = 5,
    Text = 6,
    InBody = 7,
    AfterBody = 8,
    AfterAfterBody = 9,
    InTable = 10,
    InTableText = 11,
    InCaption = 12,
    InColumnGroup = 13,
    InTableBody = 14,
    InRow = 15,
    InCell = 16,
    InFrameset = 17,
    AfterFrameset = 18,
    AfterAfterFrameset = 19,
    InSelect = 20,
    InTemplate = 21,
}

// ---------------------------------------------------------------------------
// Active formatting list
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FormattingEntry {
    pub name: String,
    pub attrs: HashMap<String, Option<String>>,
    pub node: NodeHandle,
    pub signature: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum ActiveFormattingEntry {
    Marker,
    Element(FormattingEntry),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_all_whitespace(text: &str) -> bool {
    text.chars().all(|c| matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' '))
}

fn contains_prefix(prefixes: &[&str], needle: &str) -> bool {
    prefixes.iter().any(|p| needle.starts_with(p))
}

/// Determine parse error and quirks mode for a DOCTYPE token.
pub fn doctype_error_and_quirks(doctype: &Doctype, iframe_srcdoc: bool) -> (bool, &'static str) {
    let name = doctype.name.as_ref().map(|n| n.to_lowercase());
    let public_id = doctype.public_id.as_deref();
    let system_id = doctype.system_id.as_deref();

    let acceptable: &[(&str, Option<&str>, Option<&str>)] = &[
        ("html", None, None),
        ("html", None, Some("about:legacy-compat")),
        ("html", Some("-//W3C//DTD HTML 4.0//EN"), None),
        ("html", Some("-//W3C//DTD HTML 4.0//EN"), Some("http://www.w3.org/TR/REC-html40/strict.dtd")),
        ("html", Some("-//W3C//DTD HTML 4.01//EN"), None),
        ("html", Some("-//W3C//DTD HTML 4.01//EN"), Some("http://www.w3.org/TR/html4/strict.dtd")),
        ("html", Some("-//W3C//DTD XHTML 1.0 Strict//EN"), Some("http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd")),
        ("html", Some("-//W3C//DTD XHTML 1.1//EN"), Some("http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd")),
    ];

    let name_ref = name.as_deref();
    let parse_error = !acceptable.iter().any(|(n, p, s)| {
        name_ref == Some(*n) && public_id == *p && system_id == *s
    });

    let public_lower = public_id.map(|s| s.to_lowercase());
    let system_lower = system_id.map(|s| s.to_lowercase());

    let quirks_mode = if doctype.force_quirks {
        "quirks"
    } else if iframe_srcdoc {
        "no-quirks"
    } else if name_ref != Some("html") {
        "quirks"
    } else if let Some(ref pl) = public_lower {
        if QUIRKY_PUBLIC_MATCHES.iter().any(|m| pl.as_str() == *m) {
            "quirks"
        } else if let Some(ref sl) = system_lower {
            if QUIRKY_SYSTEM_MATCHES.iter().any(|m| sl.as_str() == *m) {
                "quirks"
            } else if contains_prefix(QUIRKY_PUBLIC_PREFIXES, pl) {
                "quirks"
            } else if contains_prefix(LIMITED_QUIRKY_PUBLIC_PREFIXES, pl) {
                "limited-quirks"
            } else if contains_prefix(HTML4_PUBLIC_PREFIXES, pl) {
                "limited-quirks"
            } else {
                "no-quirks"
            }
        } else if contains_prefix(QUIRKY_PUBLIC_PREFIXES, pl) {
            "quirks"
        } else if contains_prefix(LIMITED_QUIRKY_PUBLIC_PREFIXES, pl) {
            "limited-quirks"
        } else if contains_prefix(HTML4_PUBLIC_PREFIXES, pl) {
            "quirks"
        } else {
            "no-quirks"
        }
    } else if let Some(ref sl) = system_lower {
        if QUIRKY_SYSTEM_MATCHES.iter().any(|m| sl.as_str() == *m) {
            "quirks"
        } else {
            "no-quirks"
        }
    } else {
        "no-quirks"
    };

    (parse_error, quirks_mode)
}

// ---------------------------------------------------------------------------
// Reprocess instruction returned by mode handlers
// ---------------------------------------------------------------------------

/// When a mode handler needs to reprocess a token in a different mode, it
/// returns this value.  `force_html` causes the outer loop to skip the
/// foreign-content check.
struct Reprocess {
    mode: InsertionMode,
    token: Token,
    force_html: bool,
}

/// Return type of every individual mode handler.
type ModeResult = Option<Reprocess>;

fn reprocess(mode: InsertionMode, token: Token) -> ModeResult {
    Some(Reprocess { mode, token, force_html: false })
}

#[allow(dead_code)]
fn reprocess_html(mode: InsertionMode, token: Token) -> ModeResult {
    Some(Reprocess { mode, token, force_html: true })
}

// ---------------------------------------------------------------------------
// TreeBuilder
// ---------------------------------------------------------------------------

pub struct TreeBuilder {
    // Tree output
    pub document: NodeHandle,
    pub errors: Vec<ParseError>,

    // Configuration
    pub fragment_context: Option<FragmentContext>,
    pub fragment_context_element: Option<NodeHandle>,
    pub iframe_srcdoc: bool,
    pub collect_errors: bool,

    // Parser state
    pub mode: InsertionMode,
    pub original_mode: Option<InsertionMode>,
    pub table_text_original_mode: Option<InsertionMode>,
    pub open_elements: Vec<NodeHandle>,
    pub head_element: Option<NodeHandle>,
    pub form_element: Option<NodeHandle>,
    pub frameset_ok: bool,
    pub quirks_mode: String,
    pub ignore_lf: bool,
    pub active_formatting: Vec<ActiveFormattingEntry>,
    pub insert_from_table: bool,
    pub pending_table_text: Vec<String>,
    pub template_modes: Vec<InsertionMode>,
    pub tokenizer_state_override: Option<TokenSinkResult>,

    // Tokenizer position info (set by tokenizer before emitting)
    pub last_token_line: Option<usize>,
    pub last_token_column: Option<usize>,
    pub buffer: Option<String>,
}

impl TreeBuilder {
    pub fn new(
        fragment_context: Option<FragmentContext>,
        iframe_srcdoc: bool,
        collect_errors: bool,
    ) -> Self {
        let document = if fragment_context.is_some() {
            new_document_fragment()
        } else {
            new_document()
        };

        let mut tb = TreeBuilder {
            document,
            errors: Vec::new(),
            fragment_context,
            fragment_context_element: None,
            iframe_srcdoc,
            collect_errors,
            mode: InsertionMode::Initial,
            original_mode: None,
            table_text_original_mode: None,
            open_elements: Vec::new(),
            head_element: None,
            form_element: None,
            frameset_ok: true,
            quirks_mode: "no-quirks".to_string(),
            ignore_lf: false,
            active_formatting: Vec::new(),
            insert_from_table: false,
            pending_table_text: Vec::new(),
            template_modes: Vec::new(),
            tokenizer_state_override: None,
            last_token_line: None,
            last_token_column: None,
            buffer: None,
        };

        if tb.fragment_context.is_some() {
            tb.setup_fragment_parsing();
        }

        tb
    }

    fn setup_fragment_parsing(&mut self) {
        let ctx = self.fragment_context.as_ref().unwrap();
        let namespace = ctx.namespace.clone();
        let context_name = ctx.tag_name.clone();
        let name = context_name.to_lowercase();

        // Create root html element
        let root = new_element("html", HTML_NAMESPACE, HashMap::new());
        append_child(&self.document, &root);
        self.open_elements.push(Rc::clone(&root));

        // Create fake context element for foreign content
        if let Some(ref ns) = namespace {
            if ns != "html" {
                let mut adjusted_name = context_name.clone();
                if ns == "svg" {
                    adjusted_name = self.adjust_svg_tag_name(&context_name);
                }
                let ns_url = match ns.as_str() {
                    "svg" => SVG_NAMESPACE,
                    "math" => MATHML_NAMESPACE,
                    _ => HTML_NAMESPACE,
                };
                let context_element = new_element(&adjusted_name, ns_url, HashMap::new());
                append_child(&root, &context_element);
                self.open_elements.push(Rc::clone(&context_element));
                self.fragment_context_element = Some(context_element);
            }
        }

        // Set initial mode based on context element
        if name == "html" {
            self.mode = InsertionMode::BeforeHead;
        } else if namespace.as_deref().unwrap_or("html") == "html" || namespace.is_none() {
            self.mode = match name.as_str() {
                "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
                "tr" => InsertionMode::InRow,
                "td" | "th" => InsertionMode::InCell,
                "caption" => InsertionMode::InCaption,
                "colgroup" => InsertionMode::InColumnGroup,
                "table" => InsertionMode::InTable,
                _ => InsertionMode::InBody,
            };
        } else {
            self.mode = InsertionMode::InBody;
        }

        self.frameset_ok = false;
    }
}

/// Helper: create a new document-fragment node.
fn new_document_fragment() -> NodeHandle {
    use std::cell::RefCell;
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Document,
        name: "#document-fragment".to_string(),
        namespace: HTML_NAMESPACE.to_string(),
        attrs: HashMap::new(),
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }))
}

// ===========================================================================
// Utility methods
// ===========================================================================

impl TreeBuilder {
    // -----------------------------------------------------------------------
    // Parse error reporting
    // -----------------------------------------------------------------------

    pub fn parse_error(&mut self, code: &str, tag_name: Option<&str>) {
        self.parse_error_with_token(code, tag_name, None);
    }

    pub fn parse_error_with_token(&mut self, code: &str, tag_name: Option<&str>, token: Option<&Token>) {
        if !self.collect_errors {
            return;
        }
        let line = self.last_token_line;
        let mut column = self.last_token_column;
        let mut end_column: Option<usize> = None;

        // Calculate start and end columns based on token type for precise highlighting
        // Note: column from tokenizer points AFTER the last character (0-indexed)
        if let Some(Token::Tag(tag)) = token {
            let mut tag_len = tag.name.len() + 2; // < + name + >
            if tag.kind == TagKind::End {
                tag_len += 1; // </name>
            }
            // Add attribute lengths
            for (attr_name, attr_value) in &tag.attrs {
                tag_len += 1 + attr_name.len(); // space + name
                if let Some(ref val) = attr_value {
                    tag_len += 1 + 2 + val.len(); // = + "value"
                }
            }
            if tag.self_closing {
                tag_len += 1; // /
            }
            // column points after >, so start is column - tag_len + 1 (for 1-indexed)
            if let Some(col) = column {
                let start_column = col.saturating_sub(tag_len) + 1;
                column = Some(start_column);
                end_column = Some(start_column + tag_len);
            }
        }

        let message = generate_error_message(code, tag_name);
        let source_html = self.buffer.clone();
        self.errors.push(ParseError::new(
            code,
            line,
            column,
            Some(&message),
            source_html,
            end_column,
        ));
    }

    // -----------------------------------------------------------------------
    // Scope checking
    // -----------------------------------------------------------------------

    fn has_element_in_scope(&self, target: &str, terminators: &phf::Set<&str>, check_integration_points: bool) -> bool {
        for node in self.open_elements.iter().rev() {
            let data = node.borrow();
            if data.name == target {
                return true;
            }
            let ns = &data.namespace;
            if ns == HTML_NAMESPACE || ns.is_empty() {
                if terminators.contains(data.name.as_str()) {
                    return false;
                }
            } else if check_integration_points {
                let is_hip = self.is_html_integration_point_data(&data);
                let is_mip = self.is_mathml_text_integration_point_data(&data);
                if is_hip || is_mip {
                    return false;
                }
            }
        }
        false
    }

    fn in_scope(&self, name: &str) -> bool {
        self.has_element_in_scope(name, &DEFAULT_SCOPE_TERMINATORS, true)
    }

    fn has_in_button_scope(&self, target: &str) -> bool {
        self.has_element_in_scope(target, &BUTTON_SCOPE_TERMINATORS, true)
    }

    fn has_in_table_scope(&self, name: &str) -> bool {
        self.has_element_in_scope(name, &TABLE_SCOPE_TERMINATORS, false)
    }

    fn has_in_list_item_scope(&self, name: &str) -> bool {
        self.has_element_in_scope(name, &LIST_ITEM_SCOPE_TERMINATORS, true)
    }

    fn has_in_definition_scope(&self, name: &str) -> bool {
        self.has_element_in_scope(name, &DEFINITION_SCOPE_TERMINATORS, true)
    }

    fn has_any_in_scope(&self, names: &[&str]) -> bool {
        for node in self.open_elements.iter().rev() {
            let data = node.borrow();
            if names.contains(&data.name.as_str()) {
                return true;
            }
            let ns = &data.namespace;
            if (ns == HTML_NAMESPACE || ns.is_empty()) && DEFAULT_SCOPE_TERMINATORS.contains(data.name.as_str()) {
                return false;
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Stack operations
    // -----------------------------------------------------------------------

    fn pop_current(&mut self) -> Option<NodeHandle> {
        self.open_elements.pop()
    }

    fn pop_until_inclusive(&mut self, name: &str) {
        while let Some(node) = self.open_elements.pop() {
            if node.borrow().name == name {
                break;
            }
        }
    }

    fn pop_until_any_inclusive(&mut self, names: &[&str]) {
        while let Some(node) = self.open_elements.pop() {
            if names.contains(&node.borrow().name.as_str()) {
                return;
            }
        }
    }

    fn clear_stack_until(&mut self, names: &[&str]) {
        while let Some(node) = self.open_elements.last() {
            let data = node.borrow();
            let ns = &data.namespace;
            if names.contains(&data.name.as_str()) && (ns == HTML_NAMESPACE || ns.is_empty()) {
                break;
            }
            drop(data);
            self.open_elements.pop();
        }
    }

    fn generate_implied_end_tags(&mut self, exclude: Option<&str>) {
        while let Some(node) = self.open_elements.last() {
            let name = node.borrow().name.clone();
            if IMPLIED_END_TAGS.contains(name.as_str()) && exclude != Some(name.as_str()) {
                self.open_elements.pop();
            } else {
                break;
            }
        }
    }

    fn find_last_on_stack(&self, name: &str) -> Option<NodeHandle> {
        for node in self.open_elements.iter().rev() {
            if node.borrow().name == name {
                return Some(Rc::clone(node));
            }
        }
        None
    }

    fn remove_from_open_elements(&mut self, target: &NodeHandle) -> bool {
        for i in 0..self.open_elements.len() {
            if Rc::ptr_eq(&self.open_elements[i], target) {
                self.open_elements.remove(i);
                return true;
            }
        }
        false
    }

    fn stack_index(&self, target: &NodeHandle) -> Option<usize> {
        self.open_elements.iter().position(|n| Rc::ptr_eq(n, target))
    }

    fn is_special_element(&self, node: &NodeHandle) -> bool {
        let data = node.borrow();
        if data.namespace != HTML_NAMESPACE && !data.namespace.is_empty() {
            return false;
        }
        SPECIAL_ELEMENTS.contains(data.name.as_str())
    }

    // -----------------------------------------------------------------------
    // Close helpers
    // -----------------------------------------------------------------------

    fn close_p_element(&mut self) -> bool {
        if self.has_in_button_scope("p") {
            self.generate_implied_end_tags(Some("p"));
            if let Some(top) = self.open_elements.last() {
                if top.borrow().name != "p" {
                    self.parse_error("end-tag-too-early", Some("p"));
                }
            }
            self.pop_until_inclusive("p");
            true
        } else {
            false
        }
    }

    fn close_element_by_name(&mut self, name: &str) {
        let mut index = self.open_elements.len();
        while index > 0 {
            index -= 1;
            if self.open_elements[index].borrow().name == name {
                self.open_elements.truncate(index);
                return;
            }
        }
    }

    fn any_other_end_tag(&mut self, name: &str) {
        let mut index = self.open_elements.len();
        while index > 0 {
            index -= 1;
            let node_name = self.open_elements[index].borrow().name.clone();
            if node_name == name {
                if index != self.open_elements.len() - 1 {
                    self.parse_error("end-tag-too-early", None);
                }
                self.open_elements.truncate(index);
                return;
            }
            if self.is_special_element(&self.open_elements[index]) {
                self.parse_error("unexpected-end-tag", Some(name));
                return;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Current node helpers
    // -----------------------------------------------------------------------

    fn current_node_or_html(&self) -> NodeHandle {
        if let Some(node) = self.open_elements.last() {
            return Rc::clone(node);
        }
        // Fall back to html element in document
        let doc = self.document.borrow();
        for child in &doc.children {
            if child.borrow().name == "html" {
                return Rc::clone(child);
            }
        }
        Rc::clone(&self.document)
    }

    // -----------------------------------------------------------------------
    // DOM manipulation
    // -----------------------------------------------------------------------

    fn create_root(&mut self, attrs: HashMap<String, Option<String>>) -> NodeHandle {
        let node = new_element("html", HTML_NAMESPACE, attrs);
        append_child(&self.document, &node);
        self.open_elements.push(Rc::clone(&node));
        node
    }

    fn insert_element(&mut self, tag: &Tag, push: bool, namespace: &str) -> NodeHandle {
        let node = if tag.name == "template" && namespace == HTML_NAMESPACE {
            new_template(namespace, tag.attrs.clone())
        } else {
            new_element(&tag.name, namespace, tag.attrs.clone())
        };

        if !self.insert_from_table {
            let target = self.current_node_or_html();
            let target_data = target.borrow();
            if target_data.kind == NodeKind::Template {
                if let Some(ref content) = target_data.template_content {
                    let content = Rc::clone(content);
                    drop(target_data);
                    append_child(&content, &node);
                } else {
                    drop(target_data);
                    append_child(&target, &node);
                }
            } else {
                drop(target_data);
                append_child(&target, &node);
            }
            if push {
                self.open_elements.push(Rc::clone(&node));
            }
            return node;
        }

        let target = self.current_node_or_html();
        let foster = self.should_foster_parenting_for_tag(&target, Some(&tag.name));
        let (parent, position) = self.appropriate_insertion_location(None, foster);
        self.insert_node_at(&parent, position, &node);
        if push {
            self.open_elements.push(Rc::clone(&node));
        }
        node
    }

    fn insert_element_html(&mut self, tag: &Tag, push: bool) -> NodeHandle {
        self.insert_element(tag, push, HTML_NAMESPACE)
    }

    fn insert_phantom(&mut self, name: &str) -> NodeHandle {
        let tag = Tag::new_start(name);
        self.insert_element_html(&tag, true)
    }

    fn insert_body_if_missing(&mut self) {
        let html_node = self.find_last_on_stack("html");
        let node = new_element("body", HTML_NAMESPACE, HashMap::new());
        if let Some(ref html) = html_node {
            append_child(html, &node);
        }
        self.open_elements.push(node);
    }

    fn append_comment_to_document(&mut self, text: &str) {
        let node = new_comment(text);
        append_child(&self.document, &node);
    }

    fn append_comment(&mut self, text: &str, parent_override: Option<&NodeHandle>) {
        let parent = if let Some(p) = parent_override {
            Rc::clone(p)
        } else {
            self.current_node_or_html()
        };
        // If parent is a template, insert into content fragment
        let actual_parent = {
            let data = parent.borrow();
            if data.kind == NodeKind::Template {
                data.template_content.as_ref().map(Rc::clone)
            } else {
                None
            }
        };
        let target = actual_parent.unwrap_or(parent);
        let node = new_comment(text);
        append_child(&target, &node);
    }

    fn append_text(&mut self, text: &str) {
        let text = if self.ignore_lf {
            self.ignore_lf = false;
            if text.starts_with('\n') {
                let rest = &text[1..];
                if rest.is_empty() {
                    return;
                }
                rest.to_string()
            } else {
                text.to_string()
            }
        } else {
            text.to_string()
        };

        if self.open_elements.is_empty() {
            return;
        }

        // Fast path: not table foster parenting and not template
        let target = self.open_elements.last().unwrap();
        let target_name = target.borrow().name.clone();
        let target_kind = target.borrow().kind.clone();
        if !TABLE_FOSTER_TARGETS.contains(&target_name.as_str()) && target_kind != NodeKind::Template {
            let target = Rc::clone(target);
            let target_data = target.borrow();
            if let Some(last) = target_data.children.last() {
                let mut last_data = last.borrow_mut();
                if last_data.kind == NodeKind::Text {
                    if let Some(ref mut d) = last_data.data {
                        d.push_str(&text);
                    } else {
                        last_data.data = Some(text);
                    }
                    return;
                }
            }
            drop(target_data);
            let text_node = new_text(&text);
            append_child(&target, &text_node);
            return;
        }

        let target = self.current_node_or_html();
        let foster = self.should_foster_parenting_for_text(&target);

        if foster {
            self.reconstruct_active_formatting_elements();
        }

        let (parent, position) = self.appropriate_insertion_location(None, foster);

        // Coalesce with adjacent text node
        let parent_data = parent.borrow();
        if position > 0 {
            if let Some(prev) = parent_data.children.get(position - 1) {
                let mut prev_data = prev.borrow_mut();
                if prev_data.kind == NodeKind::Text {
                    if let Some(ref mut d) = prev_data.data {
                        d.push_str(&text);
                    } else {
                        prev_data.data = Some(text);
                    }
                    return;
                }
            }
        }
        drop(parent_data);

        let text_node = new_text(&text);
        let parent_data = parent.borrow();
        let reference = if position < parent_data.children.len() {
            Some(Rc::clone(&parent_data.children[position]))
        } else {
            None
        };
        drop(parent_data);
        if let Some(ref r) = reference {
            insert_before(&parent, &text_node, r);
        } else {
            append_child(&parent, &text_node);
        }
    }

    fn add_missing_attributes(&self, node: &NodeHandle, attrs: &HashMap<String, Option<String>>) {
        if attrs.is_empty() {
            return;
        }
        let mut data = node.borrow_mut();
        for (name, value) in attrs {
            if !data.attrs.contains_key(name) {
                data.attrs.insert(name.clone(), value.clone());
            }
        }
    }

    fn insert_node_at(&self, parent: &NodeHandle, index: usize, node: &NodeHandle) {
        let parent_data = parent.borrow();
        let reference = if index < parent_data.children.len() {
            Some(Rc::clone(&parent_data.children[index]))
        } else {
            None
        };
        drop(parent_data);
        if let Some(ref r) = reference {
            insert_before(parent, node, r);
        } else {
            append_child(parent, node);
        }
    }

    // -----------------------------------------------------------------------
    // Active formatting elements
    // -----------------------------------------------------------------------

    fn push_formatting_marker(&mut self) {
        self.active_formatting.push(ActiveFormattingEntry::Marker);
    }

    fn clear_active_formatting_up_to_marker(&mut self) {
        while let Some(entry) = self.active_formatting.pop() {
            if matches!(entry, ActiveFormattingEntry::Marker) {
                break;
            }
        }
    }

    fn attrs_signature(attrs: &HashMap<String, Option<String>>) -> Vec<(String, String)> {
        let mut items: Vec<(String, String)> = attrs
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().unwrap_or_default()))
            .collect();
        items.sort();
        items
    }

    fn find_active_formatting_index(&self, name: &str) -> Option<usize> {
        for i in (0..self.active_formatting.len()).rev() {
            match &self.active_formatting[i] {
                ActiveFormattingEntry::Marker => break,
                ActiveFormattingEntry::Element(e) if e.name == name => return Some(i),
                _ => {}
            }
        }
        None
    }

    fn find_active_formatting_index_by_node(&self, target: &NodeHandle) -> Option<usize> {
        for i in (0..self.active_formatting.len()).rev() {
            if let ActiveFormattingEntry::Element(e) = &self.active_formatting[i] {
                if Rc::ptr_eq(&e.node, target) {
                    return Some(i);
                }
            }
        }
        None
    }

    fn has_active_formatting_entry(&self, name: &str) -> bool {
        for i in (0..self.active_formatting.len()).rev() {
            match &self.active_formatting[i] {
                ActiveFormattingEntry::Marker => break,
                ActiveFormattingEntry::Element(e) if e.name == name => return true,
                _ => {}
            }
        }
        false
    }

    fn remove_last_active_formatting_by_name(&mut self, name: &str) {
        for i in (0..self.active_formatting.len()).rev() {
            match &self.active_formatting[i] {
                ActiveFormattingEntry::Marker => break,
                ActiveFormattingEntry::Element(e) if e.name == name => {
                    self.active_formatting.remove(i);
                    return;
                }
                _ => {}
            }
        }
    }

    fn remove_last_open_element_by_name(&mut self, name: &str) {
        for i in (0..self.open_elements.len()).rev() {
            if self.open_elements[i].borrow().name == name {
                self.open_elements.remove(i);
                return;
            }
        }
    }

    fn find_active_formatting_duplicate(&self, name: &str, attrs: &HashMap<String, Option<String>>) -> Option<usize> {
        let signature = Self::attrs_signature(attrs);
        let mut matches = Vec::new();
        for (i, entry) in self.active_formatting.iter().enumerate() {
            match entry {
                ActiveFormattingEntry::Marker => matches.clear(),
                ActiveFormattingEntry::Element(e) if e.name == name && e.signature == signature => {
                    matches.push(i);
                }
                _ => {}
            }
        }
        if matches.len() >= 3 {
            Some(matches[0])
        } else {
            None
        }
    }

    fn append_active_formatting_entry(&mut self, name: &str, attrs: &HashMap<String, Option<String>>, node: &NodeHandle) {
        let entry_attrs = attrs.clone();
        let signature = Self::attrs_signature(&entry_attrs);
        self.active_formatting.push(ActiveFormattingEntry::Element(FormattingEntry {
            name: name.to_string(),
            attrs: entry_attrs,
            node: Rc::clone(node),
            signature,
        }));
    }

    fn remove_formatting_entry(&mut self, index: usize) {
        self.active_formatting.remove(index);
    }

    fn reconstruct_active_formatting_elements(&mut self) {
        if self.active_formatting.is_empty() {
            return;
        }
        // Check last entry
        match &self.active_formatting[self.active_formatting.len() - 1] {
            ActiveFormattingEntry::Marker => return,
            ActiveFormattingEntry::Element(e) => {
                if self.open_elements.iter().any(|n| Rc::ptr_eq(n, &e.node)) {
                    return;
                }
            }
        }

        let mut index = self.active_formatting.len() - 1;
        loop {
            if index == 0 {
                break;
            }
            index -= 1;
            let should_stop = match &self.active_formatting[index] {
                ActiveFormattingEntry::Marker => true,
                ActiveFormattingEntry::Element(e) => {
                    self.open_elements.iter().any(|n| Rc::ptr_eq(n, &e.node))
                }
            };
            if should_stop {
                index += 1;
                break;
            }
        }

        while index < self.active_formatting.len() {
            let (name, attrs) = match &self.active_formatting[index] {
                ActiveFormattingEntry::Element(e) => (e.name.clone(), e.attrs.clone()),
                _ => {
                    index += 1;
                    continue;
                }
            };
            let tag = Tag::new(TagKind::Start, name, attrs, false);
            let new_node = self.insert_element_html(&tag, true);
            if let ActiveFormattingEntry::Element(ref mut e) = self.active_formatting[index] {
                e.node = new_node;
            }
            index += 1;
        }
    }

    // -----------------------------------------------------------------------
    // Foster parenting
    // -----------------------------------------------------------------------

    fn should_foster_parenting_for_tag(&self, target: &NodeHandle, for_tag: Option<&str>) -> bool {
        if !self.insert_from_table {
            return false;
        }
        let name = target.borrow().name.clone();
        if !TABLE_FOSTER_TARGETS.contains(&name.as_str()) {
            return false;
        }
        if let Some(tag) = for_tag {
            if TABLE_ALLOWED_CHILDREN.contains(&tag) {
                return false;
            }
        }
        true
    }

    fn should_foster_parenting_for_text(&self, target: &NodeHandle) -> bool {
        if !self.insert_from_table {
            return false;
        }
        let name = target.borrow().name.clone();
        TABLE_FOSTER_TARGETS.contains(&name.as_str())
    }

    fn appropriate_insertion_location(&self, override_target: Option<&NodeHandle>, foster_parenting: bool) -> (NodeHandle, usize) {
        let target = if let Some(t) = override_target {
            Rc::clone(t)
        } else {
            self.current_node_or_html()
        };

        let target_name = target.borrow().name.clone();
        if foster_parenting && ["table", "tbody", "tfoot", "thead", "tr"].contains(&target_name.as_str()) {
            let last_template = self.find_last_on_stack("template");
            let last_table = self.find_last_on_stack("table");

            if let Some(ref tmpl) = last_template {
                let tmpl_idx = self.stack_index(tmpl);
                let tbl_idx = last_table.as_ref().and_then(|t| self.stack_index(t));
                if tbl_idx.is_none() || tmpl_idx.unwrap_or(0) > tbl_idx.unwrap_or(0) {
                    let data = tmpl.borrow();
                    if let Some(ref content) = data.template_content {
                        let len = content.borrow().children.len();
                        return (Rc::clone(content), len);
                    }
                }
            }

            if last_table.is_none() {
                let len = target.borrow().children.len();
                return (target, len);
            }

            let table = last_table.unwrap();
            let parent = get_parent(&table);
            if let Some(parent) = parent {
                let parent_data = parent.borrow();
                let position = parent_data.children.iter().position(|c| Rc::ptr_eq(c, &table)).unwrap_or(0);
                drop(parent_data);
                return (parent, position);
            }
            let len = target.borrow().children.len();
            return (target, len);
        }

        // Template: insert into content fragment
        let data = target.borrow();
        if data.kind == NodeKind::Template {
            if let Some(ref content) = data.template_content {
                let len = content.borrow().children.len();
                return (Rc::clone(content), len);
            }
        }
        let len = data.children.len();
        drop(data);
        (target, len)
    }

    // -----------------------------------------------------------------------
    // Foreign content helpers
    // -----------------------------------------------------------------------

    fn adjust_svg_tag_name(&self, name: &str) -> String {
        let lowered = name.to_lowercase();
        SVG_TAG_NAME_ADJUSTMENTS.get(lowered.as_str()).map(|s| s.to_string()).unwrap_or_else(|| name.to_string())
    }

    fn prepare_foreign_attributes(&self, namespace: &str, attrs: &HashMap<String, Option<String>>) -> HashMap<String, Option<String>> {
        if attrs.is_empty() {
            return HashMap::new();
        }
        let mut adjusted = HashMap::new();
        for (name, value) in attrs {
            let lower_name = name.to_lowercase();
            let mut final_name = name.clone();

            if namespace == "math" {
                if let Some(adj) = MATHML_ATTRIBUTE_ADJUSTMENTS.get(lower_name.as_str()) {
                    final_name = adj.to_string();
                }
            } else if namespace == "svg" {
                if let Some(adj) = SVG_ATTRIBUTE_ADJUSTMENTS.get(lower_name.as_str()) {
                    final_name = adj.to_string();
                }
            }

            let check_name = final_name.to_lowercase();
            if let Some(fa) = FOREIGN_ATTRIBUTE_ADJUSTMENTS.get(check_name.as_str()) {
                let (prefix, local, _ns) = *fa;
                final_name = if prefix.is_empty() {
                    local.to_string()
                } else {
                    format!("{}:{}", prefix, local)
                };
            }

            adjusted.insert(final_name, value.clone());
        }
        adjusted
    }

    #[allow(dead_code)]
    fn node_attribute_value(node: &NodeHandle, name: &str) -> Option<String> {
        let target = name.to_lowercase();
        let data = node.borrow();
        for (attr_name, attr_value) in &data.attrs {
            if attr_name.to_lowercase() == target {
                return Some(attr_value.clone().unwrap_or_default());
            }
        }
        None
    }

    #[allow(dead_code)]
    fn is_html_integration_point(&self, node: &NodeHandle) -> bool {
        let data = node.borrow();
        self.is_html_integration_point_data(&data)
    }

    fn is_html_integration_point_data(&self, data: &NodeData) -> bool {
        if data.namespace == MATHML_NAMESPACE && data.name == "annotation-xml" {
            let encoding = data.attrs.iter().find(|(k, _)| k.to_lowercase() == "encoding");
            if let Some((_, val)) = encoding {
                let enc = val.as_deref().unwrap_or("").to_lowercase();
                return enc == "text/html" || enc == "application/xhtml+xml";
            }
            return false;
        }
        let prefix = NAMESPACE_URL_TO_PREFIX.get(data.namespace.as_str()).unwrap_or(&"html");
        is_html_integration_point(prefix, &data.name)
    }

    #[allow(dead_code)]
    fn is_mathml_text_integration_point(&self, node: &NodeHandle) -> bool {
        let data = node.borrow();
        self.is_mathml_text_integration_point_data(&data)
    }

    fn is_mathml_text_integration_point_data(&self, data: &NodeData) -> bool {
        if data.namespace != MATHML_NAMESPACE {
            return false;
        }
        is_mathml_text_integration_point("math", &data.name)
    }

    fn should_use_foreign_content(&self, token: &Token) -> bool {
        if self.open_elements.is_empty() {
            return false;
        }
        let current = self.open_elements.last().unwrap();
        let data = current.borrow();
        if data.namespace == HTML_NAMESPACE || data.namespace.is_empty() {
            return false;
        }
        if matches!(token, Token::EOF(_)) {
            return false;
        }
        if self.is_mathml_text_integration_point_data(&data) {
            if matches!(token, Token::Characters(_)) {
                return false;
            }
            if let Token::Tag(ref tag) = token {
                if tag.kind == TagKind::Start {
                    let name_lower = tag.name.to_lowercase();
                    if name_lower != "mglyph" && name_lower != "malignmark" {
                        return false;
                    }
                }
            }
        }
        if data.namespace == MATHML_NAMESPACE && data.name == "annotation-xml" {
            if let Token::Tag(ref tag) = token {
                if tag.kind == TagKind::Start && tag.name.to_lowercase() == "svg" {
                    return false;
                }
            }
        }
        if self.is_html_integration_point_data(&data) {
            if matches!(token, Token::Characters(_)) {
                return false;
            }
            if let Token::Tag(ref tag) = token {
                if tag.kind == TagKind::Start {
                    return false;
                }
            }
        }
        true
    }

    fn foreign_breakout_font(tag: &Tag) -> bool {
        for name in tag.attrs.keys() {
            let lower = name.to_lowercase();
            if lower == "color" || lower == "face" || lower == "size" {
                return true;
            }
        }
        false
    }

    fn pop_until_html_or_integration_point(&mut self) {
        while let Some(node) = self.open_elements.last() {
            let data = node.borrow();
            if data.namespace == HTML_NAMESPACE || data.namespace.is_empty() {
                return;
            }
            if self.is_html_integration_point_data(&data) {
                return;
            }
            if let Some(ref ctx) = self.fragment_context_element {
                if Rc::ptr_eq(node, ctx) {
                    return;
                }
            }
            drop(data);
            self.open_elements.pop();
        }
    }

    // -----------------------------------------------------------------------
    // Table helpers
    // -----------------------------------------------------------------------

    fn close_table_cell(&mut self) -> bool {
        if self.has_in_table_scope("td") {
            self.end_table_cell("td");
            return true;
        }
        if self.has_in_table_scope("th") {
            self.end_table_cell("th");
            return true;
        }
        false
    }

    fn end_table_cell(&mut self, name: &str) {
        self.generate_implied_end_tags(Some(name));
        while let Some(node) = self.open_elements.pop() {
            let node_data = node.borrow();
            if node_data.name == name && (node_data.namespace == HTML_NAMESPACE || node_data.namespace.is_empty()) {
                break;
            }
        }
        self.clear_active_formatting_up_to_marker();
        self.mode = InsertionMode::InRow;
    }

    fn flush_pending_table_text(&mut self) {
        let data: String = self.pending_table_text.drain(..).collect();
        if data.is_empty() {
            return;
        }
        if is_all_whitespace(&data) {
            self.append_text(&data);
            return;
        }
        self.parse_error("foster-parenting-character", None);
        let previous = self.insert_from_table;
        self.insert_from_table = true;
        self.reconstruct_active_formatting_elements();
        self.append_text(&data);
        self.insert_from_table = previous;
    }

    fn close_table_element(&mut self) -> bool {
        if !self.has_in_table_scope("table") {
            self.parse_error("unexpected-end-tag", Some("table"));
            return false;
        }
        self.generate_implied_end_tags(None);
        while let Some(node) = self.open_elements.pop() {
            if node.borrow().name == "table" {
                break;
            }
        }
        self.reset_insertion_mode();
        true
    }

    fn close_caption_element(&mut self) -> bool {
        if !self.has_in_table_scope("caption") {
            self.parse_error("unexpected-end-tag", Some("caption"));
            return false;
        }
        self.generate_implied_end_tags(None);
        while let Some(node) = self.open_elements.pop() {
            if node.borrow().name == "caption" {
                break;
            }
        }
        self.clear_active_formatting_up_to_marker();
        self.mode = InsertionMode::InTable;
        true
    }

    fn end_tr_element(&mut self) {
        self.clear_stack_until(&["tr", "template", "html"]);
        if let Some(top) = self.open_elements.last() {
            if top.borrow().name == "tr" {
                self.open_elements.pop();
            }
        }
        if !self.template_modes.is_empty() {
            self.mode = *self.template_modes.last().unwrap();
        } else {
            self.mode = InsertionMode::InTableBody;
        }
    }

    fn reset_insertion_mode(&mut self) {
        let mut idx = self.open_elements.len();
        while idx > 0 {
            idx -= 1;
            let name = self.open_elements[idx].borrow().name.clone();
            match name.as_str() {
                "select" => { self.mode = InsertionMode::InSelect; return; }
                "td" | "th" => { self.mode = InsertionMode::InCell; return; }
                "tr" => { self.mode = InsertionMode::InRow; return; }
                "tbody" | "tfoot" | "thead" => { self.mode = InsertionMode::InTableBody; return; }
                "caption" => { self.mode = InsertionMode::InCaption; return; }
                "table" => { self.mode = InsertionMode::InTable; return; }
                "template" => {
                    if let Some(&m) = self.template_modes.last() {
                        self.mode = m;
                        return;
                    }
                }
                "head" => { self.mode = InsertionMode::InHead; return; }
                "html" => { self.mode = InsertionMode::InBody; return; }
                _ => {}
            }
        }
        self.mode = InsertionMode::InBody;
    }

    // -----------------------------------------------------------------------
    // Adoption agency algorithm
    // -----------------------------------------------------------------------

    fn adoption_agency(&mut self, subject: &str) {
        // Step 1: current node is subject and not in active formatting
        if let Some(top) = self.open_elements.last() {
            if top.borrow().name == subject && !self.has_active_formatting_entry(subject) {
                self.pop_until_inclusive(subject);
                return;
            }
        }

        // Step 2: outer loop (max 8 iterations)
        for _ in 0..8 {
            let fmt_idx = match self.find_active_formatting_index(subject) {
                Some(i) => i,
                None => return,
            };

            let fmt_node = match &self.active_formatting[fmt_idx] {
                ActiveFormattingEntry::Element(e) => Rc::clone(&e.node),
                _ => return,
            };

            // Step 4: not in open elements
            let fmt_stack_idx = match self.stack_index(&fmt_node) {
                Some(i) => i,
                None => {
                    self.parse_error("adoption-agency-1.3", None);
                    self.remove_formatting_entry(fmt_idx);
                    return;
                }
            };

            // Step 5: not in scope
            if !self.in_scope(subject) {
                self.parse_error("adoption-agency-1.3", None);
                return;
            }

            // Step 6: not current node
            if !Rc::ptr_eq(&fmt_node, self.open_elements.last().unwrap()) {
                self.parse_error("adoption-agency-1.3", None);
            }

            // Step 7: find furthest block
            let mut furthest_block_idx = None;
            for i in (fmt_stack_idx + 1)..self.open_elements.len() {
                if self.is_special_element(&self.open_elements[i]) {
                    furthest_block_idx = Some(i);
                    break;
                }
            }

            if furthest_block_idx.is_none() {
                // Pop up to and including formatting element
                while let Some(popped) = self.open_elements.pop() {
                    if Rc::ptr_eq(&popped, &fmt_node) {
                        break;
                    }
                }
                self.remove_formatting_entry(fmt_idx);
                return;
            }

            let furthest_block_idx = furthest_block_idx.unwrap();
            let furthest_block = Rc::clone(&self.open_elements[furthest_block_idx]);

            // Step 8: bookmark
            let mut bookmark = fmt_idx + 1;

            // Steps 9-10: inner loop
            let mut node = Rc::clone(&furthest_block);
            let mut last_node = Rc::clone(&furthest_block);
            let mut inner_loop_counter = 0;

            loop {
                inner_loop_counter += 1;

                // Node = element above node
                let node_idx = self.stack_index(&node).unwrap();
                if node_idx == 0 {
                    break;
                }
                node = Rc::clone(&self.open_elements[node_idx - 1]);

                // If node is formatting element, break
                if Rc::ptr_eq(&node, &fmt_node) {
                    break;
                }

                // Find active formatting entry for node
                let mut node_fmt_idx = self.find_active_formatting_index_by_node(&node);

                if inner_loop_counter > 3 {
                    if let Some(nfi) = node_fmt_idx {
                        self.remove_formatting_entry(nfi);
                        if nfi < bookmark {
                            bookmark -= 1;
                        }
                        node_fmt_idx = None;
                    }
                }

                if node_fmt_idx.is_none() {
                    let ni = self.stack_index(&node).unwrap();
                    self.open_elements.remove(ni);
                    continue;
                }

                let node_fmt_idx = node_fmt_idx.unwrap();

                // Create replacement element
                let (entry_name, entry_ns, entry_attrs) = {
                    let entry = match &self.active_formatting[node_fmt_idx] {
                        ActiveFormattingEntry::Element(e) => e,
                        _ => break,
                    };
                    (entry.name.clone(), entry.node.borrow().namespace.clone(), entry.attrs.clone())
                };
                let new_elem = new_element(&entry_name, &entry_ns, entry_attrs);

                // Update formatting entry node
                if let ActiveFormattingEntry::Element(ref mut e) = self.active_formatting[node_fmt_idx] {
                    e.node = Rc::clone(&new_elem);
                }
                // Replace in open elements
                let ni = self.stack_index(&node).unwrap();
                self.open_elements[ni] = Rc::clone(&new_elem);
                node = new_elem;

                // If last_node is furthest_block, update bookmark
                if Rc::ptr_eq(&last_node, &furthest_block) {
                    bookmark = node_fmt_idx + 1;
                }

                // Reparent last_node
                if let Some(old_parent) = get_parent(&last_node) {
                    remove_child(&old_parent, &last_node);
                }
                append_child(&node, &last_node);

                last_node = Rc::clone(&node);
            }

            // Step 11: Insert last_node into common ancestor
            let common_ancestor = if fmt_stack_idx > 0 {
                Rc::clone(&self.open_elements[fmt_stack_idx - 1])
            } else {
                Rc::clone(&self.document)
            };

            if let Some(old_parent) = get_parent(&last_node) {
                remove_child(&old_parent, &last_node);
            }

            let ca_kind = common_ancestor.borrow().kind.clone();
            if self.should_foster_parenting_for_tag(&common_ancestor, Some(&last_node.borrow().name)) {
                let (parent, position) = self.appropriate_insertion_location(Some(&common_ancestor), true);
                self.insert_node_at(&parent, position, &last_node);
            } else if ca_kind == NodeKind::Template {
                let content = common_ancestor.borrow().template_content.as_ref().map(Rc::clone);
                if let Some(content) = content {
                    append_child(&content, &last_node);
                } else {
                    append_child(&common_ancestor, &last_node);
                }
            } else {
                append_child(&common_ancestor, &last_node);
            }

            // Step 12: Create new formatting element
            let (entry_name, entry_ns, entry_attrs) = {
                let entry = match &self.active_formatting[fmt_idx] {
                    ActiveFormattingEntry::Element(e) => e,
                    _ => return,
                };
                (entry.name.clone(), entry.node.borrow().namespace.clone(), entry.attrs.clone())
            };
            let new_formatting_element = new_element(&entry_name, &entry_ns, entry_attrs.clone());

            // Step 13: Move children of furthest block
            let children: Vec<NodeHandle> = furthest_block.borrow().children.iter().map(Rc::clone).collect();
            for child in children {
                remove_child(&furthest_block, &child);
                append_child(&new_formatting_element, &child);
            }
            append_child(&furthest_block, &new_formatting_element);

            // Step 14: Update formatting list
            let sig = Self::attrs_signature(&entry_attrs);
            let new_entry = ActiveFormattingEntry::Element(FormattingEntry {
                name: entry_name,
                attrs: entry_attrs,
                node: Rc::clone(&new_formatting_element),
                signature: sig,
            });
            self.remove_formatting_entry(fmt_idx);
            if bookmark > 0 {
                bookmark -= 1;
            }
            let insert_at = bookmark.min(self.active_formatting.len());
            self.active_formatting.insert(insert_at, new_entry);

            // Step 15: Update open elements
            self.remove_from_open_elements(&fmt_node);
            let fb_idx = self.stack_index(&furthest_block).unwrap();
            self.open_elements.insert(fb_idx + 1, new_formatting_element);
        }
    }

    // -----------------------------------------------------------------------
    // selectedcontent population (post-parse)
    // -----------------------------------------------------------------------

    fn populate_selectedcontent(&self, root: &NodeHandle) {
        let mut selects = Vec::new();
        Self::find_elements_recursive(root, "select", &mut selects);

        for select in &selects {
            let sc = Self::find_element_recursive(select, "selectedcontent");
            if sc.is_none() {
                continue;
            }
            let sc = sc.unwrap();

            let mut options = Vec::new();
            Self::find_elements_recursive(select, "option", &mut options);
            if options.is_empty() {
                continue;
            }

            let mut selected_option = None;
            for opt in &options {
                let data = opt.borrow();
                if data.attrs.contains_key("selected") {
                    selected_option = Some(Rc::clone(opt));
                    break;
                }
            }
            if selected_option.is_none() {
                selected_option = Some(Rc::clone(&options[0]));
            }

            let source = selected_option.unwrap();
            Self::clone_children(&source, &sc);
        }
    }

    fn find_elements_recursive(node: &NodeHandle, name: &str, result: &mut Vec<NodeHandle>) {
        if node.borrow().name == name {
            result.push(Rc::clone(node));
        }
        let children: Vec<NodeHandle> = node.borrow().children.iter().map(Rc::clone).collect();
        for child in children {
            Self::find_elements_recursive(&child, name, result);
        }
    }

    fn find_element_recursive(node: &NodeHandle, name: &str) -> Option<NodeHandle> {
        if node.borrow().name == name {
            return Some(Rc::clone(node));
        }
        let children: Vec<NodeHandle> = node.borrow().children.iter().map(Rc::clone).collect();
        for child in children {
            if let Some(found) = Self::find_element_recursive(&child, name) {
                return Some(found);
            }
        }
        None
    }

    fn clone_children(source: &NodeHandle, target: &NodeHandle) {
        let children: Vec<NodeHandle> = source.borrow().children.iter().map(Rc::clone).collect();
        for child in children {
            let cloned = Self::deep_clone_node(&child);
            append_child(target, &cloned);
        }
    }

    fn deep_clone_node(node: &NodeHandle) -> NodeHandle {
        use std::cell::RefCell;
        let data = node.borrow();
        let cloned = Rc::new(RefCell::new(NodeData {
            kind: data.kind.clone(),
            name: data.name.clone(),
            namespace: data.namespace.clone(),
            attrs: data.attrs.clone(),
            data: data.data.clone(),
            children: Vec::new(),
            parent: None,
            template_content: data.template_content.as_ref().map(|tc| Self::deep_clone_node(tc)),
            doctype_data: data.doctype_data.clone(),
        }));
        for child in &data.children {
            let child_clone = Self::deep_clone_node(child);
            child_clone.borrow_mut().parent = Some(Rc::downgrade(&cloned));
            cloned.borrow_mut().children.push(child_clone);
        }
        cloned
    }
}

// ===========================================================================
// Mode handlers: Initial, BeforeHtml, BeforeHead, InHead, InHeadNoscript,
//                AfterHead, Text
// ===========================================================================

impl TreeBuilder {
    fn mode_initial(&mut self, token: Token) -> ModeResult {
        match token {
            Token::Characters(ref ct) => {
                if is_all_whitespace(&ct.data) {
                    return None;
                }
                self.parse_error("expected-doctype-but-got-chars", None);
                self.quirks_mode = "quirks".to_string();
                reprocess(InsertionMode::BeforeHtml, token)
            }
            Token::Comment(ref ct) => {
                self.append_comment_to_document(&ct.data);
                None
            }
            Token::EOF(_) => {
                self.parse_error("expected-doctype-but-got-eof", None);
                self.quirks_mode = "quirks".to_string();
                self.mode = InsertionMode::BeforeHtml;
                reprocess(InsertionMode::BeforeHtml, token)
            }
            Token::Tag(ref tag) => {
                if tag.kind == TagKind::Start {
                    self.parse_error_with_token("expected-doctype-but-got-start-tag", Some(&tag.name), Some(&token));
                } else {
                    self.parse_error_with_token("expected-doctype-but-got-end-tag", Some(&tag.name), Some(&token));
                }
                self.quirks_mode = "quirks".to_string();
                reprocess(InsertionMode::BeforeHtml, token)
            }
            Token::Doctype(_) => None, // handled in process_token
        }
    }

    fn handle_doctype(&mut self, token: &Token) -> TokenSinkResult {
        if self.mode != InsertionMode::Initial {
            self.parse_error("unexpected-doctype", None);
            return TokenSinkResult::Continue;
        }
        if let Token::Doctype(ref dt) = token {
            let (parse_error, quirks_mode) = doctype_error_and_quirks(&dt.doctype, self.iframe_srcdoc);

            // Create doctype node
            let doctype_data = crate::node::DoctypeData {
                name: dt.doctype.name.clone(),
                public_id: dt.doctype.public_id.clone(),
                system_id: dt.doctype.system_id.clone(),
            };
            let node = new_doctype(doctype_data);
            append_child(&self.document, &node);

            if parse_error {
                self.parse_error("unknown-doctype", None);
            }
            self.quirks_mode = quirks_mode.to_string();
            self.mode = InsertionMode::BeforeHtml;
        }
        TokenSinkResult::Continue
    }

    fn mode_before_html(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) if is_all_whitespace(&ct.data) => None,
            Token::Comment(ct) => {
                self.append_comment_to_document(&ct.data);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start && tag.name == "html" {
                    self.create_root(tag.attrs.clone());
                    self.mode = InsertionMode::BeforeHead;
                    None
                } else if tag.kind == TagKind::End && ["head", "body", "html", "br"].contains(&tag.name.as_str()) {
                    self.create_root(HashMap::new());
                    self.mode = InsertionMode::BeforeHead;
                    reprocess(InsertionMode::BeforeHead, token)
                } else if tag.kind == TagKind::End {
                    self.parse_error("unexpected-end-tag-before-html", Some(&tag.name));
                    None
                } else {
                    self.create_root(HashMap::new());
                    self.mode = InsertionMode::BeforeHead;
                    reprocess(InsertionMode::BeforeHead, token)
                }
            }
            Token::EOF(_) => {
                self.create_root(HashMap::new());
                self.mode = InsertionMode::BeforeHead;
                reprocess(InsertionMode::BeforeHead, token)
            }
            Token::Characters(_) => {
                self.create_root(HashMap::new());
                self.mode = InsertionMode::BeforeHead;
                reprocess(InsertionMode::BeforeHead, token)
            }
            _ => {
                self.create_root(HashMap::new());
                self.mode = InsertionMode::BeforeHead;
                reprocess(InsertionMode::BeforeHead, token)
            }
        }
    }

    fn mode_before_head(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\0') {
                    self.parse_error("invalid-codepoint-before-head", None);
                    data = data.replace('\0', "");
                    if data.is_empty() {
                        return None;
                    }
                }
                if is_all_whitespace(&data) {
                    return None;
                }
                self.head_element = Some(self.insert_phantom("head"));
                self.mode = InsertionMode::InHead;
                let new_token = Token::Characters(CharacterTokens::new(data));
                reprocess(InsertionMode::InHead, new_token)
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start && tag.name == "html" {
                    let html = Rc::clone(&self.open_elements[0]);
                    self.add_missing_attributes(&html, &tag.attrs);
                    None
                } else if tag.kind == TagKind::Start && tag.name == "head" {
                    let head = self.insert_element_html(tag, true);
                    self.head_element = Some(head);
                    self.mode = InsertionMode::InHead;
                    None
                } else if tag.kind == TagKind::End && ["head", "body", "html", "br"].contains(&tag.name.as_str()) {
                    self.head_element = Some(self.insert_phantom("head"));
                    self.mode = InsertionMode::InHead;
                    reprocess(InsertionMode::InHead, token)
                } else if tag.kind == TagKind::End {
                    self.parse_error("unexpected-end-tag-before-head", Some(&tag.name));
                    None
                } else {
                    self.head_element = Some(self.insert_phantom("head"));
                    self.mode = InsertionMode::InHead;
                    reprocess(InsertionMode::InHead, token)
                }
            }
            Token::EOF(_) => {
                self.head_element = Some(self.insert_phantom("head"));
                self.mode = InsertionMode::InHead;
                reprocess(InsertionMode::InHead, token)
            }
            _ => {
                self.head_element = Some(self.insert_phantom("head"));
                self.mode = InsertionMode::InHead;
                reprocess(InsertionMode::InHead, token)
            }
        }
    }

    fn mode_in_head(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                if is_all_whitespace(&ct.data) {
                    self.append_text(&ct.data);
                    return None;
                }
                let data = ct.data.clone();
                let mut i = 0;
                for ch in data.chars() {
                    if !matches!(ch, '\t' | '\n' | '\x0C' | '\r' | ' ') {
                        break;
                    }
                    i += ch.len_utf8();
                }
                let leading_ws = &data[..i];
                let remaining = &data[i..];
                if !leading_ws.is_empty() {
                    if let Some(current) = self.open_elements.last() {
                        if has_child_nodes(current) {
                            self.append_text(leading_ws);
                        }
                    }
                }
                self.pop_current();
                self.mode = InsertionMode::AfterHead;
                let new_token = Token::Characters(CharacterTokens::new(remaining.to_string()));
                reprocess(InsertionMode::AfterHead, new_token)
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "html" => {
                            self.pop_current();
                            self.mode = InsertionMode::AfterHead;
                            reprocess(InsertionMode::AfterHead, token)
                        }
                        "base" | "basefont" | "bgsound" | "link" | "meta" => {
                            self.insert_element_html(tag, false);
                            None
                        }
                        "template" => {
                            self.insert_element_html(tag, true);
                            self.push_formatting_marker();
                            self.frameset_ok = false;
                            self.mode = InsertionMode::InTemplate;
                            self.template_modes.push(InsertionMode::InTemplate);
                            None
                        }
                        "title" | "style" | "script" | "noframes" => {
                            self.insert_element_html(tag, true);
                            self.original_mode = Some(self.mode);
                            self.mode = InsertionMode::Text;
                            None
                        }
                        "noscript" => {
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InHeadNoscript;
                            None
                        }
                        _ => {
                            self.pop_current();
                            self.mode = InsertionMode::AfterHead;
                            reprocess(InsertionMode::AfterHead, token)
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "template" => {
                            let has_template = self.open_elements.iter().any(|n| n.borrow().name == "template");
                            if !has_template {
                                return None;
                            }
                            self.generate_implied_end_tags(None);
                            self.pop_until_inclusive("template");
                            self.clear_active_formatting_up_to_marker();
                            self.template_modes.pop();
                            self.reset_insertion_mode();
                            None
                        }
                        "head" => {
                            self.pop_current();
                            self.mode = InsertionMode::AfterHead;
                            None
                        }
                        "body" | "html" | "br" => {
                            self.pop_current();
                            self.mode = InsertionMode::AfterHead;
                            reprocess(InsertionMode::AfterHead, token)
                        }
                        _ => {
                            self.pop_current();
                            self.mode = InsertionMode::AfterHead;
                            reprocess(InsertionMode::AfterHead, token)
                        }
                    }
                }
            }
            Token::EOF(_) => {
                self.pop_current();
                self.mode = InsertionMode::AfterHead;
                reprocess(InsertionMode::AfterHead, token)
            }
            _ => {
                self.pop_current();
                self.mode = InsertionMode::AfterHead;
                reprocess(InsertionMode::AfterHead, token)
            }
        }
    }

    fn mode_in_head_noscript(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                if is_all_whitespace(&ct.data) {
                    return self.mode_in_head(token);
                }
                self.parse_error("unexpected-start-tag", Some("text"));
                self.pop_current();
                self.mode = InsertionMode::InHead;
                reprocess(InsertionMode::InHead, token)
            }
            Token::Comment(_) => self.mode_in_head(token),
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "html" => self.mode_in_body(token),
                        "basefont" | "bgsound" | "link" | "meta" | "noframes" | "style" => {
                            self.mode_in_head(token)
                        }
                        "head" | "noscript" => {
                            self.parse_error("unexpected-start-tag", Some(&tag.name));
                            None
                        }
                        _ => {
                            self.parse_error("unexpected-start-tag", Some(&tag.name));
                            self.pop_current();
                            self.mode = InsertionMode::InHead;
                            reprocess(InsertionMode::InHead, token)
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "noscript" => {
                            self.pop_current();
                            self.mode = InsertionMode::InHead;
                            None
                        }
                        "br" => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            self.pop_current();
                            self.mode = InsertionMode::InHead;
                            reprocess(InsertionMode::InHead, token)
                        }
                        _ => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            None
                        }
                    }
                }
            }
            Token::EOF(_) => {
                self.parse_error("expected-closing-tag-but-got-eof", Some("noscript"));
                self.pop_current();
                self.mode = InsertionMode::InHead;
                reprocess(InsertionMode::InHead, token)
            }
            _ => None,
        }
    }

    fn mode_after_head(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\0') {
                    self.parse_error("invalid-codepoint-in-body", None);
                    data = data.replace('\0', "");
                }
                if data.contains('\x0C') {
                    self.parse_error("invalid-codepoint-in-body", None);
                    data = data.replace('\x0C', "");
                }
                if data.is_empty() || is_all_whitespace(&data) {
                    if !data.is_empty() {
                        self.append_text(&data);
                    }
                    return None;
                }
                self.insert_body_if_missing();
                let new_token = Token::Characters(CharacterTokens::new(data));
                reprocess(InsertionMode::InBody, new_token)
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "html" => {
                            self.insert_body_if_missing();
                            reprocess(InsertionMode::InBody, token)
                        }
                        "body" => {
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InBody;
                            self.frameset_ok = false;
                            None
                        }
                        "frameset" => {
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InFrameset;
                            None
                        }
                        "input" => {
                            let input_type = tag.attrs.get("type").and_then(|v| v.as_ref()).map(|v| v.to_lowercase());
                            if input_type.as_deref() == Some("hidden") {
                                self.parse_error("unexpected-hidden-input-after-head", None);
                                return None;
                            }
                            self.insert_body_if_missing();
                            reprocess(InsertionMode::InBody, token)
                        }
                        "base" | "basefont" | "bgsound" | "link" | "meta" | "title" | "style" | "script" | "noscript" => {
                            if let Some(ref head) = self.head_element {
                                self.open_elements.push(Rc::clone(head));
                            }
                            let result = self.mode_in_head(token);
                            // Remove head from stack
                            if let Some(ref head) = self.head_element {
                                self.open_elements.retain(|n| !Rc::ptr_eq(n, head));
                            }
                            result
                        }
                        "template" => {
                            if let Some(ref head) = self.head_element {
                                self.open_elements.push(Rc::clone(head));
                            }
                            self.mode = InsertionMode::InHead;
                            reprocess(InsertionMode::InHead, token)
                        }
                        _ => {
                            self.insert_body_if_missing();
                            reprocess(InsertionMode::InBody, token)
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "template" => self.mode_in_head(token),
                        "body" | "html" | "br" => {
                            self.insert_body_if_missing();
                            reprocess(InsertionMode::InBody, token)
                        }
                        _ => {
                            self.parse_error("unexpected-end-tag-after-head", Some(&tag.name));
                            None
                        }
                    }
                }
            }
            Token::EOF(_) => {
                self.insert_body_if_missing();
                self.mode = InsertionMode::InBody;
                reprocess(InsertionMode::InBody, token)
            }
            _ => {
                self.insert_body_if_missing();
                reprocess(InsertionMode::InBody, token)
            }
        }
    }

    fn mode_text(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                self.append_text(&ct.data);
                None
            }
            Token::EOF(_) => {
                let tag_name = self.open_elements.last().map(|n| n.borrow().name.clone());
                self.parse_error("expected-named-closing-tag-but-got-eof", tag_name.as_deref());
                self.pop_current();
                self.mode = self.original_mode.unwrap_or(InsertionMode::InBody);
                reprocess(self.mode, token)
            }
            Token::Tag(_) => {
                // End tag
                self.pop_current();
                self.mode = self.original_mode.unwrap_or(InsertionMode::InBody);
                None
            }
            _ => None,
        }
    }
}

// ===========================================================================
// InBody mode and body tag handlers
// ===========================================================================

impl TreeBuilder {
    fn mode_in_body(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\0') {
                    self.parse_error("invalid-codepoint", None);
                    data = data.replace('\0', "");
                }
                if is_all_whitespace(&data) {
                    self.reconstruct_active_formatting_elements();
                    self.append_text(&data);
                    return None;
                }
                self.reconstruct_active_formatting_elements();
                self.frameset_ok = false;
                self.append_text(&data);
                None
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    self.handle_body_start_tag(&token)
                } else {
                    self.handle_body_end_tag(&token)
                }
            }
            Token::EOF(_) => self.handle_eof_in_body(token),
            _ => None,
        }
    }

    fn handle_body_start_tag(&mut self, token: &Token) -> ModeResult {
        let tag = match token {
            Token::Tag(t) => t,
            _ => return None,
        };
        let name = tag.name.as_str();
        match name {
            "html" => {
                if !self.template_modes.is_empty() {
                    self.parse_error("unexpected-start-tag", Some(name));
                    return None;
                }
                if let Some(html) = self.open_elements.first() {
                    self.add_missing_attributes(html, &tag.attrs);
                }
                None
            }
            "body" => {
                if !self.template_modes.is_empty() {
                    self.parse_error("unexpected-start-tag", Some(name));
                    return None;
                }
                if self.open_elements.len() > 1 {
                    self.parse_error("unexpected-start-tag", Some(name));
                    if let Some(body) = self.open_elements.get(1) {
                        if body.borrow().name == "body" {
                            self.add_missing_attributes(body, &tag.attrs);
                        }
                    }
                    self.frameset_ok = false;
                }
                None
            }
            "head" => {
                self.parse_error("unexpected-start-tag", Some(name));
                None
            }
            "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title" => {
                self.mode_in_head(token.clone())
            }
            "address" | "article" | "aside" | "blockquote" | "center" | "details" |
            "dialog" | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure" |
            "footer" | "header" | "hgroup" | "main" | "menu" | "nav" | "ol" |
            "search" | "section" | "summary" | "ul" | "listing" => {
                self.close_p_element();
                self.insert_element_html(tag, true);
                if name == "listing" {
                    self.ignore_lf = true;
                    self.frameset_ok = false;
                }
                None
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.close_p_element();
                if let Some(top) = self.open_elements.last() {
                    if HEADING_ELEMENTS.contains(top.borrow().name.as_str()) {
                        self.parse_error("unexpected-start-tag", Some(name));
                        self.pop_current();
                    }
                }
                self.insert_element_html(tag, true);
                self.frameset_ok = false;
                None
            }
            "pre" => {
                self.close_p_element();
                self.insert_element_html(tag, true);
                self.ignore_lf = true;
                self.frameset_ok = false;
                None
            }
            "form" => {
                if self.form_element.is_some() {
                    self.parse_error("unexpected-start-tag", Some(name));
                    return None;
                }
                self.close_p_element();
                let node = self.insert_element_html(tag, true);
                self.form_element = Some(node);
                self.frameset_ok = false;
                None
            }
            "li" => {
                self.frameset_ok = false;
                self.close_p_element();
                if self.has_in_list_item_scope("li") {
                    self.pop_until_any_inclusive(&["li"]);
                }
                self.insert_element_html(tag, true);
                None
            }
            "dd" | "dt" => {
                self.frameset_ok = false;
                self.close_p_element();
                if name == "dd" {
                    if self.has_in_definition_scope("dd") {
                        self.pop_until_any_inclusive(&["dd"]);
                    }
                    if self.has_in_definition_scope("dt") {
                        self.pop_until_any_inclusive(&["dt"]);
                    }
                } else {
                    if self.has_in_definition_scope("dt") {
                        self.pop_until_any_inclusive(&["dt"]);
                    }
                    if self.has_in_definition_scope("dd") {
                        self.pop_until_any_inclusive(&["dd"]);
                    }
                }
                self.insert_element_html(tag, true);
                None
            }
            "p" => {
                self.close_p_element();
                self.insert_element_html(tag, true);
                None
            }
            "a" => {
                if self.has_active_formatting_entry("a") {
                    self.adoption_agency("a");
                    self.remove_last_active_formatting_by_name("a");
                    self.remove_last_open_element_by_name("a");
                }
                self.reconstruct_active_formatting_elements();
                let node = self.insert_element_html(tag, true);
                self.append_active_formatting_entry("a", &tag.attrs, &node);
                None
            }
            "b" | "big" | "code" | "em" | "font" | "i" | "s" | "small" | "strike" | "strong" | "tt" | "u" | "nobr" => {
                if name == "nobr" && self.in_scope("nobr") {
                    self.adoption_agency("nobr");
                    self.remove_last_active_formatting_by_name("nobr");
                    self.remove_last_open_element_by_name("nobr");
                }
                self.reconstruct_active_formatting_elements();
                let dup = self.find_active_formatting_duplicate(name, &tag.attrs);
                if let Some(dup_idx) = dup {
                    self.remove_formatting_entry(dup_idx);
                }
                let node = self.insert_element_html(tag, true);
                self.append_active_formatting_entry(name, &tag.attrs, &node);
                None
            }
            "button" => {
                if self.in_scope("button") {
                    self.parse_error("unexpected-start-tag-implies-end-tag", Some(name));
                    self.close_element_by_name("button");
                }
                self.insert_element_html(tag, true);
                self.frameset_ok = false;
                None
            }
            "applet" | "marquee" | "object" => {
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, true);
                self.push_formatting_marker();
                self.frameset_ok = false;
                None
            }
            "br" => {
                self.close_p_element();
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, false);
                self.frameset_ok = false;
                None
            }
            "area" | "embed" | "img" | "keygen" | "wbr" => {
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, false);
                self.frameset_ok = false;
                None
            }
            "hr" => {
                self.close_p_element();
                self.insert_element_html(tag, false);
                self.frameset_ok = false;
                None
            }
            "input" => {
                let input_type = tag.attrs.get("type").and_then(|v| v.as_ref()).map(|v| v.to_lowercase());
                self.insert_element_html(tag, false);
                if input_type.as_deref() != Some("hidden") {
                    self.frameset_ok = false;
                }
                None
            }
            "param" | "source" | "track" => {
                self.insert_element_html(tag, false);
                None
            }
            "image" => {
                self.parse_error("image-start-tag", Some(name));
                let img_tag = Tag::new(TagKind::Start, "img".to_string(), tag.attrs.clone(), tag.self_closing);
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(&img_tag, false);
                self.frameset_ok = false;
                None
            }
            "table" => {
                if self.quirks_mode != "quirks" {
                    self.close_p_element();
                }
                self.insert_element_html(tag, true);
                self.frameset_ok = false;
                self.mode = InsertionMode::InTable;
                None
            }
            "textarea" => {
                self.insert_element_html(tag, true);
                self.ignore_lf = true;
                self.frameset_ok = false;
                None
            }
            "plaintext" | "xmp" | "iframe" | "noembed" | "noscript" => {
                self.close_p_element();
                self.insert_element_html(tag, true);
                self.frameset_ok = false;
                if name == "plaintext" {
                    self.tokenizer_state_override = Some(TokenSinkResult::Plaintext);
                } else {
                    self.original_mode = Some(self.mode);
                    self.mode = InsertionMode::Text;
                }
                None
            }
            "select" => {
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, true);
                self.frameset_ok = false;
                self.reset_insertion_mode();
                None
            }
            "option" => {
                if let Some(top) = self.open_elements.last() {
                    if top.borrow().name == "option" {
                        self.open_elements.pop();
                    }
                }
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, true);
                None
            }
            "optgroup" => {
                if let Some(top) = self.open_elements.last() {
                    if top.borrow().name == "option" {
                        self.open_elements.pop();
                    }
                }
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, true);
                None
            }
            "math" => {
                self.reconstruct_active_formatting_elements();
                let attrs = self.prepare_foreign_attributes("math", &tag.attrs);
                let new_tag = Tag::new(TagKind::Start, tag.name.clone(), attrs, tag.self_closing);
                self.insert_element(&new_tag, !tag.self_closing, MATHML_NAMESPACE);
                None
            }
            "svg" => {
                self.reconstruct_active_formatting_elements();
                let adjusted_name = self.adjust_svg_tag_name(&tag.name);
                let attrs = self.prepare_foreign_attributes("svg", &tag.attrs);
                let new_tag = Tag::new(TagKind::Start, adjusted_name, attrs, tag.self_closing);
                self.insert_element(&new_tag, !tag.self_closing, SVG_NAMESPACE);
                None
            }
            "rp" | "rt" => {
                self.generate_implied_end_tags(Some("rtc"));
                self.insert_element_html(tag, true);
                None
            }
            "rb" | "rtc" => {
                if let Some(top) = self.open_elements.last() {
                    if ["rb", "rp", "rt", "rtc"].contains(&top.borrow().name.as_str()) {
                        self.generate_implied_end_tags(None);
                    }
                }
                self.insert_element_html(tag, true);
                None
            }
            "frameset" => {
                if !self.frameset_ok {
                    self.parse_error("unexpected-start-tag-ignored", Some(name));
                    return None;
                }
                let mut body_index = None;
                for (i, elem) in self.open_elements.iter().enumerate() {
                    if elem.borrow().name == "body" {
                        body_index = Some(i);
                        break;
                    }
                }
                if body_index.is_none() {
                    self.parse_error("unexpected-start-tag-ignored", Some(name));
                    return None;
                }
                let body_idx = body_index.unwrap();
                let body_elem = Rc::clone(&self.open_elements[body_idx]);
                if let Some(parent) = get_parent(&body_elem) {
                    remove_child(&parent, &body_elem);
                }
                self.open_elements.truncate(body_idx);
                self.insert_element_html(tag, true);
                self.mode = InsertionMode::InFrameset;
                None
            }
            "caption" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => {
                self.parse_error("unexpected-start-tag-ignored", Some(name));
                None
            }
            "col" | "frame" => {
                if self.fragment_context.is_none() {
                    self.parse_error("unexpected-start-tag-ignored", Some(name));
                    return None;
                }
                self.insert_element_html(tag, false);
                None
            }
            _ => {
                // Default handler
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(tag, true);
                if tag.self_closing {
                    self.parse_error("non-void-html-element-start-tag-with-trailing-solidus", Some(name));
                }
                self.frameset_ok = false;
                None
            }
        }
    }

    fn handle_body_end_tag(&mut self, token: &Token) -> ModeResult {
        let tag = match token {
            Token::Tag(t) => t,
            _ => return None,
        };
        let name = tag.name.as_str();
        match name {
            "body" => {
                if self.in_scope("body") {
                    self.mode = InsertionMode::AfterBody;
                }
                None
            }
            "html" => {
                if self.in_scope("body") {
                    reprocess(InsertionMode::AfterBody, token.clone())
                } else {
                    None
                }
            }
            "p" => {
                if !self.close_p_element() {
                    self.parse_error("unexpected-end-tag", Some(name));
                    let phantom = Tag::new_start("p");
                    self.insert_element_html(&phantom, true);
                    self.close_p_element();
                }
                None
            }
            "li" => {
                if !self.has_in_list_item_scope("li") {
                    self.parse_error("unexpected-end-tag", Some(name));
                    return None;
                }
                self.pop_until_any_inclusive(&["li"]);
                None
            }
            "dd" | "dt" => {
                if !self.has_in_definition_scope(name) {
                    self.parse_error("unexpected-end-tag", Some(name));
                    return None;
                }
                self.pop_until_any_inclusive(&["dd", "dt"]);
                None
            }
            "form" => {
                if self.form_element.is_none() {
                    self.parse_error("unexpected-end-tag", Some(name));
                    return None;
                }
                let form = self.form_element.take().unwrap();
                let removed = self.remove_from_open_elements(&form);
                if !removed {
                    self.parse_error("unexpected-end-tag", Some(name));
                }
                None
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if !self.has_any_in_scope(&["h1", "h2", "h3", "h4", "h5", "h6"]) {
                    self.parse_error("unexpected-end-tag", Some(name));
                    return None;
                }
                self.generate_implied_end_tags(None);
                if let Some(top) = self.open_elements.last() {
                    if top.borrow().name != name {
                        self.parse_error("end-tag-too-early", Some(name));
                    }
                }
                while let Some(popped) = self.open_elements.pop() {
                    if HEADING_ELEMENTS.contains(popped.borrow().name.as_str()) {
                        break;
                    }
                }
                None
            }
            "address" | "article" | "aside" | "blockquote" | "button" | "center" |
            "details" | "dialog" | "dir" | "div" | "dl" | "fieldset" | "figcaption" |
            "figure" | "footer" | "header" | "hgroup" | "listing" | "main" | "menu" |
            "nav" | "ol" | "pre" | "search" | "section" | "summary" | "table" | "ul" => {
                if !self.in_scope(name) {
                    self.parse_error("unexpected-end-tag", Some(name));
                    return None;
                }
                self.generate_implied_end_tags(None);
                if let Some(top) = self.open_elements.last() {
                    if top.borrow().name != name {
                        self.parse_error("end-tag-too-early", Some(name));
                    }
                }
                self.pop_until_any_inclusive(&[name]);
                None
            }
            "applet" | "marquee" | "object" => {
                if !self.in_scope(name) {
                    self.parse_error("unexpected-end-tag", Some(name));
                    return None;
                }
                while let Some(popped) = self.open_elements.pop() {
                    if popped.borrow().name == name {
                        break;
                    }
                }
                self.clear_active_formatting_up_to_marker();
                None
            }
            "template" => {
                let has_template = self.open_elements.iter().any(|n| n.borrow().name == "template");
                if !has_template {
                    return None;
                }
                self.generate_implied_end_tags(None);
                self.pop_until_inclusive("template");
                self.clear_active_formatting_up_to_marker();
                if !self.template_modes.is_empty() {
                    self.template_modes.pop();
                }
                self.reset_insertion_mode();
                None
            }
            "br" => {
                self.parse_error_with_token("unexpected-end-tag", Some(name), Some(token));
                let br_tag = Tag::new_start("br");
                self.close_p_element();
                self.reconstruct_active_formatting_elements();
                self.insert_element_html(&br_tag, false);
                self.frameset_ok = false;
                None
            }
            _ => {
                // Formatting elements use adoption agency
                if FORMATTING_ELEMENTS.contains(name) {
                    self.adoption_agency(name);
                    return None;
                }
                // Any other end tag
                self.any_other_end_tag(name);
                None
            }
        }
    }

    fn handle_eof_in_body(&mut self, token: Token) -> ModeResult {
        if !self.template_modes.is_empty() {
            return self.mode_in_template(token);
        }
        for node in &self.open_elements {
            let name = node.borrow().name.clone();
            if !["dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
                 "tbody", "td", "tfoot", "th", "thead", "tr", "body", "html"].contains(&name.as_str()) {
                self.parse_error("expected-closing-tag-but-got-eof", Some(&name));
                break;
            }
        }
        self.mode = InsertionMode::AfterBody;
        reprocess(InsertionMode::AfterBody, token)
    }
}

// ===========================================================================
// Table modes, Select, Template, Frameset, AfterBody, AfterAfterBody
// ===========================================================================

impl TreeBuilder {
    fn mode_in_table(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\0') {
                    self.parse_error("unexpected-null-character", None);
                    data = data.replace('\0', "");
                    if data.is_empty() { return None; }
                }
                self.pending_table_text.clear();
                self.table_text_original_mode = Some(self.mode);
                self.mode = InsertionMode::InTableText;
                let new_token = Token::Characters(CharacterTokens::new(data));
                reprocess(InsertionMode::InTableText, new_token)
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "caption" => {
                            self.clear_stack_until(&["table", "template", "html"]);
                            self.push_formatting_marker();
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InCaption;
                            None
                        }
                        "colgroup" => {
                            self.clear_stack_until(&["table", "template", "html"]);
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InColumnGroup;
                            None
                        }
                        "col" => {
                            self.clear_stack_until(&["table", "template", "html"]);
                            let implied = Tag::new_start("colgroup");
                            self.insert_element_html(&implied, true);
                            self.mode = InsertionMode::InColumnGroup;
                            reprocess(InsertionMode::InColumnGroup, token)
                        }
                        "tbody" | "tfoot" | "thead" => {
                            self.clear_stack_until(&["table", "template", "html"]);
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InTableBody;
                            None
                        }
                        "td" | "th" | "tr" => {
                            self.clear_stack_until(&["table", "template", "html"]);
                            let implied = Tag::new_start("tbody");
                            self.insert_element_html(&implied, true);
                            self.mode = InsertionMode::InTableBody;
                            reprocess(InsertionMode::InTableBody, token)
                        }
                        "table" => {
                            self.parse_error("unexpected-start-tag-implies-end-tag", Some("table"));
                            let closed = self.close_table_element();
                            if closed {
                                reprocess(self.mode, token)
                            } else {
                                None
                            }
                        }
                        "style" | "script" => {
                            self.insert_element_html(tag, true);
                            self.original_mode = Some(self.mode);
                            self.mode = InsertionMode::Text;
                            None
                        }
                        "template" => self.mode_in_head(token),
                        "input" => {
                            let input_type = tag.attrs.get("type").and_then(|v| v.as_ref()).map(|v| v.to_lowercase());
                            if input_type.as_deref() == Some("hidden") {
                                self.parse_error("unexpected-hidden-input-in-table", None);
                                self.insert_element_html(tag, true);
                                self.open_elements.pop();
                                return None;
                            }
                            self.parse_error("unexpected-start-tag-implies-table-voodoo", Some(&tag.name));
                            let prev = self.insert_from_table;
                            self.insert_from_table = true;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                        "form" => {
                            self.parse_error("unexpected-form-in-table", None);
                            if self.form_element.is_none() {
                                let node = self.insert_element_html(tag, true);
                                self.form_element = Some(node);
                                self.open_elements.pop();
                            }
                            None
                        }
                        _ => {
                            self.parse_error("unexpected-start-tag-implies-table-voodoo", Some(&tag.name));
                            let prev = self.insert_from_table;
                            self.insert_from_table = true;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "table" => {
                            self.close_table_element();
                            None
                        }
                        "body" | "caption" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            None
                        }
                        _ => {
                            self.parse_error("unexpected-end-tag-implies-table-voodoo", Some(&tag.name));
                            let prev = self.insert_from_table;
                            self.insert_from_table = true;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                    }
                }
            }
            Token::EOF(_) => {
                if !self.template_modes.is_empty() {
                    return self.mode_in_template(token);
                }
                if self.has_in_table_scope("table") {
                    self.parse_error("expected-closing-tag-but-got-eof", Some("table"));
                }
                None
            }
            _ => None,
        }
    }

    fn mode_in_table_text(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\x0C') {
                    self.parse_error("invalid-codepoint-in-table-text", None);
                    data = data.replace('\x0C', "");
                }
                if !data.is_empty() {
                    self.pending_table_text.push(data);
                }
                None
            }
            _ => {
                self.flush_pending_table_text();
                let original = self.table_text_original_mode.unwrap_or(InsertionMode::InTable);
                self.table_text_original_mode = None;
                self.mode = original;
                reprocess(original, token)
            }
        }
    }

    fn mode_in_caption(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(_) => self.mode_in_body(token),
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr" | "td" | "th" => {
                            self.parse_error("unexpected-start-tag-implies-end-tag", Some(&tag.name));
                            if self.close_caption_element() {
                                reprocess(InsertionMode::InTable, token)
                            } else {
                                None
                            }
                        }
                        "table" => {
                            self.parse_error("unexpected-start-tag-implies-end-tag", Some(&tag.name));
                            if self.close_caption_element() {
                                reprocess(InsertionMode::InTable, token)
                            } else {
                                self.mode_in_body(token)
                            }
                        }
                        _ => self.mode_in_body(token),
                    }
                } else {
                    match tag.name.as_str() {
                        "caption" => {
                            self.close_caption_element();
                            None
                        }
                        "table" => {
                            if self.close_caption_element() {
                                reprocess(InsertionMode::InTable, token)
                            } else {
                                None
                            }
                        }
                        "tbody" | "tfoot" | "thead" => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            None
                        }
                        _ => self.mode_in_body(token),
                    }
                }
            }
            Token::EOF(_) => self.mode_in_body(token),
            _ => None,
        }
    }

    fn mode_in_column_group(&mut self, token: Token) -> ModeResult {
        let current_name = self.open_elements.last().map(|n| n.borrow().name.clone()).unwrap_or_default();
        match &token {
            Token::Characters(ct) => {
                let data = ct.data.clone();
                let stripped = data.trim_start_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C'));
                if stripped.len() < data.len() {
                    let ws = &data[..data.len() - stripped.len()];
                    self.append_text(ws);
                }
                if stripped.is_empty() {
                    return None;
                }
                if current_name == "html" {
                    self.parse_error("unexpected-characters-in-column-group", None);
                    return None;
                }
                if current_name == "template" {
                    self.parse_error("unexpected-characters-in-template-column-group", None);
                    return None;
                }
                self.parse_error("unexpected-characters-in-column-group", None);
                self.pop_current();
                self.mode = InsertionMode::InTable;
                let new_token = Token::Characters(CharacterTokens::new(stripped.to_string()));
                reprocess(InsertionMode::InTable, new_token)
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "html" => self.mode_in_body(token),
                        "col" => {
                            self.insert_element_html(tag, true);
                            self.open_elements.pop();
                            None
                        }
                        "template" => self.mode_in_head(token),
                        _ => {
                            if current_name == "colgroup" {
                                self.pop_current();
                                self.mode = InsertionMode::InTable;
                                reprocess(InsertionMode::InTable, token)
                            } else if current_name == "template" {
                                self.parse_error("unexpected-start-tag-in-template-column-group", Some(&tag.name));
                                None
                            } else {
                                self.parse_error("unexpected-start-tag-in-column-group", Some(&tag.name));
                                None
                            }
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "colgroup" => {
                            if current_name == "colgroup" {
                                self.pop_current();
                                self.mode = InsertionMode::InTable;
                            } else {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                            }
                            None
                        }
                        "col" => {
                            self.parse_error("unexpected-end-tag", Some("col"));
                            None
                        }
                        "template" => self.mode_in_head(token),
                        _ => {
                            if current_name != "html" {
                                self.pop_current();
                                self.mode = InsertionMode::InTable;
                            }
                            reprocess(InsertionMode::InTable, token)
                        }
                    }
                }
            }
            Token::EOF(_) => {
                if current_name == "colgroup" {
                    self.pop_current();
                    self.mode = InsertionMode::InTable;
                    reprocess(InsertionMode::InTable, token)
                } else if current_name == "template" {
                    self.mode_in_template(token)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn mode_in_table_body(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(_) | Token::Comment(_) => self.mode_in_table(token),
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "tr" => {
                            self.clear_stack_until(&["tbody", "tfoot", "thead", "template", "html"]);
                            self.insert_element_html(tag, true);
                            self.mode = InsertionMode::InRow;
                            None
                        }
                        "td" | "th" => {
                            self.parse_error("unexpected-cell-in-table-body", None);
                            self.clear_stack_until(&["tbody", "tfoot", "thead", "template", "html"]);
                            let implied = Tag::new_start("tr");
                            self.insert_element_html(&implied, true);
                            self.mode = InsertionMode::InRow;
                            reprocess(InsertionMode::InRow, token)
                        }
                        "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "table" => {
                            let current_name = self.open_elements.last().map(|n| n.borrow().name.clone()).unwrap_or_default();
                            if current_name == "template" {
                                self.parse_error("unexpected-start-tag-in-template-table-context", Some(&tag.name));
                                return None;
                            }
                            if let Some(ref ctx) = self.fragment_context {
                                if current_name == "html" && ["tbody", "tfoot", "thead"].contains(&ctx.tag_name.to_lowercase().as_str()) {
                                    self.parse_error("unexpected-start-tag", None);
                                    return None;
                                }
                            }
                            if !self.open_elements.is_empty() {
                                self.open_elements.pop();
                                self.mode = InsertionMode::InTable;
                                return reprocess(InsertionMode::InTable, token);
                            }
                            None
                        }
                        _ => self.mode_in_table(token),
                    }
                } else {
                    match tag.name.as_str() {
                        "tbody" | "tfoot" | "thead" => {
                            if !self.has_in_table_scope(&tag.name) {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                                return None;
                            }
                            self.clear_stack_until(&["tbody", "tfoot", "thead", "template", "html"]);
                            self.pop_current();
                            self.mode = InsertionMode::InTable;
                            None
                        }
                        "table" => {
                            let current_name = self.open_elements.last().map(|n| n.borrow().name.clone()).unwrap_or_default();
                            if current_name == "template" {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                                return None;
                            }
                            if let Some(ref ctx) = self.fragment_context {
                                if current_name == "html" && ["tbody", "tfoot", "thead"].contains(&ctx.tag_name.to_lowercase().as_str()) {
                                    self.parse_error("unexpected-end-tag", Some(&tag.name));
                                    return None;
                                }
                            }
                            if ["tbody", "tfoot", "thead"].contains(&current_name.as_str()) {
                                self.open_elements.pop();
                            }
                            self.mode = InsertionMode::InTable;
                            reprocess(InsertionMode::InTable, token)
                        }
                        "caption" | "col" | "colgroup" | "td" | "th" | "tr" => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            None
                        }
                        _ => self.mode_in_table(token),
                    }
                }
            }
            Token::EOF(_) => self.mode_in_table(token),
            _ => None,
        }
    }

    fn mode_in_row(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(_) | Token::Comment(_) => self.mode_in_table(token),
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "td" | "th" => {
                            self.clear_stack_until(&["tr", "template", "html"]);
                            self.insert_element_html(tag, true);
                            self.push_formatting_marker();
                            self.mode = InsertionMode::InCell;
                            None
                        }
                        "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr" | "table" => {
                            if !self.has_in_table_scope("tr") {
                                self.parse_error("unexpected-start-tag-implies-end-tag", Some(&tag.name));
                                return None;
                            }
                            self.end_tr_element();
                            reprocess(self.mode, token)
                        }
                        _ => {
                            let prev = self.insert_from_table;
                            self.insert_from_table = true;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "tr" => {
                            if !self.has_in_table_scope("tr") {
                                self.parse_error("unexpected-end-tag", Some("tr"));
                                return None;
                            }
                            self.end_tr_element();
                            None
                        }
                        "table" | "tbody" | "tfoot" | "thead" => {
                            if self.has_in_table_scope(&tag.name) {
                                self.end_tr_element();
                                reprocess(self.mode, token)
                            } else {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                                None
                            }
                        }
                        "caption" | "col" | "group" | "td" | "th" => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            None
                        }
                        _ => {
                            let prev = self.insert_from_table;
                            self.insert_from_table = true;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                    }
                }
            }
            Token::EOF(_) => self.mode_in_table(token),
            _ => None,
        }
    }

    fn mode_in_cell(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(_) => {
                let prev = self.insert_from_table;
                self.insert_from_table = false;
                let result = self.mode_in_body(token);
                self.insert_from_table = prev;
                result
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => {
                            if self.close_table_cell() {
                                reprocess(self.mode, token)
                            } else {
                                self.parse_error("unexpected-start-tag-in-cell-fragment", Some(&tag.name));
                                None
                            }
                        }
                        _ => {
                            let prev = self.insert_from_table;
                            self.insert_from_table = false;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                    }
                } else {
                    match tag.name.as_str() {
                        "td" | "th" => {
                            if !self.has_in_table_scope(&tag.name) {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                                return None;
                            }
                            self.end_table_cell(&tag.name);
                            None
                        }
                        "table" | "tbody" | "tfoot" | "thead" | "tr" => {
                            if !self.has_in_table_scope(&tag.name) {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                                return None;
                            }
                            self.close_table_cell();
                            reprocess(self.mode, token)
                        }
                        _ => {
                            let prev = self.insert_from_table;
                            self.insert_from_table = false;
                            let result = self.mode_in_body(token);
                            self.insert_from_table = prev;
                            result
                        }
                    }
                }
            }
            Token::EOF(_) => {
                if self.close_table_cell() {
                    reprocess(self.mode, token)
                } else {
                    self.mode_in_table(token)
                }
            }
            _ => None,
        }
    }

    fn mode_in_select(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\0') {
                    self.parse_error("invalid-codepoint-in-select", None);
                    data = data.replace('\0', "");
                }
                if data.contains('\x0C') {
                    self.parse_error("invalid-codepoint-in-select", None);
                    data = data.replace('\x0C', "");
                }
                if !data.is_empty() {
                    self.reconstruct_active_formatting_elements();
                    self.append_text(&data);
                }
                None
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "html" => reprocess(InsertionMode::InBody, token),
                        "option" => {
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "option" { self.open_elements.pop(); }
                            }
                            self.reconstruct_active_formatting_elements();
                            self.insert_element_html(tag, true);
                            None
                        }
                        "optgroup" => {
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "option" { self.open_elements.pop(); }
                            }
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "optgroup" { self.open_elements.pop(); }
                            }
                            self.reconstruct_active_formatting_elements();
                            self.insert_element_html(tag, true);
                            None
                        }
                        "select" => {
                            self.parse_error("unexpected-start-tag-implies-end-tag", Some("select"));
                            self.pop_until_any_inclusive(&["select"]);
                            self.reset_insertion_mode();
                            None
                        }
                        "input" | "textarea" => {
                            self.parse_error("unexpected-start-tag-implies-end-tag", Some(&tag.name));
                            self.pop_until_any_inclusive(&["select"]);
                            self.reset_insertion_mode();
                            reprocess(self.mode, token)
                        }
                        "script" | "template" => self.mode_in_head(token),
                        "hr" => {
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "option" { self.open_elements.pop(); }
                            }
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "optgroup" { self.open_elements.pop(); }
                            }
                            self.reconstruct_active_formatting_elements();
                            self.insert_element_html(tag, false);
                            None
                        }
                        "p" | "div" | "span" | "button" | "datalist" | "selectedcontent" => {
                            self.reconstruct_active_formatting_elements();
                            self.insert_element_html(tag, !tag.self_closing);
                            None
                        }
                        "br" | "img" => {
                            self.reconstruct_active_formatting_elements();
                            self.insert_element_html(tag, false);
                            None
                        }
                        "keygen" => {
                            self.reconstruct_active_formatting_elements();
                            self.insert_element_html(tag, false);
                            None
                        }
                        _ if FORMATTING_ELEMENTS.contains(tag.name.as_str()) => {
                            self.reconstruct_active_formatting_elements();
                            let node = self.insert_element_html(tag, true);
                            self.append_active_formatting_entry(&tag.name, &tag.attrs, &node);
                            None
                        }
                        "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" | "table" => {
                            self.parse_error("unexpected-start-tag-implies-end-tag", Some(&tag.name));
                            self.pop_until_any_inclusive(&["select"]);
                            self.reset_insertion_mode();
                            reprocess(self.mode, token)
                        }
                        _ => None,
                    }
                } else {
                    match tag.name.as_str() {
                        "optgroup" => {
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "option" { self.open_elements.pop(); }
                            }
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "optgroup" {
                                    self.open_elements.pop();
                                } else {
                                    self.parse_error("unexpected-end-tag", Some(&tag.name));
                                }
                            }
                            None
                        }
                        "option" => {
                            if let Some(top) = self.open_elements.last() {
                                if top.borrow().name == "option" {
                                    self.open_elements.pop();
                                } else {
                                    self.parse_error("unexpected-end-tag", Some(&tag.name));
                                }
                            }
                            None
                        }
                        "select" => {
                            self.pop_until_any_inclusive(&["select"]);
                            self.reset_insertion_mode();
                            None
                        }
                        "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" | "table" => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            self.pop_until_any_inclusive(&["select"]);
                            self.reset_insertion_mode();
                            reprocess(self.mode, token)
                        }
                        _ if FORMATTING_ELEMENTS.contains(tag.name.as_str()) || tag.name == "a" => {
                            self.adoption_agency(&tag.name);
                            None
                        }
                        "p" | "div" | "span" | "button" | "datalist" | "selectedcontent" => {
                            // Close element if on stack after select
                            let tag_name = tag.name.clone();
                            let select_idx = self.open_elements.iter().position(|n| n.borrow().name == "select");
                            let target_idx = self.open_elements.iter().rposition(|n| n.borrow().name == tag_name);
                            if let (Some(si), Some(ti)) = (select_idx, target_idx) {
                                if ti > si {
                                    while let Some(popped) = self.open_elements.pop() {
                                        if popped.borrow().name == tag_name { break; }
                                    }
                                } else {
                                    self.parse_error("unexpected-end-tag", Some(&tag.name));
                                }
                            } else {
                                self.parse_error("unexpected-end-tag", Some(&tag.name));
                            }
                            None
                        }
                        _ => {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            None
                        }
                    }
                }
            }
            Token::EOF(_) => self.mode_in_body(token),
            _ => None,
        }
    }

    fn mode_in_template(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(_) | Token::Comment(_) => self.mode_in_body(token),
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "caption" | "colgroup" | "tbody" | "tfoot" | "thead" => {
                            self.template_modes.pop();
                            self.template_modes.push(InsertionMode::InTable);
                            self.mode = InsertionMode::InTable;
                            reprocess(InsertionMode::InTable, token)
                        }
                        "col" => {
                            self.template_modes.pop();
                            self.template_modes.push(InsertionMode::InColumnGroup);
                            self.mode = InsertionMode::InColumnGroup;
                            reprocess(InsertionMode::InColumnGroup, token)
                        }
                        "tr" => {
                            self.template_modes.pop();
                            self.template_modes.push(InsertionMode::InTableBody);
                            self.mode = InsertionMode::InTableBody;
                            reprocess(InsertionMode::InTableBody, token)
                        }
                        "td" | "th" => {
                            self.template_modes.pop();
                            self.template_modes.push(InsertionMode::InRow);
                            self.mode = InsertionMode::InRow;
                            reprocess(InsertionMode::InRow, token)
                        }
                        "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style" | "template" | "title" => {
                            self.mode_in_head(token)
                        }
                        _ => {
                            self.template_modes.pop();
                            self.template_modes.push(InsertionMode::InBody);
                            self.mode = InsertionMode::InBody;
                            reprocess(InsertionMode::InBody, token)
                        }
                    }
                } else if tag.name == "template" {
                    self.mode_in_head(token)
                } else if ["base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style", "template", "title"].contains(&tag.name.as_str()) {
                    self.mode_in_head(token)
                } else {
                    None
                }
            }
            Token::EOF(_) => {
                let has_template = self.open_elements.iter().any(|n| n.borrow().name == "template");
                if !has_template {
                    return None;
                }
                self.parse_error("expected-closing-tag-but-got-eof", Some("template"));
                self.pop_until_inclusive("template");
                self.clear_active_formatting_up_to_marker();
                self.template_modes.pop();
                self.reset_insertion_mode();
                reprocess(self.mode, token)
            }
            _ => None,
        }
    }

    fn mode_after_body(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                if is_all_whitespace(&ct.data) {
                    self.mode_in_body(token);
                    return None;
                }
                reprocess(InsertionMode::InBody, token)
            }
            Token::Comment(ct) => {
                let parent = self.open_elements.first().map(Rc::clone);
                self.append_comment(&ct.data, parent.as_ref());
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start && tag.name == "html" {
                    reprocess(InsertionMode::InBody, token)
                } else if tag.kind == TagKind::End && tag.name == "html" {
                    self.mode = InsertionMode::AfterAfterBody;
                    None
                } else {
                    reprocess(InsertionMode::InBody, token)
                }
            }
            Token::EOF(_) => None,
            _ => None,
        }
    }

    fn mode_after_after_body(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                if is_all_whitespace(&ct.data) {
                    self.mode_in_body(token);
                    return None;
                }
                self.parse_error("unexpected-char-after-body", None);
                reprocess(InsertionMode::InBody, token)
            }
            Token::Comment(ct) => {
                if self.fragment_context.is_some() {
                    if let Some(html) = self.find_last_on_stack("html") {
                        let comment = new_comment(&ct.data);
                        append_child(&html, &comment);
                    }
                    return None;
                }
                self.append_comment_to_document(&ct.data);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start && tag.name == "html" {
                    reprocess(InsertionMode::InBody, token)
                } else {
                    self.parse_error("unexpected-token-after-body", None);
                    reprocess(InsertionMode::InBody, token)
                }
            }
            Token::EOF(_) => None,
            _ => None,
        }
    }

    fn mode_in_frameset(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let whitespace: String = ct.data.chars().filter(|c| matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')).collect();
                if !whitespace.is_empty() {
                    self.append_text(&whitespace);
                }
                None
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    match tag.name.as_str() {
                        "html" => reprocess(InsertionMode::InBody, token),
                        "frameset" => {
                            self.insert_element_html(tag, true);
                            None
                        }
                        "frame" => {
                            self.insert_element_html(tag, true);
                            self.open_elements.pop();
                            None
                        }
                        "noframes" => {
                            self.insert_element_html(tag, true);
                            self.original_mode = Some(self.mode);
                            self.mode = InsertionMode::Text;
                            None
                        }
                        _ => {
                            self.parse_error("unexpected-token-in-frameset", None);
                            None
                        }
                    }
                } else if tag.name == "frameset" {
                    if let Some(top) = self.open_elements.last() {
                        if top.borrow().name == "html" {
                            self.parse_error("unexpected-end-tag", Some(&tag.name));
                            return None;
                        }
                    }
                    self.open_elements.pop();
                    if let Some(top) = self.open_elements.last() {
                        if top.borrow().name != "frameset" {
                            self.mode = InsertionMode::AfterFrameset;
                        }
                    }
                    None
                } else {
                    self.parse_error("unexpected-token-in-frameset", None);
                    None
                }
            }
            Token::EOF(_) => {
                let top_name = self.open_elements.last().map(|t| t.borrow().name.clone());
                if let Some(ref name) = top_name {
                    if name != "html" {
                        self.parse_error("expected-closing-tag-but-got-eof", Some(name));
                    }
                }
                None
            }
            _ => {
                self.parse_error("unexpected-token-in-frameset", None);
                None
            }
        }
    }

    fn mode_after_frameset(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let whitespace: String = ct.data.chars().filter(|c| matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')).collect();
                if !whitespace.is_empty() {
                    self.append_text(&whitespace);
                }
                None
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start && tag.name == "html" {
                    reprocess(InsertionMode::InBody, token)
                } else if tag.kind == TagKind::End && tag.name == "html" {
                    self.mode = InsertionMode::AfterAfterFrameset;
                    None
                } else if tag.kind == TagKind::Start && tag.name == "noframes" {
                    self.insert_element_html(tag, true);
                    self.original_mode = Some(self.mode);
                    self.mode = InsertionMode::Text;
                    None
                } else {
                    self.parse_error("unexpected-token-after-frameset", None);
                    self.mode = InsertionMode::InFrameset;
                    reprocess(InsertionMode::InFrameset, token)
                }
            }
            Token::EOF(_) => None,
            _ => {
                self.parse_error("unexpected-token-after-frameset", None);
                self.mode = InsertionMode::InFrameset;
                reprocess(InsertionMode::InFrameset, token)
            }
        }
    }

    fn mode_after_after_frameset(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                if is_all_whitespace(&ct.data) {
                    self.mode_in_body(token);
                    return None;
                }
                self.parse_error("unexpected-token-after-after-frameset", None);
                self.mode = InsertionMode::InFrameset;
                reprocess(InsertionMode::InFrameset, token)
            }
            Token::Comment(ct) => {
                self.append_comment_to_document(&ct.data);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start && tag.name == "html" {
                    reprocess(InsertionMode::InBody, token)
                } else if tag.kind == TagKind::Start && tag.name == "noframes" {
                    self.insert_element_html(tag, true);
                    self.original_mode = Some(self.mode);
                    self.mode = InsertionMode::Text;
                    None
                } else {
                    self.parse_error("unexpected-token-after-after-frameset", None);
                    self.mode = InsertionMode::InFrameset;
                    reprocess(InsertionMode::InFrameset, token)
                }
            }
            Token::EOF(_) => None,
            _ => {
                self.parse_error("unexpected-token-after-after-frameset", None);
                self.mode = InsertionMode::InFrameset;
                reprocess(InsertionMode::InFrameset, token)
            }
        }
    }
}

// ===========================================================================
// Main entry points: process_token, process_characters, finish, foreign content
// ===========================================================================

impl TreeBuilder {
    fn dispatch_mode(&mut self, mode: InsertionMode, token: Token) -> ModeResult {
        match mode {
            InsertionMode::Initial => self.mode_initial(token),
            InsertionMode::BeforeHtml => self.mode_before_html(token),
            InsertionMode::BeforeHead => self.mode_before_head(token),
            InsertionMode::InHead => self.mode_in_head(token),
            InsertionMode::InHeadNoscript => self.mode_in_head_noscript(token),
            InsertionMode::AfterHead => self.mode_after_head(token),
            InsertionMode::Text => self.mode_text(token),
            InsertionMode::InBody => self.mode_in_body(token),
            InsertionMode::AfterBody => self.mode_after_body(token),
            InsertionMode::AfterAfterBody => self.mode_after_after_body(token),
            InsertionMode::InTable => self.mode_in_table(token),
            InsertionMode::InTableText => self.mode_in_table_text(token),
            InsertionMode::InCaption => self.mode_in_caption(token),
            InsertionMode::InColumnGroup => self.mode_in_column_group(token),
            InsertionMode::InTableBody => self.mode_in_table_body(token),
            InsertionMode::InRow => self.mode_in_row(token),
            InsertionMode::InCell => self.mode_in_cell(token),
            InsertionMode::InFrameset => self.mode_in_frameset(token),
            InsertionMode::AfterFrameset => self.mode_after_frameset(token),
            InsertionMode::AfterAfterFrameset => self.mode_after_after_frameset(token),
            InsertionMode::InSelect => self.mode_in_select(token),
            InsertionMode::InTemplate => self.mode_in_template(token),
        }
    }

    fn handle_foreign_content(&mut self, token: Token) -> ModeResult {
        match &token {
            Token::Characters(ct) => {
                let mut data = ct.data.clone();
                if data.contains('\0') {
                    self.parse_error("invalid-codepoint", None);
                    data = data.replace('\0', "\u{FFFD}");
                }
                if !is_all_whitespace(&data) {
                    self.frameset_ok = false;
                }
                self.reconstruct_active_formatting_elements();
                self.append_text(&data);
                None
            }
            Token::Comment(ct) => {
                self.append_comment(&ct.data, None);
                None
            }
            Token::Tag(tag) => {
                if tag.kind == TagKind::Start {
                    // Check for HTML breakout tags
                    let name_lower = tag.name.to_lowercase();
                    let html_breakout = [
                        "b", "big", "blockquote", "body", "br", "center", "code", "dd", "div",
                        "dl", "dt", "em", "embed", "h1", "h2", "h3", "h4", "h5", "h6",
                        "head", "hr", "i", "img", "li", "listing", "menu", "meta", "nobr",
                        "ol", "p", "pre", "ruby", "s", "small", "span", "strong", "strike",
                        "sub", "sup", "table", "tt", "u", "ul", "var",
                    ];
                    let is_breakout = html_breakout.contains(&name_lower.as_str())
                        || (name_lower == "font" && Self::foreign_breakout_font(tag));

                    if is_breakout {
                        self.parse_error("unexpected-html-element-in-foreign-content", Some(&tag.name));
                        self.pop_until_html_or_integration_point();
                        return reprocess(self.mode, token);
                    }

                    // Insert as foreign element
                    let current = self.open_elements.last().unwrap();
                    let current_ns = current.borrow().namespace.clone();

                    let namespace = match current_ns.as_str() {
                        ns if ns == SVG_NAMESPACE => "svg",
                        ns if ns == MATHML_NAMESPACE => "math",
                        _ => "html",
                    };

                    let adjusted_name = if namespace == "svg" {
                        self.adjust_svg_tag_name(&tag.name)
                    } else {
                        tag.name.clone()
                    };

                    let attrs = self.prepare_foreign_attributes(namespace, &tag.attrs);
                    let new_tag = Tag::new(TagKind::Start, adjusted_name, attrs, tag.self_closing);
                    let ns_url = match namespace {
                        "svg" => SVG_NAMESPACE,
                        "math" => MATHML_NAMESPACE,
                        _ => HTML_NAMESPACE,
                    };
                    self.insert_element(&new_tag, !tag.self_closing, ns_url);
                    None
                } else {
                    // End tag in foreign content
                    let name_lower = tag.name.to_lowercase();

                    // Pop from the end of the stack until we find a matching element or reach HTML
                    let mut i = self.open_elements.len();
                    while i > 0 {
                        i -= 1;
                        let node_name = self.open_elements[i].borrow().name.to_lowercase();
                        let node_ns = self.open_elements[i].borrow().namespace.clone();

                        if node_name == name_lower {
                            self.open_elements.truncate(i);
                            return None;
                        }

                        if node_ns == HTML_NAMESPACE || node_ns.is_empty() {
                            return self.dispatch_mode(self.mode, token);
                        }
                    }
                    None
                }
            }
            _ => None,
        }
    }

    /// Main token processing loop with reprocessing support.
    pub fn do_process_token(&mut self, token: Token) -> TokenSinkResult {
        // Handle DOCTYPE specially
        if matches!(&token, Token::Doctype(_)) {
            return self.handle_doctype(&token);
        }

        // Ignore leading LF after <pre>/<textarea>/<listing>
        let token = if self.ignore_lf {
            if let Token::Characters(ref ct) = token {
                if ct.data.starts_with('\n') {
                    self.ignore_lf = false;
                    let rest = &ct.data[1..];
                    if rest.is_empty() {
                        return TokenSinkResult::Continue;
                    }
                    Token::Characters(CharacterTokens::new(rest.to_string()))
                } else {
                    self.ignore_lf = false;
                    token
                }
            } else {
                self.ignore_lf = false;
                token
            }
        } else {
            token
        };

        // Check foreign content
        let use_foreign = self.should_use_foreign_content(&token);

        let mut result = if use_foreign {
            self.handle_foreign_content(token)
        } else {
            let mode = self.mode;
            self.dispatch_mode(mode, token)
        };

        // Reprocess loop
        let mut safety = 0;
        while let Some(rp) = result {
            safety += 1;
            if safety > 200 {
                break; // Prevent infinite loops
            }
            self.mode = rp.mode;
            if rp.force_html || !self.should_use_foreign_content(&rp.token) {
                result = self.dispatch_mode(rp.mode, rp.token);
            } else {
                result = self.handle_foreign_content(rp.token);
            }
        }

        // Return plaintext override if set
        if let Some(override_result) = self.tokenizer_state_override.take() {
            return override_result;
        }

        TokenSinkResult::Continue
    }

    pub fn do_process_characters(&mut self, data: &str) -> TokenSinkResult {
        // Fast path: if in IN_BODY mode, skip foreign content check
        let token = Token::Characters(CharacterTokens::new(data.to_string()));
        self.do_process_token(token)
    }

    /// Finalize the tree after all tokens have been processed.
    /// Returns the root node (document or document-fragment).
    pub fn finish(&mut self) -> NodeHandle {
        // For fragment parsing, extract children from html wrapper into document-fragment
        if self.fragment_context.is_some() {
            let doc = Rc::clone(&self.document);
            let doc_children: Vec<NodeHandle> = doc.borrow().children.iter().map(Rc::clone).collect();

            // Note: children[0] is always the "html" element created in setup_fragment_parsing
            if let Some(html_node) = doc_children.first() {
                // Handle fragment_context_element: if present and parented under root,
                // reparent its children to root first, then remove the context element
                if let Some(ref ctx_elem) = self.fragment_context_element {
                    let ctx_parent = get_parent(ctx_elem);
                    let is_child_of_root = ctx_parent
                        .as_ref()
                        .map_or(false, |p| Rc::ptr_eq(p, html_node));
                    if is_child_of_root {
                        let ctx_children: Vec<NodeHandle> =
                            ctx_elem.borrow().children.iter().map(Rc::clone).collect();
                        for child in ctx_children {
                            remove_child(ctx_elem, &child);
                            append_child(html_node, &child);
                        }
                        remove_child(html_node, ctx_elem);
                    }
                }

                // Move all of html's children to the document-fragment
                let html_children: Vec<NodeHandle> =
                    html_node.borrow().children.iter().map(Rc::clone).collect();
                for child in html_children {
                    remove_child(html_node, &child);
                    append_child(&doc, &child);
                }
                // Remove the html wrapper from the document
                remove_child(&doc, html_node);
            }
        }

        // Populate selectedcontent for <select> elements
        let root = Rc::clone(&self.document);
        self.populate_selectedcontent(&root);

        Rc::clone(&self.document)
    }
}

// ===========================================================================
// TokenSink implementation
// ===========================================================================

impl TokenSink for TreeBuilder {
    fn process_token(&mut self, token: Token) -> TokenSinkResult {
        self.do_process_token(token)
    }

    fn process_characters(&mut self, data: &str) -> TokenSinkResult {
        self.do_process_characters(data)
    }

    fn set_token_position(&mut self, line: usize, column: usize, buffer: &str) {
        self.last_token_line = Some(line);
        self.last_token_column = Some(column);
        self.buffer = Some(buffer.to_string());
    }
}

// ===========================================================================
// Token::clone implementation (needed for reprocessing)
// ===========================================================================

impl Clone for Token {
    fn clone(&self) -> Self {
        match self {
            Token::Tag(t) => Token::Tag(t.clone()),
            Token::Characters(c) => Token::Characters(c.clone()),
            Token::Comment(c) => Token::Comment(c.clone()),
            Token::Doctype(d) => Token::Doctype(d.clone()),
            Token::EOF(e) => Token::EOF(*e),
        }
    }
}

// ===========================================================================
// Constants used by the tree builder
// ===========================================================================

/// Elements whose text content goes through foster parenting when insert_from_table is set.
const TABLE_FOSTER_TARGETS: &[&str] = &["table", "tbody", "tfoot", "thead", "tr"];

/// Children that are allowed directly in table elements (don't trigger foster parenting).
const TABLE_ALLOWED_CHILDREN: &[&str] = &[
    "caption", "colgroup", "col", "tbody", "tfoot", "thead", "tr", "td", "th",
    "table", "template", "script", "style",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_all_whitespace() {
        assert!(is_all_whitespace(""));
        assert!(is_all_whitespace(" \t\n\r"));
        assert!(!is_all_whitespace("abc"));
        assert!(!is_all_whitespace(" a "));
    }

    #[test]
    fn test_insertion_mode_values() {
        assert_eq!(InsertionMode::Initial as u8, 0);
        assert_eq!(InsertionMode::InTemplate as u8, 21);
    }

    #[test]
    fn test_treebuilder_new() {
        let tb = TreeBuilder::new(None, false, false);
        assert_eq!(tb.mode, InsertionMode::Initial);
        assert!(tb.open_elements.is_empty());
        assert!(tb.errors.is_empty());
        assert_eq!(tb.quirks_mode, "no-quirks");
    }

    #[test]
    fn test_treebuilder_fragment_parsing() {
        let ctx = FragmentContext {
            tag_name: "div".to_string(),
            namespace: None,
        };
        let tb = TreeBuilder::new(Some(ctx), false, false);
        assert_eq!(tb.mode, InsertionMode::InBody);
        assert!(!tb.open_elements.is_empty());
    }

    #[test]
    fn test_treebuilder_fragment_table() {
        let ctx = FragmentContext {
            tag_name: "table".to_string(),
            namespace: None,
        };
        let tb = TreeBuilder::new(Some(ctx), false, false);
        assert_eq!(tb.mode, InsertionMode::InTable);
    }

    #[test]
    fn test_treebuilder_fragment_tr() {
        let ctx = FragmentContext {
            tag_name: "tr".to_string(),
            namespace: None,
        };
        let tb = TreeBuilder::new(Some(ctx), false, false);
        assert_eq!(tb.mode, InsertionMode::InRow);
    }

    #[test]
    fn test_treebuilder_fragment_html_root() {
        let ctx = FragmentContext {
            tag_name: "html".to_string(),
            namespace: None,
        };
        let tb = TreeBuilder::new(Some(ctx), false, false);
        assert_eq!(tb.mode, InsertionMode::BeforeHead);
    }

    #[test]
    fn test_doctype_error_and_quirks_standard() {
        let dt = Doctype {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
            force_quirks: false,
        };
        let (error, quirks) = doctype_error_and_quirks(&dt, false);
        assert!(!error);
        assert_eq!(quirks, "no-quirks");
    }

    #[test]
    fn test_doctype_error_and_quirks_force() {
        let dt = Doctype {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
            force_quirks: true,
        };
        let (_, quirks) = doctype_error_and_quirks(&dt, false);
        assert_eq!(quirks, "quirks");
    }

    #[test]
    fn test_doctype_error_and_quirks_wrong_name() {
        let dt = Doctype {
            name: Some("nothtml".to_string()),
            public_id: None,
            system_id: None,
            force_quirks: false,
        };
        let (error, quirks) = doctype_error_and_quirks(&dt, false);
        assert!(error);
        assert_eq!(quirks, "quirks");
    }

    #[test]
    fn test_doctype_iframe_srcdoc() {
        let dt = Doctype {
            name: Some("html".to_string()),
            public_id: Some("-//wrong//".to_string()),
            system_id: None,
            force_quirks: false,
        };
        let (_, quirks) = doctype_error_and_quirks(&dt, true);
        assert_eq!(quirks, "no-quirks");
    }

    #[test]
    fn test_token_sink_impl() {
        let mut tb = TreeBuilder::new(None, false, false);
        let result = tb.process_token(Token::EOF(EOFToken));
        assert_eq!(result, TokenSinkResult::Continue);
    }

    #[test]
    fn test_simple_html_document() {
        let mut tb = TreeBuilder::new(None, false, true);

        // Process DOCTYPE
        let dt = Doctype {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
            force_quirks: false,
        };
        tb.do_process_token(Token::Doctype(DoctypeToken::new(dt)));

        // Process <html>
        let html_tag = Tag::new_start("html");
        tb.do_process_token(Token::Tag(html_tag));

        // Process <head>
        let head_tag = Tag::new_start("head");
        tb.do_process_token(Token::Tag(head_tag));

        // Process </head>
        let end_head = Tag::new_end("head");
        tb.do_process_token(Token::Tag(end_head));

        // Process <body>
        let body_tag = Tag::new_start("body");
        tb.do_process_token(Token::Tag(body_tag));

        // Process text
        tb.do_process_token(Token::Characters(CharacterTokens::new("Hello".to_string())));

        // Process </body>
        let end_body = Tag::new_end("body");
        tb.do_process_token(Token::Tag(end_body));

        // Process </html>
        let end_html = Tag::new_end("html");
        tb.do_process_token(Token::Tag(end_html));

        // Process EOF
        tb.do_process_token(Token::EOF(EOFToken));

        tb.finish();

        // Check document structure
        let doc = tb.document.borrow();
        // Should have doctype + html
        assert!(doc.children.len() >= 2, "Document should have at least 2 children (doctype + html), has {}", doc.children.len());

        let html = &doc.children[1];
        assert_eq!(html.borrow().name, "html");

        let html_data = html.borrow();
        assert!(html_data.children.len() >= 2, "HTML should have head + body");

        let head = &html_data.children[0];
        assert_eq!(head.borrow().name, "head");

        let body = &html_data.children[1];
        assert_eq!(body.borrow().name, "body");

        let body_data = body.borrow();
        assert!(!body_data.children.is_empty(), "Body should have text child");
        assert_eq!(body_data.children[0].borrow().data.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_implicit_html_body_creation() {
        let mut tb = TreeBuilder::new(None, false, false);

        // Just send text directly - should create html and body implicitly
        tb.do_process_token(Token::Characters(CharacterTokens::new("test".to_string())));
        tb.do_process_token(Token::EOF(EOFToken));
        tb.finish();

        let doc = tb.document.borrow();
        assert!(!doc.children.is_empty());
    }

    #[test]
    fn test_comment_tokens() {
        let mut tb = TreeBuilder::new(None, false, false);

        // Comment in initial mode goes to document
        tb.do_process_token(Token::Comment(CommentToken::new("initial".to_string())));

        let doc = tb.document.borrow();
        assert_eq!(doc.children.len(), 1);
        assert_eq!(doc.children[0].borrow().name, "#comment");
        assert_eq!(doc.children[0].borrow().data.as_deref(), Some("initial"));
    }

    #[test]
    fn test_attrs_signature() {
        let mut attrs = HashMap::new();
        attrs.insert("b".to_string(), Some("2".to_string()));
        attrs.insert("a".to_string(), Some("1".to_string()));
        let sig = TreeBuilder::attrs_signature(&attrs);
        assert_eq!(sig, vec![("a".to_string(), "1".to_string()), ("b".to_string(), "2".to_string())]);
    }
}
