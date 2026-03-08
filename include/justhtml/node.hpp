#pragma once

/// @file node.hpp
/// @brief DOM node types for representing parsed HTML trees.
///
/// Implements SimpleDomNode, ElementNode, TemplateNode, TextNode hierarchy
/// with tree manipulation (append/remove/insert/replace/clone) and
/// serialization helpers (to_text, to_markdown).

#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "justhtml/tokens.hpp"

namespace justhtml {

// Forward declarations
class SimpleDomNode;
class ElementNode;
class TemplateNode;
class TextNode;

/// Type alias for shared node pointer (base type).
using NodePtr = std::shared_ptr<SimpleDomNode>;

// ============================================================================
// MarkdownBuilder - internal helper for to_markdown()
// ============================================================================

/// @brief Internal builder for producing Markdown output.
class MarkdownBuilder {
public:
    MarkdownBuilder();

    void newline(int count = 1);
    void ensure_newlines(int count);
    void raw(const std::string& s);
    void text(const std::string& s, bool preserve_whitespace = false);
    std::string finish() const;

    /// Access the buffer (needed for checking if buffer is empty).
    bool buf_empty() const { return buf_.empty(); }
    int newline_count() const { return newline_count_; }

private:
    void rstrip_last_segment();

    std::vector<std::string> buf_;
    int newline_count_ = 0;
    bool pending_space_ = false;
};

// ============================================================================
// Free functions for markdown helpers
// ============================================================================

/// Escape special Markdown characters in text.
std::string markdown_escape_text(const std::string& s);

/// Produce a backtick-fenced inline code span.
std::string markdown_code_span(const std::string& s);

// ============================================================================
// SimpleDomNode - base node type
// ============================================================================

/// @brief Base DOM node representing elements, text, comments, documents, etc.
class SimpleDomNode : public std::enable_shared_from_this<SimpleDomNode> {
public:
    std::string name;
    std::weak_ptr<SimpleDomNode> parent;
    std::optional<Attributes> attrs;                    // None for comment/doctype
    std::shared_ptr<std::vector<NodePtr>> children;     // nullptr for comment/doctype
    std::optional<std::string> data;                    // text data or doctype info
    std::optional<std::string> namespace_;              // HTML namespace

    // Doctype-specific: when name == "!doctype", data can hold a Doctype struct
    std::optional<Doctype> doctype_data;

    SimpleDomNode(const std::string& name,
                  const std::optional<Attributes>& attrs = std::nullopt,
                  const std::optional<std::string>& data = std::nullopt,
                  const std::optional<std::string>& ns = std::nullopt);

    virtual ~SimpleDomNode() = default;

    /// Append a child node.
    void append_child(const NodePtr& node);

    /// Remove a child node. Throws if not found.
    void remove_child(const NodePtr& node);

    /// Insert node before reference_node. If reference is nullptr, append.
    void insert_before(const NodePtr& node, const NodePtr& reference_node);

    /// Replace old_node with new_node. Returns old_node. Throws if not found.
    NodePtr replace_child(const NodePtr& new_node, const NodePtr& old_node);

    /// Check if this node has children.
    bool has_child_nodes() const;

    /// Get the node's own text value (for text nodes: data; others: "").
    std::string text_property() const;

    /// Get concatenated text of descendants.
    std::string to_text(const std::string& separator = " ", bool strip = true) const;

    /// Convert to Markdown representation.
    virtual std::string to_markdown() const;

    /// Convert to HTML. Delegates to the serialize module.
    std::string to_html(int indent = 0, int indent_size = 2, bool pretty = true) const;

    /// Clone this node (optionally deep).
    virtual NodePtr clone_node(bool deep = false) const;

    /// Get template_content (returns nullptr for non-template nodes).
    virtual NodePtr get_template_content() const { return nullptr; }

    /// Get children as a vector (always returns a valid ref, empty for leaf nodes).
    const std::vector<NodePtr>& get_children() const;
};

// ============================================================================
// ElementNode - element with template_content support
// ============================================================================

/// @brief An HTML element node with optional template_content.
class ElementNode : public SimpleDomNode {
public:
    NodePtr template_content;

    ElementNode(const std::string& name,
                const std::optional<Attributes>& attrs,
                const std::optional<std::string>& ns);

    ~ElementNode() override = default;

    NodePtr clone_node(bool deep = false) const override;
    NodePtr get_template_content() const override { return template_content; }
};

// ============================================================================
// TemplateNode - <template> element
// ============================================================================

/// @brief A <template> element node.
class TemplateNode : public ElementNode {
public:
    TemplateNode(const std::string& name,
                 const std::optional<Attributes>& attrs = std::nullopt,
                 const std::optional<std::string>& data = std::nullopt,
                 const std::optional<std::string>& ns = std::nullopt);

    ~TemplateNode() override = default;

    NodePtr clone_node(bool deep = false) const override;
};

// ============================================================================
// TextNode - text content leaf node
// ============================================================================

/// @brief A text node (leaf, no children).
class TextNode : public SimpleDomNode {
public:
    explicit TextNode(const std::optional<std::string>& data);

    ~TextNode() override = default;

    std::string to_markdown() const override;
    NodePtr clone_node(bool deep = false) const override;
};

// ============================================================================
// Free helper functions
// ============================================================================

/// Walk the tree to produce Markdown output (used internally).
void to_markdown_walk(const NodePtr& node, MarkdownBuilder& builder,
                      bool preserve_whitespace, int list_depth);

/// Collect text from node tree (helper for to_text).
void to_text_collect(const NodePtr& node, std::vector<std::string>& parts, bool strip);

}  // namespace justhtml
