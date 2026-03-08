#pragma once

/// @file encoding.hpp
/// @brief HTML encoding sniffing and decoding.
///
/// Implements HTML5 encoding sniffing: BOM detection, meta charset prescan,
/// encoding label normalization, and bytes-to-UTF-8 conversion.

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace justhtml {

/// Normalize an encoding label to a canonical name.
/// Returns nullopt if the label is unknown.
/// @param label Encoding label (e.g., "utf-8", "latin1", "windows-1252").
/// @return Canonical encoding name, or nullopt.
std::optional<std::string> normalize_encoding_label(std::string_view label);

/// Sniff the encoding of an HTML byte stream.
/// @param data Raw bytes of the HTML document.
/// @param transport_encoding Optional transport-layer encoding (e.g., from HTTP).
/// @return Pair of (encoding_name, bom_byte_count).
std::pair<std::string, size_t> sniff_html_encoding(
    const std::vector<uint8_t>& data,
    const std::optional<std::string>& transport_encoding = std::nullopt);

/// Decode an HTML byte stream to a UTF-8 string.
/// @param data Raw bytes of the HTML document.
/// @param transport_encoding Optional transport-layer encoding.
/// @return Pair of (decoded_utf8_string, encoding_name).
std::pair<std::string, std::string> decode_html(
    const std::vector<uint8_t>& data,
    const std::optional<std::string>& transport_encoding = std::nullopt);

}  // namespace justhtml
