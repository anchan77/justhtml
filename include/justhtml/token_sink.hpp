#pragma once

/// @file token_sink.hpp
/// @brief Abstract interface for consuming tokens from the tokenizer.

#include "justhtml/tokens.hpp"

namespace justhtml {

/// Abstract interface for receiving tokens from the tokenizer.
/// Implemented by the tree builder and the stream sink.
class TokenSink {
public:
    virtual ~TokenSink() = default;

    /// Process a token (Tag, CommentToken, DoctypeToken, EOFToken).
    /// Returns a TokenSinkResult to control tokenizer behavior.
    virtual TokenSinkResult process_token(const Token& token) = 0;

    /// Process character data.
    virtual void process_characters(const std::string& data) = 0;
};

}  // namespace justhtml
