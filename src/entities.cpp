/// @file entities.cpp
/// @brief HTML5 character entity decoding implementation.

#include "justhtml/entities.hpp"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <string>
#include <unordered_map>
#include <unordered_set>

namespace justhtml {

namespace {

/// Encode a Unicode codepoint to UTF-8.
std::string codepoint_to_utf8(uint32_t cp) {
    std::string result;
    if (cp <= 0x7F) {
        result += static_cast<char>(cp);
    } else if (cp <= 0x7FF) {
        result += static_cast<char>(0xC0 | (cp >> 6));
        result += static_cast<char>(0x80 | (cp & 0x3F));
    } else if (cp <= 0xFFFF) {
        result += static_cast<char>(0xE0 | (cp >> 12));
        result += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
        result += static_cast<char>(0x80 | (cp & 0x3F));
    } else if (cp <= 0x10FFFF) {
        result += static_cast<char>(0xF0 | (cp >> 18));
        result += static_cast<char>(0x80 | ((cp >> 12) & 0x3F));
        result += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
        result += static_cast<char>(0x80 | (cp & 0x3F));
    }
    return result;
}

/// HTML5 numeric character reference replacements (Windows-1252 range).
const std::unordered_map<uint32_t, std::string>& numeric_replacements() {
    static const std::unordered_map<uint32_t, std::string> data = {
        {0x00, "\xef\xbf\xbd"},  // U+FFFD REPLACEMENT CHARACTER
        {0x80, "\xe2\x82\xac"},  // U+20AC EURO SIGN
        {0x82, "\xe2\x80\x9a"},  // U+201A SINGLE LOW-9 QUOTATION MARK
        {0x83, "\xc6\x92"},      // U+0192 LATIN SMALL LETTER F WITH HOOK
        {0x84, "\xe2\x80\x9e"},  // U+201E DOUBLE LOW-9 QUOTATION MARK
        {0x85, "\xe2\x80\xa6"},  // U+2026 HORIZONTAL ELLIPSIS
        {0x86, "\xe2\x80\xa0"},  // U+2020 DAGGER
        {0x87, "\xe2\x80\xa1"},  // U+2021 DOUBLE DAGGER
        {0x88, "\xcb\x86"},      // U+02C6 MODIFIER LETTER CIRCUMFLEX ACCENT
        {0x89, "\xe2\x80\xb0"},  // U+2030 PER MILLE SIGN
        {0x8A, "\xc5\xa0"},      // U+0160 LATIN CAPITAL LETTER S WITH CARON
        {0x8B, "\xe2\x80\xb9"},  // U+2039 SINGLE LEFT-POINTING ANGLE QUOTATION MARK
        {0x8C, "\xc5\x92"},      // U+0152 LATIN CAPITAL LIGATURE OE
        {0x8E, "\xc5\xbd"},      // U+017D LATIN CAPITAL LETTER Z WITH CARON
        {0x91, "\xe2\x80\x98"},  // U+2018 LEFT SINGLE QUOTATION MARK
        {0x92, "\xe2\x80\x99"},  // U+2019 RIGHT SINGLE QUOTATION MARK
        {0x93, "\xe2\x80\x9c"},  // U+201C LEFT DOUBLE QUOTATION MARK
        {0x94, "\xe2\x80\x9d"},  // U+201D RIGHT DOUBLE QUOTATION MARK
        {0x95, "\xe2\x80\xa2"},  // U+2022 BULLET
        {0x96, "\xe2\x80\x93"},  // U+2013 EN DASH
        {0x97, "\xe2\x80\x94"},  // U+2014 EM DASH
        {0x98, "\xcb\x9c"},      // U+02DC SMALL TILDE
        {0x99, "\xe2\x84\xa2"},  // U+2122 TRADE MARK SIGN
        {0x9A, "\xc5\xa1"},      // U+0161 LATIN SMALL LETTER S WITH CARON
        {0x9B, "\xe2\x80\xba"},  // U+203A SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
        {0x9C, "\xc5\x93"},      // U+0153 LATIN SMALL LIGATURE OE
        {0x9E, "\xc5\xbe"},      // U+017E LATIN SMALL LETTER Z WITH CARON
        {0x9F, "\xc5\xb8"},      // U+0178 LATIN CAPITAL LETTER Y WITH DIAERESIS
    };
    return data;
}

/// The complete named entity lookup table (2125 entities).
const std::unordered_map<std::string, std::string>& named_entities() {
    static const std::unordered_map<std::string, std::string> data = {
#include "entities_data.inc"
    };
    return data;
}

/// Legacy entity names that can be used without semicolons.
const std::unordered_set<std::string>& legacy_entities() {
    static const std::unordered_set<std::string> data = {
        "gt",     "lt",     "amp",    "quot",   "nbsp",   "AMP",    "QUOT",   "GT",
        "LT",     "COPY",   "REG",    "AElig",  "Aacute", "Acirc",  "Agrave", "Aring",
        "Atilde", "Auml",   "Ccedil", "ETH",    "Eacute", "Ecirc",  "Egrave", "Euml",
        "Iacute", "Icirc",  "Igrave", "Iuml",   "Ntilde", "Oacute", "Ocirc",  "Ograve",
        "Oslash", "Otilde", "Ouml",   "THORN",  "Uacute", "Ucirc",  "Ugrave", "Uuml",
        "Yacute", "aacute", "acirc",  "acute",  "aelig",  "agrave", "aring",  "atilde",
        "auml",   "brvbar", "ccedil", "cedil",  "cent",   "copy",   "curren", "deg",
        "divide", "eacute", "ecirc",  "egrave", "eth",    "euml",   "frac12", "frac14",
        "frac34", "iacute", "icirc",  "iexcl",  "igrave", "iquest", "iuml",   "laquo",
        "macr",   "micro",  "middot", "not",    "ntilde", "oacute", "ocirc",  "ograve",
        "ordf",   "ordm",   "oslash", "otilde", "ouml",   "para",   "plusmn", "pound",
        "raquo",  "reg",    "sect",   "shy",    "sup1",   "sup2",   "sup3",   "szlig",
        "thorn",  "times",  "uacute", "ucirc",  "ugrave", "uml",    "uuml",   "yacute",
        "yen",    "yuml",
    };
    return data;
}

bool is_hex_digit(char c) {
    return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
}

bool is_alnum(char c) {
    return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

bool is_alpha(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

}  // anonymous namespace

std::string decode_numeric_entity(std::string_view text, bool is_hex) {
    uint32_t codepoint = 0;
    int base = is_hex ? 16 : 10;

    for (char c : text) {
        uint32_t digit;
        if (c >= '0' && c <= '9') {
            digit = c - '0';
        } else if (c >= 'a' && c <= 'f') {
            digit = 10 + (c - 'a');
        } else if (c >= 'A' && c <= 'F') {
            digit = 10 + (c - 'A');
        } else {
            break;
        }
        codepoint = codepoint * base + digit;
        if (codepoint > 0x10FFFF) {
            // Early exit: already too large
            return "\xef\xbf\xbd";  // U+FFFD
        }
    }

    // Apply HTML5 replacements
    const auto& replacements = numeric_replacements();
    auto it = replacements.find(codepoint);
    if (it != replacements.end()) {
        return it->second;
    }

    // Invalid ranges per HTML5 spec
    if (codepoint > 0x10FFFF) {
        return "\xef\xbf\xbd";  // U+FFFD REPLACEMENT CHARACTER
    }
    if (codepoint >= 0xD800 && codepoint <= 0xDFFF) {
        return "\xef\xbf\xbd";  // Surrogate range
    }

    return codepoint_to_utf8(codepoint);
}

std::string lookup_named_entity(const std::string& name) {
    const auto& entities = named_entities();
    auto it = entities.find(name);
    if (it != entities.end()) {
        return it->second;
    }
    return "";
}

bool is_legacy_entity(const std::string& name) {
    return legacy_entities().count(name) > 0;
}

std::string decode_entities_in_text(std::string_view text, bool in_attribute) {
    std::string result;
    result.reserve(text.size());

    size_t i = 0;
    size_t length = text.size();

    while (i < length) {
        // Find next '&'
        size_t next_amp = text.find('&', i);
        if (next_amp == std::string_view::npos) {
            result.append(text.substr(i));
            break;
        }

        if (next_amp > i) {
            result.append(text.substr(i, next_amp - i));
        }

        i = next_amp;
        size_t j = i + 1;

        // Check for numeric entity
        if (j < length && text[j] == '#') {
            j++;
            bool is_hex = false;

            if (j < length && (text[j] == 'x' || text[j] == 'X')) {
                is_hex = true;
                j++;
            }

            // Collect digits
            size_t digit_start = j;
            if (is_hex) {
                while (j < length && is_hex_digit(text[j])) {
                    j++;
                }
            } else {
                while (j < length && text[j] >= '0' && text[j] <= '9') {
                    j++;
                }
            }

            bool has_semicolon = j < length && text[j] == ';';
            std::string_view digit_text = text.substr(digit_start, j - digit_start);

            if (!digit_text.empty()) {
                result.append(decode_numeric_entity(digit_text, is_hex));
                i = has_semicolon ? j + 1 : j;
                continue;
            }

            // Invalid numeric entity, keep as-is
            size_t end = has_semicolon ? j + 1 : j;
            result.append(text.substr(i, end - i));
            i = end;
            continue;
        }

        // Named entity: collect alphanumeric characters
        while (j < length && (is_alpha(text[j]) || (text[j] >= '0' && text[j] <= '9'))) {
            j++;
        }

        std::string entity_name(text.substr(i + 1, j - i - 1));
        bool has_semicolon = j < length && text[j] == ';';

        if (entity_name.empty()) {
            result += '&';
            i++;
            continue;
        }

        const auto& entities = named_entities();
        const auto& legacy = legacy_entities();

        // Try exact match first (with semicolon expected)
        if (has_semicolon) {
            auto it = entities.find(entity_name);
            if (it != entities.end()) {
                result.append(it->second);
                i = j + 1;
                continue;
            }
            // Semicolon present but no exact match, try legacy prefix match in text
            if (!in_attribute) {
                std::string best_match;
                size_t best_match_len = 0;
                for (size_t k = entity_name.size(); k > 0; k--) {
                    std::string prefix = entity_name.substr(0, k);
                    if (legacy.count(prefix) && entities.count(prefix)) {
                        best_match = entities.at(prefix);
                        best_match_len = k;
                        break;
                    }
                }
                if (!best_match.empty()) {
                    result.append(best_match);
                    i = i + 1 + best_match_len;
                    continue;
                }
            }
        }

        // Try without semicolon for legacy compatibility
        if (legacy.count(entity_name) && entities.count(entity_name)) {
            char next_char = (j < length) ? text[j] : '\0';
            if (in_attribute && next_char && (is_alnum(next_char) || next_char == '=')) {
                result += '&';
                i++;
                continue;
            }

            result.append(entities.at(entity_name));
            i = j;
            continue;
        }

        // Try longest prefix match for legacy entities without semicolon
        std::string best_match;
        size_t best_match_len = 0;
        for (size_t k = entity_name.size(); k > 0; k--) {
            std::string prefix = entity_name.substr(0, k);
            if (legacy.count(prefix) && entities.count(prefix)) {
                best_match = entities.at(prefix);
                best_match_len = k;
                break;
            }
        }

        if (!best_match.empty()) {
            if (in_attribute) {
                result += '&';
                i++;
                continue;
            }
            result.append(best_match);
            i = i + 1 + best_match_len;
            continue;
        }

        // No match found
        if (has_semicolon) {
            result.append(text.substr(i, j + 1 - i));
            i = j + 1;
        } else {
            result += '&';
            i++;
        }
    }

    return result;
}

}  // namespace justhtml
