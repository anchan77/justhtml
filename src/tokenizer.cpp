/// @file tokenizer.cpp
/// @brief HTML5 tokenizer state machine implementation.
///
/// Port of the Python tokenizer (justhtml/tokenizer.py) to C++.
/// Processes input HTML character-by-character and emits tokens via TokenSink.

#include "justhtml/tokenizer.hpp"

#include <algorithm>
#include <cstdint>
#include <string>

#include "justhtml/entities.hpp"
#include "justhtml/errors.hpp"

namespace justhtml {

// UTF-8 replacement character U+FFFD
static const char* REPLACEMENT_CHAR = "\xef\xbf\xbd";

// -------------------------
// Static data
// -------------------------

const std::unordered_set<std::string> Tokenizer::RCDATA_ELEMENTS = {"title", "textarea"};

const std::unordered_set<std::string> Tokenizer::RAWTEXT_SWITCH_TAGS = {
    "script", "style", "xmp", "iframe", "noembed", "noframes", "textarea", "title",
};

// State dispatch table
const Tokenizer::StateHandler Tokenizer::STATE_HANDLERS[NUM_STATES] = {
    &Tokenizer::state_data,                                             // DATA = 0
    &Tokenizer::state_tag_open,                                         // TAG_OPEN = 1
    &Tokenizer::state_end_tag_open,                                     // END_TAG_OPEN = 2
    &Tokenizer::state_tag_name,                                         // TAG_NAME = 3
    &Tokenizer::state_before_attribute_name,                            // BEFORE_ATTRIBUTE_NAME = 4
    &Tokenizer::state_attribute_name,                                   // ATTRIBUTE_NAME = 5
    &Tokenizer::state_after_attribute_name,                             // AFTER_ATTRIBUTE_NAME = 6
    &Tokenizer::state_before_attribute_value,                           // BEFORE_ATTRIBUTE_VALUE = 7
    &Tokenizer::state_attribute_value_double,                           // ATTRIBUTE_VALUE_DOUBLE = 8
    &Tokenizer::state_attribute_value_single,                           // ATTRIBUTE_VALUE_SINGLE = 9
    &Tokenizer::state_attribute_value_unquoted,                         // ATTRIBUTE_VALUE_UNQUOTED = 10
    &Tokenizer::state_after_attribute_value_quoted,                     // AFTER_ATTRIBUTE_VALUE_QUOTED = 11
    &Tokenizer::state_self_closing_start_tag,                           // SELF_CLOSING_START_TAG = 12
    &Tokenizer::state_markup_declaration_open,                          // MARKUP_DECLARATION_OPEN = 13
    &Tokenizer::state_comment_start,                                    // COMMENT_START = 14
    &Tokenizer::state_comment_start_dash,                               // COMMENT_START_DASH = 15
    &Tokenizer::state_comment,                                          // COMMENT_ = 16
    &Tokenizer::state_comment_end_dash,                                 // COMMENT_END_DASH = 17
    &Tokenizer::state_comment_end,                                      // COMMENT_END = 18
    &Tokenizer::state_comment_end_bang,                                 // COMMENT_END_BANG = 19
    &Tokenizer::state_bogus_comment,                                    // BOGUS_COMMENT = 20
    &Tokenizer::state_doctype,                                          // DOCTYPE_ = 21
    &Tokenizer::state_before_doctype_name,                              // BEFORE_DOCTYPE_NAME = 22
    &Tokenizer::state_doctype_name,                                     // DOCTYPE_NAME = 23
    &Tokenizer::state_after_doctype_name,                               // AFTER_DOCTYPE_NAME = 24
    &Tokenizer::state_bogus_doctype,                                    // BOGUS_DOCTYPE = 25
    &Tokenizer::state_after_doctype_public_keyword,                     // AFTER_DOCTYPE_PUBLIC_KEYWORD = 26
    &Tokenizer::state_after_doctype_system_keyword,                     // AFTER_DOCTYPE_SYSTEM_KEYWORD = 27
    &Tokenizer::state_before_doctype_public_identifier,                 // BEFORE_DOCTYPE_PUBLIC_IDENTIFIER = 28
    &Tokenizer::state_doctype_public_identifier_double_quoted,          // 29
    &Tokenizer::state_doctype_public_identifier_single_quoted,          // 30
    &Tokenizer::state_after_doctype_public_identifier,                  // 31
    &Tokenizer::state_between_doctype_public_and_system_identifiers,    // 32
    &Tokenizer::state_before_doctype_system_identifier,                 // 33
    &Tokenizer::state_doctype_system_identifier_double_quoted,          // 34
    &Tokenizer::state_doctype_system_identifier_single_quoted,          // 35
    &Tokenizer::state_after_doctype_system_identifier,                  // 36
    &Tokenizer::state_cdata_section,                                    // CDATA_SECTION = 37
    &Tokenizer::state_cdata_section_bracket,                            // CDATA_SECTION_BRACKET = 38
    &Tokenizer::state_cdata_section_end,                                // CDATA_SECTION_END = 39
    &Tokenizer::state_rcdata,                                           // RCDATA = 40
    &Tokenizer::state_rcdata_less_than_sign,                            // RCDATA_LESS_THAN_SIGN = 41
    &Tokenizer::state_rcdata_end_tag_open,                              // RCDATA_END_TAG_OPEN = 42
    &Tokenizer::state_rcdata_end_tag_name,                              // RCDATA_END_TAG_NAME = 43
    &Tokenizer::state_rawtext,                                          // RAWTEXT = 44
    &Tokenizer::state_rawtext_less_than_sign,                           // RAWTEXT_LESS_THAN_SIGN = 45
    &Tokenizer::state_rawtext_end_tag_open,                             // RAWTEXT_END_TAG_OPEN = 46
    &Tokenizer::state_rawtext_end_tag_name,                             // RAWTEXT_END_TAG_NAME = 47
    &Tokenizer::state_plaintext,                                        // PLAINTEXT = 48
    &Tokenizer::state_script_data_escaped,                              // SCRIPT_DATA_ESCAPED = 49
    &Tokenizer::state_script_data_escaped_dash,                         // 50
    &Tokenizer::state_script_data_escaped_dash_dash,                    // 51
    &Tokenizer::state_script_data_escaped_less_than_sign,               // 52
    &Tokenizer::state_script_data_escaped_end_tag_open,                 // 53
    &Tokenizer::state_script_data_escaped_end_tag_name,                 // 54
    &Tokenizer::state_script_data_double_escape_start,                  // 55
    &Tokenizer::state_script_data_double_escaped,                       // 56
    &Tokenizer::state_script_data_double_escaped_dash,                  // 57
    &Tokenizer::state_script_data_double_escaped_dash_dash,             // 58
    &Tokenizer::state_script_data_double_escaped_less_than_sign,        // 59
    &Tokenizer::state_script_data_double_escape_end,                    // 60
};

// -------------------------
// Helper: is ASCII whitespace
// -------------------------
static inline bool is_ascii_ws(char c) {
    return c == '\t' || c == '\n' || c == '\f' || c == ' ';
}

static inline bool is_ascii_alpha(char c) {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}

static inline char to_lower(char c) {
    if (c >= 'A' && c <= 'Z') return static_cast<char>(c + 32);
    return c;
}

// -------------------------
// Constructor
// -------------------------

Tokenizer::Tokenizer(TokenSink& sink, const TokenizerOpts& opts, bool collect_errors)
    : sink_(sink), opts_(opts), collect_errors_(collect_errors) {}

// -------------------------
// Initialize
// -------------------------

void Tokenizer::initialize(const std::string& html) {
    buffer_ = html;

    // Discard BOM (U+FEFF) - In UTF-8, BOM is 0xEF 0xBB 0xBF
    if (opts_.discard_bom && buffer_.size() >= 3) {
        if (static_cast<unsigned char>(buffer_[0]) == 0xEF &&
            static_cast<unsigned char>(buffer_[1]) == 0xBB &&
            static_cast<unsigned char>(buffer_[2]) == 0xBF) {
            buffer_.erase(0, 3);
        }
    }

    // Normalize newlines per section 13.2.2.5
    // Replace \r\n with \n, then remaining \r with \n
    {
        std::string::size_type p = 0;
        while ((p = buffer_.find('\r', p)) != std::string::npos) {
            if (p + 1 < buffer_.size() && buffer_[p + 1] == '\n') {
                buffer_.erase(p, 1);  // remove the \r, leave the \n
            } else {
                buffer_[p] = '\n';
            }
            ++p;
        }
    }

    length_ = buffer_.size();
    pos_ = 0;
    reconsume_ = false;
    current_char_ = std::nullopt;
    last_token_line_ = 1;
    last_token_column_ = 0;
    errors_.clear();
    text_buffer_.clear();
    text_start_pos_ = 0;
    current_tag_name_.clear();
    current_tag_attrs_.clear();
    current_attr_name_.clear();
    current_attr_value_.clear();
    current_attr_value_has_amp_ = false;
    current_comment_.clear();
    current_doctype_name_.clear();
    current_doctype_public_ = std::nullopt;
    current_doctype_system_ = std::nullopt;
    current_doctype_force_quirks_ = false;
    current_tag_self_closing_ = false;
    current_tag_kind_ = TagKind::Start;
    rawtext_tag_name_ = opts_.initial_rawtext_tag;
    temp_buffer_.clear();
    last_start_tag_name_ = std::nullopt;
    original_tag_name_.clear();

    if (opts_.initial_state.has_value()) {
        state_ = opts_.initial_state.value();
    } else {
        state_ = DATA;
    }

    // Pre-compute newline positions for O(log n) line lookups
    if (collect_errors_) {
        newline_positions_ = std::vector<size_t>();
        for (size_t i = 0; i < buffer_.size(); ++i) {
            if (buffer_[i] == '\n') {
                newline_positions_->push_back(i);
            }
        }
    } else {
        newline_positions_ = std::nullopt;
    }
}

// -------------------------
// Step / Run
// -------------------------

bool Tokenizer::step() {
    return (this->*STATE_HANDLERS[state_])();
}

void Tokenizer::run(const std::string& html) {
    initialize(html);
    while (true) {
        if (step()) break;
    }
}

// -------------------------
// UTF-8 helpers
// -------------------------

void Tokenizer::append_codepoint(std::string& out, char32_t cp) {
    if (cp <= 0x7F) {
        out.push_back(static_cast<char>(cp));
    } else if (cp <= 0x7FF) {
        out.push_back(static_cast<char>(0xC0 | (cp >> 6)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else if (cp <= 0xFFFF) {
        out.push_back(static_cast<char>(0xE0 | (cp >> 12)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else if (cp <= 0x10FFFF) {
        out.push_back(static_cast<char>(0xF0 | (cp >> 18)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 12) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    }
}

char32_t Tokenizer::char_at(const std::string& buf, size_t pos) {
    if (pos >= buf.size()) return 0;
    auto uc = static_cast<unsigned char>(buf[pos]);
    if (uc < 0x80) return uc;
    if ((uc & 0xE0) == 0xC0 && pos + 1 < buf.size()) {
        return ((uc & 0x1F) << 6) | (static_cast<unsigned char>(buf[pos + 1]) & 0x3F);
    }
    if ((uc & 0xF0) == 0xE0 && pos + 2 < buf.size()) {
        return ((uc & 0x0F) << 12) | ((static_cast<unsigned char>(buf[pos + 1]) & 0x3F) << 6) |
               (static_cast<unsigned char>(buf[pos + 2]) & 0x3F);
    }
    if ((uc & 0xF8) == 0xF0 && pos + 3 < buf.size()) {
        return ((uc & 0x07) << 18) | ((static_cast<unsigned char>(buf[pos + 1]) & 0x3F) << 12) |
               ((static_cast<unsigned char>(buf[pos + 2]) & 0x3F) << 6) |
               (static_cast<unsigned char>(buf[pos + 3]) & 0x3F);
    }
    return uc;  // fallback
}

size_t Tokenizer::char_len(const std::string& buf, size_t pos) {
    if (pos >= buf.size()) return 0;
    auto uc = static_cast<unsigned char>(buf[pos]);
    if (uc < 0x80) return 1;
    if ((uc & 0xE0) == 0xC0) return 2;
    if ((uc & 0xF0) == 0xE0) return 3;
    if ((uc & 0xF8) == 0xF0) return 4;
    return 1;  // invalid byte, treat as 1
}

std::string Tokenizer::to_lower_ascii(const std::string& s) {
    std::string result = s;
    for (auto& ch : result) {
        if (ch >= 'A' && ch <= 'Z') ch = static_cast<char>(ch + 32);
    }
    return result;
}

// -------------------------
// Character consumption
// -------------------------

std::optional<char32_t> Tokenizer::get_char() {
    if (reconsume_) {
        reconsume_ = false;
        return current_char_;
    }
    if (pos_ >= length_) {
        current_char_ = std::nullopt;
        return std::nullopt;
    }
    char c = buffer_[pos_];
    pos_++;
    current_char_ = static_cast<char32_t>(static_cast<unsigned char>(c));
    return current_char_;
}

std::optional<char32_t> Tokenizer::peek_char(int offset) const {
    size_t peek_pos = pos_ + static_cast<size_t>(offset);
    if (peek_pos < length_) {
        return static_cast<char32_t>(static_cast<unsigned char>(buffer_[peek_pos]));
    }
    return std::nullopt;
}

void Tokenizer::reconsume_current() {
    reconsume_ = true;
}

// -------------------------
// Text buffer management
// -------------------------

void Tokenizer::append_text(const std::string& text) {
    if (text_buffer_.empty()) {
        text_start_pos_ = pos_;
    }
    text_buffer_ += text;
}

void Tokenizer::append_text_char(char32_t ch) {
    if (text_buffer_.empty()) {
        text_start_pos_ = pos_;
    }
    if (ch <= 0x7F) {
        text_buffer_.push_back(static_cast<char>(ch));
    } else {
        append_codepoint(text_buffer_, ch);
    }
}

void Tokenizer::flush_text() {
    if (text_buffer_.empty()) return;

    std::string data = std::move(text_buffer_);
    text_buffer_.clear();

    size_t raw_len = data.size();

    // Check for null characters in DATA state
    if (state_ == DATA) {
        for (size_t i = 0; i < data.size(); ++i) {
            if (data[i] == '\0') {
                emit_error("unexpected-null-character");
            }
        }
    }

    // Decode entities based on state:
    // - RCDATA state (title, textarea): decode character references
    // - RAWTEXT state (style, script, etc): do NOT decode
    // - PLAINTEXT state: do NOT decode
    // - CDATA sections: do NOT decode
    bool should_decode = true;
    if (state_ >= PLAINTEXT) {
        should_decode = false;
    } else if (state_ >= RAWTEXT) {
        should_decode = false;
    } else if (state_ >= CDATA_SECTION && state_ <= CDATA_SECTION_END) {
        should_decode = false;
    }

    if (should_decode) {
        if (data.find('&') != std::string::npos) {
            data = decode_entities_in_text(data);
        }
    }

    // Apply XML coercion if enabled
    if (opts_.xml_coercion) {
        data = coerce_text_for_xml(data);
    }

    // Record position at END of raw text
    if (collect_errors_) {
        record_text_end_position(raw_len);
    }
    sink_.process_characters(data);
}

// -------------------------
// Attribute helpers
// -------------------------

void Tokenizer::append_attr_value_char(char32_t ch) {
    if (ch <= 0x7F) {
        current_attr_value_.push_back(static_cast<char>(ch));
    } else {
        append_codepoint(current_attr_value_, ch);
    }
}

void Tokenizer::finish_attribute() {
    if (current_attr_name_.empty()) return;

    std::string name = std::move(current_attr_name_);
    current_attr_name_.clear();

    // Check for duplicate
    bool is_duplicate = current_tag_attrs_.count(name) > 0;

    if (is_duplicate) {
        emit_error("duplicate-attribute");
        current_attr_value_.clear();
        current_attr_value_has_amp_ = false;
        return;
    }

    std::string value = std::move(current_attr_value_);
    current_attr_value_.clear();

    if (current_attr_value_has_amp_) {
        value = decode_entities_in_text(value, /*in_attribute=*/true);
    }
    current_tag_attrs_[name] = value;
    current_attr_value_has_amp_ = false;
}

// -------------------------
// Tag emission
// -------------------------

bool Tokenizer::emit_current_tag() {
    std::string name = std::move(current_tag_name_);
    current_tag_name_.clear();

    Attributes attrs = std::move(current_tag_attrs_);
    current_tag_attrs_.clear();

    Tag tag(current_tag_kind_, name, std::move(attrs), current_tag_self_closing_);

    bool switched_to_rawtext = false;
    if (current_tag_kind_ == TagKind::Start) {
        last_start_tag_name_ = name;
        bool needs_rawtext_check =
            RAWTEXT_SWITCH_TAGS.count(name) > 0 || name == "plaintext";
        if (needs_rawtext_check) {
            // Default to "html" namespace (will be refined when tree builder is implemented)
            std::string ns = "html";
            if (ns == "html") {
                if (RCDATA_ELEMENTS.count(name) > 0) {
                    state_ = RCDATA;
                    rawtext_tag_name_ = name;
                    switched_to_rawtext = true;
                } else if (RAWTEXT_SWITCH_TAGS.count(name) > 0) {
                    state_ = RAWTEXT;
                    rawtext_tag_name_ = name;
                    switched_to_rawtext = true;
                } else {
                    // Must be "plaintext"
                    state_ = PLAINTEXT;
                    switched_to_rawtext = true;
                }
            }
        }
    }

    if (collect_errors_) {
        record_token_position();
    }
    TokenSinkResult result = sink_.process_token(tag);
    if (result == TokenSinkResult::Plaintext) {
        state_ = PLAINTEXT;
        switched_to_rawtext = true;
    }

    current_attr_name_.clear();
    current_attr_value_.clear();
    current_tag_self_closing_ = false;
    current_tag_kind_ = TagKind::Start;
    return switched_to_rawtext;
}

// -------------------------
// Comment emission
// -------------------------

void Tokenizer::emit_comment() {
    std::string data = std::move(current_comment_);
    current_comment_.clear();
    if (opts_.xml_coercion) {
        data = coerce_comment_for_xml(data);
    }
    emit_token(CommentToken(std::move(data)));
}

// -------------------------
// Doctype emission
// -------------------------

void Tokenizer::emit_doctype() {
    std::optional<std::string> name;
    if (!current_doctype_name_.empty()) {
        name = std::move(current_doctype_name_);
    }
    current_doctype_name_.clear();

    std::optional<std::string> public_id = std::move(current_doctype_public_);
    std::optional<std::string> system_id = std::move(current_doctype_system_);
    current_doctype_public_ = std::nullopt;
    current_doctype_system_ = std::nullopt;

    Doctype doctype(std::move(name), std::move(public_id), std::move(system_id),
                    current_doctype_force_quirks_);
    current_doctype_force_quirks_ = false;
    emit_token(DoctypeToken(std::move(doctype)));
}

// -------------------------
// Token emission
// -------------------------

void Tokenizer::emit_token(const Token& token) {
    if (collect_errors_) {
        record_token_position();
    }
    sink_.process_token(token);
}

// -------------------------
// Error emission
// -------------------------

void Tokenizer::emit_error(const std::string& code) {
    if (!collect_errors_) return;

    size_t p = (pos_ > 0) ? pos_ - 1 : 0;
    // Find last newline before or at position p
    auto last_newline = buffer_.rfind('\n', p);
    int column;
    if (last_newline == std::string::npos) {
        column = static_cast<int>(p) + 1;  // 1-indexed from start
    } else {
        column = static_cast<int>(p - last_newline);  // 1-indexed from after newline
    }

    std::string message = generate_error_message(code);
    int line = get_line_at_pos(pos_);
    errors_.emplace_back(code, line, column, message, buffer_);
}

// -------------------------
// Position tracking
// -------------------------

int Tokenizer::get_line_at_pos(size_t pos) const {
    if (!newline_positions_.has_value()) return 1;
    const auto& nlp = newline_positions_.value();
    // bisect_right equivalent: count of newline positions <= (pos - 1)
    int target = static_cast<int>(pos) - 1;
    auto it = std::upper_bound(nlp.begin(), nlp.end(), static_cast<size_t>(target < 0 ? 0 : target));
    return static_cast<int>(std::distance(nlp.begin(), it)) + 1;
}

void Tokenizer::record_token_position() {
    size_t p = pos_;
    auto last_newline = buffer_.rfind('\n', p > 0 ? p - 1 : 0);
    int column;
    if (last_newline == std::string::npos || p == 0) {
        column = static_cast<int>(p);
    } else {
        column = static_cast<int>(p - last_newline - 1);
    }
    last_token_line_ = get_line_at_pos(p);
    last_token_column_ = column;
}

void Tokenizer::record_text_end_position(size_t raw_len) {
    size_t end_pos = text_start_pos_ + raw_len;
    auto last_newline = buffer_.rfind('\n', end_pos > 0 ? end_pos - 1 : 0);
    int column;
    if (last_newline == std::string::npos || end_pos == 0) {
        column = static_cast<int>(end_pos);
    } else {
        column = static_cast<int>(end_pos - last_newline - 1);
    }
    last_token_line_ = get_line_at_pos(end_pos);
    last_token_column_ = column;
}

// -------------------------
// Consume helpers
// -------------------------

bool Tokenizer::consume_if(const std::string& literal) {
    size_t end = pos_ + literal.size();
    if (end > length_) return false;
    if (buffer_.compare(pos_, literal.size(), literal) != 0) return false;
    pos_ = end;
    return true;
}

bool Tokenizer::consume_case_insensitive(const std::string& literal) {
    size_t end = pos_ + literal.size();
    if (end > length_) return false;
    for (size_t i = 0; i < literal.size(); ++i) {
        char a = buffer_[pos_ + i];
        char b = literal[i];
        if (to_lower(a) != to_lower(b)) return false;
    }
    pos_ = end;
    return true;
}

bool Tokenizer::consume_comment_run() {
    if (pos_ >= length_) return false;
    // Scan for run of characters that are not '-' or '\0'
    size_t start = pos_;
    while (pos_ < length_ && buffer_[pos_] != '-' && buffer_[pos_] != '\0') {
        pos_++;
    }
    if (pos_ > start) {
        current_comment_.append(buffer_, start, pos_ - start);
        return true;
    }
    return false;
}

// -------------------------
// XML coercion
// -------------------------

std::string Tokenizer::coerce_text_for_xml(const std::string& text) {
    // Fast path for ASCII-only text
    bool has_formfeed = false;
    bool all_ascii = true;
    for (char c : text) {
        if (c == '\f') has_formfeed = true;
        if (static_cast<unsigned char>(c) > 0x7F) all_ascii = false;
    }

    if (all_ascii) {
        if (has_formfeed) {
            std::string result = text;
            for (auto& c : result) {
                if (c == '\f') c = ' ';
            }
            return result;
        }
        return text;
    }

    // For non-ASCII text, replace form feeds with spaces and
    // replace XML-invalid codepoints with replacement character.
    // This is a simplified version - for full compliance we'd check
    // U+FDD0-U+FDEF and U+xFFFE/U+xFFFF ranges.
    std::string result;
    result.reserve(text.size());
    size_t i = 0;
    while (i < text.size()) {
        unsigned char uc = static_cast<unsigned char>(text[i]);
        if (uc < 0x80) {
            if (text[i] == '\f') {
                result.push_back(' ');
            } else {
                result.push_back(text[i]);
            }
            ++i;
        } else {
            // Decode UTF-8 to check codepoint
            char32_t cp = char_at(text, i);
            size_t clen = char_len(text, i);

            // Check if codepoint is in XML-invalid range
            bool invalid = false;
            if (cp >= 0xFDD0 && cp <= 0xFDEF) {
                invalid = true;
            }
            // Check U+xFFFE and U+xFFFF for each plane
            if ((cp & 0xFFFF) == 0xFFFE || (cp & 0xFFFF) == 0xFFFF) {
                invalid = true;
            }

            if (invalid) {
                result.append(REPLACEMENT_CHAR);
            } else {
                result.append(text, i, clen);
            }
            i += clen;
        }
    }
    return result;
}

std::string Tokenizer::coerce_comment_for_xml(const std::string& text) {
    // Replace "--" with "- -"
    if (text.find("--") == std::string::npos) return text;
    std::string result;
    result.reserve(text.size() + text.size() / 4);
    for (size_t i = 0; i < text.size(); ++i) {
        result.push_back(text[i]);
        if (text[i] == '-' && i + 1 < text.size() && text[i + 1] == '-') {
            result.push_back(' ');
        }
    }
    return result;
}

// =========================================================================
// State handlers
// =========================================================================

// -------------------------
// DATA state
// -------------------------
bool Tokenizer::state_data() {
    size_t pos = pos_;
    while (true) {
        if (reconsume_) {
            reconsume_ = false;
            if (pos > 0) --pos;
            pos_ = pos;
        }

        if (pos >= length_) {
            pos_ = length_;
            current_char_ = std::nullopt;
            flush_text();
            emit_token(EOFToken());
            return true;
        }

        // Optimized loop: find next '<'
        size_t next_lt = buffer_.find('<', pos);
        if (next_lt == std::string::npos) next_lt = length_;

        size_t end = next_lt;

        if (end > pos) {
            append_text(buffer_.substr(pos, end - pos));
            pos = end;
            pos_ = pos;
            if (pos >= length_) continue;
        }

        // We're at '<'
        char c = buffer_[pos];
        pos++;
        pos_ = pos;
        current_char_ = static_cast<char32_t>(static_cast<unsigned char>(c));

        // Peek-ahead optimization for common tag starts
        if (pos < length_) {
            char nc = buffer_[pos];
            if (is_ascii_alpha(nc)) {
                flush_text();
                // Inline start tag setup
                current_tag_kind_ = TagKind::Start;
                current_tag_name_.clear();
                current_attr_name_.clear();
                current_attr_value_.clear();
                current_attr_value_has_amp_ = false;
                current_tag_self_closing_ = false;

                current_tag_name_.push_back(to_lower(nc));
                pos_++;
                state_ = TAG_NAME;
                return state_tag_name();
            }

            if (nc == '!') {
                // Peek ahead for comment <!--
                if (pos + 2 < length_ && buffer_[pos + 1] == '-' && buffer_[pos + 2] == '-') {
                    flush_text();
                    pos_ += 3;  // Consume !--
                    current_comment_.clear();
                    state_ = COMMENT_START;
                    return state_comment_start();
                }
            }

            if (nc == '/') {
                // Check next char for end tag
                if (pos + 1 < length_) {
                    char nnc = buffer_[pos + 1];
                    if (is_ascii_alpha(nnc)) {
                        flush_text();
                        // Inline end tag setup
                        current_tag_kind_ = TagKind::End;
                        current_tag_name_.clear();
                        current_attr_name_.clear();
                        current_attr_value_.clear();
                        current_attr_value_has_amp_ = false;
                        current_tag_self_closing_ = false;

                        current_tag_name_.push_back(to_lower(nnc));
                        pos_ += 2;  // Consume / and nnc
                        state_ = TAG_NAME;
                        return state_tag_name();
                    }
                }
            }
        }

        flush_text();
        state_ = TAG_OPEN;
        return state_tag_open();
    }
}

// -------------------------
// TAG_OPEN state
// -------------------------
bool Tokenizer::state_tag_open() {
    auto c = get_char();
    if (!c.has_value()) {
        emit_error("eof-before-tag-name");
        append_text("<");
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char ch = static_cast<char>(c.value());
    if (ch == '!') {
        state_ = MARKUP_DECLARATION_OPEN;
        return false;
    }
    if (ch == '/') {
        state_ = END_TAG_OPEN;
        return false;
    }
    if (ch == '?') {
        emit_error("unexpected-question-mark-instead-of-tag-name");
        current_comment_.clear();
        reconsume_current();
        state_ = BOGUS_COMMENT;
        return false;
    }

    emit_error("invalid-first-character-of-tag-name");
    append_text("<");
    reconsume_current();
    state_ = DATA;
    return false;
}

// -------------------------
// END_TAG_OPEN state
// -------------------------
bool Tokenizer::state_end_tag_open() {
    auto c = get_char();
    if (!c.has_value()) {
        emit_error("eof-before-tag-name");
        append_text("<");
        append_text("/");
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char ch = static_cast<char>(c.value());
    if (ch == '>') {
        emit_error("empty-end-tag");
        state_ = DATA;
        return false;
    }

    emit_error("invalid-first-character-of-tag-name");
    current_comment_.clear();
    reconsume_current();
    state_ = BOGUS_COMMENT;
    return false;
}

// -------------------------
// TAG_NAME state
// -------------------------
bool Tokenizer::state_tag_name() {
    size_t pos = pos_;

    while (true) {
        // Fast path: scan a run of tag name characters
        if (pos < length_) {
            char first = buffer_[pos];
            if (first != '\t' && first != '\n' && first != '\f' && first != ' ' &&
                first != '/' && first != '>' && first != '\0') {
                // Scan run of non-terminator characters
                size_t run_start = pos;
                while (pos < length_) {
                    char ch = buffer_[pos];
                    if (ch == '\t' || ch == '\n' || ch == '\f' || ch == ' ' ||
                        ch == '/' || ch == '>' || ch == '\0') {
                        break;
                    }
                    pos++;
                }
                // Append the run (lowercased)
                for (size_t i = run_start; i < pos; ++i) {
                    current_tag_name_.push_back(to_lower(buffer_[i]));
                }

                if (pos < length_) {
                    char next_char = buffer_[pos];
                    if (is_ascii_ws(next_char)) {
                        pos++;
                        pos_ = pos;
                        state_ = BEFORE_ATTRIBUTE_NAME;
                        return state_before_attribute_name();
                    }
                    if (next_char == '>') {
                        pos++;
                        pos_ = pos;
                        if (!emit_current_tag()) {
                            state_ = DATA;
                        }
                        return false;
                    }
                    if (next_char == '/') {
                        pos++;
                        pos_ = pos;
                        state_ = SELF_CLOSING_START_TAG;
                        return state_self_closing_start_tag();
                    }
                }
            }
        }

        // Slow path: get one character at a time
        if (pos >= length_) {
            pos_ = pos;
            emit_error("eof-in-tag");
            emit_token(EOFToken());
            return true;
        }

        char c = buffer_[pos];
        pos++;
        current_char_ = static_cast<char32_t>(static_cast<unsigned char>(c));

        if (is_ascii_ws(c)) {
            pos_ = pos;
            state_ = BEFORE_ATTRIBUTE_NAME;
            return state_before_attribute_name();
        }
        if (c == '/') {
            pos_ = pos;
            state_ = SELF_CLOSING_START_TAG;
            return state_self_closing_start_tag();
        }
        if (c == '>') {
            pos_ = pos;
            emit_current_tag();
            state_ = DATA;
            return false;
        }
        // c == '\0' - the only remaining possibility after fast path
        pos_ = pos;
        emit_error("unexpected-null-character");
        current_tag_name_.append(REPLACEMENT_CHAR);
    }
}

// -------------------------
// BEFORE_ATTRIBUTE_NAME state
// -------------------------
bool Tokenizer::state_before_attribute_name() {
    while (true) {
        // Skip whitespace
        if (!reconsume_) {
            while (pos_ < length_ && is_ascii_ws(buffer_[pos_])) {
                pos_++;
            }
        }

        // Get char
        std::optional<char32_t> c_opt;
        if (reconsume_) {
            reconsume_ = false;
            c_opt = current_char_;
        } else if (pos_ >= length_) {
            c_opt = std::nullopt;
        } else {
            c_opt = static_cast<char32_t>(static_cast<unsigned char>(buffer_[pos_]));
            pos_++;
        }
        current_char_ = c_opt;

        if (c_opt.has_value()) {
            char c = static_cast<char>(c_opt.value());
            if (is_ascii_ws(c)) continue;
        }

        if (!c_opt.has_value()) {
            emit_error("eof-in-tag");
            flush_text();
            emit_token(EOFToken());
            return true;
        }

        char c = static_cast<char>(c_opt.value());
        if (c == '/') {
            state_ = SELF_CLOSING_START_TAG;
            return false;
        }
        if (c == '>') {
            finish_attribute();
            if (!emit_current_tag()) {
                state_ = DATA;
            }
            return false;
        }
        if (c == '=') {
            emit_error("unexpected-equals-sign-before-attribute-name");
            current_attr_name_.clear();
            current_attr_value_.clear();
            current_attr_value_has_amp_ = false;
            current_attr_name_.push_back('=');
            state_ = ATTRIBUTE_NAME;
            return false;
        }

        current_attr_name_.clear();
        current_attr_value_.clear();
        current_attr_value_has_amp_ = false;
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_attr_name_.append(REPLACEMENT_CHAR);
        } else if (c >= 'A' && c <= 'Z') {
            current_attr_name_.push_back(to_lower(c));
        } else {
            current_attr_name_.push_back(c);
        }
        state_ = ATTRIBUTE_NAME;
        return false;
    }
}

// -------------------------
// ATTRIBUTE_NAME state
// -------------------------
bool Tokenizer::state_attribute_name() {
    size_t pos = pos_;

    while (true) {
        // Fast path: scan a run of attribute name characters
        if (pos < length_) {
            char first = buffer_[pos];
            if (first != '\t' && first != '\n' && first != '\f' && first != ' ' &&
                first != '/' && first != '>' && first != '=' && first != '\0' &&
                first != '"' && first != '\'' && first != '<') {
                size_t run_start = pos;
                while (pos < length_) {
                    char ch = buffer_[pos];
                    if (ch == '\t' || ch == '\n' || ch == '\f' || ch == ' ' ||
                        ch == '/' || ch == '>' || ch == '=' || ch == '\0' ||
                        ch == '"' || ch == '\'' || ch == '<') {
                        break;
                    }
                    pos++;
                }
                // Append the run (lowercased)
                for (size_t i = run_start; i < pos; ++i) {
                    current_attr_name_.push_back(to_lower(buffer_[i]));
                }

                if (pos < length_) {
                    char next_c = buffer_[pos];
                    if (next_c == '=') {
                        pos++;
                        pos_ = pos;
                        state_ = BEFORE_ATTRIBUTE_VALUE;
                        return state_before_attribute_value();
                    }
                    if (is_ascii_ws(next_c)) {
                        pos++;
                        pos_ = pos;
                        finish_attribute();
                        state_ = AFTER_ATTRIBUTE_NAME;
                        return false;
                    }
                    if (next_c == '>') {
                        pos++;
                        pos_ = pos;
                        finish_attribute();
                        if (!emit_current_tag()) {
                            state_ = DATA;
                        }
                        return false;
                    }
                    if (next_c == '/') {
                        pos++;
                        pos_ = pos;
                        finish_attribute();
                        state_ = SELF_CLOSING_START_TAG;
                        return state_self_closing_start_tag();
                    }
                }
            }
        }

        // Slow path
        pos_ = pos;
        auto c_opt = get_char();
        pos = pos_;

        if (!c_opt.has_value()) {
            emit_error("eof-in-tag");
            flush_text();
            emit_token(EOFToken());
            return true;
        }

        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            finish_attribute();
            state_ = AFTER_ATTRIBUTE_NAME;
            return false;
        }
        if (c == '/') {
            finish_attribute();
            state_ = SELF_CLOSING_START_TAG;
            return state_self_closing_start_tag();
        }
        if (c == '=') {
            state_ = BEFORE_ATTRIBUTE_VALUE;
            return state_before_attribute_value();
        }
        if (c == '>') {
            finish_attribute();
            if (!emit_current_tag()) {
                state_ = DATA;
            }
            return false;
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_attr_name_.append(REPLACEMENT_CHAR);
            continue;
        }
        emit_error("unexpected-character-in-attribute-name");
        current_attr_name_.push_back(c);
    }
}

// -------------------------
// AFTER_ATTRIBUTE_NAME state
// -------------------------
bool Tokenizer::state_after_attribute_name() {
    while (true) {
        // Skip whitespace
        if (!reconsume_) {
            while (pos_ < length_ && is_ascii_ws(buffer_[pos_])) {
                pos_++;
            }
        }

        // Get char
        std::optional<char32_t> c_opt;
        if (pos_ >= length_) {
            c_opt = std::nullopt;
        } else {
            c_opt = static_cast<char32_t>(static_cast<unsigned char>(buffer_[pos_]));
            pos_++;
        }
        current_char_ = c_opt;

        if (c_opt.has_value()) {
            char c = static_cast<char>(c_opt.value());
            if (is_ascii_ws(c)) continue;
        }

        if (!c_opt.has_value()) {
            emit_error("eof-in-tag");
            flush_text();
            emit_token(EOFToken());
            return true;
        }

        char c = static_cast<char>(c_opt.value());
        if (c == '/') {
            finish_attribute();
            state_ = SELF_CLOSING_START_TAG;
            return false;
        }
        if (c == '=') {
            state_ = BEFORE_ATTRIBUTE_VALUE;
            return false;
        }
        if (c == '>') {
            finish_attribute();
            if (!emit_current_tag()) {
                state_ = DATA;
            }
            return false;
        }
        finish_attribute();
        current_attr_name_.clear();
        current_attr_value_.clear();
        current_attr_value_has_amp_ = false;
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_attr_name_.append(REPLACEMENT_CHAR);
        } else if (c >= 'A' && c <= 'Z') {
            current_attr_name_.push_back(to_lower(c));
        } else {
            current_attr_name_.push_back(c);
        }
        state_ = ATTRIBUTE_NAME;
        return false;
    }
}

// -------------------------
// BEFORE_ATTRIBUTE_VALUE state
// -------------------------
bool Tokenizer::state_before_attribute_value() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-tag");
            flush_text();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) continue;
        if (c == '"') {
            state_ = ATTRIBUTE_VALUE_DOUBLE;
            return state_attribute_value_double();
        }
        if (c == '\'') {
            state_ = ATTRIBUTE_VALUE_SINGLE;
            return state_attribute_value_single();
        }
        if (c == '>') {
            emit_error("missing-attribute-value");
            finish_attribute();
            if (!emit_current_tag()) {
                state_ = DATA;
            }
            return false;
        }
        reconsume_current();
        state_ = ATTRIBUTE_VALUE_UNQUOTED;
        return state_attribute_value_unquoted();
    }
}

// -------------------------
// ATTRIBUTE_VALUE_DOUBLE state
// -------------------------
bool Tokenizer::state_attribute_value_double() {
    while (true) {
        // Fast path: scan for next '"', '&', or '\0'
        size_t pos = pos_;
        if (pos < length_) {
            size_t next_quote = buffer_.find('"', pos);
            if (next_quote == std::string::npos) next_quote = length_;

            // Check for '&' or '\0' in the range
            size_t end = next_quote;
            for (size_t i = pos; i < next_quote; ++i) {
                if (buffer_[i] == '&' || buffer_[i] == '\0') {
                    end = i;
                    break;
                }
            }

            if (end > pos) {
                current_attr_value_.append(buffer_, pos, end - pos);
                pos_ = end;
            }
        }

        // Check for EOF
        if (pos_ >= length_) {
            current_char_ = std::nullopt;
            emit_error("eof-in-tag");
            emit_token(EOFToken());
            return true;
        }

        char c = buffer_[pos_];
        pos_++;
        current_char_ = static_cast<char32_t>(static_cast<unsigned char>(c));

        if (c == '"') {
            state_ = AFTER_ATTRIBUTE_VALUE_QUOTED;
            return state_after_attribute_value_quoted();
        }
        if (c == '&') {
            append_attr_value_char('&');
            current_attr_value_has_amp_ = true;
        } else {
            // c == '\0'
            emit_error("unexpected-null-character");
            current_attr_value_.append(REPLACEMENT_CHAR);
        }
    }
}

// -------------------------
// ATTRIBUTE_VALUE_SINGLE state
// -------------------------
bool Tokenizer::state_attribute_value_single() {
    while (true) {
        // Fast path: scan for next '\'', '&', or '\0'
        size_t pos = pos_;
        if (pos < length_) {
            size_t next_quote = buffer_.find('\'', pos);
            if (next_quote == std::string::npos) next_quote = length_;

            size_t end = next_quote;
            for (size_t i = pos; i < next_quote; ++i) {
                if (buffer_[i] == '&' || buffer_[i] == '\0') {
                    end = i;
                    break;
                }
            }

            if (end > pos) {
                current_attr_value_.append(buffer_, pos, end - pos);
                pos_ = end;
            }
        }

        if (pos_ >= length_) {
            current_char_ = std::nullopt;
            emit_error("eof-in-tag");
            emit_token(EOFToken());
            return true;
        }

        char c = buffer_[pos_];
        pos_++;
        current_char_ = static_cast<char32_t>(static_cast<unsigned char>(c));

        if (c == '\'') {
            state_ = AFTER_ATTRIBUTE_VALUE_QUOTED;
            return state_after_attribute_value_quoted();
        }
        if (c == '&') {
            append_attr_value_char('&');
            current_attr_value_has_amp_ = true;
        } else {
            // c == '\0'
            emit_error("unexpected-null-character");
            current_attr_value_.append(REPLACEMENT_CHAR);
        }
    }
}

// -------------------------
// ATTRIBUTE_VALUE_UNQUOTED state
// -------------------------
static inline bool is_attr_value_unquoted_terminator(char c) {
    return c == '\t' || c == '\n' || c == '\f' || c == ' ' ||
           c == '>' || c == '&' || c == '"' || c == '\'' ||
           c == '<' || c == '=' || c == '`' || c == '\0';
}

bool Tokenizer::state_attribute_value_unquoted() {
    while (true) {
        // Fast path: scan for next terminator
        if (!reconsume_) {
            size_t pos = pos_;
            if (pos < length_) {
                size_t run_start = pos;
                while (pos < length_ && !is_attr_value_unquoted_terminator(buffer_[pos])) {
                    pos++;
                }
                if (pos > run_start) {
                    current_attr_value_.append(buffer_, run_start, pos - run_start);
                    pos_ = pos;
                }
            }
        }

        // Get char
        std::optional<char32_t> c_opt;
        if (reconsume_) {
            reconsume_ = false;
            c_opt = current_char_;
        } else if (pos_ >= length_) {
            c_opt = std::nullopt;
        } else {
            c_opt = static_cast<char32_t>(static_cast<unsigned char>(buffer_[pos_]));
            pos_++;
        }
        current_char_ = c_opt;

        if (!c_opt.has_value()) {
            emit_error("eof-in-tag");
            emit_token(EOFToken());
            return true;
        }

        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            finish_attribute();
            state_ = BEFORE_ATTRIBUTE_NAME;
            return false;
        }
        if (c == '>') {
            finish_attribute();
            if (!emit_current_tag()) {
                state_ = DATA;
            }
            return false;
        }
        if (c == '&') {
            append_attr_value_char('&');
            current_attr_value_has_amp_ = true;
            continue;
        }
        if (c == '"' || c == '\'' || c == '<' || c == '=' || c == '`') {
            emit_error("unexpected-character-in-unquoted-attribute-value");
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_attr_value_.append(REPLACEMENT_CHAR);
            continue;
        }
        append_attr_value_char(c_opt.value());
    }
}

// -------------------------
// AFTER_ATTRIBUTE_VALUE_QUOTED state
// -------------------------
bool Tokenizer::state_after_attribute_value_quoted() {
    // Get char (inline)
    std::optional<char32_t> c_opt;
    if (pos_ >= length_) {
        c_opt = std::nullopt;
    } else {
        c_opt = static_cast<char32_t>(static_cast<unsigned char>(buffer_[pos_]));
        pos_++;
    }
    current_char_ = c_opt;

    if (!c_opt.has_value()) {
        emit_error("eof-in-tag");
        flush_text();
        emit_token(EOFToken());
        return true;
    }

    char c = static_cast<char>(c_opt.value());
    if (is_ascii_ws(c)) {
        finish_attribute();
        state_ = BEFORE_ATTRIBUTE_NAME;
        return false;
    }
    if (c == '/') {
        finish_attribute();
        state_ = SELF_CLOSING_START_TAG;
        return false;
    }
    if (c == '>') {
        finish_attribute();
        if (!emit_current_tag()) {
            state_ = DATA;
        }
        return false;
    }
    emit_error("missing-whitespace-between-attributes");
    finish_attribute();
    reconsume_current();
    state_ = BEFORE_ATTRIBUTE_NAME;
    return false;
}

// -------------------------
// SELF_CLOSING_START_TAG state
// -------------------------
bool Tokenizer::state_self_closing_start_tag() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-tag");
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '>') {
        current_tag_self_closing_ = true;
        emit_current_tag();
        state_ = DATA;
        return false;
    }
    emit_error("unexpected-character-after-solidus-in-tag");
    reconsume_current();
    state_ = BEFORE_ATTRIBUTE_NAME;
    return false;
}

// -------------------------
// MARKUP_DECLARATION_OPEN state
// -------------------------
bool Tokenizer::state_markup_declaration_open() {
    // Note: Comment handling (<!--) is optimized in DATA state fast-path.
    // Check for <!-- first (may not have been caught by fast path)
    if (pos_ + 1 < length_ && buffer_[pos_] == '-' && buffer_[pos_ + 1] == '-') {
        pos_ += 2;
        current_comment_.clear();
        state_ = COMMENT_START;
        return false;
    }

    if (consume_case_insensitive("DOCTYPE")) {
        current_doctype_name_.clear();
        current_doctype_public_ = std::nullopt;
        current_doctype_system_ = std::nullopt;
        current_doctype_force_quirks_ = false;
        state_ = DOCTYPE_;
        return false;
    }
    if (consume_if("[CDATA[")) {
        // CDATA sections are only valid in foreign content (SVG/MathML)
        // For now, default to HTML namespace - treat as bogus comment
        emit_error("cdata-in-html-content");
        current_comment_.clear();
        current_comment_.append("[CDATA[");
        state_ = BOGUS_COMMENT;
        return false;
    }
    emit_error("incorrectly-opened-comment");
    current_comment_.clear();
    state_ = BOGUS_COMMENT;
    return false;
}

// -------------------------
// COMMENT_START state
// -------------------------
bool Tokenizer::state_comment_start() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-comment");
        emit_comment();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        state_ = COMMENT_START_DASH;
        return false;
    }
    if (c == '>') {
        emit_error("abrupt-closing-of-empty-comment");
        emit_comment();
        state_ = DATA;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        current_comment_.append(REPLACEMENT_CHAR);
    } else {
        current_comment_.push_back(c);
    }
    state_ = COMMENT_;
    return false;
}

