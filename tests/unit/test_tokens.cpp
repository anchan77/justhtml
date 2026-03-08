/// @file test_tokens.cpp
/// @brief Unit tests for token types and ParseError.

#include <catch2/catch_test_macros.hpp>

#include "justhtml/token_sink.hpp"
#include "justhtml/tokens.hpp"

using namespace justhtml;

TEST_CASE("Tag creation", "[tokens]") {
    SECTION("Start tag with attributes") {
        Attributes attrs;
        attrs["href"] = "https://example.com";
        attrs["class"] = "link";
        Tag tag(TagKind::Start, "a", attrs, false);

        REQUIRE(tag.kind == TagKind::Start);
        REQUIRE(tag.name == "a");
        REQUIRE(tag.attrs.size() == 2);
        REQUIRE(tag.attrs.at("href") == "https://example.com");
        REQUIRE(tag.self_closing == false);
    }

    SECTION("Self-closing tag") {
        Tag tag(TagKind::Start, "br", {}, true);
        REQUIRE(tag.self_closing == true);
        REQUIRE(tag.name == "br");
    }

    SECTION("End tag") {
        Tag tag(TagKind::End, "div");
        REQUIRE(tag.kind == TagKind::End);
        REQUIRE(tag.name == "div");
        REQUIRE(tag.attrs.empty());
    }

    SECTION("Attribute with no value") {
        Attributes attrs;
        attrs["disabled"] = std::nullopt;
        Tag tag(TagKind::Start, "input", attrs);
        REQUIRE(tag.attrs.at("disabled") == std::nullopt);
    }
}

TEST_CASE("CharacterTokens", "[tokens]") {
    CharacterTokens ct("Hello, world!");
    REQUIRE(ct.data == "Hello, world!");
}

TEST_CASE("CommentToken", "[tokens]") {
    CommentToken ct("This is a comment");
    REQUIRE(ct.data == "This is a comment");
}

TEST_CASE("Doctype", "[tokens]") {
    SECTION("Full doctype") {
        Doctype dt("html", "public-id", "system-id", false);
        REQUIRE(dt.name == "html");
        REQUIRE(dt.public_id == "public-id");
        REQUIRE(dt.system_id == "system-id");
        REQUIRE(dt.force_quirks == false);
    }

    SECTION("Minimal doctype") {
        Doctype dt;
        REQUIRE(!dt.name.has_value());
        REQUIRE(!dt.public_id.has_value());
        REQUIRE(!dt.system_id.has_value());
        REQUIRE(dt.force_quirks == false);
    }

    SECTION("Force quirks") {
        Doctype dt("html", std::nullopt, std::nullopt, true);
        REQUIRE(dt.force_quirks == true);
    }
}

TEST_CASE("DoctypeToken wraps Doctype", "[tokens]") {
    DoctypeToken dtt(Doctype("html"));
    REQUIRE(dtt.doctype.name == "html");
}

TEST_CASE("TokenSinkResult values", "[tokens]") {
    REQUIRE(static_cast<int>(TokenSinkResult::Continue) == 0);
    REQUIRE(static_cast<int>(TokenSinkResult::Plaintext) == 1);
}

TEST_CASE("ParseError", "[tokens]") {
    SECTION("Basic ParseError") {
        ParseError err("unexpected-tag");
        REQUIRE(err.code == "unexpected-tag");
        REQUIRE(err.message == "unexpected-tag");
        REQUIRE(!err.line.has_value());
        REQUIRE(!err.column.has_value());
    }

    SECTION("ParseError with location") {
        ParseError err("missing-end-tag", 5, 10, "Missing end tag");
        REQUIRE(err.code == "missing-end-tag");
        REQUIRE(err.line == 5);
        REQUIRE(err.column == 10);
        REQUIRE(err.message == "Missing end tag");
    }

    SECTION("ParseError to_string without location") {
        ParseError err("bad-tag");
        REQUIRE(err.to_string() == "bad-tag");
    }

    SECTION("ParseError to_string with location") {
        ParseError err("bad-tag", 3, 7);
        REQUIRE(err.to_string() == "(3,7): bad-tag");
    }

    SECTION("ParseError to_string with message different from code") {
        ParseError err("bad-tag", 3, 7, "Tag is invalid");
        REQUIRE(err.to_string() == "(3,7): bad-tag - Tag is invalid");
    }

    SECTION("ParseError repr") {
        ParseError err("bad-tag", 3, 7);
        REQUIRE(err.repr() == "ParseError(\"bad-tag\", line=3, column=7)");
    }

    SECTION("ParseError repr without location") {
        ParseError err("bad-tag");
        REQUIRE(err.repr() == "ParseError(\"bad-tag\")");
    }

    SECTION("ParseError equality") {
        ParseError err1("bad-tag", 1, 2);
        ParseError err2("bad-tag", 1, 2);
        ParseError err3("bad-tag", 1, 3);
        ParseError err4("other-tag", 1, 2);

        REQUIRE(err1 == err2);
        REQUIRE(err1 != err3);
        REQUIRE(err1 != err4);
    }
}

TEST_CASE("Token variant", "[tokens]") {
    SECTION("Holds Tag") {
        Token tok = Tag(TagKind::Start, "div");
        REQUIRE(std::holds_alternative<Tag>(tok));
    }

    SECTION("Holds CharacterTokens") {
        Token tok = CharacterTokens("hello");
        REQUIRE(std::holds_alternative<CharacterTokens>(tok));
    }

    SECTION("Holds CommentToken") {
        Token tok = CommentToken("comment");
        REQUIRE(std::holds_alternative<CommentToken>(tok));
    }

    SECTION("Holds DoctypeToken") {
        Token tok = DoctypeToken(Doctype("html"));
        REQUIRE(std::holds_alternative<DoctypeToken>(tok));
    }

    SECTION("Holds EOFToken") {
        Token tok = EOFToken{};
        REQUIRE(std::holds_alternative<EOFToken>(tok));
    }
}
