#pragma once

/// @file serialize.hpp
/// @brief HTML serialization utilities for DOM nodes.
///
/// Provides to_html, to_test_format, and related helpers for converting
/// DOM nodes back to HTML strings or html5lib-compatible test format.

#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "justhtml/tokens.hpp"

namespace justhtml {

// Forward declaration
class SimpleDomNode;
using NodePtr = std::shared_ptr<SimpleDomNode>;

/// Escape text content (< > &).
std::string escape_text(const std::optional<std::string>& text);

/// Choose the appropriate quote character for an attribute value.
char choose_attr_quote(const std::optional<std::string>& value);

/// Escape an attribute value for the given quote character.
std::string escape_attr_value(const std::optional<std::string>& value, char quote_char);

/// Check if an attribute value can be left unquoted.
bool can_unquote_attr_value(const std::optional<std::string>& value);

/// Serialize an opening tag with attributes.
std::string serialize_start_tag(const std::string& name,
                                const std::optional<Attributes>& attrs);

/// Serialize a closing tag.
std::string serialize_end_tag(const std::string& name);

/// Convert a node tree to HTML string.
/// @param node The root node to serialize.
/// @param indent Current indentation level.
/// @param indent_size Number of spaces per indentation level.
/// @param pretty Whether to pretty-print.
std::string to_html(const NodePtr& node, int indent = 0,
                    int indent_size = 2, bool pretty = true);

/// Convert a node to html5lib-compatible test format.
/// @param node The node to format.
/// @param indent Current indentation level.
std::string to_test_format(const NodePtr& node, int indent = 0);

}  // namespace justhtml