// -------------------------
// COMMENT_START_DASH state
// -------------------------
bool Tokenizer::state_comment_start_dash() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-comment");
        emit_comment();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        state_ = COMMENT_END;
        return false;
    }
    if (c == '>') {
        emit_error("abrupt-closing-of-empty-comment");
        emit_comment();
        state_ = DATA;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        current_comment_.push_back('-');
        current_comment_.append(REPLACEMENT_CHAR);
    } else {
        current_comment_.push_back('-');
        current_comment_.push_back(c);
    }
    state_ = COMMENT_;
    return false;
}

// -------------------------
// COMMENT state
// -------------------------
bool Tokenizer::state_comment() {
    while (true) {
        if (consume_comment_run()) continue;

        // Get char inline
        std::optional<char32_t> c_opt;
        if (pos_ >= length_) {
            c_opt = std::nullopt;
        } else {
            c_opt = static_cast<char32_t>(static_cast<unsigned char>(buffer_[pos_]));
            pos_++;
        }
        current_char_ = c_opt;

        if (!c_opt.has_value()) {
            emit_error("eof-in-comment");
            emit_comment();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '-') {
            state_ = COMMENT_END_DASH;
            return false;
        }
        // c == '\0'
        emit_error("unexpected-null-character");
        current_comment_.append(REPLACEMENT_CHAR);
    }
}

