/// @file stream.cpp
/// @brief Implementation of the SAX-style streaming API.

#include "justhtml/stream.hpp"
#include "justhtml/encoding.hpp"
#include "justhtml/tokenizer.hpp"

namespace justhtml {

// ============================================================================
// StreamSink
// ============================================================================

TokenSinkResult StreamSink::process_token(const Token& token) {
    if (std::holds_alternative<Tag>(token)) {
        const auto& tag = std::get<Tag>(token);
        if (tag.kind == TagKind::Start) {
            StreamEvent event;
            event.type = StreamEventType::Start;
            event.data = StartTagData{tag.name, tag.attrs};
            tokens.push_back(std::move(event));
            open_elements_depth_++;
        } else {
            StreamEvent event;
            event.type = StreamEventType::End;
            event.data = tag.name;
            tokens.push_back(std::move(event));
            if (open_elements_depth_ > 0) {
                open_elements_depth_--;
            }
        }
    } else if (std::holds_alternative<CommentToken>(token)) {
        const auto& comment = std::get<CommentToken>(token);
        StreamEvent event;
        event.type = StreamEventType::Comment;
        event.data = comment.data;
        tokens.push_back(std::move(event));
    } else if (std::holds_alternative<DoctypeToken>(token)) {
        const auto& dt = std::get<DoctypeToken>(token);
        StreamEvent event;
        event.type = StreamEventType::Doctype;
        event.data = DoctypeData{dt.doctype.name, dt.doctype.public_id, dt.doctype.system_id};
        tokens.push_back(std::move(event));
    }
    // EOFToken is ignored

    return TokenSinkResult::Continue;
}

void StreamSink::process_characters(const std::string& data) {
    StreamEvent event;
    event.type = StreamEventType::Text;
    event.data = data;
    tokens.push_back(std::move(event));
}

// ============================================================================
// stream() function
// ============================================================================

std::vector<StreamEvent> stream(const std::string& html) {
    StreamSink sink;
    Tokenizer tokenizer(sink);
    tokenizer.initialize(html);

    std::vector<StreamEvent> result;

    while (true) {
        bool is_eof = tokenizer.step();

        // Yield any tokens produced by this step
        if (!sink.tokens.empty()) {
            // Coalesce text tokens
            std::string text_buffer;
            for (auto& event : sink.tokens) {
                if (event.type == StreamEventType::Text) {
                    text_buffer += std::get<std::string>(event.data);
                } else {
                    if (!text_buffer.empty()) {
                        StreamEvent text_event;
                        text_event.type = StreamEventType::Text;
                        text_event.data = std::move(text_buffer);
                        result.push_back(std::move(text_event));
                        text_buffer.clear();
                    }
                    result.push_back(std::move(event));
                }
            }
            if (!text_buffer.empty()) {
                StreamEvent text_event;
                text_event.type = StreamEventType::Text;
                text_event.data = std::move(text_buffer);
                result.push_back(std::move(text_event));
            }
            sink.tokens.clear();
        }

        if (is_eof) break;
    }

    return result;
}

std::vector<StreamEvent> stream_bytes(const std::vector<uint8_t>& data,
                                       const std::optional<std::string>& encoding) {
    auto [html_str, enc] = decode_html(data, encoding);
    return stream(html_str);
}

}  // namespace justhtml
