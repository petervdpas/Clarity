// Engines beside the screen (DESIGN 5.12, conformance/screen-engines.json)
// as Clarity.UI runs them in the program's own process: one instance per
// canvas, which keeps its memory from frame to frame. Each frame hands over
// the input since the last (clarity_pointer, clarity_key, a hear by its
// function's name), then calls clarity_tick with the milliseconds since the
// last frame and clarity_draw for the canvas's shapes; what the answers
// tell comes back with the frame. A frame that fails ends the engine.

#pragma once

#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "engine/engine.hpp"

namespace clarity {

/// An engine beside the screen for one canvas.
class ScreenEngine {
public:
    /// Loads `bytes` as engine `name` for one canvas; throws EngineError for a
    /// module that is no engine, imports more than `uses` or lacks its frame
    /// loop. With `opens`, clarity_open runs before the first frame.
    static std::unique_ptr<ScreenEngine> load(const std::string& name, std::vector<uint8_t> bytes, std::vector<std::string> uses, bool opens, EngineLimits limits);

    /// The engine with `services` in place of the system's.
    void serve(EngineServices services) { engine_ = engine_.with_services(std::move(services)); }

    /// Whether it takes the pointer and keys: it exports clarity_pointer and clarity_key.
    bool takes_pointer() const { return pointer_; }
    bool takes_keys() const { return keys_; }

    /// One frame: `input` first, as Clarity.UI sends it ([{"pointer":{kind,x,y}} |
    /// {"key":{kind,name}} | {"hear":{name,values}}]), then `ms`. Gives
    /// {"shapes": [...], "tells": [...]}; throws EngineError, and once a frame
    /// failed every frame fails alike.
    nlohmann::json frame(double ms, const nlohmann::json& input);

private:
    ScreenEngine(std::string name, Engine engine, bool pointer, bool keys) : name_(std::move(name)), engine_(std::move(engine)), pointer_(pointer), keys_(keys) {}
    nlohmann::json frame_of(double ms, const nlohmann::json& input);
    std::string name_;
    Engine engine_;
    bool pointer_;
    bool keys_;
    std::optional<std::string> ended_;
};

/// What a frame of an instance that is gone answers.
inline constexpr const char* screen_engine_gone = "The engine ended.";

/// The engines beside the screen a program has open, each by its number: the
/// screen holds only the number. A frame runs with only its own instance
/// held, so the canvases of several windows draw at once.
class ScreenInstances {
public:
    /// Makes a new engine by its name: none when the program has no engine of
    /// that name; throws EngineError when it does not start.
    using Maker = std::function<std::unique_ptr<ScreenEngine>(const std::string& name)>;
    explicit ScreenInstances(Maker make) : make_(std::move(make)) {}

    /// Keeps `engine` open: its number.
    uint64_t open(std::unique_ptr<ScreenEngine> engine);
    /// One frame of instance `id`; one that is gone says so.
    nlohmann::json frame(uint64_t id, double ms, const nlohmann::json& input);
    /// {"pointer", "keys"} of instance `id`; none for one that is gone.
    nlohmann::json hears(uint64_t id);
    void close(uint64_t id);
    size_t count();

    /// Answers Clarity.UI's commands of engines (clarity_ui.h: engine_open,
    /// engine_hears, engine_frame, engine_close) as {"value"} or {"error"};
    /// none for another command or a name the program has no engine of.
    std::optional<nlohmann::json> answer(const std::string& command, const nlohmann::json& args);

private:
    struct Held {
        std::mutex turn;
        std::unique_ptr<ScreenEngine> engine;
    };
    std::shared_ptr<Held> held(uint64_t id);
    Maker make_;
    std::mutex mutex_;
    std::map<uint64_t, std::shared_ptr<Held>> open_;
    uint64_t next_ = 0;
};

}  // namespace clarity