// -------------------------
// COMMENT_END_DASH state
// -------------------------
bool Tokenizer::state_comment_end_dash() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-comment");
        emit_comment();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        state_ = COMMENT_END;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        current_comment_.push_back('-');
        current_comment_.append(REPLACEMENT_CHAR);
        state_ = COMMENT_;
        return false;
    }
    current_comment_.push_back('-');
    current_comment_.push_back(c);
    state_ = COMMENT_;
    return false;
}

// -------------------------
// COMMENT_END state
// -------------------------
bool Tokenizer::state_comment_end() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-comment");
        emit_comment();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '>') {
        emit_comment();
        state_ = DATA;
        return false;
    }
    if (c == '!') {
        state_ = COMMENT_END_BANG;
        return false;
    }
    if (c == '-') {
        current_comment_.push_back('-');
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        current_comment_.append("--");
        current_comment_.append(REPLACEMENT_CHAR);
        state_ = COMMENT_;
        return false;
    }
    emit_error("incorrectly-closed-comment");
    current_comment_.append("--");
    current_comment_.push_back(c);
    state_ = COMMENT_;
    return false;
}

// -------------------------
// COMMENT_END_BANG state
// -------------------------
bool Tokenizer::state_comment_end_bang() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-comment");
        emit_comment();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        current_comment_.append("--!");
        state_ = COMMENT_END_DASH;
        return false;
    }
    if (c == '>') {
        emit_error("incorrectly-closed-comment");
        emit_comment();
        state_ = DATA;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        current_comment_.append("--!");
        current_comment_.append(REPLACEMENT_CHAR);
        state_ = COMMENT_;
        return false;
    }
    current_comment_.append("--!");
    current_comment_.push_back(c);
    state_ = COMMENT_;
    return false;
}

