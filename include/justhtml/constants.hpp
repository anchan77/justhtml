#pragma once

/// @file constants.hpp
/// @brief HTML5 specification constants for tree building and tokenization.

#include <array>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

namespace justhtml {

/// Namespace URLs used in foreign content.
namespace ns {
inline constexpr const char* HTML = "http://www.w3.org/1999/xhtml";
inline constexpr const char* MATHML = "http://www.w3.org/1998/Math/MathML";
inline constexpr const char* SVG = "http://www.w3.org/2000/svg";
inline constexpr const char* XLINK = "http://www.w3.org/1999/xlink";
inline constexpr const char* XML = "http://www.w3.org/XML/1998/namespace";
inline constexpr const char* XMLNS = "http://www.w3.org/2000/xmlns/";
}  // namespace ns

namespace constants {

/// Foreign attribute adjustment: (prefix, local_name, namespace_url).
struct ForeignAttr {
    std::optional<std::string> prefix;
    std::string local_name;
    std::string namespace_url;
};

/// Get the foreign attribute adjustments map.
const std::unordered_map<std::string, ForeignAttr>& foreign_attribute_adjustments();

/// Get the MathML attribute case adjustments.
const std::unordered_map<std::string, std::string>& mathml_attribute_adjustments();

/// Get the SVG attribute case adjustments.
const std::unordered_map<std::string, std::string>& svg_attribute_adjustments();

/// Get the SVG tag name case adjustments.
const std::unordered_map<std::string, std::string>& svg_tag_name_adjustments();

/// Namespace URL to prefix map.
const std::unordered_map<std::string, std::string>& namespace_url_to_prefix();

/// Pair of (namespace_prefix, element_name) for integration points.
using NsElement = std::pair<std::string, std::string>;

struct NsElementHash {
    std::size_t operator()(const NsElement& p) const {
        auto h1 = std::hash<std::string>{}(p.first);
        auto h2 = std::hash<std::string>{}(p.second);
        return h1 ^ (h2 << 1);
    }
};

using NsElementSet = std::unordered_set<NsElement, NsElementHash>;

/// HTML integration point elements (namespace_url, element_name pairs).
const NsElementSet& html_integration_point_elements();

/// MathML text integration point elements.
const NsElementSet& mathml_text_integration_point_elements();

/// HTML integration point set (prefix, name pairs).
const NsElementSet& html_integration_point_set();

/// MathML text integration point set (prefix, name pairs).
const NsElementSet& mathml_text_integration_point_set();

/// Quirky public identifier prefixes (lowercase).
const std::vector<std::string>& quirky_public_prefixes();

/// Quirky public identifier exact matches (lowercase).
const std::vector<std::string>& quirky_public_matches();

/// Quirky system identifier exact matches.
const std::vector<std::string>& quirky_system_matches();

/// Limited-quirks public identifier prefixes.
const std::vector<std::string>& limited_quirky_public_prefixes();

/// HTML 4.01 public identifier prefixes.
const std::vector<std::string>& html4_public_prefixes();

/// Heading elements {h1..h6}.
const std::unordered_set<std::string>& heading_elements();

/// Formatting elements.
const std::unordered_set<std::string>& formatting_elements();

/// Special elements.
const std::unordered_set<std::string>& special_elements();

/// Default scope terminators.
const std::unordered_set<std::string>& default_scope_terminators();

/// Button scope terminators (default + button).
const std::unordered_set<std::string>& button_scope_terminators();

/// List item scope terminators (default + ol, ul).
const std::unordered_set<std::string>& list_item_scope_terminators();

/// Definition scope terminators (default + dl).
const std::unordered_set<std::string>& definition_scope_terminators();

/// Table scope terminators.
const std::unordered_set<std::string>& table_scope_terminators();

/// Table foster parenting targets.
const std::unordered_set<std::string>& table_foster_targets();

/// Foreign content breakout elements.
const std::unordered_set<std::string>& foreign_breakout_elements();

/// Table allowed children.
const std::unordered_set<std::string>& table_allowed_children();

/// Implied end tags.
const std::unordered_set<std::string>& implied_end_tags();

/// Void (self-closing) elements.
const std::unordered_set<std::string>& void_elements();

}  // namespace constants
}  // namespace justhtml
