#pragma once

/// @file errors.hpp
/// @brief Centralized error message definitions for HTML parsing errors.

#include <optional>
#include <string>

namespace justhtml {

/// Generate a human-readable error message from an error code.
/// @param code The error code string (kebab-case format).
/// @param tag_name Optional tag name for context in messages.
/// @return Human-readable error message string.
std::string generate_error_message(const std::string& code,
                                   const std::optional<std::string>& tag_name = std::nullopt);

}  // namespace justhtml
