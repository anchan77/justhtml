/// @file encoding.cpp
/// @brief HTML encoding sniffing and decoding implementation.

#include "justhtml/encoding.hpp"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <optional>
#include <string>
#include <unordered_set>

namespace justhtml {

namespace {

const std::unordered_set<uint8_t> kAsciiWhitespace = {0x09, 0x0A, 0x0C, 0x0D, 0x20};

bool is_ascii_whitespace(uint8_t b) {
    return b == 0x09 || b == 0x0A || b == 0x0C || b == 0x0D || b == 0x20;
}

uint8_t ascii_lower(uint8_t b) {
    if (b >= 0x41 && b <= 0x5A) return b | 0x20;
    return b;
}

bool is_ascii_alpha(uint8_t b) {
    uint8_t lo = ascii_lower(b);
    return lo >= 0x61 && lo <= 0x7A;
}

size_t skip_ascii_whitespace(const std::vector<uint8_t>& data, size_t i) {
    size_t n = data.size();
    while (i < n && is_ascii_whitespace(data[i])) {
        i++;
    }
    return i;
}

std::vector<uint8_t> strip_ascii_whitespace(const std::vector<uint8_t>& value) {
    size_t start = 0;
    size_t end = value.size();
    while (start < end && is_ascii_whitespace(value[start])) start++;
    while (end > start && is_ascii_whitespace(value[end - 1])) end--;
    return std::vector<uint8_t>(value.begin() + start, value.begin() + end);
}

std::optional<std::string> normalize_meta_declared_encoding(const std::vector<uint8_t>& label) {
    std::string label_str(label.begin(), label.end());
    auto enc = normalize_encoding_label(label_str);
    if (!enc) return std::nullopt;

    // Per HTML meta charset handling: ignore UTF-16/UTF-32 declarations -> UTF-8
    if (*enc == "utf-16" || *enc == "utf-16le" || *enc == "utf-16be" ||
        *enc == "utf-32" || *enc == "utf-32le" || *enc == "utf-32be") {
        return "utf-8";
    }
    return enc;
}

std::pair<std::optional<std::string>, size_t> sniff_bom(const std::vector<uint8_t>& data) {
    if (data.size() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF) {
        return {"utf-8", 3};
    }
    if (data.size() >= 2 && data[0] == 0xFF && data[1] == 0xFE) {
        return {"utf-16le", 2};
    }
    if (data.size() >= 2 && data[0] == 0xFE && data[1] == 0xFF) {
        return {"utf-16be", 2};
    }
    return {std::nullopt, 0};
}

std::optional<std::vector<uint8_t>> extract_charset_from_content(
    const std::vector<uint8_t>& content_bytes) {
    if (content_bytes.empty()) return std::nullopt;

    // Normalize: whitespace to space, lowercase
    std::vector<uint8_t> s;
    s.reserve(content_bytes.size());
    for (uint8_t ch : content_bytes) {
        if (is_ascii_whitespace(ch)) {
            s.push_back(0x20);
        } else {
            s.push_back(ascii_lower(ch));
        }
    }

    // Find "charset"
    const std::vector<uint8_t> charset_key = {'c', 'h', 'a', 'r', 's', 'e', 't'};
    auto it = std::search(s.begin(), s.end(), charset_key.begin(), charset_key.end());
    if (it == s.end()) return std::nullopt;

    size_t i = (it - s.begin()) + charset_key.size();
    size_t n = s.size();

    // Skip whitespace
    while (i < n && is_ascii_whitespace(s[i])) i++;
    if (i >= n || s[i] != 0x3D) return std::nullopt;  // '='
    i++;
    while (i < n && is_ascii_whitespace(s[i])) i++;
    if (i >= n) return std::nullopt;

    uint8_t quote = 0;
    if (s[i] == 0x22 || s[i] == 0x27) {  // '"' or '\''
        quote = s[i];
        i++;
    }

    size_t start = i;
    while (i < n) {
        uint8_t ch = s[i];
        if (quote != 0) {
            if (ch == quote) break;
        } else {
            if (is_ascii_whitespace(ch) || ch == 0x3B) break;  // ';'
        }
        i++;
    }

    if (quote != 0 && (i >= n || s[i] != quote)) {
        return std::nullopt;
    }

    return std::vector<uint8_t>(s.begin() + start, s.begin() + i);
}

std::optional<std::string> prescan_for_meta_charset(const std::vector<uint8_t>& data) {
    size_t max_non_comment = 1024;
    size_t max_total_scan = 65536;

    size_t n = data.size();
    size_t i = 0;
    size_t non_comment = 0;

    while (i < n && i < max_total_scan && non_comment < max_non_comment) {
        if (data[i] != 0x3C) {  // '<'
            i++;
            non_comment++;
            continue;
        }

        // Comment: <!--
        if (i + 3 < n && data[i + 1] == 0x21 && data[i + 2] == 0x2D && data[i + 3] == 0x2D) {
            // Find -->
            size_t end = i + 4;
            bool found = false;
            while (end + 2 < n) {
                if (data[end] == 0x2D && data[end + 1] == 0x2D && data[end + 2] == 0x3E) {
                    found = true;
                    break;
                }
                end++;
            }
            if (!found) return std::nullopt;
            i = end + 3;
            continue;
        }

        // Tag open
        size_t j = i + 1;
        if (j < n && data[j] == 0x2F) {  // '/'
            // Skip end tag
            size_t k = i;
            uint8_t quote = 0;
            while (k < n && k < max_total_scan && non_comment < max_non_comment) {
                uint8_t ch = data[k];
                if (quote == 0) {
                    if (ch == 0x22 || ch == 0x27) {
                        quote = ch;
                    } else if (ch == 0x3E) {
                        k++;
                        non_comment++;
                        break;
                    }
                } else {
                    if (ch == quote) quote = 0;
                }
                k++;
                non_comment++;
            }
            i = k;
            continue;
        }

        if (j >= n || !is_ascii_alpha(data[j])) {
            i++;
            non_comment++;
            continue;
        }

        size_t name_start = j;
        while (j < n && is_ascii_alpha(data[j])) j++;

        // Check if tag name is "meta"
        size_t name_len = j - name_start;
        bool is_meta = (name_len == 4);
        if (is_meta) {
            is_meta = (ascii_lower(data[name_start]) == 'm' &&
                       ascii_lower(data[name_start + 1]) == 'e' &&
                       ascii_lower(data[name_start + 2]) == 't' &&
                       ascii_lower(data[name_start + 3]) == 'a');
        }

        if (!is_meta) {
            // Skip the rest of this tag
            size_t k = i;
            uint8_t quote = 0;
            while (k < n && k < max_total_scan && non_comment < max_non_comment) {
                uint8_t ch = data[k];
                if (quote == 0) {
                    if (ch == 0x22 || ch == 0x27) {
                        quote = ch;
                    } else if (ch == 0x3E) {
                        k++;
                        non_comment++;
                        break;
                    }
                } else {
                    if (ch == quote) quote = 0;
                }
                k++;
                non_comment++;
            }
            i = k;
            continue;
        }

        // Parse meta tag attributes
        std::optional<std::vector<uint8_t>> charset;
        std::optional<std::vector<uint8_t>> http_equiv;
        std::optional<std::vector<uint8_t>> content;

        size_t k = j;
        bool saw_gt = false;
        size_t start_i = i;

        while (k < n && k < max_total_scan) {
            uint8_t ch = data[k];
            if (ch == 0x3E) {  // '>'
                saw_gt = true;
                k++;
                break;
            }
            if (ch == 0x3C) break;  // '<'
            if (is_ascii_whitespace(ch) || ch == 0x2F) {  // '/'
                k++;
                continue;
            }

            // Attribute name
            size_t attr_start = k;
            while (k < n) {
                uint8_t c = data[k];
                if (is_ascii_whitespace(c) || c == 0x3D || c == 0x3E || c == 0x2F || c == 0x3C) {
                    break;
                }
                k++;
            }

            // Build lowercase attr name
            std::vector<uint8_t> attr_name;
            for (size_t ai = attr_start; ai < k; ai++) {
                attr_name.push_back(ascii_lower(data[ai]));
            }

            k = skip_ascii_whitespace(data, k);

            std::optional<std::vector<uint8_t>> value;
            if (k < n && data[k] == 0x3D) {  // '='
                k++;
                k = skip_ascii_whitespace(data, k);
                if (k >= n) break;

                uint8_t quote = 0;
                if (data[k] == 0x22 || data[k] == 0x27) {
                    quote = data[k];
                    k++;
                    size_t val_start = k;
                    // Find closing quote
                    size_t end_quote = k;
                    bool found_quote = false;
                    while (end_quote < n) {
                        if (data[end_quote] == quote) {
                            found_quote = true;
                            break;
                        }
                        end_quote++;
                    }
                    if (!found_quote) {
                        // Unclosed quote: ignore this meta
                        i++;
                        non_comment++;
                        charset.reset();
                        http_equiv.reset();
                        content.reset();
                        saw_gt = false;
                        break;
                    }
                    value = std::vector<uint8_t>(data.begin() + val_start,
                                                  data.begin() + end_quote);
                    k = end_quote + 1;
                } else {
                    size_t val_start = k;
                    while (k < n) {
                        uint8_t c = data[k];
                        if (is_ascii_whitespace(c) || c == 0x3E || c == 0x3C) break;
                        k++;
                    }
                    value = std::vector<uint8_t>(data.begin() + val_start, data.begin() + k);
                }
            }

            // Match attribute
            std::string attr_name_str(attr_name.begin(), attr_name.end());
            if (attr_name_str == "charset" && value) {
                charset = strip_ascii_whitespace(*value);
            } else if (attr_name_str == "http-equiv") {
                http_equiv = value;
            } else if (attr_name_str == "content") {
                content = value;
            }
        }

        if (saw_gt) {
            if (charset && !charset->empty()) {
                auto enc = normalize_meta_declared_encoding(*charset);
                if (enc) return enc;
            }
            if (http_equiv && content) {
                // Check if http-equiv is "content-type"
                std::string he_str;
                for (uint8_t b : *http_equiv) {
                    he_str += static_cast<char>(ascii_lower(b));
                }
                if (he_str == "content-type") {
                    auto extracted = extract_charset_from_content(*content);
                    if (extracted) {
                        auto enc = normalize_meta_declared_encoding(*extracted);
                        if (enc) return enc;
                    }
                }
            }
            i = k;
            size_t consumed = i - start_i;
            non_comment += consumed;
        } else {
            i++;
            non_comment++;
        }
    }

    return std::nullopt;
}

// Windows-1252 to Unicode codepoint mapping for 0x80-0x9F range
const uint32_t kCP1252_80_9F[] = {
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021,
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008D, 0x017D, 0x008F,
    0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
};

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

std::string decode_windows_1252(const std::vector<uint8_t>& data) {
    std::string result;
    result.reserve(data.size());
    for (uint8_t b : data) {
        if (b < 0x80) {
            result += static_cast<char>(b);
        } else if (b >= 0x80 && b <= 0x9F) {
            result += codepoint_to_utf8(kCP1252_80_9F[b - 0x80]);
        } else {
            // 0xA0-0xFF map directly to U+00A0-U+00FF
            result += codepoint_to_utf8(b);
        }
    }
    return result;
}

std::string decode_iso_8859_2(const std::vector<uint8_t>& data) {
    // ISO-8859-2 is a single-byte encoding where 0x80-0xFF map to specific codepoints.
    // For simplicity, 0x00-0x7F = ASCII, 0xA0-0xFF map to codepoints per the standard.
    static const uint32_t iso_8859_2_map[] = {
        // 0xA0 - 0xFF
        0x00A0, 0x0104, 0x02D8, 0x0141, 0x00A4, 0x013D, 0x015A, 0x00A7,
        0x00A8, 0x0160, 0x015E, 0x0164, 0x0179, 0x00AD, 0x017D, 0x017B,
        0x00B0, 0x0105, 0x02DB, 0x0142, 0x00B4, 0x013E, 0x015B, 0x02C7,
        0x00B8, 0x0161, 0x015F, 0x0165, 0x017A, 0x02DD, 0x017E, 0x017C,
        0x0154, 0x00C1, 0x00C2, 0x0102, 0x00C4, 0x0139, 0x0106, 0x00C7,
        0x010C, 0x00C9, 0x0118, 0x00CB, 0x011A, 0x00CD, 0x00CE, 0x010E,
        0x0110, 0x0143, 0x0147, 0x00D3, 0x00D4, 0x0150, 0x00D6, 0x00D7,
        0x0158, 0x016E, 0x00DA, 0x0170, 0x00DC, 0x00DD, 0x0162, 0x00DF,
        0x0155, 0x00E1, 0x00E2, 0x0103, 0x00E4, 0x013A, 0x0107, 0x00E7,
        0x010D, 0x00E9, 0x0119, 0x00EB, 0x011B, 0x00ED, 0x00EE, 0x010F,
        0x0111, 0x0144, 0x0148, 0x00F3, 0x00F4, 0x0151, 0x00F6, 0x00F7,
        0x0159, 0x016F, 0x00FA, 0x0171, 0x00FC, 0x00FD, 0x0163, 0x02D9,
    };

    std::string result;
    result.reserve(data.size());
    for (uint8_t b : data) {
        if (b < 0x80) {
            result += static_cast<char>(b);
        } else if (b >= 0x80 && b < 0xA0) {
            // 0x80-0x9F: same as Latin-1 control chars
            result += codepoint_to_utf8(b);
        } else {
            result += codepoint_to_utf8(iso_8859_2_map[b - 0xA0]);
        }
    }
    return result;
}

std::string decode_utf16(const std::vector<uint8_t>& data, bool big_endian) {
    std::string result;
    size_t i = 0;
    size_t n = data.size();

    while (i + 1 < n) {
        uint16_t code;
        if (big_endian) {
            code = (static_cast<uint16_t>(data[i]) << 8) | data[i + 1];
        } else {
            code = data[i] | (static_cast<uint16_t>(data[i + 1]) << 8);
        }
        i += 2;

        if (code >= 0xD800 && code <= 0xDBFF) {
            // High surrogate
            if (i + 1 < n) {
                uint16_t low;
                if (big_endian) {
                    low = (static_cast<uint16_t>(data[i]) << 8) | data[i + 1];
                } else {
                    low = data[i] | (static_cast<uint16_t>(data[i + 1]) << 8);
                }
                if (low >= 0xDC00 && low <= 0xDFFF) {
                    uint32_t cp = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                    result += codepoint_to_utf8(cp);
                    i += 2;
                    continue;
                }
            }
            result += "\xef\xbf\xbd";  // Replacement character
            continue;
        }

        result += codepoint_to_utf8(code);
    }

    if (i < n) {
        // Odd byte at end
        result += "\xef\xbf\xbd";
    }

    return result;
}

std::string decode_utf8_with_replacement(const std::vector<uint8_t>& data) {
    std::string result;
    result.reserve(data.size());
    size_t i = 0;
    size_t n = data.size();

    while (i < n) {
        uint8_t b = data[i];
        if (b < 0x80) {
            result += static_cast<char>(b);
            i++;
        } else if ((b & 0xE0) == 0xC0) {
            if (i + 1 < n && (data[i + 1] & 0xC0) == 0x80) {
                result += static_cast<char>(b);
                result += static_cast<char>(data[i + 1]);
                i += 2;
            } else {
                result += "\xef\xbf\xbd";
                i++;
            }
        } else if ((b & 0xF0) == 0xE0) {
            if (i + 2 < n && (data[i + 1] & 0xC0) == 0x80 && (data[i + 2] & 0xC0) == 0x80) {
                result += static_cast<char>(b);
                result += static_cast<char>(data[i + 1]);
                result += static_cast<char>(data[i + 2]);
                i += 3;
            } else {
                result += "\xef\xbf\xbd";
                i++;
            }
        } else if ((b & 0xF8) == 0xF0) {
            if (i + 3 < n && (data[i + 1] & 0xC0) == 0x80 &&
                (data[i + 2] & 0xC0) == 0x80 && (data[i + 3] & 0xC0) == 0x80) {
                result += static_cast<char>(b);
                result += static_cast<char>(data[i + 1]);
                result += static_cast<char>(data[i + 2]);
                result += static_cast<char>(data[i + 3]);
                i += 4;
            } else {
                result += "\xef\xbf\xbd";
                i++;
            }
        } else {
            result += "\xef\xbf\xbd";
            i++;
        }
    }

    return result;
}

}  // anonymous namespace

std::optional<std::string> normalize_encoding_label(std::string_view label) {
    if (label.empty()) return std::nullopt;

    // Trim whitespace
    size_t start = 0;
    size_t end = label.size();
    while (start < end && (label[start] == ' ' || label[start] == '\t' ||
                           label[start] == '\n' || label[start] == '\r')) {
        start++;
    }
    while (end > start && (label[end - 1] == ' ' || label[end - 1] == '\t' ||
                           label[end - 1] == '\n' || label[end - 1] == '\r')) {
        end--;
    }
    if (start >= end) return std::nullopt;

    // Lowercase
    std::string s;
    s.reserve(end - start);
    for (size_t i = start; i < end; i++) {
        s += static_cast<char>(std::tolower(static_cast<unsigned char>(label[i])));
    }

    // Security: never allow UTF-7
    if (s == "utf-7" || s == "utf7" || s == "x-utf-7") {
        return "windows-1252";
    }

    if (s == "utf-8" || s == "utf8") return "utf-8";

    // HTML treats latin-1 labels as windows-1252
    if (s == "iso-8859-1" || s == "iso8859-1" || s == "latin1" || s == "latin-1" ||
        s == "l1" || s == "cp819" || s == "ibm819") {
        return "windows-1252";
    }

    if (s == "windows-1252" || s == "windows1252" || s == "cp1252" || s == "x-cp1252") {
        return "windows-1252";
    }

    if (s == "iso-8859-2" || s == "iso8859-2" || s == "latin2" || s == "latin-2") {
        return "iso-8859-2";
    }

    if (s == "euc-jp" || s == "eucjp") return "euc-jp";

    if (s == "utf-16" || s == "utf16") return "utf-16";
    if (s == "utf-16le" || s == "utf16le") return "utf-16le";
    if (s == "utf-16be" || s == "utf16be") return "utf-16be";

    return std::nullopt;
}

std::pair<std::string, size_t> sniff_html_encoding(
    const std::vector<uint8_t>& data,
    const std::optional<std::string>& transport_encoding) {
    // Transport overrides everything
    if (transport_encoding) {
        auto transport = normalize_encoding_label(*transport_encoding);
        if (transport) return {*transport, 0};
    }

    // BOM detection
    auto [bom_enc, bom_len] = sniff_bom(data);
    if (bom_enc) return {*bom_enc, bom_len};

    // Meta charset prescan
    auto meta_enc = prescan_for_meta_charset(data);
    if (meta_enc) return {*meta_enc, 0};

    return {"windows-1252", 0};
}

std::pair<std::string, std::string> decode_html(
    const std::vector<uint8_t>& data,
    const std::optional<std::string>& transport_encoding) {
    auto [enc, bom_len] = sniff_html_encoding(data, transport_encoding);

    // Allowlist supported decoders
    static const std::unordered_set<std::string> supported = {
        "utf-8", "windows-1252", "iso-8859-2", "euc-jp", "utf-16", "utf-16le", "utf-16be",
    };
    if (supported.count(enc) == 0) {
        enc = "windows-1252";
        bom_len = 0;
    }

    std::vector<uint8_t> payload(data.begin() + bom_len, data.end());

    if (enc == "windows-1252") {
        return {decode_windows_1252(payload), "windows-1252"};
    }

    if (enc == "iso-8859-2") {
        return {decode_iso_8859_2(payload), "iso-8859-2"};
    }

    if (enc == "utf-16le") {
        return {decode_utf16(payload, false), "utf-16le"};
    }

    if (enc == "utf-16be") {
        return {decode_utf16(payload, true), "utf-16be"};
    }

    if (enc == "utf-16") {
        // UTF-16 auto-detects BOM in payload, or defaults to LE
        if (payload.size() >= 2) {
            if (payload[0] == 0xFE && payload[1] == 0xFF) {
                std::vector<uint8_t> inner(payload.begin() + 2, payload.end());
                return {decode_utf16(inner, true), "utf-16"};
            }
            if (payload[0] == 0xFF && payload[1] == 0xFE) {
                std::vector<uint8_t> inner(payload.begin() + 2, payload.end());
                return {decode_utf16(inner, false), "utf-16"};
            }
        }
        return {decode_utf16(payload, false), "utf-16"};
    }

    if (enc == "euc-jp") {
        // Simplified: treat as ASCII where possible, replace unknown as U+FFFD
        // Full EUC-JP support would require a large mapping table
        std::string result;
        result.reserve(payload.size());
        size_t i = 0;
        while (i < payload.size()) {
            uint8_t b = payload[i];
            if (b < 0x80) {
                result += static_cast<char>(b);
                i++;
            } else if (b == 0x8E && i + 1 < payload.size()) {
                // Half-width katakana
                result += "\xef\xbf\xbd";
                i += 2;
            } else if (b == 0x8F && i + 2 < payload.size()) {
                // JIS X 0212
                result += "\xef\xbf\xbd";
                i += 3;
            } else if (b >= 0xA1 && b <= 0xFE && i + 1 < payload.size()) {
                // JIS X 0208
                result += "\xef\xbf\xbd";
                i += 2;
            } else {
                result += "\xef\xbf\xbd";
                i++;
            }
        }
        return {result, "euc-jp"};
    }

    // Default: UTF-8
    return {decode_utf8_with_replacement(payload), "utf-8"};
}

}  // namespace justhtml
