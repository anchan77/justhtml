//! DOM node model for HTML5 tree construction.
//!
//! Provides a tree of nodes representing the parsed HTML document.
//! Corresponds to Python's `node.py`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};

/// A handle to a DOM node (shared ownership).
pub type NodeHandle = Rc<RefCell<NodeData>>;
/// A weak reference to a DOM node (for parent pointers).
pub type WeakNodeHandle = Weak<RefCell<NodeData>>;

/// The kind of a DOM node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// Document root node.
    Document,
    /// Element node (e.g., `<div>`, `<p>`).
    Element,
    /// Text node.
    Text,
    /// Comment node.
    Comment,
    /// DOCTYPE node.
    Doctype,
    /// Template element (has an associated document fragment for content).
    Template,
}

/// Internal data for a DOM node.
#[derive(Debug, Clone)]
pub struct NodeData {
    pub kind: NodeKind,
    pub name: String,
    pub namespace: String,
    pub attrs: HashMap<String, Option<String>>,
    pub data: Option<String>,
    pub children: Vec<NodeHandle>,
    pub parent: Option<WeakNodeHandle>,
    /// For Template nodes: the document fragment that holds template content.
    pub template_content: Option<NodeHandle>,
    /// For Doctype nodes: stores the Doctype data.
    pub doctype_data: Option<DoctypeData>,
}

/// DOCTYPE-specific data.
#[derive(Debug, Clone)]
pub struct DoctypeData {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
}

/// Create a new document node.
pub fn new_document() -> NodeHandle {
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Document,
        name: "#document".to_string(),
        namespace: "http://www.w3.org/1999/xhtml".to_string(),
        attrs: HashMap::new(),
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }))
}

/// Create a new element node.
pub fn new_element(name: &str, namespace: &str, attrs: HashMap<String, Option<String>>) -> NodeHandle {
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Element,
        name: name.to_string(),
        namespace: namespace.to_string(),
        attrs,
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }))
}

/// Create a new template element node with its own document fragment for content.
pub fn new_template(namespace: &str, attrs: HashMap<String, Option<String>>) -> NodeHandle {
    let content = Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Document,
        name: "#document-fragment".to_string(),
        namespace: namespace.to_string(),
        attrs: HashMap::new(),
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }));
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Template,
        name: "template".to_string(),
        namespace: namespace.to_string(),
        attrs,
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: Some(content),
        doctype_data: None,
    }))
}

/// Create a new text node.
pub fn new_text(data: &str) -> NodeHandle {
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Text,
        name: "#text".to_string(),
        namespace: String::new(),
        attrs: HashMap::new(),
        data: Some(data.to_string()),
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }))
}

/// Create a new comment node.
pub fn new_comment(data: &str) -> NodeHandle {
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Comment,
        name: "#comment".to_string(),
        namespace: String::new(),
        attrs: HashMap::new(),
        data: Some(data.to_string()),
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }))
}

/// Create a new doctype node.
pub fn new_doctype(doctype_data: DoctypeData) -> NodeHandle {
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Doctype,
        name: "!doctype".to_string(),
        namespace: String::new(),
        attrs: HashMap::new(),
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: Some(doctype_data),
    }))
}

/// Append a child node to a parent. Sets the child's parent pointer.
pub fn append_child(parent: &NodeHandle, child: &NodeHandle) {
    // Remove from old parent if any
    {
        let child_data = child.borrow();
        if let Some(ref old_parent_weak) = child_data.parent {
            if let Some(old_parent) = old_parent_weak.upgrade() {
                if !Rc::ptr_eq(&old_parent, parent) {
                    drop(child_data);
                    remove_child_internal(&old_parent, child);
                } else {
                    drop(child_data);
                    // Already a child of this parent - remove first to re-append
                    remove_child_internal(parent, child);
                }
            }
        }
    }
    child.borrow_mut().parent = Some(Rc::downgrade(parent));
    parent.borrow_mut().children.push(Rc::clone(child));
}

