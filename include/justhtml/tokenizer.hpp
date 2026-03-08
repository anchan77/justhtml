#pragma once

/// @file tokenizer.hpp
/// @brief HTML5 tokenizer state machine.
///
/// Implements the WHATWG HTML5 tokenizer with 61 states, consuming input
/// character-by-character and emitting tokens via a TokenSink interface.

#include <functional>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_set>
#include <vector>

#include "justhtml/token_sink.hpp"
#include "justhtml/tokens.hpp"

namespace justhtml {

/// Configuration options for the tokenizer.
struct TokenizerOpts {
    bool exact_errors = false;
    bool discard_bom = true;
    std::optional<int> initial_state;
    std::optional<std::string> initial_rawtext_tag;
    bool xml_coercion = false;

    TokenizerOpts() = default;
};

/// @brief HTML5 tokenizer state machine.
///
/// Processes input HTML one character at a time, emitting tokens (Tag,
/// CharacterTokens, CommentToken, DoctypeToken, EOFToken) via a TokenSink.
class Tokenizer {
public:
    // Tokenizer state constants
    enum State : int {
        DATA = 0,
        TAG_OPEN = 1,
        END_TAG_OPEN = 2,
        TAG_NAME = 3,
        BEFORE_ATTRIBUTE_NAME = 4,
        ATTRIBUTE_NAME = 5,
        AFTER_ATTRIBUTE_NAME = 6,
        BEFORE_ATTRIBUTE_VALUE = 7,
        ATTRIBUTE_VALUE_DOUBLE = 8,
        ATTRIBUTE_VALUE_SINGLE = 9,
        ATTRIBUTE_VALUE_UNQUOTED = 10,
        AFTER_ATTRIBUTE_VALUE_QUOTED = 11,
        SELF_CLOSING_START_TAG = 12,
        MARKUP_DECLARATION_OPEN = 13,
        COMMENT_START = 14,
        COMMENT_START_DASH = 15,
        COMMENT_ = 16,  // COMMENT is a macro on some platforms
        COMMENT_END_DASH = 17,
        COMMENT_END = 18,
        COMMENT_END_BANG = 19,
        BOGUS_COMMENT = 20,
        DOCTYPE_ = 21,
        BEFORE_DOCTYPE_NAME = 22,
        DOCTYPE_NAME = 23,
        AFTER_DOCTYPE_NAME = 24,
        BOGUS_DOCTYPE = 25,
        AFTER_DOCTYPE_PUBLIC_KEYWORD = 26,
        AFTER_DOCTYPE_SYSTEM_KEYWORD = 27,
        BEFORE_DOCTYPE_PUBLIC_IDENTIFIER = 28,
        DOCTYPE_PUBLIC_IDENTIFIER_DOUBLE_QUOTED = 29,
        DOCTYPE_PUBLIC_IDENTIFIER_SINGLE_QUOTED = 30,
        AFTER_DOCTYPE_PUBLIC_IDENTIFIER = 31,
        BETWEEN_DOCTYPE_PUBLIC_AND_SYSTEM_IDENTIFIERS = 32,
        BEFORE_DOCTYPE_SYSTEM_IDENTIFIER = 33,
        DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED = 34,
        DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED = 35,
        AFTER_DOCTYPE_SYSTEM_IDENTIFIER = 36,
        CDATA_SECTION = 37,
        CDATA_SECTION_BRACKET = 38,
        CDATA_SECTION_END = 39,
        RCDATA = 40,
        RCDATA_LESS_THAN_SIGN = 41,
        RCDATA_END_TAG_OPEN = 42,
        RCDATA_END_TAG_NAME = 43,
        RAWTEXT = 44,
        RAWTEXT_LESS_THAN_SIGN = 45,
        RAWTEXT_END_TAG_OPEN = 46,
        RAWTEXT_END_TAG_NAME = 47,
        PLAINTEXT = 48,
        SCRIPT_DATA_ESCAPED = 49,
        SCRIPT_DATA_ESCAPED_DASH = 50,
        SCRIPT_DATA_ESCAPED_DASH_DASH = 51,
        SCRIPT_DATA_ESCAPED_LESS_THAN_SIGN = 52,
        SCRIPT_DATA_ESCAPED_END_TAG_OPEN = 53,
        SCRIPT_DATA_ESCAPED_END_TAG_NAME = 54,
        SCRIPT_DATA_DOUBLE_ESCAPE_START = 55,
        SCRIPT_DATA_DOUBLE_ESCAPED = 56,
        SCRIPT_DATA_DOUBLE_ESCAPED_DASH = 57,
        SCRIPT_DATA_DOUBLE_ESCAPED_DASH_DASH = 58,
        SCRIPT_DATA_DOUBLE_ESCAPED_LESS_THAN_SIGN = 59,
        SCRIPT_DATA_DOUBLE_ESCAPE_END = 60,
        NUM_STATES = 61,
    };

    /// Construct a tokenizer with a sink and optional configuration.
    Tokenizer(TokenSink& sink, const TokenizerOpts& opts = {},
              bool collect_errors = false);

    /// Initialize the tokenizer with input HTML.
    void initialize(const std::string& html);

    /// Run one step of the state machine. Returns true if EOF reached.
    bool step();

    /// Run the tokenizer to completion on the given HTML.
    void run(const std::string& html);

    /// Access collected parse errors.
    const std::vector<ParseError>& get_errors() const { return errors_; }

    /// Get current state (for testing).
    int get_state() const { return state_; }

