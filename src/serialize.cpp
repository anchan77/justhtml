/// @file serialize.cpp
/// @brief Implementation of HTML serialization utilities.

#include "justhtml/serialize.hpp"
#include "justhtml/constants.hpp"
#include "justhtml/node.hpp"

#include <algorithm>
#include <sstream>
#include <unordered_set>

namespace justhtml {

// ============================================================================
// Helper sets
// ============================================================================

static const std::unordered_set<std::string>& preformatted_elements() {
    static const std::unordered_set<std::string> s = {"pre", "textarea"};
    return s;
}

// ============================================================================
// Escape and quote helpers
// ============================================================================

std::string escape_text(const std::optional<std::string>& text) {
    if (!text.has_value() || text->empty()) return "";
    std::string result;
    result.reserve(text->size());
    for (char ch : *text) {
        switch (ch) {
            case '&': result += "&amp;"; break;
            case '<': result += "&lt;"; break;
            case '>': result += "&gt;"; break;
            default: result += ch; break;
        }
    }
    return result;
}

char choose_attr_quote(const std::optional<std::string>& value) {
    if (!value.has_value()) return '"';
    const auto& v = *value;
    if (v.find('"') != std::string::npos && v.find('\'') == std::string::npos) {
        return '\'';
    }
    return '"';
}

std::string escape_attr_value(const std::optional<std::string>& value, char quote_char) {
    if (!value.has_value()) return "";
    std::string result;
    result.reserve(value->size());
    for (char ch : *value) {
        if (ch == '&') {
            result += "&amp;";
        } else if (quote_char == '"' && ch == '"') {
            result += "&quot;";
        } else if (quote_char == '\'' && ch == '\'') {
            result += "&#39;";
        } else {
            result += ch;
        }
    }
    return result;
}

bool can_unquote_attr_value(const std::optional<std::string>& value) {
    if (!value.has_value()) return false;
    const auto& v = *value;
    if (v.empty()) return false;
    for (char ch : v) {
        if (ch == '>') return false;
        if (ch == '"' || ch == '\'' || ch == '=') return false;
        if (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\f' || ch == '\r') return false;
    }
    return true;
}

std::string serialize_start_tag(const std::string& name,
                                const std::optional<Attributes>& attrs) {
    std::string result = "<" + name;
    if (attrs.has_value()) {
        for (const auto& [key, value] : *attrs) {
            if (!value.has_value() || value->empty()) {
                result += " " + key;
            } else {
                if (can_unquote_attr_value(value)) {
                    std::string escaped;
                    for (char ch : *value) {
                        if (ch == '&') escaped += "&amp;";
                        else escaped += ch;
                    }
                    result += " " + key + "=" + escaped;
                } else {
                    char quote = choose_attr_quote(value);
                    std::string escaped = escape_attr_value(value, quote);
                    result += " " + key + "=" + quote + escaped + quote;
                }
            }
        }
    }
    result += ">";
    return result;
}

std::string serialize_end_tag(const std::string& name) {
    return "</" + name + ">";
}

// ============================================================================
// Internal helpers
// ============================================================================

static bool is_whitespace_text_node(const NodePtr& node) {
    if (node->name != "#text") return false;
    const auto& d = node->data;
    if (!d.has_value()) return true;
    for (char ch : *d) {
        if (ch != ' ' && ch != '\t' && ch != '\n' && ch != '\r' && ch != '\f') {
            return false;
        }
    }
    return true;
}

static bool should_pretty_indent_children(const std::vector<NodePtr>& children) {
    bool has_comment = false;
    bool has_non_whitespace_text = false;
    for (const auto& child : children) {
        if (child->name == "#comment") {
            has_comment = true;
            break;
        }
        if (child->name == "#text") {
            if (child->data.has_value()) {
                for (char ch : *child->data) {
                    if (ch != ' ' && ch != '\t' && ch != '\n' && ch != '\r' && ch != '\f') {
                        has_non_whitespace_text = true;
                        break;
                    }
                }
            }
            if (has_non_whitespace_text) break;
        }
    }

    if (has_comment || has_non_whitespace_text) return false;

    for (const auto& child : children) {
        if (child->name == "#text" || child->name == "#comment") continue;
        if (constants::special_elements().count(child->name) == 0) {
            return false;
        }
    }
    return true;
}

static std::string node_to_html(const NodePtr& node, int indent, int indent_size,
                                bool pretty, bool in_pre);

static std::string node_to_html(const NodePtr& node, int indent, int indent_size,
                                bool pretty, bool in_pre) {
    std::string prefix = (pretty && !in_pre) ? std::string(static_cast<size_t>(indent * indent_size), ' ') : "";
    const std::string& name = node->name;
    bool content_pre = in_pre || preformatted_elements().count(name) > 0;
    std::string newline_str = (pretty && !content_pre) ? "\n" : "";

    // Text node
    if (name == "#text") {
        if (pretty && !in_pre) {
            std::string text = node->data.value_or("");
            // Trim
            size_t start = text.find_first_not_of(" \t\n\r\f");
            if (start == std::string::npos) return "";
            size_t end = text.find_last_not_of(" \t\n\r\f");
            text = text.substr(start, end - start + 1);
            if (text.empty()) return "";
            return prefix + escape_text(text);
        }
        return node->data.has_value() ? escape_text(node->data) : "";
    }

    // Comment node
    if (name == "#comment") {
        return prefix + "<!--" + node->data.value_or("") + "-->";
    }

    // Doctype
    if (name == "!doctype") {
        return prefix + "<!DOCTYPE html>";
    }

    // Document fragment
    if (name == "#document-fragment") {
        std::vector<std::string> parts;
        const auto& ch = node->get_children();
        for (const auto& child : ch) {
            std::string child_html = node_to_html(child, indent, indent_size, pretty, in_pre);
            if (!child_html.empty()) {
                parts.push_back(child_html);
            }
        }
        if (pretty) {
            std::string result;
            for (size_t i = 0; i < parts.size(); i++) {
                if (i > 0) result += newline_str;
                result += parts[i];
            }
            return result;
        }
        std::string result;
        for (const auto& p : parts) result += p;
        return result;
    }

    // Element node
    std::string open_tag = serialize_start_tag(name, node->attrs);

    // Void elements
    if (constants::void_elements().count(name) > 0) {
        return prefix + open_tag;
    }

    // Elements with children
    const auto& children = node->get_children();
    if (children.empty()) {
        return prefix + open_tag + serialize_end_tag(name);
    }

    // Check if all children are text-only
    bool all_text = true;
    for (const auto& c : children) {
        if (c->name != "#text") { all_text = false; break; }
    }

    if (all_text && pretty && !content_pre) {
        return prefix + open_tag + escape_text(node->to_text("", false)) + serialize_end_tag(name);
    }

    if (pretty && content_pre) {
        std::string inner;
        for (const auto& child : children) {
            inner += node_to_html(child, indent + 1, indent_size, pretty, true);
        }
        return prefix + open_tag + inner + serialize_end_tag(name);
    }

    if (pretty && !content_pre && !should_pretty_indent_children(children)) {
        std::string inner;
        for (const auto& child : children) {
            inner += node_to_html(child, 0, indent_size, false, content_pre);
        }
        return prefix + open_tag + inner + serialize_end_tag(name);
    }

    // Render with child indentation
    std::vector<std::string> parts;
    parts.push_back(prefix + open_tag);
    for (const auto& child : children) {
        if (pretty && !content_pre && is_whitespace_text_node(child)) {
            continue;
        }
        std::string child_html = node_to_html(child, indent + 1, indent_size, pretty, content_pre);
        if (!child_html.empty()) {
            parts.push_back(child_html);
        }
    }
    parts.push_back(prefix + serialize_end_tag(name));

    if (pretty) {
        std::string result;
        for (size_t i = 0; i < parts.size(); i++) {
            if (i > 0) result += "\n";
            result += parts[i];
        }
        return result;
    }
    std::string result;
    for (const auto& p : parts) result += p;
    return result;
}

// ============================================================================
// Public API
// ============================================================================

std::string to_html(const NodePtr& node, int indent, int indent_size, bool pretty) {
    if (node->name == "#document") {
        std::vector<std::string> parts;
        for (const auto& child : node->get_children()) {
            parts.push_back(node_to_html(child, indent, indent_size, pretty, false));
        }
        if (pretty) {
            std::string result;
            for (size_t i = 0; i < parts.size(); i++) {
                if (i > 0) result += "\n";
                result += parts[i];
            }
            return result;
        }
        std::string result;
        for (const auto& p : parts) result += p;
        return result;
    }
    return node_to_html(node, indent, indent_size, pretty, false);
}

// serialize_to_html is the function called by node.to_html() via forward declaration
std::string serialize_to_html(const NodePtr& node, int indent, int indent_size, bool pretty) {
    return to_html(node, indent, indent_size, pretty);
}

// ============================================================================
// Test format helpers
// ============================================================================

static std::string qualified_name(const NodePtr& node) {
    if (node->namespace_.has_value() && node->namespace_.value() != "html") {
        return node->namespace_.value() + " " + node->name;
    }
    return node->name;
}

static const std::unordered_map<std::string, std::string>& foreign_attr_adjustments() {
    // Map of attribute names that need namespace prefix in test format
    static const std::unordered_map<std::string, std::string> m = {
        {"xlink:actuate", "xlink:actuate"},
        {"xlink:arcrole", "xlink:arcrole"},
        {"xlink:href", "xlink:href"},
        {"xlink:role", "xlink:role"},
        {"xlink:show", "xlink:show"},
        {"xlink:title", "xlink:title"},
        {"xlink:type", "xlink:type"},
        {"xml:lang", "xml:lang"},
        {"xml:space", "xml:space"},
        {"xmlns", "xmlns"},
        {"xmlns:xlink", "xmlns:xlink"},
    };
    return m;
}

static std::vector<std::string> attrs_to_test_format(const NodePtr& node, int indent) {
    if (!node->attrs.has_value() || node->attrs->empty()) return {};

    std::vector<std::string> formatted;
    std::string padding(static_cast<size_t>(indent + 2), ' ');

    // Prepare display names for sorting
    std::vector<std::pair<std::string, std::string>> display_attrs;
    for (const auto& [attr_name, attr_value] : *node->attrs) {
        std::string value = attr_value.value_or("");
        std::string display_name = attr_name;
        if (node->namespace_.has_value() && node->namespace_.value() != "html") {
            std::string lower_name = attr_name;
            for (auto& ch : lower_name) {
                if ('A' <= ch && ch <= 'Z') ch = static_cast<char>(ch + 32);
            }
            if (foreign_attr_adjustments().count(lower_name)) {
                // Replace ':' with ' ' for test format
                display_name = attr_name;
                for (auto& ch : display_name) {
                    if (ch == ':') ch = ' ';
                }
            }
        }
        display_attrs.emplace_back(display_name, value);
    }

    // Sort by display name
    std::sort(display_attrs.begin(), display_attrs.end(),
              [](const auto& a, const auto& b) { return a.first < b.first; });

    for (const auto& [display_name, value] : display_attrs) {
        formatted.push_back("| " + padding + display_name + "=\"" + value + "\"");
    }
    return formatted;
}

static std::string doctype_to_test_format(const NodePtr& node) {
    std::string result = "| <!DOCTYPE";

    // Get doctype data - it's stored in doctype_data field
    if (node->doctype_data.has_value()) {
        const auto& dt = *node->doctype_data;
        std::string name = dt.name.value_or("");
        if (!name.empty()) {
            result += " " + name;
        } else {
            result += " ";
        }

        if (dt.public_id.has_value() || dt.system_id.has_value()) {
            std::string pub = dt.public_id.value_or("");
            std::string sys = dt.system_id.value_or("");
            result += " \"" + pub + "\"";
            result += " \"" + sys + "\"";
        }
    } else if (node->data.has_value()) {
        // Fallback: if it's stored as simple string data
        result += " " + node->data.value();
    } else {
        result += " ";
    }

    result += ">";
    return result;
}

static std::string node_to_test_format(const NodePtr& node, int indent);

static std::string node_to_test_format(const NodePtr& node, int indent) {
    if (node->name == "#comment") {
        std::string comment = node->data.value_or("");
        return "| " + std::string(static_cast<size_t>(indent), ' ') + "<!-- " + comment + " -->";
    }

    if (node->name == "!doctype") {
        return doctype_to_test_format(node);
    }

    if (node->name == "#text") {
        std::string text = node->data.value_or("");
        return "| " + std::string(static_cast<size_t>(indent), ' ') + "\"" + text + "\"";
    }

    // Regular element
    std::string line = "| " + std::string(static_cast<size_t>(indent), ' ') + "<" + qualified_name(node) + ">";
    auto attribute_lines = attrs_to_test_format(node, indent);

    // Template special handling
    auto tc = node->get_template_content();
    if (node->name == "template" &&
        (!node->namespace_.has_value() || node->namespace_.value() == "html") &&
        tc) {
        std::vector<std::string> sections = {line};
        for (const auto& al : attribute_lines) {
            sections.push_back(al);
        }
        std::string content_line = "| " + std::string(static_cast<size_t>(indent + 2), ' ') + "content";
        sections.push_back(content_line);
        for (const auto& child : tc->get_children()) {
            sections.push_back(node_to_test_format(child, indent + 4));
        }
        std::string result;
        for (size_t i = 0; i < sections.size(); i++) {
            if (i > 0) result += "\n";
            result += sections[i];
        }
        return result;
    }

    // Regular element with children
    std::vector<std::string> child_lines;
    for (const auto& child : node->get_children()) {
        child_lines.push_back(node_to_test_format(child, indent + 2));
    }

    std::vector<std::string> sections = {line};
    for (const auto& al : attribute_lines) {
        sections.push_back(al);
    }
    for (const auto& cl : child_lines) {
        sections.push_back(cl);
    }

    std::string result;
    for (size_t i = 0; i < sections.size(); i++) {
        if (i > 0) result += "\n";
        result += sections[i];
    }
    return result;
}

std::string to_test_format(const NodePtr& node, int indent) {
    if (node->name == "#document" || node->name == "#document-fragment") {
        std::vector<std::string> parts;
        for (const auto& child : node->get_children()) {
            parts.push_back(node_to_test_format(child, 0));
        }
        std::string result;
        for (size_t i = 0; i < parts.size(); i++) {
            if (i > 0) result += "\n";
            result += parts[i];
        }
        return result;
    }
    return node_to_test_format(node, indent);
}

}  // namespace justhtml
