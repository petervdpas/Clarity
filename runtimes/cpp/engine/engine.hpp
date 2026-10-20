// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports only the services of Clarity it declares (`uses
// clock, random, log`), so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next, and runs within the limits of its engine:
// a call that runs too long is stopped, and an engine never gets more memory
// than its limit. Hosted by wasm3; a generated application
// has this code only when it declares an engine.

#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/errors.hpp"

namespace clarity {

/// An engine that could not load, or that failed while it ran.
class EngineError : public Error {
public:
    explicit EngineError(const std::string& what) : Error(what) {}
};

/// What one call of an engine may take: how long it may run, and how much
/// memory the engine may grow to.
struct EngineLimits {
    uint32_t seconds;
    uint32_t megabytes;
};

/// What the services give: the system's clock, a fresh seed for every call
/// and the standard error, unless a test fixes the clock (milliseconds since
/// 1970) or the seed, or keeps the lines of the log.
struct EngineServices {
    std::optional<int64_t> clock;
    std::optional<uint64_t> seed;
    /// Takes a line of the log, as "engine <Name>: <text>"; none for the standard error.
    std::function<void(const std::string&)> log;
};

class Engine {
public:
    /// Loads `bytes` as engine `name` with the functions it is declared
    /// with; throws EngineError for a module that is no engine, imports
    /// anything but the services it uses, or lacks one of them.
    static Engine load(const std::string& name, std::vector<uint8_t> bytes, const std::vector<std::string>& functions, const std::vector<std::string>& uses,
                       EngineLimits limits);

    /// The engine with `services` in place of the system's.
    Engine with_services(EngineServices services) const {
        Engine served = *this;
        served.services_ = std::move(services);
        return served;
    }

    /// Calls `function` with `args`, a JSON array: its value, the engine's
    /// refusal (a RefusedError in its own words), or an EngineError.
    nlohmann::json call(const std::string& function, const nlohmann::json& args) const;

    /// Calls `function` and gives its value as the type code declares.
    template <class T>
    T call_as(const std::string& function, const nlohmann::json& args) const {
        auto value = call(function, args);
        try {
            return value.get<T>();
        } catch (const nlohmann::json::exception& e) {
            throw EngineError("engine " + name_ + " gave " + function + " a value of another type: " + e.what());
        }
    }

private:
    struct Loaded;
    Engine(std::string name, std::shared_ptr<Loaded> loaded) : name_(std::move(name)), loaded_(std::move(loaded)) {}
    std::string name_;
    std::shared_ptr<Loaded> loaded_;
    EngineServices services_;
};

}  // namespace clarity
