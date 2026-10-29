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
/// This machine's system, as the cases name it: "windows", "macos" or "linux".
std::string settings_os();
/// The folder of file store `store` when a folder setting names it, on
/// system `os`: the `kept` value when it is text that is not empty, else
/// `fallback` (the setting's default), else a folder named after the store;
/// a relative folder is in `settings_folder`, the folder of the settings file.
std::string store_folder_in(const std::string& os, const std::string& settings_folder, const std::string& store, const nlohmann::json& kept, const std::string& fallback);
/// The settings `program` kept; an empty object when it kept none, or they cannot be read.
nlohmann::json read_settings(const std::string& program);
/// Keeps `values` as the settings of `program`, written whole and then put in place.
void keep_settings(const std::string& program, const nlohmann::json& values);

/// The system's protected store of secret settings (DESIGN 5.27): `get`
/// gives a secret or none, `set` keeps one ("" forgets it). Empty in a
/// program without secret settings; one with them calls secrets_use
/// (secrets/secrets.hpp) to fill it.
struct SecretStore {
    std::function<std::optional<std::string>(const std::string& program, const std::string& name)> get;
    std::function<void(const std::string& program, const std::string& name, const std::string& value)> set;
};
extern SecretStore secret_store;

/// read_settings with the secrets named in `secrets` (an array of names, or
/// null), each from the store, beside the file's values.
nlohmann::json read_settings_with(const std::string& program, const nlohmann::json& secrets);
/// keep_settings with those named in `secrets` kept in the store, never in the file.
void keep_settings_with(const std::string& program, const nlohmann::json& values, const nlohmann::json& secrets);

}  // namespace clarity
