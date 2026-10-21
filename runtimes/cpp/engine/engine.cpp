#include "engine/engine.hpp"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <map>
#include <mutex>
#include <optional>
#include <random>

#include "clarity/text.hpp"
#include "clarity/ui.hpp"

extern "C" {
#include "m3_env.h"
#include "wasm3.h"
}

namespace clarity {

/// The module's bytes, which wasm3 reads on every call, and its environment.
struct Engine::Loaded {
    std::vector<uint8_t> bytes;
    IM3Environment environment = nullptr;
    std::vector<std::string> uses;
    EngineLimits limits{};
    EngineState state = EngineState::none;
    bool opens = false;
    /// The instances an engine with state keeps between calls, by window (one,
    /// at 0, for state per app), and the turn its calls take.
    std::mutex turn;
    std::map<uint64_t, std::unique_ptr<M3Runtime, void (*)(IM3Runtime)>> kept;
    /// Hears of the windows that close, for state per window; it goes first when the engine does.
    std::shared_ptr<void> listening;
    ~Loaded() {
        // The kept instances live in the environment, so they go before it.
        listening.reset();
        kept.clear();
        if (environment) m3_FreeEnvironment(environment);
    }
};

namespace {

/// The room an engine's calls run with.
constexpr uint32_t engine_stack = 1u << 20;

/// Why a call stopped when it ran past its time limit.
constexpr char late[] = "clarity: the call ran past its time limit";

using Clock = std::chrono::steady_clock;

/// When the call running on this thread must end; none while no call runs.
thread_local std::optional<Clock::time_point> deadline;
/// Yields since the clock was last read: reading it at every turn would slow loops down.
thread_local uint32_t turns = 0;

/// Asked by wasm3 at every call and every turn of a loop.
M3Result watch() {
    if (!deadline || ++turns % 4096 != 0) return m3Err_none;
    return Clock::now() >= *deadline ? late : m3Err_none;
}

/// The state of the services during the call running on this thread.
struct Serving {
    const std::string* name;
    const EngineServices* services;
    uint64_t random;
};
thread_local Serving* serving = nullptr;

/// The next number of splitmix64, the random sequence every host gives alike.
int64_t splitmix(uint64_t& state) {
    state += 0x9E3779B97F4A7C15u;
    uint64_t z = state;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9u;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBu;
    return static_cast<int64_t>(z ^ (z >> 31));
}

m3ApiRawFunction(clock_service) {
    (void)runtime;
    (void)_ctx;
    (void)_mem;
    m3ApiReturnType(int64_t);
    if (serving->services->clock) m3ApiReturn(*serving->services->clock);
    m3ApiReturn(std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count());
}

m3ApiRawFunction(random_service) {
    (void)runtime;
    (void)_ctx;
    (void)_mem;
    m3ApiReturnType(int64_t);
    m3ApiReturn(splitmix(serving->random));
}

m3ApiRawFunction(log_service) {
    (void)_ctx;
    (void)_mem;
    m3ApiGetArg(int32_t, at);
    m3ApiGetArg(int32_t, len);
    uint32_t size = 0;
    uint8_t* memory = m3_GetMemory(runtime, &size, 0);
    if (!memory || at < 0 || len < 0 || static_cast<uint64_t>(static_cast<uint32_t>(at)) + static_cast<uint32_t>(len) > size)
        m3ApiTrap(m3Err_trapOutOfBoundsMemoryAccess);
    std::string line = "engine " + *serving->name + ": " + std::string(reinterpret_cast<const char*>(memory) + at, static_cast<size_t>(len));
    if (serving->services->log) {
        serving->services->log(line);
    } else {
        std::fprintf(stderr, "%s\n", line.c_str());
    }
    m3ApiSuccess();
}

// The language as it is at the call, written into the engine's room; gives its whole length.
m3ApiRawFunction(language_service) {
    (void)_ctx;
    (void)_mem;
    m3ApiReturnType(int32_t);
    m3ApiGetArg(int32_t, at);
    m3ApiGetArg(int32_t, room);
    uint32_t size = 0;
    uint8_t* memory = m3_GetMemory(runtime, &size, 0);
    if (!memory || at < 0 || room < 0 || static_cast<uint64_t>(static_cast<uint32_t>(at)) + static_cast<uint32_t>(room) > size)
        m3ApiTrap(m3Err_trapOutOfBoundsMemoryAccess);
    std::string language = (serving->services->language ? serving->services->language() : clarity::language()).value_or("");
    std::memcpy(memory + at, language.data(), std::min(language.size(), static_cast<size_t>(room)));
    m3ApiReturn(static_cast<int32_t>(language.size()));
}

/// The services an engine may use: functions of module `clarity`, with
/// their types as wasm3 writes signatures.
struct Service {
    const char* name;
    const char* type;
    M3RawCall function;
};
constexpr Service services[] = {
    {"clock", "I()", clock_service},
    {"random", "I()", random_service},
    {"log", "v(ii)", log_service},
    {"language", "i(ii)", language_service},
};

/// Sets the deadline of a call while it runs.
struct Timed {
    explicit Timed(uint32_t seconds) {
        deadline = Clock::now() + std::chrono::seconds(seconds);
        turns = 0;
    }
    ~Timed() { deadline.reset(); }
    Timed(const Timed&) = delete;
    Timed& operator=(const Timed&) = delete;
};

/// What a WebAssembly module imports and exports, read from its sections.
struct Import {
    std::string from, name;
    /// Its function type as wasm3 writes signatures, `I()`; empty for an import that is no function.
    std::string type;
};

struct Sections {
    std::vector<Import> imports;
    std::vector<std::string> functions;
    bool memory = false;
};

/// A value type as wasm3 writes it in a signature.
char value_type(uint8_t type) {
    switch (type) {
        case 0x7f: return 'i';
        case 0x7e: return 'I';
        case 0x7d: return 'f';
        case 0x7c: return 'F';
        default: return '?';
    }
}

bool read_sections(const std::vector<uint8_t>& bytes, Sections& out) {
    size_t at = 8;
    bool broken = false;
    auto leb = [&]() -> uint32_t {
        uint32_t value = 0;
        for (int shift = 0; shift < 35; shift += 7) {
            if (at >= bytes.size()) break;
            uint8_t byte = bytes[at++];
            value |= static_cast<uint32_t>(byte & 0x7f) << shift;
            if (!(byte & 0x80)) return value;
        }
        broken = true;
        return 0;
    };
    auto name = [&]() -> std::string {
        uint32_t len = leb();
        if (broken || at + len > bytes.size()) {
            broken = true;
            return "";
        }
        std::string text(reinterpret_cast<const char*>(bytes.data()) + at, len);
        at += len;
        return text;
    };
    std::vector<std::string> types;
    auto byte = [&]() -> uint8_t { return at < bytes.size() ? bytes[at++] : 0; };
    while (at < bytes.size() && !broken) {
        uint8_t section = bytes[at++];
        uint32_t length = leb();
        size_t end = at + length;
        if (broken || end > bytes.size()) return false;
        if (section == 1) {
            uint32_t count = leb();
            for (uint32_t t = 0; t < count && !broken; ++t) {
                byte();  // 0x60, a function type
                std::string params, results;
                for (uint32_t n = leb(); n > 0; --n) params += value_type(byte());
                for (uint32_t m = leb(); m > 0; --m) results += value_type(byte());
                types.push_back((results.empty() ? "v" : results) + "(" + params + ")");
            }
        } else if (section == 2) {
            uint32_t count = leb();
            for (uint32_t i = 0; i < count && !broken; ++i) {
                auto from = name();
                auto what = name();
                Import import{from, what, ""};
                uint8_t kind = byte();
                if (kind == 0) {
                    uint32_t index = leb();
                    if (index < types.size()) import.type = types[index];
                } else if (kind == 1 || kind == 2) {
                    if (kind == 1) byte();  // the type of the table's elements
                    uint8_t flags = byte();
                    leb();
                    if (flags & 1) leb();
                } else if (kind == 3) {
                    byte();
                    byte();
                }
                out.imports.push_back(std::move(import));
            }
        } else if (section == 7) {
            uint32_t count = leb();
            for (uint32_t i = 0; i < count && !broken; ++i) {
                auto exported = name();
                uint8_t kind = at < bytes.size() ? bytes[at++] : 0xff;
                leb();
                if (kind == 0) out.functions.push_back(exported);
                if (kind == 2 && exported == "memory") out.memory = true;
            }
        }
        at = end;
    }
    return !broken;
}

}  // namespace

Engine Engine::load(const std::string& name, std::vector<uint8_t> bytes, const EngineDeclaration& declaration) {
    const auto& [functions, uses, limits, state, opens] = declaration;
    auto refused = [&](const std::string& why) { return EngineError("engine " + name + " " + why); };
    if (bytes.size() < 8 || std::memcmp(bytes.data(), "\0asm", 4) != 0) throw refused("is no WebAssembly module");
    Sections sections;
    if (!read_sections(bytes, sections)) throw refused("is no WebAssembly module");
    for (const auto& import : sections.imports) {
        if (import.from != "clarity" || std::find(uses.begin(), uses.end(), import.name) == uses.end())
            throw refused("imports " + import.from + "." + import.name + "; an engine imports only the services it uses");
        auto fits = std::any_of(std::begin(services), std::end(services), [&](const Service& s) { return import.name == s.name && import.type == s.type; });
        if (!fits) throw refused("imports clarity." + import.name + " with another type than Clarity gives it");
    }
    auto exported = [&](const std::string& f) { return std::find(sections.functions.begin(), sections.functions.end(), f) != sections.functions.end(); };
    for (const char* needed : {"clarity_alloc", "clarity_free"})
        if (!exported(needed)) throw refused(std::string("lacks ") + needed + " of the engine interface");
    if (!sections.memory) throw refused("lacks the memory of the engine interface");
    for (const auto& f : functions)
        if (!exported(f)) throw refused("has no function " + f);
    if (opens && !exported("clarity_open")) throw refused("lacks clarity_open, which its on open needs");
    auto loaded = std::make_shared<Loaded>();
    loaded->bytes = std::move(bytes);
    loaded->environment = m3_NewEnvironment();
    loaded->limits = limits;
    loaded->uses = uses;
    loaded->state = state;
    loaded->opens = opens;
    // A window that closes takes its instance along.
    if (state == EngineState::per_window) {
        loaded->listening = on_window_closed([kept = loaded.get()](uint64_t window) {
            std::lock_guard<std::mutex> turn(kept->turn);
            kept->kept.erase(window);
        });
    }
    m3_YieldHook = watch;
    // wasm3 must be able to read it too.
    IM3Module module = nullptr;
    if (M3Result parsed = m3_ParseModule(loaded->environment, &module, loaded->bytes.data(), static_cast<uint32_t>(loaded->bytes.size())))
        throw refused(std::string("is no WebAssembly module: ") + parsed);
    m3_FreeModule(module);
    return Engine(name, std::move(loaded));
}

namespace {

using Runtime = std::unique_ptr<M3Runtime, void (*)(IM3Runtime)>;

/// Why a call failed, in the words its error gives after the engine and the function.
struct Failed {
    std::string why;
};

/// Writes `input` into the instance, runs `function` and reads its answer as JSON.
nlohmann::json invoke(IM3Runtime runtime, const char* function, const std::string& input) {
    IM3Function alloc = nullptr, free_room = nullptr, run = nullptr;
    for (auto [found, name] : {std::pair{&alloc, "clarity_alloc"}, std::pair{&free_room, "clarity_free"}, std::pair{&run, function}})
        if (M3Result result = m3_FindFunction(found, runtime, name)) throw Failed{result};
    auto len = static_cast<int32_t>(input.size());
    int32_t at = 0;
    if (M3Result result = m3_CallV(alloc, len)) throw Failed{result};
    m3_GetResultsV(alloc, &at);
    uint32_t memory_size = 0;
    uint8_t* memory = m3_GetMemory(runtime, &memory_size, 0);
    if (!memory || static_cast<uint64_t>(static_cast<uint32_t>(at)) + input.size() > memory_size) throw Failed{"the arguments do not fit its memory"};
    std::memcpy(memory + static_cast<uint32_t>(at), input.data(), input.size());
    int64_t packed = 0;
    if (M3Result result = m3_CallV(run, at, len)) throw Failed{result};
    m3_GetResultsV(run, &packed);
    // The memory may have grown during the call.
    memory = m3_GetMemory(runtime, &memory_size, 0);
    auto place = static_cast<uint32_t>(static_cast<uint64_t>(packed) >> 32), size = static_cast<uint32_t>(packed);
    if (!memory || static_cast<uint64_t>(place) + size > memory_size) throw Failed{"its answer is outside its memory"};
    std::string output(reinterpret_cast<const char*>(memory) + place, size);
    m3_CallV(free_room, static_cast<int32_t>(place), static_cast<int32_t>(size));
    nlohmann::json answer = nlohmann::json::parse(output, nullptr, false);
    if (answer.is_discarded()) throw Failed{"its answer is no JSON"};
    return answer;
}

}  // namespace

/// A new instance of the module with its services, opened when the engine has `on open`.
static Runtime instance(IM3Environment environment, const std::vector<uint8_t>& bytes, const std::vector<std::string>& uses, EngineLimits limits, bool opens) {
    Runtime runtime(m3_NewRuntime(environment, engine_stack, nullptr), m3_FreeRuntime);
    if (!runtime) throw Failed{"no room for it"};
    IM3Module module = nullptr;
    if (M3Result result = m3_ParseModule(environment, &module, bytes.data(), static_cast<uint32_t>(bytes.size()))) throw Failed{result};
    if (M3Result result = m3_LoadModule(runtime.get(), module)) {
        m3_FreeModule(module);
        throw Failed{result};
    }
    // The memory never grows past the limit: memory.grow answers -1 beyond it.
    runtime->memory.maxPages = std::min(runtime->memory.maxPages, limits.megabytes * 16);
    // The services it uses; one it does not import stays unlinked.
    for (const auto& service : services) {
        if (std::find(uses.begin(), uses.end(), service.name) == uses.end()) continue;
        M3Result linked = m3_LinkRawFunction(module, "clarity", service.name, service.type, service.function);
        if (linked && linked != m3Err_functionLookupFailed) throw Failed{linked};
    }
    if (opens) {
        nlohmann::json opened;
        try {
            opened = invoke(runtime.get(), "clarity_open", "[]");
        } catch (const Failed& f) {
            if (f.why == late) throw;
            throw Failed{"its on open gave no answer"};
        }
        if (opened.contains("error") && opened["error"].is_string()) throw Failed{"its on open refused: " + opened["error"].get<std::string>()};
        if (!opened.contains("value")) throw Failed{"its on open gave no answer"};
    }
    return runtime;
}

void Engine::close_window(uint64_t window) const {
    if (loaded_->state != EngineState::per_window) return;
    std::lock_guard<std::mutex> turn(loaded_->turn);
    loaded_->kept.erase(window);
}

nlohmann::json Engine::call_in(uint64_t window, const std::string& function, const nlohmann::json& args) const {
    auto seconds = loaded_->limits.seconds;
    auto failed = [&](const std::string& why) {
        if (why == late) return EngineError("engine " + name_ + " failed in " + function + ": it ran longer than " + std::to_string(seconds) + " second" + (seconds == 1 ? "" : "s"));
        return EngineError("engine " + name_ + " failed in " + function + ": " + why);
    };
    std::random_device fresh;
    uint64_t seed = services_.seed ? *services_.seed : (static_cast<uint64_t>(fresh()) << 32) ^ fresh();
    Serving state{&name_, &services_, seed};
    serving = &state;
    struct Unserve {
        ~Unserve() { serving = nullptr; }
    } unserve;
    Timed timed(seconds);
    // An engine with state keeps its instance, unless a call fails or runs
    // past its limit: then its memory may be half changed, so the next call
    // starts a new one. Its calls take turns.
    bool keeps = loaded_->state != EngineState::none;
    uint64_t key = loaded_->state == EngineState::per_window ? window : 0;
    std::unique_lock<std::mutex> turn(loaded_->turn, std::defer_lock);
    if (keeps) turn.lock();
    Runtime runtime(nullptr, m3_FreeRuntime);
    if (auto found = loaded_->kept.find(key); keeps && found != loaded_->kept.end()) {
        runtime = std::move(found->second);
        loaded_->kept.erase(found);
    }
    nlohmann::json answer;
    try {
        if (!runtime) runtime = instance(loaded_->environment, loaded_->bytes, loaded_->uses, loaded_->limits, loaded_->opens);
        answer = invoke(runtime.get(), function.c_str(), args.dump());
    } catch (const Failed& f) {
        throw failed(f.why);
    }
    // The engine answered, even with a refusal, so its memory is whole.
    if (keeps) loaded_->kept.emplace(key, std::move(runtime));
    if (answer.contains("error") && answer["error"].is_string()) refuse(Text::plain(answer["error"].get<std::string>()));
    if (!answer.contains("value")) throw failed("its answer has no value");
    return answer["value"];
}

}  // namespace clarity
