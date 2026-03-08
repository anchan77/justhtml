/// @file main.cpp
/// @brief Minimal CLI entry point for justhtmlc.

#include "justhtml/justhtml.hpp"
#include <iostream>

int main(int argc, char* argv[]) {
    // Placeholder: will be expanded to read HTML and tokenize
    if (argc > 1 && std::string_view(argv[1]) == "--version") {
        std::cout << "justhtmlc " << justhtml::version() << "\n";
        return 0;
    }

    std::cout << "justhtmlc " << justhtml::version() << "\n";
    std::cout << "Usage: justhtmlc [--version] [file]\n";
    return 0;
}