// -------------------------
// BOGUS_COMMENT state
// -------------------------
bool Tokenizer::state_bogus_comment() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_comment();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '>') {
            emit_comment();
            state_ = DATA;
            return false;
        }
        if (c == '\0') {
            current_comment_.append(REPLACEMENT_CHAR);
        } else {
            current_comment_.push_back(c);
        }
    }
}

// -------------------------
// DOCTYPE state
// -------------------------
bool Tokenizer::state_doctype() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        emit_error("eof-in-doctype");
        current_doctype_force_quirks_ = true;
        emit_doctype();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (is_ascii_ws(c)) {
        state_ = BEFORE_DOCTYPE_NAME;
        return false;
    }
    if (c == '>') {
        emit_error("expected-doctype-name-but-got-right-bracket");
        current_doctype_force_quirks_ = true;
        emit_doctype();
        state_ = DATA;
        return false;
    }
    emit_error("missing-whitespace-before-doctype-name");
    reconsume_current();
    state_ = BEFORE_DOCTYPE_NAME;
    return false;
}

// -------------------------
// BEFORE_DOCTYPE_NAME state
// -------------------------
bool Tokenizer::state_before_doctype_name() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype-name");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) return false;
        if (c == '>') {
            emit_error("expected-doctype-name-but-got-right-bracket");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        if (c >= 'A' && c <= 'Z') {
            current_doctype_name_.push_back(to_lower(c));
        } else if (c == '\0') {
            emit_error("unexpected-null-character");
            current_doctype_name_.append(REPLACEMENT_CHAR);
        } else {
            current_doctype_name_.push_back(c);
        }
        state_ = DOCTYPE_NAME;
        return false;
    }
}

