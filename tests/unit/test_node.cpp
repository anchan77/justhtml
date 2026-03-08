/// @file test_node.cpp
/// @brief Unit tests for the DOM node model.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/node.hpp"

using namespace justhtml;

TEST_CASE("SimpleDomNode text property", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("Hi"));
    REQUIRE(node->text_property() == "Hi");
}

TEST_CASE("append_child noop for comment node", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("comment"));
    auto child = std::make_shared<SimpleDomNode>("span");
    parent->append_child(child);
    REQUIRE(child->parent.expired());
}

TEST_CASE("remove_child noop for comment node", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("comment"));
    auto child = std::make_shared<SimpleDomNode>("span");
    // Comment has no children, remove should not throw (noop)
    parent->remove_child(child);
    REQUIRE(child->parent.expired());
}

TEST_CASE("text property simple", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    auto text = std::make_shared<TextNode>(std::string("Hello"));
    node->append_child(text);
    REQUIRE(node->text_property() == "");
    REQUIRE(text->text_property() == "Hello");
    REQUIRE(node->to_text() == "Hello");
}

TEST_CASE("text property nested", "[node]") {
    auto root = std::make_shared<SimpleDomNode>("div");
    auto span = std::make_shared<SimpleDomNode>("span");
    auto text1 = std::make_shared<TextNode>(std::string("Hello "));
    auto text2 = std::make_shared<TextNode>(std::string("World"));

    root->append_child(text1);
    root->append_child(span);
    span->append_child(text2);

    REQUIRE(root->text_property() == "");
    REQUIRE(span->text_property() == "");
    REQUIRE(root->to_text() == "Hello World");
    REQUIRE(span->to_text() == "World");
}

TEST_CASE("text property empty", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    REQUIRE(node->text_property() == "");
}

TEST_CASE("text property comment", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("comment"));
    REQUIRE(node->text_property() == "");
}

TEST_CASE("to_text matches textContent", "[node]") {
    auto root = std::make_shared<SimpleDomNode>("div");
    auto span = std::make_shared<SimpleDomNode>("span");
    root->append_child(std::make_shared<TextNode>(std::string("Hello ")));
    root->append_child(span);
    span->append_child(std::make_shared<TextNode>(std::string("World")));

    REQUIRE(root->to_text() == "Hello World");
    REQUIRE(span->to_text() == "World");
    REQUIRE(root->to_text("", false) == "Hello World");
    REQUIRE(root->to_text("", true) == "HelloWorld");
}

TEST_CASE("to_text skips empty and whitespace segments by default", "[node]") {
    auto root = std::make_shared<SimpleDomNode>("div");
    root->append_child(std::make_shared<TextNode>(std::string("")));
    root->append_child(std::make_shared<TextNode>(std::string("   ")));
    root->append_child(std::make_shared<TextNode>(std::string("A")));
    REQUIRE(root->to_text() == "A");
}

TEST_CASE("to_text empty subtree", "[node]") {
    auto root = std::make_shared<SimpleDomNode>("div");
    REQUIRE(root->to_text() == "");
}

TEST_CASE("TextNode to_text strip false", "[node]") {
    auto t = std::make_shared<TextNode>(std::string("  A  "));
    REQUIRE(t->to_text("", false) == "  A  ");
    REQUIRE(t->to_text(" ", true) == "A");
}

TEST_CASE("TextNode to_text none data", "[node]") {
    auto t = std::make_shared<TextNode>(std::nullopt);
    REQUIRE(t->to_text() == "");
}

TEST_CASE("to_text includes template content", "[node]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("html"));
    tmpl->get_template_content()->append_child(std::make_shared<TextNode>(std::string("Inside")));

    REQUIRE(tmpl->text_property() == "");
    REQUIRE(tmpl->to_text() == "Inside");
}

TEST_CASE("to_text SimpleDomNode text node branch", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("#text", std::nullopt, std::string("Hi"));
    REQUIRE(node->to_text() == "Hi");
}

