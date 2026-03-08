#pragma once

/// @file stream.hpp
/// @brief SAX-style streaming API for HTML tokenization.
///
/// Provides a stream() function that tokenizes HTML and yields events
/// (start, end, text, comment, doctype) without building a DOM tree.

#include <optional>
#include <string>
#include <utility>
#include <variant>
#include <vector>

#include "justhtml/token_sink.hpp"
#include "justhtml/tokens.hpp"

namespace justhtml {

/// Event types for the streaming API.
enum class StreamEventType {
    Start,
    End,
    Text,
    Comment,
    Doctype,
};

/// Data for a start tag event: (name, attrs).
struct StartTagData {
    std::string name;
    Attributes attrs;
};

/// Data for a doctype event: (name, public_id, system_id).
struct DoctypeData {
    std::optional<std::string> name;
    std::optional<std::string> public_id;
    std::optional<std::string> system_id;
};

/// A stream event: (event_type, data).
struct StreamEvent {
    StreamEventType type;
    /// For Start: StartTagData; For End: string (tag name);
    /// For Text/Comment: string (data); For Doctype: DoctypeData.
    std::variant<std::string, StartTagData, DoctypeData> data;
};

/// @brief Token sink that buffers tokens for the streaming API.
class StreamSink : public TokenSink {
public:
    std::vector<StreamEvent> tokens;

    StreamSink() = default;

    TokenSinkResult process_token(const Token& token) override;
    void process_characters(const std::string& data) override;

private:
    // Dummy struct for namespace tracking (needed by tokenizer's rawtext checks)
    int open_elements_depth_ = 0;
};

/// Stream HTML events from the given HTML string.
/// @param html Input HTML string (UTF-8).
/// @return Vector of stream events.
std::vector<StreamEvent> stream(const std::string& html);

/// Stream HTML events from bytes with optional encoding.
/// @param data Raw bytes.
/// @param encoding Optional transport encoding.
/// @return Vector of stream events.
std::vector<StreamEvent> stream_bytes(const std::vector<uint8_t>& data,
                                       const std::optional<std::string>& encoding = std::nullopt);

}  // namespace justhtml
