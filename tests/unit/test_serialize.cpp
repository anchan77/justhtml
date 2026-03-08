/// @file test_serialize.cpp
/// @brief Unit tests for the HTML serialization module.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/node.hpp"
#include "justhtml/serialize.hpp"

using namespace justhtml;

TEST_CASE("escape_text basic", "[serialize]") {
    REQUIRE(escape_text(std::nullopt) == "");
    REQUIRE(escape_text(std::string("")) == "");
    REQUIRE(escape_text(std::string("hello")) == "hello");
    REQUIRE(escape_text(std::string("a<b&c>d")) == "a&lt;b&amp;c&gt;d");
}

TEST_CASE("choose_attr_quote", "[serialize]") {
    REQUIRE(choose_attr_quote(std::nullopt) == '"');
    REQUIRE(choose_attr_quote(std::string("hello")) == '"');
    REQUIRE(choose_attr_quote(std::string("foo\"bar")) == '\'');
    REQUIRE(choose_attr_quote(std::string("foo'bar\"baz")) == '"');
}

TEST_CASE("escape_attr_value", "[serialize]") {
    REQUIRE(escape_attr_value(std::nullopt, '"') == "");
    REQUIRE(escape_attr_value(std::string("hello"), '"') == "hello");
    REQUIRE(escape_attr_value(std::string("a&b"), '"') == "a&amp;b");
    REQUIRE(escape_attr_value(std::string("a\"b"), '"') == "a&quot;b");
    REQUIRE(escape_attr_value(std::string("a'b"), '\'') == "a&#39;b");
}

TEST_CASE("can_unquote_attr_value", "[serialize]") {
    REQUIRE(can_unquote_attr_value(std::nullopt) == false);
    REQUIRE(can_unquote_attr_value(std::string("")) == false);
    REQUIRE(can_unquote_attr_value(std::string("foo")) == true);
    REQUIRE(can_unquote_attr_value(std::string("foo>bar")) == false);
    REQUIRE(can_unquote_attr_value(std::string("foo\"bar")) == false);
    REQUIRE(can_unquote_attr_value(std::string("foo bar")) == false);
    REQUIRE(can_unquote_attr_value(std::string("foo<bar")) == true);
}

TEST_CASE("serialize_start_tag", "[serialize]") {
    REQUIRE(serialize_start_tag("div", std::nullopt) == "<div>");
    REQUIRE(serialize_start_tag("div", Attributes{}) == "<div>");
    REQUIRE(serialize_start_tag("span", Attributes{{"title", std::string("foo")}}) == "<span title=foo>");

    // Prefer single quotes if the value contains a double quote but no single quote
    auto tag = serialize_start_tag("span", Attributes{{"title", std::string("foo\"bar")}});
    REQUIRE(tag == "<span title='foo\"bar'>");

    // Otherwise use double quotes and escape embedded double quotes
    tag = serialize_start_tag("span", Attributes{{"title", std::string("foo'bar\"baz")}});
    REQUIRE(tag == "<span title=\"foo'bar&quot;baz\">");

    // Empty/None attribute values
    REQUIRE(serialize_start_tag("input", Attributes{{"disabled", std::nullopt}}) == "<input disabled>");
    REQUIRE(serialize_start_tag("input", Attributes{{"disabled", std::string("")}}) == "<input disabled>");
}

TEST_CASE("serialize_end_tag", "[serialize]") {
    REQUIRE(serialize_end_tag("span") == "</span>");
    REQUIRE(serialize_end_tag("div") == "</div>");
}

TEST_CASE("to_html text node", "[serialize]") {
    auto frag = std::make_shared<SimpleDomNode>("#document-fragment");
    auto div = std::make_shared<SimpleDomNode>("div");
    frag->append_child(div);
    div->append_child(std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("a<b&c")));
    auto output = to_html(frag, 0, 2, false);
    REQUIRE(output == "<div>a&lt;b&amp;c</div>");
}

TEST_CASE("to_html void elements", "[serialize]") {
    auto frag = std::make_shared<SimpleDomNode>("#document-fragment");
    auto br = std::make_shared<SimpleDomNode>("br");
    frag->append_child(br);
    auto output = to_html(frag, 0, 2, false);
    REQUIRE(output.find("<br>") != std::string::npos);
    REQUIRE(output.find("</br>") == std::string::npos);
}

