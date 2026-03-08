/// @file test_tokenizer_integration.cpp
/// @brief Integration tests for the tokenizer using real HTML snippets.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/stream.hpp"
#include "justhtml/tokenizer.hpp"

using namespace justhtml;

// ============================================================================
// End-to-end tokenizer integration tests
// ============================================================================

TEST_CASE("Integration: full HTML document", "[integration][tokenizer]") {
    auto events = stream("<!DOCTYPE html><html><head><title>Test</title></head><body><p>Hello</p></body></html>");

    // Should have doctype + various tags
    REQUIRE(!events.empty());

    // First event should be doctype
    REQUIRE(events[0].type == StreamEventType::Doctype);
    auto& dt = std::get<DoctypeData>(events[0].data);
    REQUIRE(dt.name.value() == "html");

    // Find the <title> start tag
    bool found_title_start = false;
    bool found_title_text = false;
    bool found_title_end = false;
    for (size_t i = 0; i < events.size(); i++) {
        if (events[i].type == StreamEventType::Start) {
            auto& sd = std::get<StartTagData>(events[i].data);
            if (sd.name == "title") {
                found_title_start = true;
                // Next should be text "Test"
                if (i + 1 < events.size() && events[i + 1].type == StreamEventType::Text) {
                    REQUIRE(std::get<std::string>(events[i + 1].data) == "Test");
                    found_title_text = true;
                }
            }
        }
        if (events[i].type == StreamEventType::End) {
            auto& name = std::get<std::string>(events[i].data);
            if (name == "title") found_title_end = true;
        }
    }
    REQUIRE(found_title_start);
    REQUIRE(found_title_text);
    REQUIRE(found_title_end);
}

TEST_CASE("Integration: HTML with attributes and entities", "[integration][tokenizer]") {
    auto events = stream("<a href=\"https://example.com?a=1&amp;b=2\" class='link'>Click &lt;here&gt;</a>");

    // Find the <a> start tag
    REQUIRE(events[0].type == StreamEventType::Start);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.name == "a");
    REQUIRE(data.attrs.at("href").value() == "https://example.com?a=1&b=2");
    REQUIRE(data.attrs.at("class").value() == "link");

    // Text should have decoded entities
    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "Click <here>");

    REQUIRE(events[2].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[2].data) == "a");
}

TEST_CASE("Integration: nested elements", "[integration][tokenizer]") {
    auto events = stream("<div><span><em>text</em></span></div>");

    REQUIRE(events.size() == 7);

    REQUIRE(std::get<StartTagData>(events[0].data).name == "div");
    REQUIRE(std::get<StartTagData>(events[1].data).name == "span");
    REQUIRE(std::get<StartTagData>(events[2].data).name == "em");
    REQUIRE(std::get<std::string>(events[3].data) == "text");
    REQUIRE(std::get<std::string>(events[4].data) == "em");
    REQUIRE(std::get<std::string>(events[5].data) == "span");
    REQUIRE(std::get<std::string>(events[6].data) == "div");
}

TEST_CASE("Integration: style tag rawtext", "[integration][tokenizer]") {
    auto events = stream("<style>body { color: red; } .foo > .bar { display: none; }</style>");

    REQUIRE(events.size() == 3);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "style");
    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "body { color: red; } .foo > .bar { display: none; }");
    REQUIRE(std::get<std::string>(events[2].data) == "style");
}

TEST_CASE("Integration: script with complex content", "[integration][tokenizer]") {
    auto events = stream("<script>var x = '<div>';\nif (x < 10) alert(x);</script>");

    REQUIRE(events.size() == 3);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "script");
    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "var x = '<div>';\nif (x < 10) alert(x);");
    REQUIRE(std::get<std::string>(events[2].data) == "script");
}

TEST_CASE("Integration: doctype with public and system ids", "[integration][tokenizer]") {
    auto events = stream("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">");

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Doctype);
    auto& dt = std::get<DoctypeData>(events[0].data);
    REQUIRE(dt.name.value() == "html");
    REQUIRE(dt.public_id.value() == "-//W3C//DTD XHTML 1.0 Strict//EN");
    REQUIRE(dt.system_id.value() == "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd");
}

TEST_CASE("Integration: multiple comments", "[integration][tokenizer]") {
    auto events = stream("<!-- first -->text<!-- second -->");

    REQUIRE(events.size() == 3);
    REQUIRE(events[0].type == StreamEventType::Comment);
    REQUIRE(std::get<std::string>(events[0].data) == " first ");
    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "text");
    REQUIRE(events[2].type == StreamEventType::Comment);
    REQUIRE(std::get<std::string>(events[2].data) == " second ");
}

TEST_CASE("Integration: textarea RCDATA", "[integration][tokenizer]") {
    auto events = stream("<textarea>Hello &amp; <World></textarea>");

    REQUIRE(events.size() == 3);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "textarea");
    REQUIRE(events[1].type == StreamEventType::Text);
    // RCDATA decodes entities but preserves HTML tags as text
    REQUIRE(std::get<std::string>(events[1].data) == "Hello & <World>");
    REQUIRE(std::get<std::string>(events[2].data) == "textarea");
}

TEST_CASE("Integration: many attributes", "[integration][tokenizer]") {
    auto events = stream("<div id=\"main\" class=\"container\" data-value=\"42\" style=\"color:red\" hidden>");

    REQUIRE(events.size() == 1);
    auto& data = std::get<StartTagData>(events[0].data);
    REQUIRE(data.name == "div");
    REQUIRE(data.attrs.size() == 5);
    REQUIRE(data.attrs.at("id").value() == "main");
    REQUIRE(data.attrs.at("class").value() == "container");
    REQUIRE(data.attrs.at("data-value").value() == "42");
    REQUIRE(data.attrs.at("style").value() == "color:red");
    REQUIRE(data.attrs.at("hidden").value() == "");
}

TEST_CASE("Integration: numeric entities", "[integration][tokenizer]") {
    auto events = stream("&#60;div&#62;&#169;&#x3C;&#x3E;");

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "<div>\xC2\xA9<>");
}

TEST_CASE("Integration: BOM removal", "[integration][tokenizer]") {
    // BOM (U+FEFF) in UTF-8 is EF BB BF
    std::string html = "\xEF\xBB\xBF<div>test</div>";
    auto events = stream(html);

    // BOM should be discarded, first event is the tag
    REQUIRE(!events.empty());
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "div");
}

TEST_CASE("Integration: CR normalization", "[integration][tokenizer]") {
    auto events = stream("line1\rline2\r\nline3");

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "line1\nline2\nline3");
}
