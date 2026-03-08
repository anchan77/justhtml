/// @file test_constants.cpp
/// @brief Unit tests for HTML5 spec constants.

#include <catch2/catch_test_macros.hpp>

#include "justhtml/constants.hpp"

using namespace justhtml;
using namespace justhtml::constants;

TEST_CASE("Void elements", "[constants]") {
    const auto& ve = void_elements();
    REQUIRE(ve.count("br") == 1);
    REQUIRE(ve.count("hr") == 1);
    REQUIRE(ve.count("img") == 1);
    REQUIRE(ve.count("input") == 1);
    REQUIRE(ve.count("meta") == 1);
    REQUIRE(ve.count("div") == 0);
    REQUIRE(ve.count("p") == 0);
    REQUIRE(ve.size() == 14);
}

TEST_CASE("Heading elements", "[constants]") {
    const auto& he = heading_elements();
    REQUIRE(he.size() == 6);
    REQUIRE(he.count("h1") == 1);
    REQUIRE(he.count("h6") == 1);
    REQUIRE(he.count("h7") == 0);
}

TEST_CASE("Special elements", "[constants]") {
    const auto& se = special_elements();
    REQUIRE(se.count("div") == 1);
    REQUIRE(se.count("p") == 1);
    REQUIRE(se.count("table") == 1);
    REQUIRE(se.count("template") == 1);
    REQUIRE(se.count("span") == 0);
}

TEST_CASE("Formatting elements", "[constants]") {
    const auto& fe = formatting_elements();
    REQUIRE(fe.count("a") == 1);
    REQUIRE(fe.count("b") == 1);
    REQUIRE(fe.count("em") == 1);
    REQUIRE(fe.count("strong") == 1);
    REQUIRE(fe.count("div") == 0);
    REQUIRE(fe.size() == 14);
}

TEST_CASE("Scope terminators", "[constants]") {
    SECTION("Default scope") {
        const auto& ds = default_scope_terminators();
        REQUIRE(ds.count("html") == 1);
        REQUIRE(ds.count("table") == 1);
        REQUIRE(ds.count("template") == 1);
        REQUIRE(ds.count("button") == 0);
    }

    SECTION("Button scope includes button") {
        const auto& bs = button_scope_terminators();
        REQUIRE(bs.count("html") == 1);
        REQUIRE(bs.count("button") == 1);
    }

    SECTION("List item scope includes ol/ul") {
        const auto& ls = list_item_scope_terminators();
        REQUIRE(ls.count("ol") == 1);
        REQUIRE(ls.count("ul") == 1);
    }

    SECTION("Definition scope includes dl") {
        const auto& ds = definition_scope_terminators();
        REQUIRE(ds.count("dl") == 1);
    }
}

TEST_CASE("SVG tag name adjustments", "[constants]") {
    const auto& adj = svg_tag_name_adjustments();
    REQUIRE(adj.at("clippath") == "clipPath");
    REQUIRE(adj.at("foreignobject") == "foreignObject");
    REQUIRE(adj.at("lineargradient") == "linearGradient");
    REQUIRE(adj.count("div") == 0);
}

TEST_CASE("SVG attribute adjustments", "[constants]") {
    const auto& adj = svg_attribute_adjustments();
    REQUIRE(adj.at("viewbox") == "viewBox");
    REQUIRE(adj.at("preserveaspectratio") == "preserveAspectRatio");
    REQUIRE(adj.at("attributename") == "attributeName");
}

TEST_CASE("MathML attribute adjustments", "[constants]") {
    const auto& adj = mathml_attribute_adjustments();
    REQUIRE(adj.at("definitionurl") == "definitionURL");
    REQUIRE(adj.size() == 1);
}

TEST_CASE("Foreign attribute adjustments", "[constants]") {
    const auto& adj = foreign_attribute_adjustments();
    auto it = adj.find("xlink:href");
    REQUIRE(it != adj.end());
    REQUIRE(it->second.prefix == "xlink");
    REQUIRE(it->second.local_name == "href");
    REQUIRE(it->second.namespace_url == ns::XLINK);

    auto xmlns = adj.find("xmlns");
    REQUIRE(xmlns != adj.end());
    REQUIRE(!xmlns->second.prefix.has_value());
}

TEST_CASE("HTML integration points", "[constants]") {
    const auto& hip = html_integration_point_set();
    REQUIRE(hip.count({"svg", "foreignObject"}) == 1);
    REQUIRE(hip.count({"math", "annotation-xml"}) == 1);
}

TEST_CASE("MathML text integration points", "[constants]") {
    const auto& mtip = mathml_text_integration_point_set();
    REQUIRE(mtip.count({"math", "mi"}) == 1);
    REQUIRE(mtip.count({"math", "mtext"}) == 1);
}

TEST_CASE("Table foster targets", "[constants]") {
    const auto& tft = table_foster_targets();
    REQUIRE(tft.count("table") == 1);
    REQUIRE(tft.count("tbody") == 1);
    REQUIRE(tft.count("tr") == 1);
    REQUIRE(tft.size() == 5);
}

TEST_CASE("Implied end tags", "[constants]") {
    const auto& iet = implied_end_tags();
    REQUIRE(iet.count("dd") == 1);
    REQUIRE(iet.count("p") == 1);
    REQUIRE(iet.count("li") == 1);
    REQUIRE(iet.count("div") == 0);
}

TEST_CASE("Quirky public prefixes", "[constants]") {
    const auto& qpp = quirky_public_prefixes();
    REQUIRE(qpp.size() > 50);
    // Check that the first prefix is present
    REQUIRE(qpp[0] == "-//advasoft ltd//dtd html 3.0 aswedit + extensions//");
}

TEST_CASE("Foreign breakout elements", "[constants]") {
    const auto& fbe = foreign_breakout_elements();
    REQUIRE(fbe.count("div") == 1);
    REQUIRE(fbe.count("span") == 1);
    REQUIRE(fbe.count("table") == 1);
    REQUIRE(fbe.count("foreignObject") == 0);
}
