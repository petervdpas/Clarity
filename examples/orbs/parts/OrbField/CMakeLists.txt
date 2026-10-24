# The library that draws custom part OrbField, with SFML. Clarity builds it
# into the program; the target keeps the part's name.
cmake_minimum_required(VERSION 3.20)
project(OrbField LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
# SFML 3 where it is (MSYS2 on Windows), else SFML 2.6 (Debian and Ubuntu).
# The two name their components and their targets differently, and the
# part's own code follows SFML_VERSION_MAJOR.
find_package(SFML 3 COMPONENTS Graphics Window System QUIET)
if(SFML_FOUND)
    set(ORBFIELD_SFML SFML::Graphics SFML::Window SFML::System)
else()
    find_package(SFML 2.6 COMPONENTS graphics window system REQUIRED)
    set(ORBFIELD_SFML sfml-graphics sfml-window sfml-system)
endif()

add_library(OrbField STATIC OrbField.cpp)
target_include_directories(OrbField PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(OrbField PRIVATE ${ORBFIELD_SFML})
