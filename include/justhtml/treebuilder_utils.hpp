#pragma once

/// @file treebuilder_utils.hpp
/// @brief Tree builder utility types and functions.

#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "justhtml/tokens.hpp"

namespace justhtml {

/// HTML5 tree builder insertion modes.
enum class InsertionMode : int {
    Initial = 0,
    BeforeHtml = 1,
    BeforeHead = 2,
    InHead = 3,
    InHeadNoscript = 4,
    AfterHead = 5,
    Text = 6,
    InBody = 7,
    AfterBody = 8,
    AfterAfterBody = 9,
    InTable = 10,
    InTableText = 11,
    InCaption = 12,
    InColumnGroup = 13,
    InTableBody = 14,
    InRow = 15,
    InCell = 16,
    InFrameset = 17,
    AfterFrameset = 18,
    AfterAfterFrameset = 19,
    InSelect = 20,
    InTemplate = 21,
};

/// Check if text consists only of HTML5 whitespace characters.
/// HTML5 whitespace: tab, newline, form feed, carriage return, space.
bool is_all_whitespace(std::string_view text);

/// Check if needle starts with any of the prefixes.
bool contains_prefix(const std::vector<std::string>& prefixes, const std::string& needle);

/// Determine DOCTYPE error and quirks mode.
/// @param doctype The DOCTYPE token data.
/// @param iframe_srcdoc Whether parsing an iframe srcdoc document.
/// @return pair of (has_error, quirks_mode) where quirks_mode is
///         "quirks", "limited-quirks", or "no-quirks".
std::pair<bool, std::string> doctype_error_and_quirks(const Doctype& doctype,
                                                       bool iframe_srcdoc = false);

}  // namespace justhtml
