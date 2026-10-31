#include "clarity/list_fields.hpp"

#include <cctype>
#include <charconv>

#include "clarity/text.hpp"
#include "clarity/values.hpp"

namespace clarity {

using nlohmann::json;

namespace {

bool digits(const std::string& text, size_t from, size_t to) {
    if (from >= to) return false;
    for (size_t i = from; i < to; ++i)
        if (!std::isdigit(static_cast<unsigned char>(text[i]))) return false;
    return true;
}

/// `-12`: an optional minus and digits.
bool whole(const std::string& text) { return digits(text, text.rfind('-', 0) == 0 ? 1 : 0, text.size()); }

/// `-12.5`: a whole number with, or without, digits after one point.
bool decimal(const std::string& text) {
    size_t from = text.rfind('-', 0) == 0 ? 1 : 0;
    size_t point = text.find('.', from);
    if (point == std::string::npos) return digits(text, from, text.size());
    return digits(text, from, point) && digits(text, point + 1, text.size());
}

/// A decimal written alike for the same value: `1.50` and `1.5` are one item.
std::string plain(std::string text) {
    bool negative = text.rfind('-', 0) == 0;
    if (negative) text.erase(0, 1);
    size_t point = text.find('.');
    std::string w = text.substr(0, point);
    std::string f = point == std::string::npos ? "" : text.substr(point + 1);
    while (w.size() > 1 && w[0] == '0') w.erase(0, 1);
    while (!f.empty() && f.back() == '0') f.pop_back();
    std::string shown = f.empty() ? w : w + "." + f;
    return negative && shown != "0" ? "-" + shown : shown;
}

std::string text_of(const json& item) { return item.is_string() ? item.get<std::string>() : item.dump(); }

}  // namespace

bool same_item(const std::string& kind, const json& a, const json& b) {
    if (kind == "decimal") return plain(text_of(a)) == plain(text_of(b));
    return a == b;
}

AddedItem add_typed(const std::string& kind, const json& items, const std::string& typed, const json& values) {
    json list = items.is_array() ? items : json::array();
    std::string text = trimmed(typed);
    if (text.empty()) return {list};
    json item = text;
    if (kind == "integer") {
        int64_t n = 0;
        if (!whole(text) || std::from_chars(text.data(), text.data() + text.size(), n).ec != std::errc{}) return {list, true};
        item = n;
    } else if (kind == "decimal") {
        if (!decimal(text)) return {list, true};
    } else if (kind == "date") {
        auto date = Date::parse(text);
        if (!date) return {list, true};
        item = date->str();
    } else if (kind == "enum") {
        bool known = !values.is_array() || values.empty();
        for (const auto& v : values.is_array() ? values : json::array()) known = known || v == item;
        if (!known) return {list, true};
    }
    for (const auto& i : list)
        if (same_item(kind, i, item)) return {list};
    list.push_back(item);
    return {list};
}

}  // namespace clarity
