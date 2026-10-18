#include "clarity/text.hpp"

#include "clarity/errors.hpp"

namespace clarity {

namespace {

std::optional<std::string> named(const std::string& format) {
    if (format.empty()) return std::nullopt;
    return format;
}

}  // namespace

std::string Slot::plain() const {
    if (value.is_null()) return "";
    if (value.is_boolean()) return value.get<bool>() ? "yes" : "no";
    if (value.is_string()) return value.get<std::string>();
    return value.dump();
}

Slot slot_text(const std::string& value) { return {"text", value, {}, {}, {}}; }
Slot slot_integer(int64_t value, const std::string& format) { return {"integer", value, {}, named(format), {}}; }

Slot slot_decimal(const Decimal& value, int scale, const std::string& format) {
    Slot s{"decimal", value.str(), {}, named(format), {}};
    if (scale >= 0) s.scale = scale;
    return s;
}

Slot slot_money(const Decimal& value, const std::string& format) { return {"money", value.str(), {}, named(format), {}}; }

Slot slot_date(const std::optional<Date>& value, const std::string& format) {
    return {"date", value ? nlohmann::json(value->str()) : nlohmann::json(), {}, named(format), {}};
}

Slot slot_date_time(const std::optional<DateTime>& value, const std::string& format) {
    return {"dateTime", value ? nlohmann::json(value->str()) : nlohmann::json(), {}, named(format), {}};
}

Slot slot_duration(int64_t seconds) { return {"duration", seconds, {}, {}, {}}; }
Slot slot_bool(bool value) { return {"bool", value, {}, {}, {}}; }
Slot slot_enum(const std::string& value, const std::string& enum_name) { return {"enum", value, {}, {}, enum_name}; }

Text Text::plain(const std::string& text) {
    std::string escaped;
    for (char c : text) {
        if (c == '{' || c == '}') escaped += c;
        escaped += c;
    }
    return Text(escaped);
}

Text Text::with(const std::string& path, Slot value) const {
    Text t = *this;
    t.values_[path] = std::move(value);
    return t;
}

std::string Text::show() const {
    std::string out;
    const std::string& t = template_;
    size_t i = 0;
    while (i < t.size()) {
        char c = t[i];
        if ((c == '{' || c == '}') && i + 1 < t.size() && t[i + 1] == c) {
            // {{ and }} are braces.
            out += c;
            i += 2;
        } else if (c == '{') {
            size_t end = t.find('}', i + 1);
            if (end == std::string::npos) end = t.size();
            auto slot = values_.find(t.substr(i + 1, end - i - 1));
            if (slot != values_.end()) out += slot->second.plain();
            i = end + 1;
        } else {
            out += c;
            ++i;
        }
    }
    return out;
}

void to_json(nlohmann::json& out, const Decimal& value) { out = value.str(); }

void from_json(const nlohmann::json& in, Decimal& value) {
    if (in.is_string()) value = Decimal::parse(in.get<std::string>());
    else if (in.is_number_integer()) value = Decimal(in.get<int64_t>());
    else if (in.is_number()) value = Decimal::parse(in.dump());
    else value = Decimal();
}

void to_json(nlohmann::json& out, const Date& value) { out = value.str(); }

void from_json(const nlohmann::json& in, Date& value) {
    auto d = Date::parse(in.get<std::string>());
    if (!d) throw InvalidError("'" + in.get<std::string>() + "' is no date");
    value = *d;
}

void to_json(nlohmann::json& out, const DateTime& value) { out = value.str(); }

void from_json(const nlohmann::json& in, DateTime& value) {
    auto t = DateTime::parse(in.get<std::string>());
    if (!t) throw InvalidError("'" + in.get<std::string>() + "' is no date and time");
    value = *t;
}

void to_json(nlohmann::json& out, const Slot& slot) {
    out = {{"kind", slot.kind}, {"value", slot.value}};
    if (slot.scale) out["scale"] = *slot.scale;
    if (slot.format) out["format"] = *slot.format;
    if (slot.enum_name) out["enum"] = *slot.enum_name;
}

void from_json(const nlohmann::json& in, Slot& slot) {
    slot.kind = in.at("kind").get<std::string>();
    slot.value = in.value("value", nlohmann::json());
    if (in.contains("scale")) slot.scale = in["scale"].get<int>();
    if (in.contains("format")) slot.format = in["format"].get<std::string>();
    if (in.contains("enum")) slot.enum_name = in["enum"].get<std::string>();
}

void to_json(nlohmann::json& out, const Text& text) {
    if (text.values().empty()) {
        out = text.show();
        return;
    }
    out = {{"text", text.text()}, {"values", text.values()}};
}

void from_json(const nlohmann::json& in, Text& text) {
    if (in.is_string()) {
        text = Text::plain(in.get<std::string>());
        return;
    }
    Text t(in.at("text").get<std::string>());
    for (auto& [path, value] : in.at("values").items()) t = t.with(path, value.get<Slot>());
    text = t;
}

}  // namespace clarity
