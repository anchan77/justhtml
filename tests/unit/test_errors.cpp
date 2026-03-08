/// @file test_errors.cpp
/// @brief Unit tests for error message generation and FragmentContext.

#include <catch2/catch_test_macros.hpp>

#include "justhtml/context.hpp"
#include "justhtml/errors.hpp"

using namespace justhtml;

TEST_CASE("FragmentContext", "[context]") {
    SECTION("Default construction") {
        FragmentContext ctx;
        REQUIRE(ctx.tag_name.empty());
        REQUIRE(!ctx.ns.has_value());
    }

    SECTION("With tag name only") {
        FragmentContext ctx("div");
        REQUIRE(ctx.tag_name == "div");
        REQUIRE(!ctx.ns.has_value());
    }

    SECTION("With tag name and namespace") {
        FragmentContext ctx("svg", "http://www.w3.org/2000/svg");
        REQUIRE(ctx.tag_name == "svg");
        REQUIRE(ctx.ns == "http://www.w3.org/2000/svg");
    }
}

TEST_CASE("generate_error_message - static messages", "[errors]") {
    SECTION("DOCTYPE errors") {
        REQUIRE(generate_error_message("eof-in-doctype") ==
                "Unexpected end of file in DOCTYPE declaration");
        REQUIRE(generate_error_message("missing-whitespace-before-doctype-name") ==
                "Missing whitespace after <!DOCTYPE");
    }

    SECTION("Comment errors") {
        REQUIRE(generate_error_message("eof-in-comment") ==
                "Unexpected end of file in comment");
        REQUIRE(generate_error_message("incorrectly-closed-comment") ==
                "Comment ended with --!> instead of -->");
    }

    SECTION("Tag errors") {
        REQUIRE(generate_error_message("eof-in-tag") ==
                "Unexpected end of file in tag");
        REQUIRE(generate_error_message("empty-end-tag") ==
                "Empty end tag </> is not allowed");
    }

    SECTION("Attribute errors") {
        REQUIRE(generate_error_message("duplicate-attribute") ==
                "Duplicate attribute name");
    }

    SECTION("NULL character errors") {
        REQUIRE(generate_error_message("unexpected-null-character") ==
                "Unexpected NULL character (U+0000)");
    }

    SECTION("Character reference errors") {
        REQUIRE(generate_error_message("missing-semicolon-after-character-reference") ==
                "Missing semicolon after character reference");
    }
}

TEST_CASE("generate_error_message - tag interpolation", "[errors]") {
    SECTION("Start tag messages") {
        REQUIRE(generate_error_message("unexpected-start-tag", "div") ==
                "Unexpected <div> start tag");
        REQUIRE(generate_error_message("unexpected-start-tag-ignored", "p") ==
                "<p> start tag ignored in current context");
    }

    SECTION("End tag messages") {
        REQUIRE(generate_error_message("unexpected-end-tag", "span") ==
                "Unexpected </span> end tag");
        REQUIRE(generate_error_message("end-tag-too-early", "div") ==
                "</div> end tag closed early (unclosed children)");
    }

    SECTION("DOCTYPE with tag name") {
        REQUIRE(generate_error_message("expected-doctype-but-got-start-tag", "html") ==
                "Expected DOCTYPE but got <html> tag");
    }

    SECTION("Foster parenting messages") {
        REQUIRE(generate_error_message("unexpected-start-tag-implies-table-voodoo", "tr") ==
                "<tr> start tag in table triggers foster parenting");
    }

    SECTION("Missing tag name uses default") {
        REQUIRE(generate_error_message("unexpected-start-tag") ==
                "Unexpected <unknown> start tag");
    }
}

TEST_CASE("generate_error_message - fallback", "[errors]") {
    REQUIRE(generate_error_message("some-unknown-code") == "some-unknown-code");
}
