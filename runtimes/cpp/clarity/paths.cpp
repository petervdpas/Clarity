#include "clarity/paths.hpp"

#include <optional>
#include <vector>

#include "clarity/errors.hpp"

namespace clarity {

namespace {

/// The names of a path in a window's data: `album.tags` is the field `tags` of the record `album`.
std::vector<std::string> names_of(const std::string& path) {
    if (path.empty()) throw InvalidError("an action of the person names no value of the window");
    std::vector<std::string> names;
    size_t from = 0;
    for (size_t dot; (dot = path.find('.', from)) != std::string::npos; from = dot + 1) names.push_back(path.substr(from, dot - from));
    names.push_back(path.substr(from));
    return names;
}

/// A row of a list by its place, when `name` is one.
std::optional<size_t> place_of(const nlohmann::json& list, const std::string& name) {
    if (!list.is_array() || name.empty() || name.size() > 9 || name.find_first_not_of("0123456789") != std::string::npos) return std::nullopt;
    size_t place = std::stoul(name);
    return place < list.size() ? std::optional(place) : std::nullopt;
}

}  // namespace

std::optional<nlohmann::json> found_at(const nlohmann::json& data, const std::string& path) {
    const nlohmann::json* at = &data;
    for (const auto& name : names_of(path)) {
        if (auto place = place_of(*at, name)) at = &(*at)[*place];
        else if (at->is_object() && at->contains(name)) at = &(*at)[name];
        else return std::nullopt;
    }
    return *at;
}

nlohmann::json value_at(const nlohmann::json& data, const std::string& path) { return found_at(data, path).value_or(nullptr); }

nlohmann::json& place_at(nlohmann::json& data, const std::string& path) {
    nlohmann::json* at = &data;
    for (const auto& name : names_of(path)) {
        if (auto place = place_of(*at, name)) {
            at = &(*at)[*place];
            continue;
        }
        if (!at->is_object()) *at = nlohmann::json::object();
        at = &(*at)[name];
    }
    return *at;
}

}  // namespace clarity