// -------------------------
// DOCTYPE_NAME state
// -------------------------
bool Tokenizer::state_doctype_name() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype-name");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            state_ = AFTER_DOCTYPE_NAME;
            return false;
        }
        if (c == '>') {
            emit_doctype();
            state_ = DATA;
            return false;
        }
        if (c >= 'A' && c <= 'Z') {
            current_doctype_name_.push_back(to_lower(c));
            continue;
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_doctype_name_.append(REPLACEMENT_CHAR);
            continue;
        }
        current_doctype_name_.push_back(c);
    }
}

// -------------------------
// AFTER_DOCTYPE_NAME state
// -------------------------
bool Tokenizer::state_after_doctype_name() {
    if (consume_case_insensitive("PUBLIC")) {
        state_ = AFTER_DOCTYPE_PUBLIC_KEYWORD;
        return false;
    }
    if (consume_case_insensitive("SYSTEM")) {
        state_ = AFTER_DOCTYPE_SYSTEM_KEYWORD;
        return false;
    }
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) continue;
        if (c == '>') {
            emit_doctype();
            state_ = DATA;
            return false;
        }
        emit_error("missing-whitespace-after-doctype-name");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// BOGUS_DOCTYPE state
// -------------------------
bool Tokenizer::state_bogus_doctype() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '>') {
            emit_doctype();
            state_ = DATA;
            return false;
        }
    }
}

