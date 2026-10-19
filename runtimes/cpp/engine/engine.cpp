#include "engine/engine.hpp"

#include <cstring>

#include "clarity/text.hpp"

extern "C" {
#include "wasm3.h"
}

namespace clarity {

/// The module's bytes, which wasm3 reads on every call, and its environment.
struct Engine::Loaded {
    std::vector<uint8_t> bytes;
    IM3Environment environment = nullptr;
    ~Loaded() {
        if (environment) m3_FreeEnvironment(environment);
    }
};

namespace {

/// The room an engine's calls run with.
constexpr uint32_t engine_stack = 1u << 20;

/// What a WebAssembly module imports and exports, read from its sections.
struct Sections {
    std::string first_import;
    std::vector<std::string> functions;
    bool memory = false;
};

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
    while (at < bytes.size() && !broken) {
        uint8_t section = bytes[at++];
        uint32_t length = leb();
        size_t end = at + length;
        if (broken || end > bytes.size()) return false;
        if (section == 2 && out.first_import.empty() && leb() > 0) {
            auto from = name();
            auto what = name();
            out.first_import = from + "." + what;
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

Engine Engine::load(const std::string& name, std::vector<uint8_t> bytes, const std::vector<std::string>& functions) {
    auto refused = [&](const std::string& why) { return EngineError("engine " + name + " " + why); };
    if (bytes.size() < 8 || std::memcmp(bytes.data(), "\0asm", 4) != 0) throw refused("is no WebAssembly module");
    Sections sections;
    if (!read_sections(bytes, sections)) throw refused("is no WebAssembly module");
    if (!sections.first_import.empty()) throw refused("imports " + sections.first_import + "; an engine imports nothing");
    auto exported = [&](const std::string& f) { return std::find(sections.functions.begin(), sections.functions.end(), f) != sections.functions.end(); };
    for (const char* needed : {"clarity_alloc", "clarity_free"})
        if (!exported(needed)) throw refused(std::string("lacks ") + needed + " of the engine interface");
    if (!sections.memory) throw refused("lacks the memory of the engine interface");
    for (const auto& f : functions)
        if (!exported(f)) throw refused("has no function " + f);
    auto loaded = std::make_shared<Loaded>();
    loaded->bytes = std::move(bytes);
    loaded->environment = m3_NewEnvironment();
    // wasm3 must be able to read it too.
    IM3Module module = nullptr;
    if (M3Result parsed = m3_ParseModule(loaded->environment, &module, loaded->bytes.data(), static_cast<uint32_t>(loaded->bytes.size())))
        throw refused(std::string("is no WebAssembly module: ") + parsed);
    m3_FreeModule(module);
    return Engine(name, std::move(loaded));
}

nlohmann::json Engine::call(const std::string& function, const nlohmann::json& args) const {
    auto failed = [&](const std::string& why) { return EngineError("engine " + name_ + " failed in " + function + ": " + why); };
    std::string input = args.dump();
    std::unique_ptr<M3Runtime, void (*)(IM3Runtime)> runtime(m3_NewRuntime(loaded_->environment, engine_stack, nullptr), m3_FreeRuntime);
    if (!runtime) throw failed("no room for it");
    IM3Module module = nullptr;
    if (M3Result result = m3_ParseModule(loaded_->environment, &module, loaded_->bytes.data(), static_cast<uint32_t>(loaded_->bytes.size())))
        throw failed(result);
    if (M3Result result = m3_LoadModule(runtime.get(), module)) {
        m3_FreeModule(module);
        throw failed(result);
    }
    IM3Function alloc = nullptr, free_room = nullptr, run = nullptr;
    for (auto [found, name] : {std::pair{&alloc, "clarity_alloc"}, std::pair{&free_room, "clarity_free"}, std::pair{&run, function.c_str()}})
        if (M3Result result = m3_FindFunction(found, runtime.get(), name)) throw failed(result);
    auto len = static_cast<int32_t>(input.size());
    int32_t at = 0;
    if (M3Result result = m3_CallV(alloc, len)) throw failed(result);
    m3_GetResultsV(alloc, &at);
    uint32_t memory_size = 0;
    uint8_t* memory = m3_GetMemory(runtime.get(), &memory_size, 0);
    if (!memory || static_cast<uint64_t>(static_cast<uint32_t>(at)) + input.size() > memory_size) throw failed("the arguments do not fit its memory");
    std::memcpy(memory + static_cast<uint32_t>(at), input.data(), input.size());
    int64_t packed = 0;
    if (M3Result result = m3_CallV(run, at, len)) throw failed(result);
    m3_GetResultsV(run, &packed);
    // The memory may have grown during the call.
    memory = m3_GetMemory(runtime.get(), &memory_size, 0);
    auto place = static_cast<uint32_t>(static_cast<uint64_t>(packed) >> 32), size = static_cast<uint32_t>(packed);
    if (!memory || static_cast<uint64_t>(place) + size > memory_size) throw failed("its answer is outside its memory");
    std::string output(reinterpret_cast<const char*>(memory) + place, size);
    m3_CallV(free_room, static_cast<int32_t>(place), static_cast<int32_t>(size));
    nlohmann::json answer = nlohmann::json::parse(output, nullptr, false);
    if (answer.is_discarded()) throw failed("its answer is no JSON");
    if (answer.contains("error") && answer["error"].is_string()) refuse(Text::plain(answer["error"].get<std::string>()));
    if (!answer.contains("value")) throw failed("its answer has no value");
    return answer["value"];
}

}  // namespace clarity
