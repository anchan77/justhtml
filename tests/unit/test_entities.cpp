/// @file test_entities.cpp
/// @brief Unit tests for HTML entity decoding.

#include <catch2/catch_test_macros.hpp>

#include "justhtml/entities.hpp"

using namespace justhtml;

TEST_CASE("lookup_named_entity", "[entities]") {
    SECTION("Common entities") {
        REQUIRE(lookup_named_entity("amp") == "&");
        REQUIRE(lookup_named_entity("lt") == "<");
        REQUIRE(lookup_named_entity("gt") == ">");
        REQUIRE(lookup_named_entity("quot") == "\"");
        REQUIRE(lookup_named_entity("apos") == "'");
    }

    SECTION("Non-ASCII entities") {
        // &nbsp; = U+00A0 (non-breaking space)
        std::string nbsp = lookup_named_entity("nbsp");
        REQUIRE(!nbsp.empty());
        REQUIRE(nbsp == "\xc2\xa0");
    }

    SECTION("Unknown entity returns empty") {
        REQUIRE(lookup_named_entity("nonexistent").empty());
    }
}

TEST_CASE("is_legacy_entity", "[entities]") {
    REQUIRE(is_legacy_entity("amp") == true);
    REQUIRE(is_legacy_entity("lt") == true);
    REQUIRE(is_legacy_entity("gt") == true);
    REQUIRE(is_legacy_entity("quot") == true);
    REQUIRE(is_legacy_entity("nbsp") == true);
    REQUIRE(is_legacy_entity("copy") == true);
    REQUIRE(is_legacy_entity("reg") == true);

    // Non-legacy entities require semicolons
    REQUIRE(is_legacy_entity("prod") == false);
    REQUIRE(is_legacy_entity("notin") == false);
    REQUIRE(is_legacy_entity("zwnj") == false);
}

TEST_CASE("decode_numeric_entity", "[entities]") {
    SECTION("Decimal") {
        REQUIRE(decode_numeric_entity("60") == "<");
        REQUIRE(decode_numeric_entity("62") == ">");
        REQUIRE(decode_numeric_entity("38") == "&");
        REQUIRE(decode_numeric_entity("160") == "\xc2\xa0");  // U+00A0 NBSP
    }

    SECTION("Hexadecimal") {
        REQUIRE(decode_numeric_entity("3C", true) == "<");
        REQUIRE(decode_numeric_entity("3E", true) == ">");
        REQUIRE(decode_numeric_entity("26", true) == "&");
        REQUIRE(decode_numeric_entity("A0", true) == "\xc2\xa0");  // U+00A0
    }

    SECTION("Windows-1252 replacements") {
        REQUIRE(decode_numeric_entity("0") == "\xef\xbf\xbd");     // U+FFFD
        REQUIRE(decode_numeric_entity("80", true) == "\xe2\x82\xac");  // U+20AC (Euro)
        REQUIRE(decode_numeric_entity("91", true) == "\xe2\x80\x98");  // U+2018 (left single quote)
    }

    SECTION("Surrogate range returns replacement char") {
        REQUIRE(decode_numeric_entity("D800", true) == "\xef\xbf\xbd");
        REQUIRE(decode_numeric_entity("DFFF", true) == "\xef\xbf\xbd");
    }

    SECTION("Too large codepoint returns replacement char") {
        REQUIRE(decode_numeric_entity("110000", true) == "\xef\xbf\xbd");
    }

    SECTION("Multi-byte UTF-8") {
        // U+2603 SNOWMAN
        REQUIRE(decode_numeric_entity("2603", true) == "\xe2\x98\x83");
        // U+1F600 GRINNING FACE
        REQUIRE(decode_numeric_entity("1F600", true) == "\xf0\x9f\x98\x80");
    }
}

TEST_CASE("decode_entities_in_text - basic", "[entities]") {
    SECTION("No entities") {
        REQUIRE(decode_entities_in_text("hello world") == "hello world");
    }

    SECTION("Named entities with semicolon") {
        REQUIRE(decode_entities_in_text("&amp;") == "&");
        REQUIRE(decode_entities_in_text("&lt;") == "<");
        REQUIRE(decode_entities_in_text("&gt;") == ">");
        REQUIRE(decode_entities_in_text("&quot;") == "\"");
    }

    SECTION("Multiple entities") {
        REQUIRE(decode_entities_in_text("&lt;div&gt;") == "<div>");
    }

    SECTION("Numeric decimal") {
        REQUIRE(decode_entities_in_text("&#60;") == "<");
        REQUIRE(decode_entities_in_text("&#60;div&#62;") == "<div>");
    }

    SECTION("Numeric hex") {
        REQUIRE(decode_entities_in_text("&#x3C;") == "<");
        REQUIRE(decode_entities_in_text("&#x3C;div&#x3E;") == "<div>");
    }

    SECTION("Mixed text and entities") {
        REQUIRE(decode_entities_in_text("a &amp; b") == "a & b");
    }

    SECTION("Entity at start and end") {
        REQUIRE(decode_entities_in_text("&lt;tag&gt;") == "<tag>");
    }
}

TEST_CASE("decode_entities_in_text - legacy entities", "[entities]") {
    SECTION("Legacy entity without semicolon") {
        REQUIRE(decode_entities_in_text("&amp end") == "& end");
    }

    SECTION("Legacy entity in text context - prefix match") {
        // &not is a legacy entity, &notit is not valid
        std::string result = decode_entities_in_text("&notit;");
        // Should match &not as legacy prefix, leaving "it;" as text
        REQUIRE(result.find("it;") != std::string::npos);
    }
}

TEST_CASE("decode_entities_in_text - attribute mode", "[entities]") {
    SECTION("Legacy entity followed by = in attribute") {
        // In attribute mode, &amp followed by = should NOT decode
        REQUIRE(decode_entities_in_text("&amp=value", true) == "&amp=value");
    }

    SECTION("Legacy entity followed by alnum in attribute") {
        REQUIRE(decode_entities_in_text("&ampX", true) == "&ampX");
    }

    SECTION("Entity with semicolon in attribute") {
        REQUIRE(decode_entities_in_text("&amp;", true) == "&");
    }
}

TEST_CASE("decode_entities_in_text - edge cases", "[entities]") {
    SECTION("Lone ampersand") {
        REQUIRE(decode_entities_in_text("&") == "&");
    }

    SECTION("Ampersand followed by space") {
        REQUIRE(decode_entities_in_text("& ") == "& ");
    }

    SECTION("Unknown entity with semicolon") {
        REQUIRE(decode_entities_in_text("&unknown;") == "&unknown;");
    }

    SECTION("Empty input") {
        REQUIRE(decode_entities_in_text("") == "");
    }

    SECTION("Numeric entity without semicolon") {
        REQUIRE(decode_entities_in_text("&#60text") == "<text");
    }

    SECTION("Invalid numeric entity") {
        REQUIRE(decode_entities_in_text("&#;") == "&#;");
    }
}