// -------------------------
// AFTER_DOCTYPE_PUBLIC_KEYWORD state
// -------------------------
bool Tokenizer::state_after_doctype_public_keyword() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("missing-quote-before-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            state_ = BEFORE_DOCTYPE_PUBLIC_IDENTIFIER;
            return false;
        }
        if (c == '"') {
            emit_error("missing-whitespace-before-doctype-public-identifier");
            current_doctype_public_ = std::string();
            state_ = DOCTYPE_PUBLIC_IDENTIFIER_DOUBLE_QUOTED;
            return false;
        }
        if (c == '\'') {
            emit_error("missing-whitespace-before-doctype-public-identifier");
            current_doctype_public_ = std::string();
            state_ = DOCTYPE_PUBLIC_IDENTIFIER_SINGLE_QUOTED;
            return false;
        }
        if (c == '>') {
            emit_error("missing-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        emit_error("unexpected-character-after-doctype-public-keyword");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// AFTER_DOCTYPE_SYSTEM_KEYWORD state
// -------------------------
bool Tokenizer::state_after_doctype_system_keyword() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("missing-quote-before-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            state_ = BEFORE_DOCTYPE_SYSTEM_IDENTIFIER;
            return false;
        }
        if (c == '"') {
            emit_error("missing-whitespace-after-doctype-public-identifier");
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED;
            return false;
        }
        if (c == '\'') {
            emit_error("missing-whitespace-after-doctype-public-identifier");
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED;
            return false;
        }
        if (c == '>') {
            emit_error("missing-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        emit_error("unexpected-character-after-doctype-system-keyword");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// BEFORE_DOCTYPE_PUBLIC_IDENTIFIER state
// -------------------------
bool Tokenizer::state_before_doctype_public_identifier() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("missing-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) continue;
        if (c == '"') {
            current_doctype_public_ = std::string();
            state_ = DOCTYPE_PUBLIC_IDENTIFIER_DOUBLE_QUOTED;
            return false;
        }
        if (c == '\'') {
            current_doctype_public_ = std::string();
            state_ = DOCTYPE_PUBLIC_IDENTIFIER_SINGLE_QUOTED;
            return false;
        }
        if (c == '>') {
            emit_error("missing-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        emit_error("missing-quote-before-doctype-public-identifier");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// DOCTYPE_PUBLIC_IDENTIFIER_DOUBLE_QUOTED state
// -------------------------
bool Tokenizer::state_doctype_public_identifier_double_quoted() {
    if (!current_doctype_public_.has_value()) {
        current_doctype_public_ = std::string();
    }
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '"') {
            state_ = AFTER_DOCTYPE_PUBLIC_IDENTIFIER;
            return false;
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_doctype_public_->append(REPLACEMENT_CHAR);
            continue;
        }
        if (c == '>') {
            emit_error("abrupt-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        current_doctype_public_->push_back(c);
    }
}

// -------------------------
// DOCTYPE_PUBLIC_IDENTIFIER_SINGLE_QUOTED state
// -------------------------
bool Tokenizer::state_doctype_public_identifier_single_quoted() {
    if (!current_doctype_public_.has_value()) {
        current_doctype_public_ = std::string();
    }
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '\'') {
            state_ = AFTER_DOCTYPE_PUBLIC_IDENTIFIER;
            return false;
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_doctype_public_->append(REPLACEMENT_CHAR);
            continue;
        }
        if (c == '>') {
            emit_error("abrupt-doctype-public-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        current_doctype_public_->push_back(c);
    }
}

// -------------------------
// AFTER_DOCTYPE_PUBLIC_IDENTIFIER state
// -------------------------
bool Tokenizer::state_after_doctype_public_identifier() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("missing-whitespace-between-doctype-public-and-system-identifiers");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            state_ = BETWEEN_DOCTYPE_PUBLIC_AND_SYSTEM_IDENTIFIERS;
            return false;
        }
        if (c == '>') {
            emit_doctype();
            state_ = DATA;
            return false;
        }
        if (c == '"') {
            emit_error("missing-whitespace-between-doctype-public-and-system-identifiers");
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED;
            return false;
        }
        if (c == '\'') {
            emit_error("missing-whitespace-between-doctype-public-and-system-identifiers");
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED;
            return false;
        }
        emit_error("unexpected-character-after-doctype-public-identifier");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// BETWEEN_DOCTYPE_PUBLIC_AND_SYSTEM_IDENTIFIERS state
// -------------------------
bool Tokenizer::state_between_doctype_public_and_system_identifiers() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("missing-quote-before-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) continue;
        if (c == '>') {
            emit_doctype();
            state_ = DATA;
            return false;
        }
        if (c == '"') {
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED;
            return false;
        }
        if (c == '\'') {
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED;
            return false;
        }
        emit_error("missing-quote-before-doctype-system-identifier");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// BEFORE_DOCTYPE_SYSTEM_IDENTIFIER state
// -------------------------
bool Tokenizer::state_before_doctype_system_identifier() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("missing-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) continue;
        if (c == '"') {
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED;
            return false;
        }
        if (c == '\'') {
            current_doctype_system_ = std::string();
            state_ = DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED;
            return false;
        }
        if (c == '>') {
            emit_error("missing-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        emit_error("missing-quote-before-doctype-system-identifier");
        current_doctype_force_quirks_ = true;
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED state
// -------------------------
bool Tokenizer::state_doctype_system_identifier_double_quoted() {
    if (!current_doctype_system_.has_value()) {
        current_doctype_system_ = std::string();
    }
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '"') {
            state_ = AFTER_DOCTYPE_SYSTEM_IDENTIFIER;
            return false;
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_doctype_system_->append(REPLACEMENT_CHAR);
            continue;
        }
        if (c == '>') {
            emit_error("abrupt-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        current_doctype_system_->push_back(c);
    }
}

// -------------------------
// DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED state
// -------------------------
bool Tokenizer::state_doctype_system_identifier_single_quoted() {
    if (!current_doctype_system_.has_value()) {
        current_doctype_system_ = std::string();
    }
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == '\'') {
            state_ = AFTER_DOCTYPE_SYSTEM_IDENTIFIER;
            return false;
        }
        if (c == '\0') {
            emit_error("unexpected-null-character");
            current_doctype_system_->append(REPLACEMENT_CHAR);
            continue;
        }
        if (c == '>') {
            emit_error("abrupt-doctype-system-identifier");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            state_ = DATA;
            return false;
        }
        current_doctype_system_->push_back(c);
    }
}

// -------------------------
// AFTER_DOCTYPE_SYSTEM_IDENTIFIER state
// -------------------------
bool Tokenizer::state_after_doctype_system_identifier() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-doctype");
            current_doctype_force_quirks_ = true;
            emit_doctype();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) continue;
        if (c == '>') {
            emit_doctype();
            state_ = DATA;
            return false;
        }
        emit_error("unexpected-character-after-doctype-system-identifier");
        reconsume_current();
        state_ = BOGUS_DOCTYPE;
        return false;
    }
}

// -------------------------
// CDATA_SECTION state
// -------------------------
bool Tokenizer::state_cdata_section() {
    while (true) {
        auto c_opt = get_char();
        if (!c_opt.has_value()) {
            emit_error("eof-in-cdata");
            flush_text();
            emit_token(EOFToken());
            return true;
        }
        char c = static_cast<char>(c_opt.value());
        if (c == ']') {
            state_ = CDATA_SECTION_BRACKET;
            return false;
        }
        append_text(std::string(1, c));
    }
}

// -------------------------
// CDATA_SECTION_BRACKET state
// -------------------------
bool Tokenizer::state_cdata_section_bracket() {
    auto c_opt = get_char();
    if (c_opt.has_value() && static_cast<char>(c_opt.value()) == ']') {
        state_ = CDATA_SECTION_END;
        return false;
    }
    append_text("]");
    if (!c_opt.has_value()) {
        emit_error("eof-in-cdata");
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    reconsume_current();
    state_ = CDATA_SECTION;
    return false;
}

// -------------------------
// CDATA_SECTION_END state
// -------------------------
bool Tokenizer::state_cdata_section_end() {
    auto c_opt = get_char();
    if (c_opt.has_value() && static_cast<char>(c_opt.value()) == '>') {
        flush_text();
        state_ = DATA;
        return false;
    }
    append_text("]");
    if (!c_opt.has_value()) {
        append_text("]");
        emit_error("eof-in-cdata");
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == ']') {
        // Still might be ]]> sequence, stay in CDATA_SECTION_END
        return false;
    }
    append_text("]");
    reconsume_current();
    state_ = CDATA_SECTION;
    return false;
}

// -------------------------
// RCDATA state
// -------------------------
bool Tokenizer::state_rcdata() {
    size_t pos = pos_;
    while (true) {
        if (reconsume_) {
            reconsume_ = false;
            if (!current_char_.has_value()) {
                flush_text();
                emit_token(EOFToken());
                return true;
            }
            if (pos > 0) --pos;
            pos_ = pos;
        }

        // Find the nearest special character: '<', '&', or '\0'
        size_t lt_index = buffer_.find('<', pos);
        size_t amp_index = buffer_.find('&', pos);
        size_t null_index = buffer_.find('\0', pos);

        size_t next_special = length_;
        if (lt_index != std::string::npos && lt_index < next_special) next_special = lt_index;
        if (amp_index != std::string::npos && amp_index < next_special) next_special = amp_index;
        if (null_index != std::string::npos && null_index < next_special) next_special = null_index;

        // Consume everything up to the special character
        if (next_special > pos) {
            append_text(buffer_.substr(pos, next_special - pos));
            pos = next_special;
            pos_ = pos;
        }

        // Handle EOF
        if (pos >= length_) {
            flush_text();
            emit_token(EOFToken());
            return true;
        }

        // Handle special characters
        if (null_index == pos) {
            emit_error("unexpected-null-character");
            append_text(std::string(REPLACEMENT_CHAR));
            pos++;
            pos_ = pos;
        } else if (amp_index == pos) {
            append_text("&");
            pos++;
            pos_ = pos;
        } else {
            // lt_index == pos
            pos++;
            pos_ = pos;
            state_ = RCDATA_LESS_THAN_SIGN;
            return false;
        }
    }
}

// -------------------------
// RCDATA_LESS_THAN_SIGN state
// -------------------------
bool Tokenizer::state_rcdata_less_than_sign() {
    auto c_opt = get_char();
    if (c_opt.has_value() && static_cast<char>(c_opt.value()) == '/') {
        current_tag_name_.clear();
        state_ = RCDATA_END_TAG_OPEN;
        return false;
    }
    append_text("<");
    reconsume_current();
    state_ = RCDATA;
    return false;
}

// -------------------------
// RCDATA_END_TAG_OPEN state
// -------------------------
bool Tokenizer::state_rcdata_end_tag_open() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_alpha(c)) {
            current_tag_name_.push_back(to_lower(c));
            original_tag_name_.push_back(c);
            state_ = RCDATA_END_TAG_NAME;
            return false;
        }
    }
    text_buffer_ += "</";
    reconsume_current();
    state_ = RCDATA;
    return false;
}

// -------------------------
// RCDATA_END_TAG_NAME state
// -------------------------
bool Tokenizer::state_rcdata_end_tag_name() {
    while (true) {
        auto c_opt = get_char();
        if (c_opt.has_value()) {
            char c = static_cast<char>(c_opt.value());
            if (is_ascii_alpha(c)) {
                current_tag_name_.push_back(to_lower(c));
                original_tag_name_.push_back(c);
                continue;
            }
        }
        // End of tag name - check if it matches
        std::string tag_name = current_tag_name_;
        if (rawtext_tag_name_.has_value() && tag_name == rawtext_tag_name_.value()) {
            if (c_opt.has_value()) {
                char c = static_cast<char>(c_opt.value());
                if (c == '>') {
                    Attributes attrs;
                    Tag tag(TagKind::End, tag_name, std::move(attrs), false);
                    flush_text();
                    emit_token(tag);
                    state_ = DATA;
                    rawtext_tag_name_ = std::nullopt;
                    original_tag_name_.clear();
                    return false;
                }
                if (is_ascii_ws(c)) {
                    current_tag_kind_ = TagKind::End;
                    current_tag_attrs_.clear();
                    state_ = BEFORE_ATTRIBUTE_NAME;
                    return false;
                }
                if (c == '/') {
                    flush_text();
                    current_tag_kind_ = TagKind::End;
                    current_tag_attrs_.clear();
                    state_ = SELF_CLOSING_START_TAG;
                    return false;
                }
            }
        }
        // Not a matching end tag or EOF
        if (!c_opt.has_value()) {
            text_buffer_ += "</";
            text_buffer_ += original_tag_name_;
            current_tag_name_.clear();
            original_tag_name_.clear();
            flush_text();
            emit_token(EOFToken());
            return true;
        }
        // Not a matching end tag - emit as text
        text_buffer_ += "</";
        text_buffer_ += original_tag_name_;
        current_tag_name_.clear();
        original_tag_name_.clear();
        reconsume_current();
        state_ = RCDATA;
        return false;
    }
}

// -------------------------
// RAWTEXT state
// -------------------------
bool Tokenizer::state_rawtext() {
    size_t pos = pos_;
    while (true) {
        if (reconsume_) {
            reconsume_ = false;
            if (!current_char_.has_value()) {
                flush_text();
                emit_token(EOFToken());
                return true;
            }
            if (pos > 0) --pos;
            pos_ = pos;
        }

        // Find next '<' or '\0'
        size_t lt_index = buffer_.find('<', pos);
        size_t null_index = buffer_.find('\0', pos);
        size_t next_special = (lt_index != std::string::npos) ? lt_index : length_;
        if (null_index != std::string::npos && null_index < next_special) {
            // Handle null before '<'
            if (null_index > pos) {
                append_text(buffer_.substr(pos, null_index - pos));
            }
            emit_error("unexpected-null-character");
            append_text(std::string(REPLACEMENT_CHAR));
            pos = null_index + 1;
            pos_ = pos;
            continue;
        }
        if (lt_index == std::string::npos) {
            if (pos < length_) {
                append_text(buffer_.substr(pos, length_ - pos));
            }
            pos_ = length_;
            flush_text();
            emit_token(EOFToken());
            return true;
        }
        if (lt_index > pos) {
            append_text(buffer_.substr(pos, lt_index - pos));
        }
        pos = lt_index + 1;
        pos_ = pos;

        // Handle script escaped transition before treating '<' as markup boundary
        if (rawtext_tag_name_.has_value() && rawtext_tag_name_.value() == "script") {
            auto next1 = peek_char(0);
            auto next2 = peek_char(1);
            auto next3 = peek_char(2);
            if (next1.has_value() && static_cast<char>(next1.value()) == '!' &&
                next2.has_value() && static_cast<char>(next2.value()) == '-' &&
                next3.has_value() && static_cast<char>(next3.value()) == '-') {
                text_buffer_ += "<!--";
                get_char();  // consume '!'
                get_char();  // consume '-'
                get_char();  // consume '-'
                state_ = SCRIPT_DATA_ESCAPED;
                return false;
            }
        }
        state_ = RAWTEXT_LESS_THAN_SIGN;
        return false;
    }
}

// -------------------------
// RAWTEXT_LESS_THAN_SIGN state
// -------------------------
bool Tokenizer::state_rawtext_less_than_sign() {
    auto c_opt = get_char();
    if (c_opt.has_value() && static_cast<char>(c_opt.value()) == '/') {
        current_tag_name_.clear();
        state_ = RAWTEXT_END_TAG_OPEN;
        return false;
    }
    append_text("<");
    reconsume_current();
    state_ = RAWTEXT;
    return false;
}

// -------------------------
// RAWTEXT_END_TAG_OPEN state
// -------------------------
bool Tokenizer::state_rawtext_end_tag_open() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_alpha(c)) {
            current_tag_name_.push_back(to_lower(c));
            original_tag_name_.push_back(c);
            state_ = RAWTEXT_END_TAG_NAME;
            return false;
        }
    }
    text_buffer_ += "</";
    reconsume_current();
    state_ = RAWTEXT;
    return false;
}

