# The library that draws custom part OrbField, with SFML. Clarity builds it
# into the program; the target keeps the part's name.
cmake_minimum_required(VERSION 3.20)
project(OrbField LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
find_package(SFML 2.6 COMPONENTS graphics window system REQUIRED)

add_library(OrbField STATIC OrbField.cpp)
target_include_directories(OrbField PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(OrbField PRIVATE sfml-graphics sfml-window sfml-system)
