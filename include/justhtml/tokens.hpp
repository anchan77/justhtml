#pragma once

/// @file tokens.hpp
/// @brief Token types emitted by the HTML5 tokenizer.

#include <map>
#include <optional>
#include <string>
#include <variant>

namespace justhtml {

/// Tag kind: start tag or end tag.
enum class TagKind : int {
    Start = 0,
    End = 1,
};

/// An ordered map of attribute name -> optional value.
/// Uses std::map to preserve insertion order consistent with the Python dict behavior.
using Attributes = std::map<std::string, std::optional<std::string>>;

/// Represents an HTML start or end tag token.
struct Tag {
    TagKind kind;
    std::string name;
    Attributes attrs;
    bool self_closing = false;

    Tag() : kind(TagKind::Start), self_closing(false) {}
    Tag(TagKind kind, std::string name, Attributes attrs = {}, bool self_closing = false)
        : kind(kind), name(std::move(name)), attrs(std::move(attrs)), self_closing(self_closing) {}
};

/// Represents a run of character data.
struct CharacterTokens {
    std::string data;

    CharacterTokens() = default;
    explicit CharacterTokens(std::string data) : data(std::move(data)) {}
};

/// Represents an HTML comment.
struct CommentToken {
    std::string data;

    CommentToken() = default;
    explicit CommentToken(std::string data) : data(std::move(data)) {}
};

/// Represents a DOCTYPE declaration.
struct Doctype {
    std::optional<std::string> name;
    std::optional<std::string> public_id;
    std::optional<std::string> system_id;
    bool force_quirks = false;

    Doctype() = default;
    Doctype(std::optional<std::string> name, std::optional<std::string> public_id = std::nullopt,
            std::optional<std::string> system_id = std::nullopt, bool force_quirks = false)
        : name(std::move(name)),
          public_id(std::move(public_id)),
          system_id(std::move(system_id)),
          force_quirks(force_quirks) {}
};

/// Wrapper for a DOCTYPE token (mirrors Python's DoctypeToken).
struct DoctypeToken {
    Doctype doctype;

    DoctypeToken() = default;
    explicit DoctypeToken(Doctype doctype) : doctype(std::move(doctype)) {}
};

/// End-of-file sentinel token.
struct EOFToken {};

/// Result codes for token sink processing.
enum class TokenSinkResult : int {
    Continue = 0,
    Plaintext = 1,
};

/// A variant holding any token type.
using Token = std::variant<Tag, CharacterTokens, CommentToken, DoctypeToken, EOFToken>;

/// Represents a parse error with location information.
struct ParseError {
    std::string code;
    std::optional<int> line;
    std::optional<int> column;
    std::string message;
    std::optional<std::string> source_html;
    std::optional<int> end_column;

    ParseError() = default;
    ParseError(std::string code, std::optional<int> line = std::nullopt,
               std::optional<int> column = std::nullopt, std::string message = "",
               std::optional<std::string> source_html = std::nullopt,
               std::optional<int> end_column = std::nullopt)
        : code(std::move(code)),
          line(line),
          column(column),
          message(message.empty() ? this->code : std::move(message)),
          source_html(std::move(source_html)),
          end_column(end_column) {}

    /// String representation (for display).
    std::string to_string() const;

    /// Detailed repr-style string.
    std::string repr() const;

    /// Equality: compares code, line, column.
    bool operator==(const ParseError& other) const {
        return code == other.code && line == other.line && column == other.column;
    }
    bool operator!=(const ParseError& other) const { return !(*this == other); }
};

}  // namespace justhtml
