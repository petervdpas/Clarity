// The cases every stack's runtime shares, in the repository's conformance folder.

#pragma once

#include <fstream>
#include <sstream>
#include <string>

#include <nlohmann/json.hpp>

inline nlohmann::json conformance(const std::string& name) {
    std::ifstream in(std::string(CLARITY_CONFORMANCE) + "/" + name);
    std::stringstream text;
    text << in.rdbuf();
    return nlohmann::json::parse(text.str());
}
