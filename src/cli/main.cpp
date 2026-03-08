/// @file main.cpp
/// @brief CLI entry point for justhtmlc.
///
/// Reads HTML from stdin or a file, tokenizes using the streaming API,
/// and outputs token events to stdout.

#include "justhtml/justhtml.hpp"
#include "justhtml/stream.hpp"

#include <fstream>
#include <iostream>
#include <sstream>
#include <string>

static void print_events(const std::vector<justhtml::StreamEvent>& events) {
    for (const auto& event : events) {
        switch (event.type) {
            case justhtml::StreamEventType::Start: {
                const auto& data = std::get<justhtml::StartTagData>(event.data);
                std::cout << "start <" << data.name;
                for (const auto& [key, value] : data.attrs) {
                    std::cout << " " << key;
                    if (value.has_value()) {
                        std::cout << "=\"" << *value << "\"";
                    }
                }
                std::cout << ">\n";
                break;
            }
            case justhtml::StreamEventType::End: {
                const auto& name = std::get<std::string>(event.data);
                std::cout << "end </" << name << ">\n";
                break;
            }
            case justhtml::StreamEventType::Text: {
                const auto& text = std::get<std::string>(event.data);
                std::cout << "text \"" << text << "\"\n";
                break;
            }
            case justhtml::StreamEventType::Comment: {
                const auto& text = std::get<std::string>(event.data);
                std::cout << "comment \"" << text << "\"\n";
                break;
            }
            case justhtml::StreamEventType::Doctype: {
                const auto& dt = std::get<justhtml::DoctypeData>(event.data);
                std::cout << "doctype";
                if (dt.name.has_value()) std::cout << " " << *dt.name;
                if (dt.public_id.has_value()) std::cout << " \"" << *dt.public_id << "\"";
                if (dt.system_id.has_value()) std::cout << " \"" << *dt.system_id << "\"";
                std::cout << "\n";
                break;
            }
        }
    }
}

int main(int argc, char* argv[]) {
    if (argc > 1 && std::string_view(argv[1]) == "--version") {
        std::cout << "justhtmlc " << justhtml::version() << "\n";
        return 0;
    }

    if (argc > 1 && std::string_view(argv[1]) == "--help") {
        std::cout << "justhtmlc " << justhtml::version() << "\n";
        std::cout << "Usage: justhtmlc [--version] [--help] [file]\n";
        std::cout << "\nReads HTML from file or stdin, tokenizes it,\n";
        std::cout << "and outputs token events to stdout.\n";
        return 0;
    }

    std::string html;

    if (argc > 1) {
        // Read from file
        std::string filename = argv[1];
        std::ifstream file(filename);
        if (!file.is_open()) {
            std::cerr << "Error: could not open file: " << filename << "\n";
            return 1;
        }
        std::ostringstream ss;
        ss << file.rdbuf();
        html = ss.str();
    } else {
        // Read from stdin
        std::ostringstream ss;
        ss << std::cin.rdbuf();
        html = ss.str();
    }

    auto events = justhtml::stream(html);
    print_events(events);

    return 0;
}
