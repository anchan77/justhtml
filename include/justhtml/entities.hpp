#pragma once

/// @file entities.hpp
/// @brief HTML5 character entity (reference) decoding.
///
/// Implements HTML5 character reference decoding per WHATWG spec section 13.2.5.
/// Supports named entities (&amp;, &nbsp;), decimal (&#60;), and hex (&#x3C;).

#include <string>
#include <string_view>

namespace justhtml {

/// Decode a numeric character reference.
/// @param text The numeric part (without &# prefix or ; suffix).
/// @param is_hex Whether this is hexadecimal (&#x) or decimal (&#).
/// @return The decoded UTF-8 character(s).
std::string decode_numeric_entity(std::string_view text, bool is_hex = false);

/// Decode all HTML entities in text.
/// @param text Input text potentially containing entities.
/// @param in_attribute Whether this is an attribute value (stricter rules for legacy entities).
/// @return Text with entities decoded.
std::string decode_entities_in_text(std::string_view text, bool in_attribute = false);

/// Look up a named entity (without semicolon).
/// @param name Entity name (e.g., "amp", "nbsp").
/// @return The decoded value, or empty string if not found.
std::string lookup_named_entity(const std::string& name);

/// Check if an entity name is a legacy entity (can be used without semicolon).
/// @param name Entity name.
/// @return true if it's a legacy entity.
bool is_legacy_entity(const std::string& name);

}  // namespace justhtml