// -------------------------
// RAWTEXT_END_TAG_NAME state
// -------------------------
bool Tokenizer::state_rawtext_end_tag_name() {
    while (true) {
        auto c_opt = get_char();
        if (c_opt.has_value()) {
            char c = static_cast<char>(c_opt.value());
            if (is_ascii_alpha(c)) {
                current_tag_name_.push_back(to_lower(c));
                original_tag_name_.push_back(c);
                continue;
            }
        }
        // End of tag name - check if it matches
        std::string tag_name = current_tag_name_;
        if (rawtext_tag_name_.has_value() && tag_name == rawtext_tag_name_.value()) {
            if (c_opt.has_value()) {
                char c = static_cast<char>(c_opt.value());
                if (c == '>') {
                    Attributes attrs;
                    Tag tag(TagKind::End, tag_name, std::move(attrs), false);
                    flush_text();
                    emit_token(tag);
                    state_ = DATA;
                    rawtext_tag_name_ = std::nullopt;
                    original_tag_name_.clear();
                    return false;
                }
                if (is_ascii_ws(c)) {
                    current_tag_kind_ = TagKind::End;
                    current_tag_attrs_.clear();
                    state_ = BEFORE_ATTRIBUTE_NAME;
                    return false;
                }
                if (c == '/') {
                    flush_text();
                    current_tag_kind_ = TagKind::End;
                    current_tag_attrs_.clear();
                    state_ = SELF_CLOSING_START_TAG;
                    return false;
                }
            }
        }
        // Not a matching end tag or EOF
        if (!c_opt.has_value()) {
            text_buffer_ += "</";
            text_buffer_ += original_tag_name_;
            current_tag_name_.clear();
            original_tag_name_.clear();
            flush_text();
            emit_token(EOFToken());
            return true;
        }
        // Not a matching end tag - emit as text
        text_buffer_ += "</";
        text_buffer_ += original_tag_name_;
        current_tag_name_.clear();
        original_tag_name_.clear();
        reconsume_current();
        state_ = RAWTEXT;
        return false;
    }
}

