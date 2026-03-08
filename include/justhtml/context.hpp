#pragma once

/// @file context.hpp
/// @brief Fragment parsing context for HTML fragment parsing.

#include <optional>
#include <string>

namespace justhtml {

/// Context for fragment parsing, specifying the context element.
struct FragmentContext {
    std::string tag_name;
    std::optional<std::string> ns;  // "namespace" is a C++ keyword

    FragmentContext() = default;
    explicit FragmentContext(std::string tag_name, std::optional<std::string> ns = std::nullopt)
        : tag_name(std::move(tag_name)), ns(std::move(ns)) {}
};

}  // namespace justhtml