/// Remove a child node from a parent. Clears the child's parent pointer.
pub fn remove_child(parent: &NodeHandle, child: &NodeHandle) {
    remove_child_internal(parent, child);
    child.borrow_mut().parent = None;
}

fn remove_child_internal(parent: &NodeHandle, child: &NodeHandle) {
    let mut parent_data = parent.borrow_mut();
    parent_data.children.retain(|c| !Rc::ptr_eq(c, child));
}

/// Insert a child node before a reference node in the parent's children.
pub fn insert_before(parent: &NodeHandle, child: &NodeHandle, reference: &NodeHandle) {
    // Remove from old parent
    {
        let child_data = child.borrow();
        if let Some(ref old_parent_weak) = child_data.parent {
            if let Some(old_parent) = old_parent_weak.upgrade() {
                drop(child_data);
                remove_child_internal(&old_parent, child);
            }
        }
    }
    child.borrow_mut().parent = Some(Rc::downgrade(parent));
    let mut parent_data = parent.borrow_mut();
    let pos = parent_data
        .children
        .iter()
        .position(|c| Rc::ptr_eq(c, reference));
    match pos {
        Some(idx) => parent_data.children.insert(idx, Rc::clone(child)),
        None => parent_data.children.push(Rc::clone(child)),
    }
}

/// Insert a child node at a specific index in the parent's children.
pub fn insert_at(parent: &NodeHandle, index: usize, child: &NodeHandle) {
    // Remove from old parent
    {
        let child_data = child.borrow();
        if let Some(ref old_parent_weak) = child_data.parent {
            if let Some(old_parent) = old_parent_weak.upgrade() {
                drop(child_data);
                remove_child_internal(&old_parent, child);
            }
        }
    }
    child.borrow_mut().parent = Some(Rc::downgrade(parent));
    let mut parent_data = parent.borrow_mut();
    let idx = index.min(parent_data.children.len());
    parent_data.children.insert(idx, Rc::clone(child));
}

/// Check if a node has any child nodes.
pub fn has_child_nodes(node: &NodeHandle) -> bool {
    !node.borrow().children.is_empty()
}

/// Get the number of children.
pub fn child_count(node: &NodeHandle) -> usize {
    node.borrow().children.len()
}

/// Get the parent of a node (if any).
pub fn get_parent(node: &NodeHandle) -> Option<NodeHandle> {
    node.borrow()
        .parent
        .as_ref()
        .and_then(|w| w.upgrade())
}

/// Check if two node handles refer to the same node.
pub fn same_node(a: &NodeHandle, b: &NodeHandle) -> bool {
    Rc::ptr_eq(a, b)
}

// ---------------------------------------------------------------------------
// High-level node API functions
// ---------------------------------------------------------------------------

/// Return the node's own text value.
///
/// For text nodes this is the node data. For other nodes this is an empty string.
/// Use `node_to_text()` to get textContent semantics.
pub fn text_content(node: &NodeHandle) -> String {
    let data = node.borrow();
    if data.kind == NodeKind::Text {
        data.data.as_deref().unwrap_or("").to_string()
    } else {
        String::new()
    }
}

/// Internal helper: recursively collect text from a node tree.
fn to_text_collect(node: &NodeHandle, parts: &mut Vec<String>, strip: bool) {
    let data = node.borrow();

    if data.kind == NodeKind::Text {
        let text = data.data.as_deref().unwrap_or("");
        if text.is_empty() {
            return;
        }
        if strip {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }
            parts.push(trimmed.to_string());
        } else {
            parts.push(text.to_string());
        }
        return;
    }

    let children: Vec<NodeHandle> = data.children.iter().cloned().collect();
    let template_content = data.template_content.clone();
    drop(data);

    for child in &children {
        to_text_collect(child, parts, strip);
    }

    if let Some(ref tc) = template_content {
        to_text_collect(tc, parts, strip);
    }
}

