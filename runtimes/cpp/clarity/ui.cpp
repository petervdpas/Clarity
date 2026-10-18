#include "clarity/ui.hpp"

namespace clarity {

namespace {

template <typename T, typename F>
T part_or(const std::map<std::string, PartState>& parts, const std::string& part, F field, T design) {
    auto found = parts.find(part);
    if (found == parts.end()) return design;
    const auto& value = found->second.*field;
    return value ? T(*value) : design;
}

std::string level_name(Level level) {
    switch (level) {
    case Level::Warning: return "warning";
    case Level::Error: return "error";
    default: return "info";
    }
}

}  // namespace

bool Ui::visible(const std::string& part) const { return part_or(parts_, part, &PartState::visible, true); }
bool Ui::enabled(const std::string& part) const { return part_or(parts_, part, &PartState::enabled, true); }
bool Ui::readonly(const std::string& part, bool design) const { return part_or(parts_, part, &PartState::readonly, design); }
bool Ui::checked(const std::string& part) const { return part_or(parts_, part, &PartState::checked, false); }
std::string Ui::text(const std::string& part) const { return part_or(parts_, part, &PartState::text, std::string()); }

std::string Ui::caption(const std::string& part, const std::string& design) const {
    auto found = parts_.find(part);
    return found != parts_.end() && found->second.caption ? found->second.caption->show() : design;
}

std::string Ui::error(const std::string& part) const {
    auto found = parts_.find(part);
    return found != parts_.end() && found->second.error ? found->second.error->show() : "";
}

Reply Ui::reply(const std::string& window, nlohmann::json data) const {
    if (!data.is_object()) data = nlohmann::json::object();
    return Reply{WindowState{window, std::move(data), parts_, selected_}, messages_, open_, close_};
}

WindowState opened(const std::string& window, nlohmann::json data, const nlohmann::json& with) {
    WindowState state{window, data.is_object() ? std::move(data) : nlohmann::json::object(), {}, {}};
    for (auto& [name, value] : with.items()) state.data[name] = value;
    return state;
}

void to_json(nlohmann::json& out, const PartState& p) {
    out = nlohmann::json::object();
    if (p.visible) out["visible"] = *p.visible;
    if (p.enabled) out["enabled"] = *p.enabled;
    if (p.caption) out["caption"] = *p.caption;
    if (p.error) out["error"] = *p.error;
    if (p.readonly) out["readonly"] = *p.readonly;
    if (p.text) out["text"] = *p.text;
    if (p.checked) out["checked"] = *p.checked;
}

void from_json(const nlohmann::json& in, PartState& p) {
    auto take = [&](const char* key, auto& field) {
        if (in.contains(key) && !in[key].is_null()) field = in[key].get<typename std::decay_t<decltype(field)>::value_type>();
    };
    take("visible", p.visible);
    take("enabled", p.enabled);
    take("caption", p.caption);
    take("error", p.error);
    take("readonly", p.readonly);
    take("text", p.text);
    take("checked", p.checked);
}

void to_json(nlohmann::json& out, const WindowState& s) {
    out = {{"window", s.window}, {"data", s.data}, {"parts", s.parts}, {"selected", s.selected}};
}

void from_json(const nlohmann::json& in, WindowState& s) {
    s.window = in.value("window", "");
    s.data = in.value("data", nlohmann::json::object());
    if (in.contains("parts")) s.parts = in["parts"].get<std::map<std::string, PartState>>();
    if (in.contains("selected")) s.selected = in["selected"].get<std::map<std::string, int>>();
}

void to_json(nlohmann::json& out, const Reply& r) {
    nlohmann::json messages = nlohmann::json::array();
    for (const auto& m : r.messages) messages.push_back({{"level", level_name(m.level)}, {"text", m.text}});
    nlohmann::json open = nlohmann::json::array();
    for (const auto& o : r.open) open.push_back({{"window", o.window}, {"with", o.with}});
    out = {{"state", r.state}, {"messages", messages}, {"open", open}, {"close", r.close}};
}

void to_json(nlohmann::json& out, const RefOption& o) { out = {{"id", o.id}, {"text", o.text}}; }

}  // namespace clarity
