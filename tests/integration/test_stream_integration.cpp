/// @file test_stream_integration.cpp
/// @brief Integration tests for the streaming API.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/stream.hpp"

using namespace justhtml;

TEST_CASE("Stream integration: full page", "[integration][stream]") {
    std::string html = R"(<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Test Page</title>
</head>
<body>
    <h1>Hello World</h1>
    <p>This is a <strong>test</strong> page.</p>
    <!-- This is a comment -->
    <script>var x = 1;</script>
</body>
</html>)";

    auto events = stream(html);

    // Should have many events
    REQUIRE(events.size() > 10);

    // First event should be doctype
    REQUIRE(events[0].type == StreamEventType::Doctype);

    // Find key elements
    bool found_html = false;
    bool found_title = false;
    bool found_h1 = false;
    bool found_script = false;
    bool found_comment = false;
    int start_count = 0;
    int end_count = 0;

    for (const auto& event : events) {
        if (event.type == StreamEventType::Start) {
            start_count++;
            auto& data = std::get<StartTagData>(event.data);
            if (data.name == "html") found_html = true;
            if (data.name == "title") found_title = true;
            if (data.name == "h1") found_h1 = true;
            if (data.name == "script") found_script = true;
        }
        if (event.type == StreamEventType::End) {
            end_count++;
        }
        if (event.type == StreamEventType::Comment) {
            found_comment = true;
        }
    }

    REQUIRE(found_html);
    REQUIRE(found_title);
    REQUIRE(found_h1);
    REQUIRE(found_script);
    REQUIRE(found_comment);
    REQUIRE(start_count > 5);
    REQUIRE(end_count > 5);
}

TEST_CASE("Stream integration: entity-heavy content", "[integration][stream]") {
    auto events = stream("<p>&lt;html&gt; &amp; &quot;quotes&quot; &#169; &#x2603;</p>");

    // Find the text event
    bool found_text = false;
    for (const auto& event : events) {
        if (event.type == StreamEventType::Text) {
            auto& text = std::get<std::string>(event.data);
            REQUIRE(text.find("<html>") != std::string::npos);
            REQUIRE(text.find("&") != std::string::npos);
            REQUIRE(text.find("\"quotes\"") != std::string::npos);
            found_text = true;
        }
    }
    REQUIRE(found_text);
}

TEST_CASE("Stream integration: mixed void and regular elements", "[integration][stream]") {
    auto events = stream("<div><br><img src=\"test.png\"><p>text</p><hr></div>");

    // Count events
    int starts = 0;
    int ends = 0;
    for (const auto& event : events) {
        if (event.type == StreamEventType::Start) starts++;
        if (event.type == StreamEventType::End) ends++;
    }

    // div, br, img, p, hr = 5 start tags
    REQUIRE(starts == 5);
    // div, p = 2 end tags (void elements don't have end tags from tokenizer)
    REQUIRE(ends == 2);
}

TEST_CASE("Stream integration: deeply nested elements", "[integration][stream]") {
    auto events = stream("<div><div><div><div><div>deep</div></div></div></div></div>");

    // Should have 5 starts, 1 text, 5 ends
    int starts = 0;
    int ends = 0;
    int texts = 0;
    for (const auto& event : events) {
        if (event.type == StreamEventType::Start) starts++;
        if (event.type == StreamEventType::End) ends++;
        if (event.type == StreamEventType::Text) texts++;
    }

    REQUIRE(starts == 5);
    REQUIRE(ends == 5);
    REQUIRE(texts == 1);
}

TEST_CASE("Stream integration: consecutive text coalesced", "[integration][stream]") {
    // When the tokenizer emits multiple text tokens in one step,
    // the stream function should coalesce them
    auto events = stream("Hello World! &amp; More");

    // Should be coalesced into a single text event
    int text_count = 0;
    for (const auto& event : events) {
        if (event.type == StreamEventType::Text) text_count++;
    }
    REQUIRE(text_count == 1);
}
