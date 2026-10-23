# Clarity's part host (DESIGN 6.5), on its own with its tests:
#
#   cmake -S . -B build -G Ninja && cmake --build build && ctest --test-dir build
#
# A generated project builds it with its glue and its native parts into one
# shared library through clarity_parts.cmake.
cmake_minimum_required(VERSION 3.20)
project(clarity_part_host LANGUAGES C)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
find_package(Threads REQUIRED)

set(CJSON_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../c/third_party/cjson)

enable_testing()
add_executable(clarity_parts_tests tests/parts_test.c clarity_parts.c platform_none.c ${CJSON_DIR}/cJSON.c)
target_include_directories(clarity_parts_tests PRIVATE ${CMAKE_CURRENT_SOURCE_DIR} ${CJSON_DIR})
target_compile_definitions(clarity_parts_tests PRIVATE CLARITY_PARTS_TEST)
target_link_libraries(clarity_parts_tests PRIVATE Threads::Threads)
add_test(NAME clarity_parts_tests COMMAND clarity_parts_tests)