/// Return the concatenated text of a node's descendants.
///
/// * `separator` - Controls how text nodes are joined (default: a single space).
/// * `strip` - If true, strips each text node and drops empty segments.
///
/// Template element contents are included via `template_content`.
pub fn node_to_text(node: &NodeHandle, separator: &str, strip: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    to_text_collect(node, &mut parts, strip);
    if parts.is_empty() {
        return String::new();
    }
    parts.join(separator)
}

/// Convert a node to an HTML string.
///
/// Delegates to `serialize::to_html`.
pub fn node_to_html(node: &NodeHandle, indent: usize, indent_size: usize, pretty: bool) -> String {
    crate::serialize::to_html(node, indent, indent_size, pretty)
}

/// Convert a node to a Markdown representation.
///
/// Delegates to `markdown::to_markdown`.
pub fn node_to_markdown(node: &NodeHandle) -> String {
    crate::markdown::to_markdown(node)
}

/// Query a subtree using a CSS selector.
///
/// This is a stub implementation that will be completed in Milestone 4
/// when the selector engine is implemented.
pub fn node_query(_node: &NodeHandle, _selector: &str) -> Vec<NodeHandle> {
    // TODO: Implement in Milestone 4 with the selector engine
    Vec::new()
}

/// Create a new document-fragment node.
pub fn new_document_fragment() -> NodeHandle {
    Rc::new(RefCell::new(NodeData {
        kind: NodeKind::Document,
        name: "#document-fragment".to_string(),
        namespace: String::new(),
        attrs: HashMap::new(),
        data: None,
        children: Vec::new(),
        parent: None,
        template_content: None,
        doctype_data: None,
    }))
}