TEST_CASE("insert_before", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child1 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "1"}});
    auto child2 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "2"}});

    parent->append_child(child1);
    parent->insert_before(child2, child1);

    REQUIRE(parent->get_children().size() == 2);
    REQUIRE(parent->get_children()[0] == child2);
    REQUIRE(parent->get_children()[1] == child1);
    REQUIRE(!child2->parent.expired());
}

TEST_CASE("insert_before nullptr appends", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child1 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "1"}});
    auto child2 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "2"}});

    parent->append_child(child1);
    parent->insert_before(child2, nullptr);

    REQUIRE(parent->get_children().size() == 2);
    REQUIRE(parent->get_children()[0] == child1);
    REQUIRE(parent->get_children()[1] == child2);
}

TEST_CASE("insert_before invalid reference", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child1 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "1"}});
    auto child2 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "2"}});
    auto other = std::make_shared<SimpleDomNode>("div");

    parent->append_child(child1);

    REQUIRE_THROWS(parent->insert_before(child2, other));
}

TEST_CASE("insert_before no children allowed", "[node]") {
    auto comment = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("foo"));
    auto node = std::make_shared<SimpleDomNode>("div");

    REQUIRE_THROWS(comment->insert_before(node, nullptr));
}

TEST_CASE("TextNode none data", "[node]") {
    auto text = std::make_shared<TextNode>(std::nullopt);
    REQUIRE(text->text_property() == "");
}

TEST_CASE("SimpleDomNode text none", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("#text", std::nullopt, std::nullopt);
    REQUIRE(node->text_property() == "");
}

TEST_CASE("replace_child", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child1 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "1"}});
    auto child2 = std::make_shared<SimpleDomNode>("span", Attributes{{"id", "2"}});
    auto new_child = std::make_shared<SimpleDomNode>("p");

    parent->append_child(child1);
    parent->append_child(child2);

    auto replaced = parent->replace_child(new_child, child1);

    REQUIRE(replaced == child1);
    REQUIRE(parent->get_children().size() == 2);
    REQUIRE(parent->get_children()[0] == new_child);
    REQUIRE(parent->get_children()[1] == child2);
    REQUIRE(!new_child->parent.expired());
    REQUIRE(child1->parent.expired());
}

TEST_CASE("replace_child invalid", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child1 = std::make_shared<SimpleDomNode>("span");
    auto other = std::make_shared<SimpleDomNode>("p");

    parent->append_child(child1);

    REQUIRE_THROWS(parent->replace_child(other, other));
}

TEST_CASE("replace_child no children allowed", "[node]") {
    auto comment = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("foo"));
    auto node = std::make_shared<SimpleDomNode>("div");

    REQUIRE_THROWS(comment->replace_child(node, node));
}

TEST_CASE("has_child_nodes", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    REQUIRE_FALSE(parent->has_child_nodes());

    parent->append_child(std::make_shared<SimpleDomNode>("span"));
    REQUIRE(parent->has_child_nodes());
}

TEST_CASE("clone_node shallow", "[node]") {
    Attributes attrs{{"class", "foo"}};
    auto node = std::make_shared<SimpleDomNode>("div", attrs, std::nullopt, std::string("html"));
    auto child = std::make_shared<SimpleDomNode>("span");
    node->append_child(child);

    auto clone = node->clone_node(false);

    REQUIRE(clone->name == "div");
    REQUIRE(clone->attrs.has_value());
    REQUIRE(clone->attrs->at("class") == "foo");
    REQUIRE(clone->namespace_.value() == "html");
    REQUIRE(clone->get_children().empty());
    REQUIRE(clone != node);
}

TEST_CASE("clone_node simple", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("div", Attributes{{"id", "1"}});
    auto clone = node->clone_node();
    REQUIRE(clone->name == "div");
    REQUIRE(clone->attrs.has_value());
    REQUIRE(clone->attrs->at("id") == "1");
    REQUIRE(clone != node);
    REQUIRE(clone->get_children().empty());
}