    /// Set state (for testing/fragment parsing).
    void set_state(int s) { state_ = s; }

    /// Get last token line (for error reporting).
    int last_token_line() const { return last_token_line_; }

    /// Get last token column.
    int last_token_column() const { return last_token_column_; }

private:
    // Character consumption
    std::optional<char32_t> get_char();
    std::optional<char32_t> peek_char(int offset) const;
    void reconsume_current();
    void append_text(const std::string& text);
    void append_text_char(char32_t ch);
    void flush_text();
    void append_attr_value_char(char32_t ch);
    void finish_attribute();
    bool emit_current_tag();
    void emit_comment();
    void emit_doctype();
    void emit_token(const Token& token);
    void emit_error(const std::string& code);
    bool consume_if(const std::string& literal);
    bool consume_case_insensitive(const std::string& literal);
    bool consume_comment_run();
    int get_line_at_pos(size_t pos) const;
    void record_token_position();
    void record_text_end_position(size_t raw_len);

    // UTF-8 helpers
    static void append_codepoint(std::string& out, char32_t cp);
    static char32_t char_at(const std::string& buf, size_t pos);
    static size_t char_len(const std::string& buf, size_t pos);
    static std::string to_lower_ascii(const std::string& s);

    // XML coercion
    std::string coerce_text_for_xml(const std::string& text);
    std::string coerce_comment_for_xml(const std::string& text);

    // State handlers
    bool state_data();
    bool state_tag_open();
    bool state_end_tag_open();
    bool state_tag_name();
    bool state_before_attribute_name();
    bool state_attribute_name();
    bool state_after_attribute_name();
    bool state_before_attribute_value();
    bool state_attribute_value_double();
    bool state_attribute_value_single();
    bool state_attribute_value_unquoted();
    bool state_after_attribute_value_quoted();
    bool state_self_closing_start_tag();
    bool state_markup_declaration_open();
    bool state_comment_start();
    bool state_comment_start_dash();
    bool state_comment();
    bool state_comment_end_dash();
    bool state_comment_end();
    bool state_comment_end_bang();
    bool state_bogus_comment();
    bool state_doctype();
    bool state_before_doctype_name();
    bool state_doctype_name();
    bool state_after_doctype_name();
    bool state_bogus_doctype();
    bool state_after_doctype_public_keyword();
    bool state_after_doctype_system_keyword();
    bool state_before_doctype_public_identifier();
    bool state_doctype_public_identifier_double_quoted();
    bool state_doctype_public_identifier_single_quoted();
    bool state_after_doctype_public_identifier();
    bool state_between_doctype_public_and_system_identifiers();
    bool state_before_doctype_system_identifier();
    bool state_doctype_system_identifier_double_quoted();
    bool state_doctype_system_identifier_single_quoted();
    bool state_after_doctype_system_identifier();
    bool state_cdata_section();
    bool state_cdata_section_bracket();
    bool state_cdata_section_end();
    bool state_rcdata();
    bool state_rcdata_less_than_sign();
    bool state_rcdata_end_tag_open();
    bool state_rcdata_end_tag_name();
    bool state_rawtext();
    bool state_rawtext_less_than_sign();
    bool state_rawtext_end_tag_open();
    bool state_rawtext_end_tag_name();
    bool state_plaintext();
    bool state_script_data_escaped();
    bool state_script_data_escaped_dash();
    bool state_script_data_escaped_dash_dash();
    bool state_script_data_escaped_less_than_sign();
    bool state_script_data_escaped_end_tag_open();
    bool state_script_data_escaped_end_tag_name();
    bool state_script_data_double_escape_start();
    bool state_script_data_double_escaped();
    bool state_script_data_double_escaped_dash();
    bool state_script_data_double_escaped_dash_dash();
    bool state_script_data_double_escaped_less_than_sign();
    bool state_script_data_double_escape_end();

    // Dispatch table
    using StateHandler = bool (Tokenizer::*)();
    static const StateHandler STATE_HANDLERS[NUM_STATES];

    // Members
    TokenSink& sink_;
    TokenizerOpts opts_;
    bool collect_errors_;
    std::vector<ParseError> errors_;

    int state_ = DATA;
    std::string buffer_;
    size_t length_ = 0;
    size_t pos_ = 0;
    bool reconsume_ = false;
    std::optional<char32_t> current_char_;
    int last_token_line_ = 1;
    int last_token_column_ = 0;

    // Reusable buffers
    std::string text_buffer_;
    size_t text_start_pos_ = 0;
    std::string current_tag_name_;
    Attributes current_tag_attrs_;
    std::string current_attr_name_;
    std::string current_attr_value_;
    bool current_attr_value_has_amp_ = false;
    bool current_tag_self_closing_ = false;
    TagKind current_tag_kind_ = TagKind::Start;
    std::string current_comment_;
    std::string current_doctype_name_;
    std::optional<std::string> current_doctype_public_;
    std::optional<std::string> current_doctype_system_;
    bool current_doctype_force_quirks_ = false;
    std::optional<std::string> last_start_tag_name_;
    std::optional<std::string> rawtext_tag_name_;
    std::string original_tag_name_;
    std::string temp_buffer_;

    // Pre-computed newline positions for error reporting
    std::optional<std::vector<size_t>> newline_positions_;

    // RCDATA/RAWTEXT element sets
    static const std::unordered_set<std::string> RCDATA_ELEMENTS;
    static const std::unordered_set<std::string> RAWTEXT_SWITCH_TAGS;
};

}  // namespace justhtml
