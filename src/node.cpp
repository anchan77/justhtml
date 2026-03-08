/// @file node.cpp
/// @brief Implementation of DOM node types.

#include "justhtml/node.hpp"

#include <algorithm>
#include <cassert>
#include <cctype>
#include <sstream>
#include <stdexcept>
#include <unordered_set>

namespace justhtml {

// ============================================================================
// Markdown helpers
// ============================================================================

std::string markdown_escape_text(const std::string& s) {
    if (s.empty()) return "";
    std::string out;
    out.reserve(s.size() + s.size() / 8);
    for (char ch : s) {
        if (ch == '\\' || ch == '`' || ch == '*' || ch == '_' ||
            ch == '[' || ch == ']') {
            out.push_back('\\');
        }
        out.push_back(ch);
    }
    return out;
}

std::string markdown_code_span(const std::string& s) {
    // Use a backtick fence longer than any run of backticks inside.
    int longest = 0;
    int run = 0;
    for (char ch : s) {
        if (ch == '`') {
            run++;
            if (run > longest) longest = run;
        } else {
            run = 0;
        }
    }
    std::string fence(static_cast<size_t>(longest + 1), '`');
    // CommonMark requires a space if the content starts/ends with backticks.
    bool needs_space = (!s.empty() && s.front() == '`') ||
                       (!s.empty() && s.back() == '`');
    if (needs_space) {
        return fence + " " + s + " " + fence;
    }
    return fence + s + fence;
}

// ============================================================================
// MarkdownBuilder
// ============================================================================

MarkdownBuilder::MarkdownBuilder() = default;

void MarkdownBuilder::rstrip_last_segment() {
    if (buf_.empty()) return;
    auto& last = buf_.back();
    size_t end = last.size();
    while (end > 0 && (last[end - 1] == ' ' || last[end - 1] == '\t')) {
        end--;
    }
    if (end != last.size()) {
        last.resize(end);
    }
}

void MarkdownBuilder::newline(int count) {
    for (int i = 0; i < count; i++) {
        pending_space_ = false;
        rstrip_last_segment();
        buf_.push_back("\n");
        if (newline_count_ < 2) {
            newline_count_++;
        }
    }
}

void MarkdownBuilder::ensure_newlines(int count) {
    while (newline_count_ < count) {
        newline(1);
    }
}

void MarkdownBuilder::raw(const std::string& s) {
    if (s.empty()) return;

    if (pending_space_) {
        char first = s[0];
        if (first != ' ' && first != '\t' && first != '\n' &&
            first != '\r' && first != '\f' && !buf_.empty() && newline_count_ == 0) {
            buf_.push_back(" ");
        }
        pending_space_ = false;
    }

    buf_.push_back(s);
    if (s.find('\n') != std::string::npos) {
        int trailing = 0;
        int i = static_cast<int>(s.size()) - 1;
        while (i >= 0 && s[static_cast<size_t>(i)] == '\n') {
            trailing++;
            i--;
        }
        newline_count_ = std::min(2, trailing);
        if (trailing) {
            pending_space_ = false;
        }
    } else {
        newline_count_ = 0;
    }
}

void MarkdownBuilder::text(const std::string& s, bool preserve_whitespace) {
    if (s.empty()) return;

    if (preserve_whitespace) {
        raw(s);
        return;
    }

    for (char ch : s) {
        if (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == '\f') {
            pending_space_ = true;
            continue;
        }
        if (pending_space_) {
            if (!buf_.empty() && newline_count_ == 0) {
                buf_.push_back(" ");
            }
            pending_space_ = false;
        }
        buf_.push_back(std::string(1, ch));
        newline_count_ = 0;
    }
}

std::string MarkdownBuilder::finish() const {
    std::string out;
    for (const auto& piece : buf_) {
        out += piece;
    }
    // Strip leading and trailing spaces, tabs, and newlines
    size_t start = 0;
    while (start < out.size() && (out[start] == ' ' || out[start] == '\t' || out[start] == '\n')) {
        start++;
    }
    size_t end = out.size();
    while (end > start && (out[end - 1] == ' ' || out[end - 1] == '\t' || out[end - 1] == '\n')) {
        end--;
    }
    return out.substr(start, end - start);
}

// ============================================================================
// Markdown block elements set
// ============================================================================

static const std::unordered_set<std::string>& markdown_block_elements() {
    static const std::unordered_set<std::string> s = {
        "p", "div", "section", "article", "header", "footer", "main",
        "nav", "aside", "blockquote", "pre", "ul", "ol", "li", "hr",
        "h1", "h2", "h3", "h4", "h5", "h6", "table"
    };
    return s;
}

// Empty children vector (returned by reference for leaf nodes)
static const std::vector<NodePtr>& empty_children() {
    static const std::vector<NodePtr> v;
    return v;
}

// ============================================================================
// to_text_collect helper
// ============================================================================

void to_text_collect(const NodePtr& node, std::vector<std::string>& parts, bool strip) {
    if (node->name == "#text") {
        if (!node->data.has_value() || node->data->empty()) return;
        std::string d = node->data.value();
        if (strip) {
            // Trim
            size_t start = d.find_first_not_of(" \t\n\r\f");
            if (start == std::string::npos) return;
            size_t end = d.find_last_not_of(" \t\n\r\f");
            d = d.substr(start, end - start + 1);
            if (d.empty()) return;
        }
        parts.push_back(std::move(d));
        return;
    }

    if (node->children) {
        for (const auto& child : *node->children) {
            to_text_collect(child, parts, strip);
        }
    }

    // Check template_content
    auto tc = node->get_template_content();
    if (tc) {
        to_text_collect(tc, parts, strip);
    }
}

// ============================================================================
// to_markdown_walk
// ============================================================================

// Forward declaration for to_html (we'll implement the serialize module separately)
// For now, node.to_html() will use a simple placeholder; the real impl is in serialize.cpp
// But we need it for markdown walk (img, table).

void to_markdown_walk(const NodePtr& node, MarkdownBuilder& builder,
                      bool preserve_whitespace, int list_depth) {
    const std::string& name = node->name;

    if (name == "#text") {
        if (preserve_whitespace) {
            builder.raw(node->data.value_or(""));
        } else {
            builder.text(markdown_escape_text(node->data.value_or("")), false);
        }
        return;
    }

    if (name == "br") {
        builder.newline(1);
        return;
    }

    // Comments/doctype don't contribute.
    if (name == "#comment" || name == "!doctype") {
        return;
    }

    // Document containers contribute via descendants.
    if (!name.empty() && name[0] == '#') {
        if (node->children) {
            for (const auto& child : *node->children) {
                to_markdown_walk(child, builder, preserve_whitespace, list_depth);
            }
        }
        return;
    }

    // Lowercase the tag name
    std::string tag = name;
    for (auto& ch : tag) {
        if ('A' <= ch && ch <= 'Z') ch = static_cast<char>(ch + 32);
    }

    // Preserve <img> and <table> as HTML.
    if (tag == "img") {
        builder.raw(node->to_html(0, 2, false));
        return;
    }

    if (tag == "table") {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        builder.raw(node->to_html(0, 2, false));
        builder.ensure_newlines(2);
        return;
    }

    // Headings.
    if (tag.size() == 2 && tag[0] == 'h' && tag[1] >= '1' && tag[1] <= '6') {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        int level = tag[1] - '0';
        builder.raw(std::string(static_cast<size_t>(level), '#'));
        builder.raw(" ");
        if (node->children) {
            for (const auto& child : *node->children) {
                to_markdown_walk(child, builder, false, list_depth);
            }
        }
        builder.ensure_newlines(2);
        return;
    }

    // Horizontal rule.
    if (tag == "hr") {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        builder.raw("---");
        builder.ensure_newlines(2);
        return;
    }

    // Code blocks.
    if (tag == "pre") {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        std::string code = node->to_text("", false);
        builder.raw("```");
        builder.newline(1);
        if (!code.empty()) {
            // rstrip trailing newlines
            size_t end = code.size();
            while (end > 0 && code[end - 1] == '\n') end--;
            builder.raw(code.substr(0, end));
            builder.newline(1);
        }
        builder.raw("```");
        builder.ensure_newlines(2);
        return;
    }

    // Inline code.
    if (tag == "code" && !preserve_whitespace) {
        std::string code = node->to_text("", false);
        builder.raw(markdown_code_span(code));
        return;
    }

    // Paragraph-like blocks.
    if (tag == "p") {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        if (node->children) {
            for (const auto& child : *node->children) {
                to_markdown_walk(child, builder, false, list_depth);
            }
        }
        builder.ensure_newlines(2);
        return;
    }

    // Blockquotes.
    if (tag == "blockquote") {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        MarkdownBuilder inner;
        if (node->children) {
            for (const auto& child : *node->children) {
                to_markdown_walk(child, inner, false, list_depth);
            }
        }
        std::string text = inner.finish();
        if (!text.empty()) {
            // Split by newlines and prefix with "> "
            size_t pos = 0;
            bool first = true;
            while (pos <= text.size()) {
                size_t nl = text.find('\n', pos);
                if (nl == std::string::npos) nl = text.size();
                if (!first) {
                    builder.newline(1);
                }
                builder.raw("> ");
                builder.raw(text.substr(pos, nl - pos));
                pos = nl + 1;
                first = false;
            }
        }
        builder.ensure_newlines(2);
        return;
    }

    // Lists.
    if (tag == "ul" || tag == "ol") {
        builder.ensure_newlines(builder.buf_empty() ? 0 : 2);
        bool ordered = (tag == "ol");
        int idx = 1;
        const auto& ch_list = node->get_children();
        for (const auto& child : ch_list) {
            std::string child_name = child->name;
            for (auto& c : child_name) {
                if ('A' <= c && c <= 'Z') c = static_cast<char>(c + 32);
            }
            if (child_name != "li") continue;
            if (idx > 1) {
                builder.newline(1);
            }
            std::string indent_str(static_cast<size_t>(list_depth) * 2, ' ');
            std::string marker = ordered
                ? std::to_string(idx) + ". "
                : "- ";
            builder.raw(indent_str);
            builder.raw(marker);
            // Render list item content inline-ish.
            for (const auto& li_child : child->get_children()) {
                to_markdown_walk(li_child, builder, false, list_depth + 1);
            }
            idx++;
        }
        builder.ensure_newlines(2);
        return;
    }

    // Emphasis/strong.
    if (tag == "em" || tag == "i") {
        builder.raw("*");
        for (const auto& child : node->get_children()) {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.raw("*");
        return;
    }

    if (tag == "strong" || tag == "b") {
        builder.raw("**");
        for (const auto& child : node->get_children()) {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.raw("**");
        return;
    }

    // Links.
    if (tag == "a") {
        std::string href;
        if (node->attrs.has_value()) {
            auto it = node->attrs->find("href");
            if (it != node->attrs->end() && it->second.has_value()) {
                href = it->second.value();
            }
        }
        builder.raw("[");
        for (const auto& child : node->get_children()) {
            to_markdown_walk(child, builder, false, list_depth);
        }
        builder.raw("]");
        if (!href.empty()) {
            builder.raw("(");
            builder.raw(href);
            builder.raw(")");
        }
        return;
    }

    // Containers / unknown tags: recurse into children.
    bool next_preserve = preserve_whitespace ||
                         (tag == "textarea" || tag == "script" || tag == "style");
    if (node->children) {
        for (const auto& child : *node->children) {
            to_markdown_walk(child, builder, next_preserve, list_depth);
        }
    }

    // Check template_content for ElementNode
    auto tc = node->get_template_content();
    if (tc) {
        to_markdown_walk(tc, builder, next_preserve, list_depth);
    }

    // Add spacing after block containers to keep output readable.
    if (markdown_block_elements().count(tag)) {
        builder.ensure_newlines(2);
    }
}

// ============================================================================
// SimpleDomNode
// ============================================================================

SimpleDomNode::SimpleDomNode(const std::string& name_,
                             const std::optional<Attributes>& attrs_,
                             const std::optional<std::string>& data_,
                             const std::optional<std::string>& ns_)
    : name(name_), data(data_)
{
    if (name_.size() > 0 && name_[0] == '#') {
        // Special node: #text, #comment, #document, #document-fragment
        namespace_ = ns_;
        if (name_ == "#comment" || name_ == "!doctype") {
            children = nullptr;
            attrs = std::nullopt;
        } else {
            children = std::make_shared<std::vector<NodePtr>>();
            attrs = attrs_.has_value() ? attrs_.value() : Attributes{};
        }
    } else if (name_ == "!doctype") {
        namespace_ = ns_;
        children = nullptr;
        attrs = std::nullopt;
    } else {
        // Regular element
        namespace_ = ns_.has_value() ? ns_ : std::optional<std::string>("html");
        children = std::make_shared<std::vector<NodePtr>>();
        attrs = attrs_.has_value() ? attrs_.value() : Attributes{};
    }
}

void SimpleDomNode::append_child(const NodePtr& node) {
    if (children) {
        children->push_back(node);
        node->parent = shared_from_this();
    }
}

void SimpleDomNode::remove_child(const NodePtr& node) {
    if (!children) {
        // Comment nodes have no children - noop for Python compat
        // But if it's an element node with children, we should throw if not found
        return;
    }
    auto it = std::find(children->begin(), children->end(), node);
    if (it == children->end()) {
        throw std::runtime_error("Node not found in children");
    }
    children->erase(it);
    node->parent.reset();
}

void SimpleDomNode::insert_before(const NodePtr& node, const NodePtr& reference_node) {
    if (!children) {
        throw std::runtime_error("Node " + name + " cannot have children");
    }
    if (!reference_node) {
        append_child(node);
        return;
    }
    auto it = std::find(children->begin(), children->end(), reference_node);
    if (it == children->end()) {
        throw std::runtime_error("Reference node is not a child of this node");
    }
    children->insert(it, node);
    node->parent = shared_from_this();
}

NodePtr SimpleDomNode::replace_child(const NodePtr& new_node, const NodePtr& old_node) {
    if (!children) {
        throw std::runtime_error("Node " + name + " cannot have children");
    }
    auto it = std::find(children->begin(), children->end(), old_node);
    if (it == children->end()) {
        throw std::runtime_error("The node to be replaced is not a child of this node");
    }
    *it = new_node;
    new_node->parent = shared_from_this();
    old_node->parent.reset();
    return old_node;
}

bool SimpleDomNode::has_child_nodes() const {
    return children && !children->empty();
}

std::string SimpleDomNode::text_property() const {
    if (name == "#text") {
        return data.value_or("");
    }
    return "";
}

std::string SimpleDomNode::to_text(const std::string& separator, bool strip) const {
    // For text nodes, handle directly
    if (name == "#text") {
        if (!data.has_value()) return "";
        if (strip) {
            std::string d = data.value();
            size_t start = d.find_first_not_of(" \t\n\r\f");
            if (start == std::string::npos) return "";
            size_t end = d.find_last_not_of(" \t\n\r\f");
            return d.substr(start, end - start + 1);
        }
        return data.value();
    }

    std::vector<std::string> parts;
    // We need a shared_ptr to this node for the collect function.
    // Since we're called on an existing node, we use const_cast + shared_from_this
    // But we can just pass this as a raw pointer through a helper
    auto self = std::const_pointer_cast<SimpleDomNode>(
        const_cast<SimpleDomNode*>(this)->shared_from_this());
    to_text_collect(self, parts, strip);
    if (parts.empty()) return "";
    std::string result;
    for (size_t i = 0; i < parts.size(); i++) {
        if (i > 0) result += separator;
        result += parts[i];
    }
    return result;
}

std::string SimpleDomNode::to_markdown() const {
    MarkdownBuilder builder;
    auto self = std::const_pointer_cast<SimpleDomNode>(
        const_cast<SimpleDomNode*>(this)->shared_from_this());
    to_markdown_walk(self, builder, false, 0);
    return builder.finish();
}

// Forward-declare the serialize function (implemented in serialize.cpp)
std::string serialize_to_html(const NodePtr& node, int indent, int indent_size, bool pretty);

std::string SimpleDomNode::to_html(int indent, int indent_size, bool pretty) const {
    auto self = std::const_pointer_cast<SimpleDomNode>(
        const_cast<SimpleDomNode*>(this)->shared_from_this());
    return serialize_to_html(self, indent, indent_size, pretty);
}

NodePtr SimpleDomNode::clone_node(bool deep) const {
    auto clone = std::make_shared<SimpleDomNode>(
        name,
        attrs,
        data,
        namespace_
    );
    // Copy doctype_data if present
    clone->doctype_data = doctype_data;
    if (deep && children) {
        for (const auto& child : *children) {
            clone->append_child(child->clone_node(true));
        }
    }
    return clone;
}

const std::vector<NodePtr>& SimpleDomNode::get_children() const {
    if (children) return *children;
    return empty_children();
}

// ============================================================================
// ElementNode
// ============================================================================

ElementNode::ElementNode(const std::string& name_,
                         const std::optional<Attributes>& attrs_,
                         const std::optional<std::string>& ns_)
    : SimpleDomNode(name_, attrs_, std::nullopt, ns_)
{
    // ElementNode always has children and attrs
    children = std::make_shared<std::vector<NodePtr>>();
    attrs = attrs_.has_value() ? attrs_.value() : Attributes{};
    template_content = nullptr;
}

NodePtr ElementNode::clone_node(bool deep) const {
    auto clone = std::make_shared<ElementNode>(
        name,
        attrs,
        namespace_
    );
    if (deep) {
        if (children) {
            for (const auto& child : *children) {
                clone->append_child(child->clone_node(true));
            }
        }
    }
    return clone;
}

// ============================================================================
// TemplateNode
// ============================================================================

TemplateNode::TemplateNode(const std::string& name_,
                           const std::optional<Attributes>& attrs_,
                           const std::optional<std::string>& /* data_ */,
                           const std::optional<std::string>& ns_)
    : ElementNode(name_, attrs_, ns_)
{
    if (namespace_.has_value() && namespace_.value() == "html") {
        template_content = std::make_shared<SimpleDomNode>("#document-fragment");
    } else {
        template_content = nullptr;
    }
}

NodePtr TemplateNode::clone_node(bool deep) const {
    auto clone = std::make_shared<TemplateNode>(
        name,
        attrs,
        std::nullopt,
        namespace_
    );
    if (deep) {
        if (template_content) {
            clone->template_content = template_content->clone_node(true);
        }
        if (children) {
            for (const auto& child : *children) {
                clone->append_child(child->clone_node(true));
            }
        }
    }
    return clone;
}

// ============================================================================
// TextNode
// ============================================================================

TextNode::TextNode(const std::optional<std::string>& data_)
    : SimpleDomNode("#text", std::nullopt, data_, std::nullopt)
{
    // TextNode is a leaf - override children to nullptr
    children = nullptr;
    attrs = std::nullopt;
}

std::string TextNode::to_markdown() const {
    MarkdownBuilder builder;
    builder.text(markdown_escape_text(data.value_or("")), false);
    return builder.finish();
}

NodePtr TextNode::clone_node(bool /* deep */) const {
    return std::make_shared<TextNode>(data);
}

}  // namespace justhtml