TEST_CASE("clone_node deep", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child = std::make_shared<SimpleDomNode>("span");
    parent->append_child(child);

    auto clone = parent->clone_node(true);
    REQUIRE(clone->get_children().size() == 1);
    REQUIRE(clone->get_children()[0]->name == "span");
    REQUIRE(clone->get_children()[0] != child);
    REQUIRE(!clone->get_children()[0]->parent.expired());
    REQUIRE(clone->get_children()[0]->parent.lock() == clone);
}

TEST_CASE("clone text node", "[node]") {
    auto text = std::make_shared<TextNode>(std::string("hello"));
    auto clone = text->clone_node();
    REQUIRE(clone->data.value() == "hello");
    REQUIRE(clone != text);
}

TEST_CASE("clone template node", "[node]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("html"));
    auto content_child = std::make_shared<SimpleDomNode>("div");
    tmpl->get_template_content()->append_child(content_child);

    auto clone = tmpl->clone_node(true);
    REQUIRE(clone != tmpl);
    auto clone_tmpl = std::dynamic_pointer_cast<TemplateNode>(clone);
    REQUIRE(clone_tmpl != nullptr);
    REQUIRE(clone_tmpl->get_template_content() != nullptr);
    REQUIRE(clone_tmpl->get_template_content() != tmpl->get_template_content());
    REQUIRE(clone_tmpl->get_template_content()->get_children().size() == 1);
    REQUIRE(clone_tmpl->get_template_content()->get_children()[0]->name == "div");
}

TEST_CASE("clone template node with children", "[node]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("html"));
    auto child = std::make_shared<SimpleDomNode>("span");
    tmpl->append_child(child);

    auto clone = tmpl->clone_node(true);
    REQUIRE(clone->get_children().size() == 1);
    REQUIRE(clone->get_children()[0]->name == "span");
    REQUIRE(clone->get_children()[0] != child);
}

TEST_CASE("clone element node", "[node]") {
    auto element = std::make_shared<ElementNode>("div", Attributes{{"class", "foo"}}, std::string("html"));
    auto child = std::make_shared<SimpleDomNode>("span");
    element->append_child(child);

    // Shallow clone
    auto clone_shallow = element->clone_node(false);
    REQUIRE(clone_shallow->get_children().empty());

    // Deep clone
    auto clone_deep = element->clone_node(true);
    REQUIRE(clone_deep->get_children().size() == 1);
    REQUIRE(clone_deep->get_children()[0]->name == "span");
    REQUIRE(clone_deep->get_children()[0] != child);
}

TEST_CASE("clone node empty attrs", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("div");
    auto clone = node->clone_node();
    REQUIRE(clone->attrs.has_value());
    REQUIRE(clone->attrs->empty());
}

TEST_CASE("clone comment node", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("foo"));
    auto clone = node->clone_node();
    REQUIRE(!clone->attrs.has_value());
    REQUIRE(clone->data.value() == "foo");
}

TEST_CASE("clone template node non-html", "[node]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("svg"));
    REQUIRE(tmpl->get_template_content() == nullptr);
    auto child = std::make_shared<SimpleDomNode>("g");
    tmpl->append_child(child);

    auto clone = tmpl->clone_node(true);
    auto clone_tmpl = std::dynamic_pointer_cast<TemplateNode>(clone);
    REQUIRE(clone_tmpl != nullptr);
    REQUIRE(clone_tmpl->get_template_content() == nullptr);
    REQUIRE(clone_tmpl->namespace_.value() == "svg");
    REQUIRE(clone->get_children().size() == 1);
    REQUIRE(clone->get_children()[0]->name == "g");
}

TEST_CASE("clone template node shallow", "[node]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("html"));
    auto child = std::make_shared<SimpleDomNode>("div");
    tmpl->append_child(child);

    auto clone = tmpl->clone_node(false);
    REQUIRE(clone->name == "template");
    REQUIRE(clone->get_children().empty());
}

TEST_CASE("clone doctype", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("!doctype", std::nullopt, std::string("html"));
    auto clone = node->clone_node();
    REQUIRE(clone->name == "!doctype");
    REQUIRE(!clone->attrs.has_value());
}

TEST_CASE("clone document", "[node]") {
    auto node = std::make_shared<SimpleDomNode>("#document");
    auto clone = node->clone_node();
    REQUIRE(clone->name == "#document");
    REQUIRE(clone->get_children().empty());
}

