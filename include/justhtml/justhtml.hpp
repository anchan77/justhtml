#pragma once

/// @file justhtml.hpp
/// @brief Main convenience header for the justhtml library.

#include <string>
#include <string_view>

namespace justhtml {

/// Library version string.
constexpr const char* version() noexcept {
    return "0.1.0";
}

} // namespace justhtml
