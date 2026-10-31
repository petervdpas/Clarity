// The cases every stack's runtime shares, in the repository's conformance folder.

#pragma once

#include <fstream>
#include <map>
#include <sstream>
#include <stdexcept>
#include <string>

#include <nlohmann/json.hpp>

// Each file is read once and kept for the whole run, so a loop over a part of
// it (`for (auto& c : conformance("x.json")["cases"])`) never holds a value
// that is already gone. A file that is not there fails the test.
inline const nlohmann::json& conformance(const std::string& name) {
    static std::map<std::string, nlohmann::json> read;
    auto found = read.find(name);
    if (found != read.end()) return found->second;
    std::ifstream in(std::string(CLARITY_CONFORMANCE) + "/" + name);
    if (!in) throw std::runtime_error("no conformance file " + name);
    std::stringstream text;
    text << in.rdbuf();
    return read.emplace(name, nlohmann::json::parse(text.str())).first->second;
}