TEST_CASE("remove_child", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child = std::make_shared<SimpleDomNode>("span");
    parent->append_child(child);

    parent->remove_child(child);
    REQUIRE(parent->get_children().empty());
    REQUIRE(child->parent.expired());
}

TEST_CASE("remove_child not found", "[node]") {
    auto parent = std::make_shared<SimpleDomNode>("div");
    auto child = std::make_shared<SimpleDomNode>("span");
    REQUIRE_THROWS(parent->remove_child(child));
}

TEST_CASE("TextNode children and has_child_nodes", "[node]") {
    auto text = std::make_shared<TextNode>(std::string("hello"));
    REQUIRE(text->get_children().empty());
    REQUIRE_FALSE(text->has_child_nodes());
}

TEST_CASE("to_markdown textnode method", "[node][markdown]") {
    auto t = std::make_shared<TextNode>(std::string("a*b"));
    REQUIRE(t->to_markdown() == "a\\*b");
}

TEST_CASE("to_markdown empty textnode", "[node][markdown]") {
    auto t = std::make_shared<TextNode>(std::string(""));
    REQUIRE(t->to_markdown() == "");
}

TEST_CASE("to_markdown ignores comment and doctype", "[node][markdown]") {
    auto root = std::make_shared<SimpleDomNode>("div");
    root->append_child(std::make_shared<SimpleDomNode>("#comment", std::nullopt, std::string("nope")));
    root->append_child(std::make_shared<SimpleDomNode>("!doctype", std::nullopt, std::string("html")));
    root->append_child(std::make_shared<TextNode>(std::string("ok")));
    REQUIRE(root->to_markdown() == "ok");
}

TEST_CASE("to_markdown document container direct", "[node][markdown]") {
    auto doc = std::make_shared<SimpleDomNode>("#document");
    doc->append_child(std::make_shared<SimpleDomNode>("p"));
    REQUIRE(doc->to_markdown() == "");
}

TEST_CASE("markdown_code_span edge cases", "[node][markdown]") {
    REQUIRE(markdown_code_span("") == "``");
    REQUIRE(markdown_code_span("`x") == "`` `x ``");
    REQUIRE(markdown_code_span("x`") == "`` x` ``");
    REQUIRE(markdown_code_span("a`b`") == "`` a`b` ``");
}

TEST_CASE("MarkdownBuilder text preserve_whitespace branch", "[node][markdown]") {
    MarkdownBuilder b;
    b.text("x\n", true);
    REQUIRE(b.finish() == "x");
}

TEST_CASE("MarkdownBuilder text leading whitespace no space", "[node][markdown]") {
    MarkdownBuilder b;
    b.text("   a");
    REQUIRE(b.finish() == "a");
}

TEST_CASE("MarkdownBuilder raw inserts pending space", "[node][markdown]") {
    MarkdownBuilder b;
    b.text("a ");
    b.raw("**");
    b.raw("b");
    REQUIRE(b.finish() == "a **b");
}

TEST_CASE("MarkdownBuilder raw does not insert space before newline", "[node][markdown]") {
    MarkdownBuilder b;
    b.text("a ");
    b.raw("\n");
    REQUIRE(b.finish() == "a");
}

TEST_CASE("to_markdown includes template content", "[node][markdown]") {
    auto tmpl = std::make_shared<TemplateNode>("template", std::nullopt, std::nullopt, std::string("html"));
    tmpl->get_template_content()->append_child(std::make_shared<TextNode>(std::string("T")));
    REQUIRE(tmpl->to_markdown() == "T");
}

TEST_CASE("to_markdown unknown container walks children", "[node][markdown]") {
    auto span = std::make_shared<SimpleDomNode>("span");
    span->append_child(std::make_shared<TextNode>(std::string("Hi")));
    REQUIRE(span->to_markdown() == "Hi");
}

TEST_CASE("to_markdown document without children", "[node][markdown]") {
    auto doc = std::make_shared<SimpleDomNode>("#document");
    REQUIRE(doc->to_markdown() == "");
}
