/// @file treebuilder_utils.cpp
/// @brief Tree builder utility functions implementation.

#include "justhtml/treebuilder_utils.hpp"

#include <algorithm>
#include <cctype>
#include <optional>
#include <string>
#include <tuple>
#include <vector>

#include "justhtml/constants.hpp"

namespace justhtml {

bool is_all_whitespace(std::string_view text) {
    for (char c : text) {
        if (c != '\t' && c != '\n' && c != '\f' && c != '\r' && c != ' ') {
            return false;
        }
    }
    return true;
}

bool contains_prefix(const std::vector<std::string>& prefixes, const std::string& needle) {
    for (const auto& prefix : prefixes) {
        if (needle.size() >= prefix.size() &&
            needle.compare(0, prefix.size(), prefix) == 0) {
            return true;
        }
    }
    return false;
}

namespace {

std::string to_lower(const std::string& s) {
    std::string result = s;
    std::transform(result.begin(), result.end(), result.begin(),
                   [](unsigned char c) { return std::tolower(c); });
    return result;
}

using OptStr = std::optional<std::string>;
using DoctypeKey = std::tuple<OptStr, OptStr, OptStr>;

}  // anonymous namespace

std::pair<bool, std::string> doctype_error_and_quirks(const Doctype& doctype,
                                                       bool iframe_srcdoc) {
    OptStr name = doctype.name ? OptStr(to_lower(*doctype.name)) : std::nullopt;
    const OptStr& public_id = doctype.public_id;
    const OptStr& system_id = doctype.system_id;

    // Acceptable doctypes
    auto key = DoctypeKey(name, public_id, system_id);

    static const std::vector<DoctypeKey> acceptable = {
        {OptStr("html"), std::nullopt, std::nullopt},
        {OptStr("html"), std::nullopt, OptStr("about:legacy-compat")},
        {OptStr("html"), OptStr("-//W3C//DTD HTML 4.0//EN"), std::nullopt},
        {OptStr("html"), OptStr("-//W3C//DTD HTML 4.0//EN"),
         OptStr("http://www.w3.org/TR/REC-html40/strict.dtd")},
        {OptStr("html"), OptStr("-//W3C//DTD HTML 4.01//EN"), std::nullopt},
        {OptStr("html"), OptStr("-//W3C//DTD HTML 4.01//EN"),
         OptStr("http://www.w3.org/TR/html4/strict.dtd")},
        {OptStr("html"), OptStr("-//W3C//DTD XHTML 1.0 Strict//EN"),
         OptStr("http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd")},
        {OptStr("html"), OptStr("-//W3C//DTD XHTML 1.1//EN"),
         OptStr("http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd")},
    };

    bool parse_error = true;
    for (const auto& acc : acceptable) {
        if (key == acc) {
            parse_error = false;
            break;
        }
    }

    OptStr public_lower = public_id ? OptStr(to_lower(*public_id)) : std::nullopt;
    OptStr system_lower = system_id ? OptStr(to_lower(*system_id)) : std::nullopt;

    std::string quirks_mode;

    if (doctype.force_quirks) {
        quirks_mode = "quirks";
    } else if (iframe_srcdoc) {
        quirks_mode = "no-quirks";
    } else if (!name.has_value() || *name != "html") {
        quirks_mode = "quirks";
    } else if (public_lower.has_value()) {
        // Check quirky public matches
        const auto& qpm = constants::quirky_public_matches();
        bool is_quirky_match = false;
        for (const auto& m : qpm) {
            if (*public_lower == m) {
                is_quirky_match = true;
                break;
            }
        }
        if (is_quirky_match) {
            quirks_mode = "quirks";
        } else if (contains_prefix(constants::quirky_public_prefixes(), *public_lower)) {
            quirks_mode = "quirks";
        } else if (contains_prefix(constants::limited_quirky_public_prefixes(), *public_lower)) {
            quirks_mode = "limited-quirks";
        } else if (contains_prefix(constants::html4_public_prefixes(), *public_lower)) {
            quirks_mode = system_lower.has_value() ? "limited-quirks" : "quirks";
        } else {
            // Check system matches
            if (system_lower.has_value()) {
                const auto& qsm = constants::quirky_system_matches();
                bool is_quirky_sys = false;
                for (const auto& m : qsm) {
                    if (*system_lower == m) {
                        is_quirky_sys = true;
                        break;
                    }
                }
                quirks_mode = is_quirky_sys ? "quirks" : "no-quirks";
            } else {
                quirks_mode = "no-quirks";
            }
        }
    } else if (system_lower.has_value()) {
        // No public_id, check system matches
        const auto& qsm = constants::quirky_system_matches();
        bool is_quirky_sys = false;
        for (const auto& m : qsm) {
            if (*system_lower == m) {
                is_quirky_sys = true;
                break;
            }
        }
        quirks_mode = is_quirky_sys ? "quirks" : "no-quirks";
    } else {
        quirks_mode = "no-quirks";
    }

    return {parse_error, quirks_mode};
}

}  // namespace justhtml
