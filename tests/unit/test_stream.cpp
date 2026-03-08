/// @file test_stream.cpp
/// @brief Unit tests for the streaming API.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/stream.hpp"

using namespace justhtml;

TEST_CASE("stream: basic html", "[stream]") {
    auto events = stream("<div class=\"container\">Hello <b>World</b></div>");

    REQUIRE(events.size() == 6);

    REQUIRE(events[0].type == StreamEventType::Start);
    auto& start_data = std::get<StartTagData>(events[0].data);
    REQUIRE(start_data.name == "div");
    REQUIRE(start_data.attrs.at("class").value() == "container");

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

TEST_CASE("stream: comments", "[stream]") {
    auto events = stream("<!-- comment -->");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Comment);
    REQUIRE(std::get<std::string>(events[0].data) == " comment ");
}

TEST_CASE("stream: doctype", "[stream]") {
    auto events = stream("<!DOCTYPE html>");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Doctype);
    auto& dt = std::get<DoctypeData>(events[0].data);
    REQUIRE(dt.name.value() == "html");
    REQUIRE(!dt.public_id.has_value());
    REQUIRE(!dt.system_id.has_value());
}

TEST_CASE("stream: void elements", "[stream]") {
    auto events = stream("<br><hr>");
    REQUIRE(events.size() == 2);
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "br");
    REQUIRE(events[1].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[1].data).name == "hr");
}

TEST_CASE("stream: text coalescing", "[stream]") {
    auto events = stream("abc");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[0].data) == "abc");
}

TEST_CASE("stream: script rawtext", "[stream]") {
    auto events = stream("<script>console.log('<');</script>");
    REQUIRE(events.size() == 3);
    REQUIRE(events[0].type == StreamEventType::Start);
    REQUIRE(std::get<StartTagData>(events[0].data).name == "script");
    REQUIRE(events[1].type == StreamEventType::Text);
    REQUIRE(std::get<std::string>(events[1].data) == "console.log('<');");
    REQUIRE(events[2].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[2].data) == "script");
}

TEST_CASE("stream: unmatched end tag", "[stream]") {
    auto events = stream("</div>");
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].type == StreamEventType::End);
    REQUIRE(std::get<std::string>(events[0].data) == "div");
}