TEST_CASE("to_html comment", "[serialize]") {
    auto frag = std::make_shared<SimpleDomNode>("#document-fragment");
    auto comment = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string(" hello "));
    frag->append_child(comment);
    auto output = to_html(frag, 0, 2, true);
    REQUIRE(output.find("<!-- hello -->") != std::string::npos);
}

TEST_CASE("to_html document fragment", "[serialize]") {
    auto frag = std::make_shared<SimpleDomNode>("#document-fragment");
    auto div = std::make_shared<SimpleDomNode>("div");
    frag->append_child(div);
    auto output = to_html(frag);
    REQUIRE(output.find("<div></div>") != std::string::npos);
}

TEST_CASE("to_html empty element", "[serialize]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    auto text = std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("hello"));
    node->append_child(text);
    auto output = to_html(node, 0, 2, false);
    // Without pretty, text is not stripped
    REQUIRE(output == "<div>hello</div>");
}

TEST_CASE("to_html none attributes", "[serialize]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    node->attrs = Attributes{{"data-test", std::nullopt}};
    auto output = to_html(node);
    REQUIRE(output.find("<div data-test></div>") != std::string::npos);
}

TEST_CASE("to_html empty string attribute", "[serialize]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    node->attrs = Attributes{{"data-val", std::string("")}};
    auto output = to_html(node);
    REQUIRE(output.find("<div data-val></div>") != std::string::npos);
}

TEST_CASE("to_html pretty print skips whitespace text nodes", "[serialize]") {
    auto div = std::make_shared<SimpleDomNode>("div");
    div->append_child(std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("\n  ")));
    div->append_child(std::make_shared<SimpleDomNode>("p"));
    div->append_child(std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("\n")));
    auto output = div->to_html(0, 2, true);
    REQUIRE(output == "<div>\n  <p></p>\n</div>");
}

TEST_CASE("to_html pretty indent does not indent inline elements", "[serialize]") {
    auto div = std::make_shared<SimpleDomNode>("div");
    div->append_child(std::make_shared<SimpleDomNode>("span"));
    auto output = div->to_html(0, 2, true);
    REQUIRE(output == "<div><span></span></div>");
}

TEST_CASE("to_html pretty indent does not indent comments", "[serialize]") {
    auto div = std::make_shared<SimpleDomNode>("div");
    div->append_child(std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("x")));
    div->append_child(std::make_shared<SimpleDomNode>("p"));
    auto output = div->to_html(0, 2, true);
    REQUIRE(output == "<div><!--x--><p></p></div>");
}

TEST_CASE("to_html whitespace in fragment", "[serialize]") {
    auto frag = std::make_shared<SimpleDomNode>("#document-fragment");
    auto text_node = std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("   "));
    frag->append_child(text_node);
    auto output = to_html(frag);
    REQUIRE(output == "");
}

TEST_CASE("to_html text node pretty strips and renders", "[serialize]") {
    auto frag = std::make_shared<SimpleDomNode>("#document-fragment");
    frag->append_child(std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("  hi  ")));
    auto output = to_html(frag, 0, 2, true);
    REQUIRE(output == "hi");
}

TEST_CASE("to_html empty text node dropped when not pretty", "[serialize]") {
    auto div = std::make_shared<SimpleDomNode>("div");
    div->append_child(std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("")));
    auto output = to_html(div, 0, 2, false);
    REQUIRE(output == "<div></div>");
}

TEST_CASE("to_test_format single element", "[serialize]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    auto output = to_test_format(node);
    REQUIRE(output == "| <div>");
}

TEST_CASE("to_test_format template with attributes", "[serialize]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("html"));
    tmpl->attrs = Attributes{{"id", std::string("t1")}};
    auto child = std::make_shared<SimpleDomNode>("p");
    tmpl->get_template_content()->append_child(child);
    auto output = to_test_format(tmpl);
    REQUIRE(output.find("| <template>") != std::string::npos);
    REQUIRE(output.find("|   id=\"t1\"") != std::string::npos);
    REQUIRE(output.find("|   content") != std::string::npos);
    REQUIRE(output.find("|     <p>") != std::string::npos);
}
