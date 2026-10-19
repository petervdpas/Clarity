// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports nothing, so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next. Hosted by wasm3; a generated application
// has this code only when it declares an engine.

#pragma once

#include <cstdint>
#include <memory>
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

class Engine {
public:
    /// Loads `bytes` as engine `name` with the functions it is declared
    /// with; throws EngineError for a module that is no engine, imports
    /// anything, or lacks one of them.
    static Engine load(const std::string& name, std::vector<uint8_t> bytes, const std::vector<std::string>& functions);

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
};

}  // namespace clarity
