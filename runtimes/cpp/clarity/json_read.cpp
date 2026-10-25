// Fitting a service's JSON to an entity; see json_read.hpp.

#include "clarity/json_read.hpp"

#include <cctype>
#include <cmath>

namespace clarity {

namespace {

using nlohmann::json;

/// A name as names are matched: lower case, without underscores, hyphens and spaces.
std::string plain(const std::string& name) {
    std::string out;
    for (char c : name) {
        if (c == '_' || c == '-' || c == ' ') continue;
        out += static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    }
    return out;
}

std::string wanted(const JsonKind& kind) {
    switch (kind.is) {
        case JsonKind::Text: return "text";
        case JsonKind::Integer: return "an integer";
        case JsonKind::Decimal: return "a decimal";
        case JsonKind::Bool: return "true or false";
        case JsonKind::Date: return "a date";
        case JsonKind::DateTime: return "a date and time";
        case JsonKind::Values: return "a list";
        case JsonKind::Other: return "something else";
        case JsonKind::Record: return "one " + kind.entity;
        case JsonKind::Records: return "a list of " + kind.entity;
        case JsonKind::Enum: {
            if (kind.values.empty()) return "an enum value";
            std::string out;
            for (size_t i = 0; i + 1 < kind.values.size(); ++i) out += (i ? ", " : "") + kind.values[i];
            return kind.values.size() == 1 ? kind.values[0] : out + " or " + kind.values.back();
        }
    }
    return "something else";
}

bool digits(const std::string& text) {
    if (text.empty()) return false;
    for (char c : text) {
        if (!std::isdigit(static_cast<unsigned char>(c))) return false;
    }
    return true;
}

bool is_decimal(std::string text) {
    if (!text.empty() && text[0] == '-') text = text.substr(1);
    auto dot = text.find('.');
    if (dot == std::string::npos) return digits(text);
    return digits(text.substr(0, dot)) && digits(text.substr(dot + 1));
}

bool is_date(const std::string& text) {
    return text.size() == 10 && text[4] == '-' && text[7] == '-' && digits(text.substr(0, 4)) && digits(text.substr(5, 2)) && digits(text.substr(8, 2));
}

/// A date and time as Clarity writes it, YYYY-MM-DDTHH:MM:SS, from the same
/// with or without seconds, a fraction of a second and a Z.
std::optional<std::string> date_time(std::string text) {
    if (!text.empty() && text.back() == 'Z') text.pop_back();
    auto at = text.find_first_of("T ");
    if (at == std::string::npos || !is_date(text.substr(0, at))) return std::nullopt;
    std::string clock = text.substr(at + 1);
    clock = clock.substr(0, clock.find('.'));
    auto two = [](const std::string& p) { return p.size() == 2 && digits(p); };
    if (clock.size() == 5 && clock[2] == ':' && two(clock.substr(0, 2)) && two(clock.substr(3, 2))) return text.substr(0, at) + "T" + clock + ":00";
    if (clock.size() == 8 && clock[2] == ':' && clock[5] == ':' && two(clock.substr(0, 2)) && two(clock.substr(3, 2)) && two(clock.substr(6, 2)))
        return text.substr(0, at) + "T" + clock;
    return std::nullopt;
}

std::optional<json> fit_value(const json& given, const JsonKind& kind) {
    switch (kind.is) {
        case JsonKind::Text:
            if (given.is_string()) return given;
            if (given.is_number() || given.is_boolean()) return json(given.dump());
            return std::nullopt;
        case JsonKind::Integer:
            if (given.is_number_integer()) return json(given.get<int64_t>());
            if (given.is_number_float()) {
                double d = given.get<double>();
                if (std::floor(d) == d && std::fabs(d) < 9e15) return json(static_cast<int64_t>(d));
                return std::nullopt;
            }
            if (given.is_string()) {
                std::string text = given.get<std::string>();
                auto start = text.find_first_not_of(' ');
                auto end = text.find_last_not_of(' ');
                if (start == std::string::npos) return std::nullopt;
                text = text.substr(start, end - start + 1);
                bool negative = !text.empty() && text[0] == '-';
                if (digits(negative ? text.substr(1) : text)) return json(std::stoll(text));
            }
            return std::nullopt;
        case JsonKind::Decimal:
            if (given.is_number()) return json(given.dump());
            if (given.is_string() && is_decimal(given.get<std::string>())) return given;
            return std::nullopt;
        case JsonKind::Bool:
            if (given.is_boolean()) return given;
            return std::nullopt;
        case JsonKind::Date:
            if (!given.is_string()) return std::nullopt;
            if (is_date(given.get<std::string>())) return given;
            if (auto moment = date_time(given.get<std::string>())) return json(moment->substr(0, 10));
            return std::nullopt;
        case JsonKind::DateTime:
            if (!given.is_string()) return std::nullopt;
            if (auto moment = date_time(given.get<std::string>())) return json(*moment);
            return std::nullopt;
        case JsonKind::Enum:
            if (!given.is_string()) return std::nullopt;
            for (const auto& value : kind.values) {
                const std::string word = given.get<std::string>();
                if (value.size() == word.size() && std::equal(value.begin(), value.end(), word.begin(), [](char a, char b) {
                        return std::tolower(static_cast<unsigned char>(a)) == std::tolower(static_cast<unsigned char>(b));
                    }))
                    return json(value);
            }
            return std::nullopt;
        case JsonKind::Values: {
            if (!given.is_array() || !kind.item) return std::nullopt;
            json list = json::array();
            for (const auto& item : given) {
                auto fitted = fit_value(item, *kind.item);
                if (!fitted) return std::nullopt;
                list.push_back(*fitted);
            }
            return list;
        }
        case JsonKind::Other:
        case JsonKind::Record:
        case JsonKind::Records:
            return std::nullopt;
    }
    return std::nullopt;
}

json fit_record(const json& given, const std::string& entity, const std::vector<JsonField>& fields, const json& base);

/// A record inside the record, or a list of them, fitted to the field's own
/// entity; an error says where (DESIGN 5.25).
json fit_nested(const json& value, const JsonField& field, const std::string& entity) {
    const JsonKind& kind = field.kind;
    static const std::vector<JsonField> none;
    const std::vector<JsonField>& fields = kind.fields ? *kind.fields : none;
    std::string where = "field '" + field.name + "' of " + entity;
    if (kind.is == JsonKind::Record) {
        if (!value.is_object()) throw ValueError(where + " is one " + kind.entity + ", not " + value.dump());
        try {
            return fit_record(value, kind.entity, fields, kind.base);
        } catch (const ValueError& e) {
            throw ValueError(where + ": " + e.what());
        }
    }
    if (!value.is_array()) throw ValueError(where + " is a list of " + kind.entity + ", not " + value.dump());
    json list = json::array();
    for (size_t i = 0; i < value.size(); ++i) {
        std::string item = where + ", item " + std::to_string(i + 1);
        if (!value[i].is_object()) throw ValueError(item + " is not one " + kind.entity);
        try {
            list.push_back(fit_record(value[i], kind.entity, fields, kind.base));
        } catch (const ValueError& e) {
            throw ValueError(item + ": " + e.what());
        }
    }
    return list;
}

json fit_record(const json& given, const std::string& entity, const std::vector<JsonField>& fields, const json& base) {
    json record = base.is_object() ? base : json::object();
    for (const auto& field : fields) {
        const json* value = nullptr;
        if (given.contains(field.name)) {
            value = &given[field.name];
        } else {
            for (const auto& [name, v] : given.items()) {
                if (plain(name) == plain(field.name)) {
                    value = &v;
                    break;
                }
            }
        }
        if (!value || value->is_null() || field.kind.is == JsonKind::Other) continue;
        if (field.kind.is == JsonKind::Record || field.kind.is == JsonKind::Records) {
            record[field.name] = fit_nested(*value, field, entity);
            continue;
        }
        auto fitted = fit_value(*value, field.kind);
        if (!fitted) throw ValueError("field '" + field.name + "' of " + entity + " is " + wanted(field.kind) + ", not " + value->dump());
        record[field.name] = *fitted;
    }
    return record;
}

}  // namespace

nlohmann::json json_fit(const std::string& text, const std::string& at, bool many, const std::string& entity, const std::vector<JsonField>& fields, const nlohmann::json& base) {
    json value = json::parse(text, nullptr, false);
    if (value.is_discarded()) throw ValueError("the text is no JSON");
    if (!at.empty()) {
        std::string walked;
        size_t start = 0;
        for (;;) {
            size_t dot = at.find('.', start);
            std::string step = at.substr(start, dot == std::string::npos ? std::string::npos : dot - start);
            walked += (walked.empty() ? "" : ".") + step;
            if (!value.is_object() || !value.contains(step)) throw ValueError("the JSON has no '" + walked + "'");
            json next = value[step];
            value = std::move(next);
            if (dot == std::string::npos) break;
            start = dot + 1;
        }
    }
    if (many) {
        if (!value.is_array()) throw ValueError("the JSON is not a list of " + entity);
        json list = json::array();
        for (size_t i = 0; i < value.size(); ++i) {
            if (!value[i].is_object()) throw ValueError("item " + std::to_string(i + 1) + " of the list is not one " + entity);
            try {
                list.push_back(fit_record(value[i], entity, fields, base));
            } catch (const ValueError& e) {
                throw ValueError("item " + std::to_string(i + 1) + ": " + e.what());
            }
        }
        return list;
    }
    if (!value.is_object()) throw ValueError("the JSON is not one " + entity);
    return fit_record(value, entity, fields, base);
}

}  // namespace clarity
