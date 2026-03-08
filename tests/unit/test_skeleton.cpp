/// @file test_skeleton.cpp
/// @brief Sample test to verify the build and test framework work.

#include <catch2/catch_test_macros.hpp>
#include "justhtml/justhtml.hpp"

TEST_CASE("Library version is available", "[skeleton]") {
    REQUIRE(std::string(justhtml::version()) == "0.1.0");
}
