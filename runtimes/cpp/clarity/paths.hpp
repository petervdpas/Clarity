// Paths into a window's data (DESIGN 1.7): `album.tags` is the field `tags`
// of the record `album`, `readings` a plain value, and a number in a path is
// a row of a list by its place.

#pragma once

#include <optional>
#include <string>

#include <nlohmann/json.hpp>

namespace clarity {

/// The value at `path` in a window's data; none when it is not there.
std::optional<nlohmann::json> found_at(const nlohmann::json& data, const std::string& path);

/// The value at `path` in a window's data; null when it is not there.
nlohmann::json value_at(const nlohmann::json& data, const std::string& path);

/// The place of `path` in a window's data, made when it is not there.
nlohmann::json& place_at(nlohmann::json& data, const std::string& path);

}  // namespace clarity
