// The settings each person keeps for themselves (DESIGN 5.17): one JSON
// file per application in the folder for application settings of the
// person's system (conformance/settings.json, `file`). The screen decides
// which kept values still fit; this only reads and keeps the file.

#pragma once

#include <functional>
#include <optional>
#include <string>

#include <nlohmann/json.hpp>

namespace clarity {

/// Where `program` keeps its settings on system `os` ("windows", "macos",
/// or another), with `env` giving the environment; none when it names no
/// folder for application settings.
std::optional<std::string> settings_file_in(const std::string& os, const std::function<std::optional<std::string>(const std::string&)>& env, const std::string& program);
/// Where `program` keeps its settings on this machine.
std::optional<std::string> settings_file(const std::string& program);
/// The settings `program` kept; an empty object when it kept none, or they cannot be read.
nlohmann::json read_settings(const std::string& program);
/// Keeps `values` as the settings of `program`, written whole and then put in place.
void keep_settings(const std::string& program, const nlohmann::json& values);

}  // namespace clarity
