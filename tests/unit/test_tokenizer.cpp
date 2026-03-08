/// @file test_tokenizer.cpp
/// @brief Unit tests for the HTML5 tokenizer.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/stream.hpp"
#include "justhtml/tokenizer.hpp"

using namespace justhtml;

// Helper to get stream events from HTML
static std::vector<StreamEvent> tokenize(const std::string& html) {
    return stream(html);
}

TEST_CASE("Tokenizer: basic text", "[tokenizer]") {
    auto events = tokenize("hello");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "hello");
}

TEST_CASE("Tokenizer: basic start tag", "[tokenizer]") {
    auto events = tokenize("<div>");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Start);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.name == "div");
    REQUIRE(data.attrs.empty());
}

TEST_CASE("Tokenizer: start and end tag", "[tokenizer]") {
    auto events = tokenize("<div></div>");
    REQUIRE(events.size() == 2);
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "div");
    REQUIRE(events[1].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[1].data) == "div");
}

TEST_CASE("Tokenizer: tag with attributes", "[tokenizer]") {
    auto events = tokenize("<div class=\"container\" id=main>");
    REQUIRE(events.size() == 1);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.name == "div");
    REQUIRE(data.attrs.size() == 2);
    REQUIRE(data.attrs.at("class").value() == "container");
    REQUIRE(data.attrs.at("id").value() == "main");
}

TEST_CASE("Tokenizer: self-closing tag", "[tokenizer]") {
    auto events = tokenize("<br/>");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "br");
}

TEST_CASE("Tokenizer: comment", "[tokenizer]") {
    auto events = tokenize("<!-- comment -->");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Comment);
    REQUIRE(std::get<std::string>(events[0].data) == " comment ");
}

TEST_CASE("Tokenizer: DOCTYPE", "[tokenizer]") {
    auto events = tokenize("<!DOCTYPE html>");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Doctype);
    auto& dt = std::get<DoctypeData>(events[0].data);
    REQUIRE(dt.name.value() == "html");
    REQUIRE(!dt.public_id.has_value());
    REQUIRE(!dt.system_id.has_value());
}

TEST_CASE("Tokenizer: text with entities", "[tokenizer]") {
    auto events = tokenize("&amp; &lt;");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "& <");
}

TEST_CASE("Tokenizer: mixed content", "[tokenizer]") {
    auto events = tokenize("<div class=\"container\">Hello <b>World</b></div>");
    REQUIRE(events.size() == 6);

    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "div");
    REQUIRE(std::get<StartTagData>(events[0].data).attrs.at("class").value() == "container");

    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "Hello ");

    REQUIRE(events[2].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[2].data).name == "b");

    REQUIRE(events[3].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[3].data) == "World");

    REQUIRE(events[4].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[4].data) == "b");

    REQUIRE(events[5].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[5].data) == "div");
}

TEST_CASE("Tokenizer: uppercase tag names lowered", "[tokenizer]") {
    auto events = tokenize("<DIV></DIV>");
    REQUIRE(events.size() == 2);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "div");
    REQUIRE(std::get<std::string>(events[1].data) == "div");
}

TEST_CASE("Tokenizer: void elements", "[tokenizer]") {
    auto events = tokenize("<br><hr>");
    REQUIRE(events.size() == 2);
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "br");
    REQUIRE(events[1].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[1].data).name == "hr");
}

TEST_CASE("Tokenizer: script rawtext", "[tokenizer]") {
    auto events = tokenize("<script>console.log('<');</script>");
    REQUIRE(events.size() == 3);
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "script");
    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "console.log('<');");
    REQUIRE(events[2].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[2].data) == "script");
}

TEST_CASE("Tokenizer: unmatched end tag", "[tokenizer]") {
    auto events = tokenize("</div>");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[0].data) == "div");
}

TEST_CASE("Tokenizer: text coalescing", "[tokenizer]") {
    auto events = tokenize("abc");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "abc");
}

TEST_CASE("Tokenizer: style rawtext", "[tokenizer]") {
    auto events = tokenize("<style>.foo { color: red; }</style>");
    REQUIRE(events.size() == 3);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "style");
    REQUIRE(std::get<std::string>(events[1].data) == ".foo { color: red; }");
    REQUIRE(std::get<std::string>(events[2].data) == "style");
}

TEST_CASE("Tokenizer: attribute with single quotes", "[tokenizer]") {
    auto events = tokenize("<div class='foo'>");
    REQUIRE(events.size() == 1);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.attrs.at("class").value() == "foo");
}

TEST_CASE("Tokenizer: empty comment", "[tokenizer]") {
    auto events = tokenize("<!---->");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Comment);
    REQUIRE(std::get<std::string>(events[0].data) == "");
}

TEST_CASE("Tokenizer: multiple attributes", "[tokenizer]") {
    auto events = tokenize("<input type=\"text\" name=\"q\" value=\"search\">");
    REQUIRE(events.size() == 1);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.name == "input");
    REQUIRE(data.attrs.size() == 3);
    REQUIRE(data.attrs.at("type").value() == "text");
    REQUIRE(data.attrs.at("name").value() == "q");
    REQUIRE(data.attrs.at("value").value() == "search");
}

TEST_CASE("Tokenizer: boolean attributes", "[tokenizer]") {
    auto events = tokenize("<input disabled readonly>");
    REQUIRE(events.size() == 1);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.attrs.size() == 2);
    // Boolean attributes should have empty string value
    REQUIRE(data.attrs.at("disabled").value() == "");
    REQUIRE(data.attrs.at("readonly").value() == "");
}

TEST_CASE("Tokenizer: title RCDATA", "[tokenizer]") {
    auto events = tokenize("<title>Hello &amp; World</title>");
    REQUIRE(events.size() == 3);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "title");
    REQUIRE(events[1].type == StreamEventType::Text);
    // RCDATA entities should be decoded
    REQUIRE(std::get<std::string>(events[1].data) == "Hello & World");
    REQUIRE(std::get<std::string>(events[2].data) == "title");
}

TEST_CASE("Tokenizer: bogus comment", "[tokenizer]") {
    auto events = tokenize("<?xml version=\"1.0\"?>");
    // Should be treated as bogus comment
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Comment);
}

TEST_CASE("Tokenizer: doctype with public id", "[tokenizer]") {
    auto events = tokenize("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Doctype);
    auto& dt = std::get<DoctypeData>(events[0].data);
    REQUIRE(dt.name.value() == "html");
    REQUIRE(dt.public_id.has_value());
    REQUIRE(dt.system_id.has_value());
}

TEST_CASE("Tokenizer: empty input", "[tokenizer]") {
    auto events = tokenize("");
    REQUIRE(events.empty());
}

TEST_CASE("Tokenizer: only whitespace", "[tokenizer]") {
    auto events = tokenize("   ");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "   ");
}
