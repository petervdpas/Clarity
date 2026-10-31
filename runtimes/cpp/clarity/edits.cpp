// The edit bus: see edits.hpp.

#include "clarity/edits.hpp"

#include <algorithm>
#include <map>

namespace clarity {

using nlohmann::json;

namespace {

/// How many edits a window keeps; the oldest go first.
constexpr size_t max_edits = 500;

/// What changed from before to after, by name.
std::vector<Changed> between(const json& before, const json& after) {
    std::vector<Changed> changes;
    auto value_of = [](const json& data, const std::string& name) -> std::optional<json> {
        auto found = data.find(name);
        if (found == data.end()) return std::nullopt;
        return *found;
    };
    auto compare = [&](const std::string& name) {
        auto was = value_of(before, name);
        auto now = value_of(after, name);
        if (was != now) changes.push_back({name, was, now});
    };
    for (const auto& [name, _] : before.items()) compare(name);
    for (const auto& [name, _] : after.items()) {
        if (!before.contains(name)) compare(name);
    }
    return changes;
}

/// Names a part's own text or tick among the data an edit compares.
const std::string part_state = "#parts.";

}  // namespace

void EditBus::reset(const json& data) {
    std::lock_guard lock(mutex_);
    if (going_) return;
    baseline_ = data;
    based_ = true;
}

std::optional<json> EditBus::baseline() {
    std::lock_guard lock(mutex_);
    if (!based_) return std::nullopt;
    return baseline_;
}

bool table_edit_fits(const json& before, const json& after, const std::vector<std::string>& editable) {
    json was = before.is_array() ? before : json::array();
    json now = after.is_array() ? after : json::array();
    if (was.size() != now.size()) return false;
    for (size_t i = 0; i < was.size(); ++i) {
        const json& a = was[i];
        const json& b = now[i];
        if (!a.is_object() || !b.is_object()) {
            if (a != b) return false;
            continue;
        }
        for (const json* row : {&a, &b}) {
            for (auto it = row->begin(); it != row->end(); ++it) {
                const std::string& field = it.key();
                if (std::find(editable.begin(), editable.end(), field) != editable.end()) continue;
                // A field one of them has not is null there.
                json x = a.contains(field) ? a[field] : json(nullptr);
                json y = b.contains(field) ? b[field] : json(nullptr);
                if (x != y || a.contains(field) != b.contains(field)) return false;
            }
        }
    }
    return true;
}

void EditBus::drag(const std::string& part, const std::string& scope, const std::vector<int64_t>& steps) {
    std::lock_guard lock(mutex_);
    if (!going_) going_ = Edit{part, scope, {}, {}};
    going_->steps.insert(going_->steps.end(), steps.begin(), steps.end());
}

void EditBus::edit(const std::string& part, const std::string& scope, const json& data, const std::vector<int64_t>& steps) {
    std::lock_guard lock(mutex_);
    Edit edit{part, scope, {}, {}};
    if (going_) {
        edit.steps = std::move(going_->steps);
        going_.reset();
    }
    edit.steps.insert(edit.steps.end(), steps.begin(), steps.end());
    edit.changes = between(baseline_, data);
    baseline_ = data;
    if (edit.changes.empty() && edit.steps.empty()) return;
    done_.push_back(std::move(edit));
    if (done_.size() > max_edits) done_.erase(done_.begin(), done_.end() - max_edits);
    std::erase_if(undone_, [&](const Edit& e) { return e.scope == scope; });
}

std::optional<json> EditBus::undo(const std::string& scope, const json& data, bool back, const TakeSteps& take) {
    std::lock_guard lock(mutex_);
    auto& from = back ? done_ : undone_;
    auto& to = back ? undone_ : done_;
    auto at = std::find_if(from.rbegin(), from.rend(), [&](const Edit& e) { return e.scope == scope; });
    if (at == from.rend()) return std::nullopt;
    // Back takes the steps back last first; again does them first first.
    std::vector<int64_t> steps = at->steps;
    if (back) std::reverse(steps.begin(), steps.end());
    if (!steps.empty()) take(steps, back);
    Edit edit = std::move(*at);
    from.erase(std::next(at).base());
    json result = data.is_object() ? data : json::object();
    auto apply = [&](const Changed& c) {
        const auto& value = back ? c.before : c.after;
        if (value) result[c.name] = *value;
        else result.erase(c.name);
    };
    if (back) std::for_each(edit.changes.rbegin(), edit.changes.rend(), apply);
    else std::for_each(edit.changes.begin(), edit.changes.end(), apply);
    to.push_back(std::move(edit));
    baseline_ = result;
    return result;
}

namespace {

std::mutex buses_mutex;
/// The edit bus of each open window, by its number.
std::map<uint64_t, std::shared_ptr<EditBus>> buses;

}  // namespace

std::shared_ptr<EditBus> edits_of(uint64_t instance) {
    std::lock_guard lock(buses_mutex);
    auto& bus = buses[instance];
    if (!bus) bus = std::make_shared<EditBus>();
    return bus;
}

void edits_closed(uint64_t instance) {
    std::lock_guard lock(buses_mutex);
    buses.erase(instance);
}

json edit_data(const WindowState& state) {
    json data = state.data.is_object() ? state.data : json::object();
    for (const auto& [name, part] : state.parts) {
        json own = json::object();
        if (part.text) own["text"] = *part.text;
        if (part.checked) own["checked"] = *part.checked;
        if (!own.empty()) data[part_state + name] = own;
    }
    return data;
}

WindowState with_edit_data(WindowState state, const json& data) {
    state.data = json::object();
    for (auto& [_, part] : state.parts) {
        part.text.reset();
        part.checked.reset();
    }
    for (const auto& [name, value] : data.items()) {
        if (!name.starts_with(part_state)) {
            state.data[name] = value;
            continue;
        }
        auto& part = state.parts[name.substr(part_state.size())];
        if (!value.is_object()) continue;
        if (auto text = value.find("text"); text != value.end() && text->is_string()) part.text = text->get<std::string>();
        if (auto checked = value.find("checked"); checked != value.end() && checked->is_boolean()) part.checked = checked->get<bool>();
    }
    return state;
}

}  // namespace clarity