// -------------------------
// PLAINTEXT state
// -------------------------
bool Tokenizer::state_plaintext() {
    if (pos_ < length_) {
        std::string remaining = buffer_.substr(pos_);
        // Replace null bytes with replacement character
        bool has_null = false;
        for (size_t i = 0; i < remaining.size(); ++i) {
            if (remaining[i] == '\0') {
                has_null = true;
                break;
            }
        }
        if (has_null) {
            std::string cleaned;
            cleaned.reserve(remaining.size());
            for (char c : remaining) {
                if (c == '\0') {
                    cleaned.append(REPLACEMENT_CHAR);
                } else {
                    cleaned.push_back(c);
                }
            }
            remaining = std::move(cleaned);
            emit_error("unexpected-null-character");
        }
        append_text(remaining);
        pos_ = length_;
    }
    flush_text();
    emit_token(EOFToken());
    return true;
}

// -------------------------
// SCRIPT_DATA_ESCAPED state
// -------------------------
bool Tokenizer::state_script_data_escaped() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        append_text("-");
        state_ = SCRIPT_DATA_ESCAPED_DASH;
        return false;
    }
    if (c == '<') {
        state_ = SCRIPT_DATA_ESCAPED_LESS_THAN_SIGN;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        append_text(std::string(REPLACEMENT_CHAR));
        return false;
    }
    append_text(std::string(1, c));
    return false;
}

// -------------------------
// SCRIPT_DATA_ESCAPED_DASH state
// -------------------------
bool Tokenizer::state_script_data_escaped_dash() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        append_text("-");
        state_ = SCRIPT_DATA_ESCAPED_DASH_DASH;
        return false;
    }
    if (c == '<') {
        state_ = SCRIPT_DATA_ESCAPED_LESS_THAN_SIGN;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        append_text(std::string(REPLACEMENT_CHAR));
        state_ = SCRIPT_DATA_ESCAPED;
        return false;
    }
    append_text(std::string(1, c));
    state_ = SCRIPT_DATA_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_ESCAPED_DASH_DASH state
// -------------------------
bool Tokenizer::state_script_data_escaped_dash_dash() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        append_text("-");
        return false;
    }
    if (c == '<') {
        append_text("<");
        state_ = SCRIPT_DATA_ESCAPED_LESS_THAN_SIGN;
        return false;
    }
    if (c == '>') {
        append_text(">");
        state_ = RAWTEXT;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        append_text(std::string(REPLACEMENT_CHAR));
        state_ = SCRIPT_DATA_ESCAPED;
        return false;
    }
    append_text(std::string(1, c));
    state_ = SCRIPT_DATA_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_ESCAPED_LESS_THAN_SIGN state
// -------------------------
bool Tokenizer::state_script_data_escaped_less_than_sign() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (c == '/') {
            temp_buffer_.clear();
            state_ = SCRIPT_DATA_ESCAPED_END_TAG_OPEN;
            return false;
        }
        if (is_ascii_alpha(c)) {
            temp_buffer_.clear();
            append_text("<");
            reconsume_current();
            state_ = SCRIPT_DATA_DOUBLE_ESCAPE_START;
            return false;
        }
    }
    append_text("<");
    reconsume_current();
    state_ = SCRIPT_DATA_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_ESCAPED_END_TAG_OPEN state
// -------------------------
bool Tokenizer::state_script_data_escaped_end_tag_open() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_alpha(c)) {
            current_tag_name_.clear();
            original_tag_name_.clear();
            reconsume_current();
            state_ = SCRIPT_DATA_ESCAPED_END_TAG_NAME;
            return false;
        }
    }
    text_buffer_ += "</";
    reconsume_current();
    state_ = SCRIPT_DATA_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_ESCAPED_END_TAG_NAME state
// -------------------------
bool Tokenizer::state_script_data_escaped_end_tag_name() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_alpha(c)) {
            current_tag_name_.push_back(to_lower(c));
            original_tag_name_.push_back(c);
            temp_buffer_.push_back(c);
            return false;
        }
    }
    // Check if this is an appropriate end tag
    std::string tag_name = current_tag_name_;
    bool is_appropriate = rawtext_tag_name_.has_value() && tag_name == rawtext_tag_name_.value();

    if (is_appropriate && c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c)) {
            current_tag_kind_ = TagKind::End;
            current_tag_attrs_.clear();
            state_ = BEFORE_ATTRIBUTE_NAME;
            return false;
        }
        if (c == '/') {
            flush_text();
            current_tag_kind_ = TagKind::End;
            current_tag_attrs_.clear();
            state_ = SELF_CLOSING_START_TAG;
            return false;
        }
        if (c == '>') {
            flush_text();
            Attributes attrs;
            Tag tag(TagKind::End, tag_name, std::move(attrs), false);
            emit_token(tag);
            state_ = DATA;
            rawtext_tag_name_ = std::nullopt;
            current_tag_name_.clear();
            original_tag_name_.clear();
            return false;
        }
    }
    // Not an appropriate end tag
    text_buffer_ += "</";
    text_buffer_ += temp_buffer_;
    reconsume_current();
    state_ = SCRIPT_DATA_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_DOUBLE_ESCAPE_START state
// -------------------------
bool Tokenizer::state_script_data_double_escape_start() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c) || c == '/' || c == '>') {
            std::string temp = to_lower_ascii(temp_buffer_);
            if (temp == "script") {
                state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
            } else {
                state_ = SCRIPT_DATA_ESCAPED;
            }
            append_text(std::string(1, c));
            return false;
        }
        if (is_ascii_alpha(c)) {
            temp_buffer_.push_back(c);
            append_text(std::string(1, c));
            return false;
        }
    }
    reconsume_current();
    state_ = SCRIPT_DATA_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_DOUBLE_ESCAPED state
// -------------------------
bool Tokenizer::state_script_data_double_escaped() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        append_text("-");
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED_DASH;
        return false;
    }
    if (c == '<') {
        append_text("<");
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED_LESS_THAN_SIGN;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        append_text(std::string(REPLACEMENT_CHAR));
        return false;
    }
    append_text(std::string(1, c));
    return false;
}

// -------------------------
// SCRIPT_DATA_DOUBLE_ESCAPED_DASH state
// -------------------------
bool Tokenizer::state_script_data_double_escaped_dash() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        append_text("-");
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED_DASH_DASH;
        return false;
    }
    if (c == '<') {
        append_text("<");
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED_LESS_THAN_SIGN;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        append_text(std::string(REPLACEMENT_CHAR));
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
        return false;
    }
    append_text(std::string(1, c));
    state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_DOUBLE_ESCAPED_DASH_DASH state
// -------------------------
bool Tokenizer::state_script_data_double_escaped_dash_dash() {
    auto c_opt = get_char();
    if (!c_opt.has_value()) {
        flush_text();
        emit_token(EOFToken());
        return true;
    }
    char c = static_cast<char>(c_opt.value());
    if (c == '-') {
        append_text("-");
        return false;
    }
    if (c == '<') {
        append_text("<");
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED_LESS_THAN_SIGN;
        return false;
    }
    if (c == '>') {
        append_text(">");
        state_ = RAWTEXT;
        return false;
    }
    if (c == '\0') {
        emit_error("unexpected-null-character");
        append_text(std::string(REPLACEMENT_CHAR));
        state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
        return false;
    }
    append_text(std::string(1, c));
    state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_DOUBLE_ESCAPED_LESS_THAN_SIGN state
// -------------------------
bool Tokenizer::state_script_data_double_escaped_less_than_sign() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (c == '/') {
            temp_buffer_.clear();
            append_text("/");
            state_ = SCRIPT_DATA_DOUBLE_ESCAPE_END;
            return false;
        }
        if (is_ascii_alpha(c)) {
            temp_buffer_.clear();
            reconsume_current();
            state_ = SCRIPT_DATA_DOUBLE_ESCAPE_START;
            return false;
        }
    }
    reconsume_current();
    state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
    return false;
}

// -------------------------
// SCRIPT_DATA_DOUBLE_ESCAPE_END state
// -------------------------
bool Tokenizer::state_script_data_double_escape_end() {
    auto c_opt = get_char();
    if (c_opt.has_value()) {
        char c = static_cast<char>(c_opt.value());
        if (is_ascii_ws(c) || c == '/' || c == '>') {
            std::string temp = to_lower_ascii(temp_buffer_);
            if (temp == "script") {
                state_ = SCRIPT_DATA_ESCAPED;
            } else {
                state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
            }
            append_text(std::string(1, c));
            return false;
        }
        if (is_ascii_alpha(c)) {
            temp_buffer_.push_back(c);
            append_text(std::string(1, c));
            return false;
        }
    }
    reconsume_current();
    state_ = SCRIPT_DATA_DOUBLE_ESCAPED;
    return false;
}

}  // namespace justhtml
