cmake_minimum_required(VERSION 3.20)
project(justhtml VERSION 0.1.0 LANGUAGES CXX)

# ---- C++20 Standard ----
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# ---- Compiler Warnings ----
if(MSVC)
    add_compile_options(/W4 /permissive-)
else()
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

# ---- Export compile_commands.json for tooling ----
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ---- Library Target ----
add_library(justhtml)

target_include_directories(justhtml
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
)

# Collect source files
file(GLOB_RECURSE JUSTHTML_SOURCES
    "${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp"
)
# Exclude CLI sources from library
list(FILTER JUSTHTML_SOURCES EXCLUDE REGEX ".*/cli/.*")

target_sources(justhtml PRIVATE ${JUSTHTML_SOURCES})

# ---- CLI Target ----
add_executable(justhtmlc src/cli/main.cpp)
target_link_libraries(justhtmlc PRIVATE justhtml)

# ---- Testing ----
option(JUSTHTML_BUILD_TESTS "Build tests" ON)

if(JUSTHTML_BUILD_TESTS)
    enable_testing()

    include(FetchContent)
    FetchContent_Declare(
        Catch2
        GIT_REPOSITORY https://github.com/catchorg/Catch2.git
        GIT_TAG        v3.5.2
    )
    FetchContent_MakeAvailable(Catch2)

    # Collect test sources
    file(GLOB_RECURSE JUSTHTML_TEST_SOURCES
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/unit/*.cpp"
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/integration/*.cpp"
    )

    if(JUSTHTML_TEST_SOURCES)
        add_executable(justhtml_tests ${JUSTHTML_TEST_SOURCES})
        target_link_libraries(justhtml_tests PRIVATE justhtml Catch2::Catch2WithMain)

        # Make test data path available to tests
        target_compile_definitions(justhtml_tests PRIVATE
            JUSTHTML_TEST_DATA_DIR="${CMAKE_CURRENT_SOURCE_DIR}/tests/data"
        )

        include(CTest)
        include(Catch)
        catch_discover_tests(justhtml_tests)
    endif()
endif()
