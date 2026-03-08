/// @file tokens.cpp
/// @brief Implementation of ParseError formatting methods.

#include "justhtml/tokens.hpp"

#include <sstream>

namespace justhtml {

std::string ParseError::to_string() const {
    std::ostringstream oss;
    if (line.has_value() && column.has_value()) {
        oss << "(" << *line << "," << *column << "): " << code;
        if (message != code) {
            oss << " - " << message;
        }
    } else {
        oss << code;
        if (message != code) {
            oss << " - " << message;
        }
    }
    return oss.str();
}

std::string ParseError::repr() const {
    std::ostringstream oss;
    oss << "ParseError(\"" << code << "\"";
    if (line.has_value() && column.has_value()) {
        oss << ", line=" << *line << ", column=" << *column;
    }
    oss << ")";
    return oss.str();
}

}  // namespace justhtml
