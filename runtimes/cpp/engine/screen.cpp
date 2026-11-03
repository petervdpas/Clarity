#include "engine/screen.hpp"

#include <cmath>

namespace clarity {

std::unique_ptr<ScreenEngine> ScreenEngine::load(const std::string& name, std::vector<uint8_t> bytes, std::vector<std::string> uses, bool opens, EngineLimits limits) {
    // One instance for the canvas, kept from frame to frame; no window's closing takes it.
    Engine engine = Engine::load(name, std::move(bytes), {{}, std::move(uses), limits, EngineState::per_app, opens});
    for (const char* needed : {"clarity_tick", "clarity_draw"})
        if (!engine.exports(needed)) throw EngineError("engine " + name + " lacks " + needed + ", which an engine beside the screen needs");
    bool pointer = engine.exports("clarity_pointer"), keys = engine.exports("clarity_key");
    return std::unique_ptr<ScreenEngine>(new ScreenEngine(name, std::move(engine), pointer, keys));
}

nlohmann::json ScreenEngine::frame(double ms, const nlohmann::json& input) {
    if (ended_) throw EngineError(*ended_);
    try {
        return frame_of(ms, input);
    } catch (const std::exception& e) {
        ended_ = e.what();
        throw EngineError(*ended_);
    }
}

nlohmann::json ScreenEngine::frame_of(double ms, const nlohmann::json& input) {
    nlohmann::json told = nlohmann::json::array();
    for (const auto& given : input.is_array() ? input : nlohmann::json::array()) {
        if (!given.is_object()) continue;
        if (given.contains("pointer") && pointer_) {
            const auto& p = given["pointer"];
            engine_.call_telling("clarity_pointer", nlohmann::json::array({p.value("kind", ""), p.value("x", int64_t(0)), p.value("y", int64_t(0))}), told);
        } else if (given.contains("key") && keys_) {
            const auto& k = given["key"];
            engine_.call_telling("clarity_key", nlohmann::json::array({k.value("kind", ""), k.value("name", "")}), told);
        } else if (given.contains("hear")) {
            const auto& h = given["hear"];
            nlohmann::json values = h.contains("values") && h["values"].is_array() ? h["values"] : nlohmann::json::array();
            engine_.call_telling(h.value("name", ""), values, told);
        }
    }
    // Whole milliseconds go as whole numbers, as every renderer sends them.
    nlohmann::json tick = ms == std::trunc(ms) && std::abs(ms) < 9e15 ? nlohmann::json(static_cast<int64_t>(ms)) : nlohmann::json(ms);
    engine_.call_telling("clarity_tick", nlohmann::json::array({tick}), told);
    nlohmann::json shapes = engine_.call_telling("clarity_draw", nlohmann::json::array(), told);
    if (!shapes.is_array()) throw EngineError("engine " + name_ + " failed in clarity_draw: it gave no list of shapes");
    return {{"shapes", shapes}, {"tells", told}};
}

uint64_t ScreenInstances::open(std::unique_ptr<ScreenEngine> engine) {
    std::lock_guard lock(mutex_);
    auto held = std::make_shared<Held>();
    held->engine = std::move(engine);
    open_[++next_] = held;
    return next_;
}

std::shared_ptr<ScreenInstances::Held> ScreenInstances::held(uint64_t id) {
    std::lock_guard lock(mutex_);
    auto found = open_.find(id);
    return found == open_.end() ? nullptr : found->second;
}

nlohmann::json ScreenInstances::frame(uint64_t id, double ms, const nlohmann::json& input) {
    auto h = held(id);
    if (!h) throw EngineError(screen_engine_gone);
    std::lock_guard turn(h->turn);
    if (!h->engine) throw EngineError(screen_engine_gone);
    return h->engine->frame(ms, input);
}

nlohmann::json ScreenInstances::hears(uint64_t id) {
    auto h = held(id);
    if (!h) return nullptr;
    std::lock_guard turn(h->turn);
    if (!h->engine) return nullptr;
    return {{"pointer", h->engine->takes_pointer()}, {"keys", h->engine->takes_keys()}};
}

void ScreenInstances::close(uint64_t id) {
    std::shared_ptr<Held> h;
    {
        std::lock_guard lock(mutex_);
        auto found = open_.find(id);
        if (found == open_.end()) return;
        h = found->second;
        open_.erase(found);
    }
    // A frame that runs keeps it until it ends.
    std::lock_guard turn(h->turn);
    h->engine.reset();
}

size_t ScreenInstances::count() {
    std::lock_guard lock(mutex_);
    return open_.size();
}

std::optional<nlohmann::json> ScreenInstances::answer(const std::string& command, const nlohmann::json& args) {
    auto value = [](nlohmann::json v) { return nlohmann::json{{"value", std::move(v)}}; };
    uint64_t id = args.value("id", uint64_t(0));
    try {
        if (command == "engine_open") {
            auto made = make_ ? make_(args.value("name", "")) : nullptr;
            if (!made) return std::nullopt;
            return value(open(std::move(made)));
        }
        if (command == "engine_hears") return value(hears(id));
        if (command == "engine_frame") return value(frame(id, args.value("ms", 0.0), args.value("input", nlohmann::json::array())));
        if (command == "engine_close") {
            close(id);
            return value(nullptr);
        }
    } catch (const std::exception& e) {
        return nlohmann::json{{"error", e.what()}};
    }
    return std::nullopt;
}

}  // namespace clarity
