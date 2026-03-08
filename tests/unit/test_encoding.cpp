/// @file test_encoding.cpp
/// @brief Unit tests for encoding detection and decoding.

#include <catch2/catch_test_macros.hpp>

#include <cstdint>
#include <string>
#include <vector>

#include "justhtml/encoding.hpp"

using namespace justhtml;

// Helper to convert string to vector<uint8_t>
static std::vector<uint8_t> to_bytes(const std::string& s) {
    return std::vector<uint8_t>(s.begin(), s.end());
}

static std::vector<uint8_t> to_bytes(const char* s) {
    return to_bytes(std::string(s));
}

TEST_CASE("normalize_encoding_label", "[encoding]") {
    SECTION("UTF-8 variants") {
        REQUIRE(normalize_encoding_label("utf-8") == "utf-8");
        REQUIRE(normalize_encoding_label("UTF-8") == "utf-8");
        REQUIRE(normalize_encoding_label("utf8") == "utf-8");
        REQUIRE(normalize_encoding_label("  UTF-8  ") == "utf-8");
    }

    SECTION("Windows-1252 / Latin-1") {
        REQUIRE(normalize_encoding_label("windows-1252") == "windows-1252");
        REQUIRE(normalize_encoding_label("iso-8859-1") == "windows-1252");
        REQUIRE(normalize_encoding_label("latin1") == "windows-1252");
        REQUIRE(normalize_encoding_label("latin-1") == "windows-1252");
    }

    SECTION("UTF-7 blocked for security") {
        REQUIRE(normalize_encoding_label("utf-7") == "windows-1252");
        REQUIRE(normalize_encoding_label("UTF7") == "windows-1252");
        REQUIRE(normalize_encoding_label("x-utf-7") == "windows-1252");
    }

    SECTION("UTF-16 variants") {
        REQUIRE(normalize_encoding_label("utf-16") == "utf-16");
        REQUIRE(normalize_encoding_label("utf-16le") == "utf-16le");
        REQUIRE(normalize_encoding_label("utf-16be") == "utf-16be");
    }

    SECTION("Unknown encoding") {
        REQUIRE(normalize_encoding_label("unknown") == std::nullopt);
        REQUIRE(normalize_encoding_label("") == std::nullopt);
    }
}

TEST_CASE("sniff_html_encoding - transport encoding", "[encoding]") {
    auto data = to_bytes("<html>");
    auto [enc, bom] = sniff_html_encoding(data, "utf-8");
    REQUIRE(enc == "utf-8");
    REQUIRE(bom == 0);
}

TEST_CASE("sniff_html_encoding - BOM detection", "[encoding]") {
    SECTION("UTF-8 BOM") {
        std::vector<uint8_t> data = {0xEF, 0xBB, 0xBF, '<', 'h', 't', 'm', 'l', '>'};
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-8");
        REQUIRE(bom == 3);
    }

    SECTION("UTF-16LE BOM") {
        std::vector<uint8_t> data = {0xFF, 0xFE, '<', 0, 'h', 0};
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-16le");
        REQUIRE(bom == 2);
    }

    SECTION("UTF-16BE BOM") {
        std::vector<uint8_t> data = {0xFE, 0xFF, 0, '<', 0, 'h'};
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-16be");
        REQUIRE(bom == 2);
    }
}

TEST_CASE("sniff_html_encoding - meta charset", "[encoding]") {
    SECTION("Simple meta charset") {
        auto data = to_bytes("<meta charset=\"utf-8\">");
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-8");
    }

    SECTION("Meta charset windows-1252") {
        auto data = to_bytes("<meta charset=\"windows-1252\">");
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "windows-1252");
    }

    SECTION("Meta http-equiv content-type") {
        auto data = to_bytes(
            "<meta http-equiv=\"content-type\" content=\"text/html; charset=utf-8\">");
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-8");
    }

    SECTION("Comment before meta") {
        auto data = to_bytes("<!-- comment --><meta charset=\"utf-8\">");
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-8");
    }

    SECTION("Non-meta tag before meta") {
        auto data = to_bytes("<html><head><meta charset=\"utf-8\">");
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-8");
    }

    SECTION("UTF-16 in meta treated as UTF-8") {
        auto data = to_bytes("<meta charset=\"utf-16\">");
        auto [enc, bom] = sniff_html_encoding(data);
        REQUIRE(enc == "utf-8");
    }
}

TEST_CASE("sniff_html_encoding - default fallback", "[encoding]") {
    auto data = to_bytes("<html><body>hello</body></html>");
    auto [enc, bom] = sniff_html_encoding(data);
    REQUIRE(enc == "windows-1252");
    REQUIRE(bom == 0);
}

TEST_CASE("decode_html - UTF-8", "[encoding]") {
    auto data = to_bytes("<p>Hello, world!</p>");
    auto [text, enc] = decode_html(data, "utf-8");
    REQUIRE(text == "<p>Hello, world!</p>");
    REQUIRE(enc == "utf-8");
}

TEST_CASE("decode_html - UTF-8 with BOM", "[encoding]") {
    std::vector<uint8_t> data = {0xEF, 0xBB, 0xBF};
    auto rest = to_bytes("<p>hello</p>");
    data.insert(data.end(), rest.begin(), rest.end());
    auto [text, enc] = decode_html(data);
    REQUIRE(text == "<p>hello</p>");
    REQUIRE(enc == "utf-8");
}

TEST_CASE("decode_html - Windows-1252", "[encoding]") {
    // \x93 and \x94 are smart quotes in Windows-1252
    std::vector<uint8_t> data = {0x93, 'h', 'i', 0x94};
    auto [text, enc] = decode_html(data, "windows-1252");
    REQUIRE(enc == "windows-1252");
    // U+201C and U+201D
    REQUIRE(text.find("hi") != std::string::npos);
}

TEST_CASE("decode_html - Latin-1 label treated as Windows-1252", "[encoding]") {
    auto data = to_bytes("hello");
    auto [text, enc] = decode_html(data, "latin1");
    REQUIRE(enc == "windows-1252");
}
