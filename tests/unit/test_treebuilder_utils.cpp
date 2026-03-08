/// @file test_treebuilder_utils.cpp
/// @brief Unit tests for tree builder utility functions.

#include <catch2/catch_test_macros.hpp>

#include "justhtml/treebuilder_utils.hpp"

using namespace justhtml;

TEST_CASE("InsertionMode values", "[treebuilder_utils]") {
    REQUIRE(static_cast<int>(InsertionMode::Initial) == 0);
    REQUIRE(static_cast<int>(InsertionMode::BeforeHtml) == 1);
    REQUIRE(static_cast<int>(InsertionMode::InBody) == 7);
    REQUIRE(static_cast<int>(InsertionMode::InTemplate) == 21);
}

TEST_CASE("is_all_whitespace", "[treebuilder_utils]") {
    REQUIRE(is_all_whitespace("") == true);
    REQUIRE(is_all_whitespace(" ") == true);
    REQUIRE(is_all_whitespace("\t\n\f\r ") == true);
    REQUIRE(is_all_whitespace("  \t  \n  ") == true);
    REQUIRE(is_all_whitespace("a") == false);
    REQUIRE(is_all_whitespace(" a ") == false);
    REQUIRE(is_all_whitespace("\xc2\xa0") == false);  // &nbsp; is not HTML5 whitespace
}

TEST_CASE("contains_prefix", "[treebuilder_utils]") {
    std::vector<std::string> prefixes = {"foo", "bar", "baz"};
    REQUIRE(contains_prefix(prefixes, "foobar") == true);
    REQUIRE(contains_prefix(prefixes, "barfoo") == true);
    REQUIRE(contains_prefix(prefixes, "bazzy") == true);
    REQUIRE(contains_prefix(prefixes, "qux") == false);
    REQUIRE(contains_prefix(prefixes, "") == false);
    REQUIRE(contains_prefix(prefixes, "fo") == false);
}

TEST_CASE("doctype_error_and_quirks", "[treebuilder_utils]") {
    SECTION("Standard HTML5 doctype") {
        Doctype dt("html");
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(error == false);
        REQUIRE(mode == "no-quirks");
    }

    SECTION("Legacy compat doctype") {
        Doctype dt("html", std::nullopt, std::string("about:legacy-compat"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(error == false);
        REQUIRE(mode == "no-quirks");
    }

    SECTION("HTML 4.01 Strict") {
        Doctype dt("html", std::string("-//W3C//DTD HTML 4.01//EN"),
                   std::string("http://www.w3.org/TR/html4/strict.dtd"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(error == false);
        REQUIRE(mode == "no-quirks");
    }

    SECTION("Force quirks") {
        Doctype dt("html", std::nullopt, std::nullopt, true);
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(mode == "quirks");
    }

    SECTION("Non-html name") {
        Doctype dt("xhtml");
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(error == true);
        REQUIRE(mode == "quirks");
    }

    SECTION("Missing name") {
        Doctype dt;
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(error == true);
        REQUIRE(mode == "quirks");
    }

    SECTION("iframe srcdoc always no-quirks") {
        Doctype dt;  // Empty doctype
        auto [error, mode] = doctype_error_and_quirks(dt, true);
        REQUIRE(mode == "no-quirks");
    }

    SECTION("Quirky public match") {
        Doctype dt("html", std::string("html"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(mode == "quirks");
    }

    SECTION("Limited-quirks XHTML 1.0 Transitional") {
        Doctype dt("html", std::string("-//W3C//DTD XHTML 1.0 Transitional//EN"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(mode == "limited-quirks");
    }

    SECTION("HTML 4.01 Transitional without system id is quirks") {
        Doctype dt("html", std::string("-//W3C//DTD HTML 4.01 Transitional//EN"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(mode == "quirks");
    }

    SECTION("HTML 4.01 Transitional with system id is limited-quirks") {
        Doctype dt("html", std::string("-//W3C//DTD HTML 4.01 Transitional//EN"),
                   std::string("http://www.w3.org/TR/html4/loose.dtd"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(mode == "limited-quirks");
    }

    SECTION("IBM quirky system ID") {
        Doctype dt("html", std::nullopt,
                   std::string("http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd"));
        auto [error, mode] = doctype_error_and_quirks(dt);
        REQUIRE(mode == "quirks");
    }
}
