# The library of custom part OrbField, a control of Clarity.UI: it draws on
# the canvas Clarity.UI gives. Clarity builds it into the program; the
# target keeps the part's name.
cmake_minimum_required(VERSION 3.20)
project(OrbField LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
add_library(OrbField STATIC OrbField.cpp)
target_include_directories(OrbField PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