impl fmt::Display for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            NodeKind::Document => write!(f, "#document"),
            NodeKind::Element | NodeKind::Template => {
                if self.namespace != "http://www.w3.org/1999/xhtml" && !self.namespace.is_empty() {
                    write!(f, "<{} {}>", self.namespace, self.name)
                } else {
                    write!(f, "<{}>", self.name)
                }
            }
            NodeKind::Text => write!(f, "\"{}\"", self.data.as_deref().unwrap_or("")),
            NodeKind::Comment => write!(f, "<!-- {} -->", self.data.as_deref().unwrap_or("")),
            NodeKind::Doctype => write!(f, "<!DOCTYPE {}>", self.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document() {
        let doc = new_document();
        let data = doc.borrow();
        assert_eq!(data.kind, NodeKind::Document);
        assert_eq!(data.name, "#document");
        assert!(data.children.is_empty());
    }

    #[test]
    fn test_new_element() {
        let mut attrs = HashMap::new();
        attrs.insert("class".to_string(), Some("test".to_string()));
        let elem = new_element("div", "http://www.w3.org/1999/xhtml", attrs);
        let data = elem.borrow();
        assert_eq!(data.kind, NodeKind::Element);
        assert_eq!(data.name, "div");
        assert_eq!(data.attrs.get("class"), Some(&Some("test".to_string())));
    }

    #[test]
    fn test_new_template() {
        let tmpl = new_template("http://www.w3.org/1999/xhtml", HashMap::new());
        let data = tmpl.borrow();
        assert_eq!(data.kind, NodeKind::Template);
        assert_eq!(data.name, "template");
        assert!(data.template_content.is_some());
        let content = data.template_content.as_ref().unwrap();
        assert_eq!(content.borrow().name, "#document-fragment");
    }

    #[test]
    fn test_new_text() {
        let text = new_text("hello");
        let data = text.borrow();
        assert_eq!(data.kind, NodeKind::Text);
        assert_eq!(data.data.as_deref(), Some("hello"));
    }

    #[test]
    fn test_new_comment() {
        let comment = new_comment("test comment");
        let data = comment.borrow();
        assert_eq!(data.kind, NodeKind::Comment);
        assert_eq!(data.data.as_deref(), Some("test comment"));
    }

    #[test]
    fn test_append_child() {
        let parent = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&parent, &child);
        assert_eq!(parent.borrow().children.len(), 1);
        assert!(same_node(&parent.borrow().children[0], &child));
        assert!(get_parent(&child).is_some());
    }

    #[test]
    fn test_remove_child() {
        let parent = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&parent, &child);
        remove_child(&parent, &child);
        assert!(parent.borrow().children.is_empty());
        assert!(get_parent(&child).is_none());
    }

    #[test]
    fn test_insert_before() {
        let parent = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child1 = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child2 = new_element("p", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&parent, &child1);
        insert_before(&parent, &child2, &child1);
        assert_eq!(parent.borrow().children.len(), 2);
        assert!(same_node(&parent.borrow().children[0], &child2));
        assert!(same_node(&parent.borrow().children[1], &child1));
    }

    #[test]
    fn test_insert_at() {
        let parent = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child1 = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child2 = new_element("p", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&parent, &child1);
        insert_at(&parent, 0, &child2);
        assert_eq!(parent.borrow().children.len(), 2);
        assert!(same_node(&parent.borrow().children[0], &child2));
    }

    #[test]
    fn test_has_child_nodes() {
        let parent = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        assert!(!has_child_nodes(&parent));
        let child = new_text("hello");
        append_child(&parent, &child);
        assert!(has_child_nodes(&parent));
    }

    #[test]
    fn test_same_node() {
        let node1 = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let node2 = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        assert!(same_node(&node1, &node1));
        assert!(!same_node(&node1, &node2));
    }

    #[test]
    fn test_display() {
        let doc = new_document();
        assert_eq!(format!("{}", doc.borrow()), "#document");

        let elem = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        assert_eq!(format!("{}", elem.borrow()), "<div>");

        let svg = new_element("rect", "http://www.w3.org/2000/svg", HashMap::new());
        assert_eq!(format!("{}", svg.borrow()), "<http://www.w3.org/2000/svg rect>");

        let text = new_text("hello");
        assert_eq!(format!("{}", text.borrow()), "\"hello\"");

        let comment = new_comment("test");
        assert_eq!(format!("{}", comment.borrow()), "<!-- test -->");
    }

    #[test]
    fn test_reparenting() {
        let parent1 = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let parent2 = new_element("section", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        
        append_child(&parent1, &child);
        assert_eq!(parent1.borrow().children.len(), 1);
        
        // Reparent to parent2
        append_child(&parent2, &child);
        assert_eq!(parent1.borrow().children.len(), 0);
        assert_eq!(parent2.borrow().children.len(), 1);
        assert!(same_node(&get_parent(&child).unwrap(), &parent2));
    }

    #[test]
    fn test_doctype_node() {
        let dt = new_doctype(DoctypeData {
            name: Some("html".to_string()),
            public_id: None,
            system_id: None,
        });
        let data = dt.borrow();
        assert_eq!(data.kind, NodeKind::Doctype);
        assert!(data.doctype_data.is_some());
        let dd = data.doctype_data.as_ref().unwrap();
        assert_eq!(dd.name.as_deref(), Some("html"));
    }

    // -----------------------------------------------------------------------
    // text_content tests (ported from test_node.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_text_content_text_node() {
        let node = new_text("Hi");
        assert_eq!(text_content(&node), "Hi");
    }

    #[test]
    fn test_text_content_element_is_empty() {
        let node = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        assert_eq!(text_content(&node), "");
    }

    #[test]
    fn test_text_content_comment_is_empty() {
        let node = new_comment("comment");
        assert_eq!(text_content(&node), "");
    }

    #[test]
    fn test_text_content_none_data() {
        // TextNode with empty data
        let node = new_text("");
        assert_eq!(text_content(&node), "");
    }

    // -----------------------------------------------------------------------
    // node_to_text tests (ported from test_node.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_text_simple() {
        let node = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let text = new_text("Hello");
        append_child(&node, &text);
        assert_eq!(text_content(&node), "");
        assert_eq!(text_content(&text), "Hello");
        assert_eq!(node_to_text(&node, " ", true), "Hello");
    }

    #[test]
    fn test_to_text_nested() {
        let root = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let span = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        let text1 = new_text("Hello ");
        let text2 = new_text("World");
        append_child(&root, &text1);
        append_child(&root, &span);
        append_child(&span, &text2);

        assert_eq!(text_content(&root), "");
        assert_eq!(text_content(&span), "");
        assert_eq!(node_to_text(&root, " ", true), "Hello World");
        assert_eq!(node_to_text(&span, " ", true), "World");
    }

    #[test]
    fn test_to_text_matches_textcontent() {
        let root = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let span = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&root, &new_text("Hello "));
        append_child(&root, &span);
        append_child(&span, &new_text("World"));

        assert_eq!(node_to_text(&root, " ", true), "Hello World");
        assert_eq!(node_to_text(&span, " ", true), "World");
        assert_eq!(node_to_text(&root, "", false), "Hello World");
        assert_eq!(node_to_text(&root, "", true), "HelloWorld");
    }

    #[test]
    fn test_to_text_skips_empty_and_whitespace_segments_by_default() {
        let root = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&root, &new_text(""));
        append_child(&root, &new_text("   "));
        append_child(&root, &new_text("A"));
        assert_eq!(node_to_text(&root, " ", true), "A");
    }

    #[test]
    fn test_to_text_empty_subtree() {
        let root = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        assert_eq!(node_to_text(&root, " ", true), "");
    }

    #[test]
    fn test_textnode_to_text_strip_false() {
        let t = new_text("  A  ");
        assert_eq!(node_to_text(&t, " ", false), "  A  ");
        assert_eq!(node_to_text(&t, " ", true), "A");
    }

    #[test]
    fn test_to_text_includes_template_content() {
        let template = new_template("http://www.w3.org/1999/xhtml", HashMap::new());
        {
            let data = template.borrow();
            let tc = data.template_content.as_ref().unwrap();
            append_child(tc, &new_text("Inside"));
        }
        assert_eq!(text_content(&template), "");
        assert_eq!(node_to_text(&template, " ", true), "Inside");
    }

    #[test]
    fn test_to_text_text_node_branch() {
        let node = new_text("Hi");
        assert_eq!(node_to_text(&node, " ", true), "Hi");
    }

    // -----------------------------------------------------------------------
    // node_to_html tests (ported from test_node.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_html_method() {
        let node = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let output = node_to_html(&node, 0, 2, true);
        assert!(output.contains("<div>"));
    }

    // -----------------------------------------------------------------------
    // node_query stub test (ported from test_node.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_query_stub() {
        let parent = new_element("div", "http://www.w3.org/1999/xhtml", HashMap::new());
        let child = new_element("span", "http://www.w3.org/1999/xhtml", HashMap::new());
        append_child(&parent, &child);
        // Stub returns empty for now (actual implementation in Milestone 4)
        let results = node_query(&parent, "span");
        assert!(results.is_empty());
    }

    // -----------------------------------------------------------------------
    // node_to_markdown tests (ported from test_node.py)
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_markdown_method() {
        let t = new_text("a*b");
        assert_eq!(node_to_markdown(&t), "a\\*b");
    }

    #[test]
    fn test_to_markdown_empty_textnode() {
        let t = new_text("");
        assert_eq!(node_to_markdown(&t), "");
    }

    // -----------------------------------------------------------------------
    // new_document_fragment tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_new_document_fragment() {
        let frag = new_document_fragment();
        let data = frag.borrow();
        assert_eq!(data.name, "#document-fragment");
        assert!(data.children.is_empty());
    }
}
