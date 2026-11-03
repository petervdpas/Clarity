// Engines beside the screen on C++, as Clarity.UI runs them in the
// program's process: one instance per canvas, against the cases every host
// shares (conformance/screen-engines.json).

#include <fstream>
#include <iterator>

#include "check.hpp"
#include "conformance.hpp"
#include "engine/screen.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::vector<uint8_t> engine_bytes(const std::string& file) {
    std::ifstream in(std::string(CLARITY_CONFORMANCE) + "/engines/" + file, std::ios::binary);
    return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

std::vector<std::string> uses() { return conformance("screen-engines.json")["uses"].get<std::vector<std::string>>(); }

std::unique_ptr<ScreenEngine> screen(bool opens, uint32_t seconds = 5) { return ScreenEngine::load("Screen", engine_bytes("screen.wasm"), uses(), opens, {seconds, 256}); }

/// The input as the conformance file writes it, as Clarity.UI sends it.
json input_of(const json& given) {
    json out = json::array();
    for (const auto& i : given.is_array() ? given : json::array()) {
        if (i.contains("pointer")) out.push_back({{"pointer", {{"kind", i["pointer"]}, {"x", i["x"]}, {"y", i["y"]}}}});
        else if (i.contains("hear")) out.push_back({{"hear", {{"name", i["hear"]}, {"values", i.value("values", json::array())}}}});
        else out.push_back({{"key", {{"kind", i["key"]}, {"name", i["name"]}}}});
    }
    return out;
}

}  // namespace

TEST(each_frame_gives_the_canvas_its_shapes) {
    for (const auto& run : conformance("screen-engines.json")["frames"]) {
        auto engine = screen(run.value("opens", false));
        const auto& ticks = run["ticks"];
        for (size_t i = 0; i < ticks.size(); ++i) CHECK_EQ(engine->frame(ticks[i].get<double>(), json::array())["shapes"], run["draws"][i]);
    }
}

TEST(a_frame_that_fails_or_runs_too_long_ends_the_engine) {
    for (const auto& fails : conformance("screen-engines.json")["fails"]) {
        auto engine = screen(true, fails["seconds"].get<uint32_t>());
        for (const auto& tick : fails["ticks"]) engine->frame(tick.get<double>(), json::array());
        std::string why;
        try {
            engine->frame(fails["tick"].get<double>(), input_of(fails.value("input", json::array())));
        } catch (const EngineError& e) {
            why = e.what();
        }
        CHECK_EQ(why, fails["fails"].get<std::string>());
        // Ended, it draws no more.
        std::string again;
        try {
            engine->frame(16, json::array());
        } catch (const EngineError& e) {
            again = e.what();
        }
        CHECK_EQ(again, fails["fails"].get<std::string>());
    }
}

TEST(pointer_keys_and_hears_reach_the_engine_before_its_tick_and_it_tells) {
    const auto& cases = conformance("screen-engines.json");
    for (const char* group : {"input", "tells"}) {
        for (const auto& run : cases[group]) {
            auto engine = screen(run.value("opens", false));
            CHECK(engine->takes_pointer() && engine->takes_keys());
            for (const auto& frame : run["frames"]) {
                json drawn = engine->frame(frame["tick"].get<double>(), input_of(frame["input"]));
                CHECK_EQ(drawn["shapes"], frame["draws"]);
                if (frame.contains("tells")) CHECK_EQ(drawn["tells"], frame["tells"]);
            }
        }
    }
}

TEST(an_engine_beside_the_screen_written_in_clarity_runs) {
    const auto& written = conformance("screen-engines.json")["written"];
    for (const auto& file : written["engines"]) {
        auto engine = ScreenEngine::load(written["name"], engine_bytes(file), {}, false, {5, 256});
        for (const auto& frame : written["frames"]) {
            json drawn = engine->frame(frame["tick"].get<double>(), input_of(frame["input"]));
            CHECK_EQ(drawn["tells"], frame["tells"]);
            CHECK_EQ(drawn["shapes"], frame["draws"]);
        }
    }
}

TEST(an_engine_beside_the_screen_logs_and_exports_its_frame_loop) {
    for (const auto& run : conformance("screen-engines.json")["logs"]) {
        std::vector<std::string> lines;
        auto engine = screen(run.value("opens", false));
        engine->serve({std::nullopt, std::nullopt, [&](const std::string& line) { lines.push_back(line); }, nullptr});
        engine->frame(0, input_of(run["input"]));
        CHECK_EQ(json(lines), run["lines"]);
    }
    std::string why;
    try {
        ScreenEngine::load("Test", engine_bytes("test.wasm"), {"clock", "random", "log", "language"}, false, {5, 256});
    } catch (const EngineError& e) {
        why = e.what();
    }
    CHECK_EQ(why, std::string("engine Test lacks clarity_tick, which an engine beside the screen needs"));
}

// Clarity.UI's commands: an instance by number, its frames, its end; a
// name the program has no engine of is left to the library.
TEST(instances_go_by_number_and_a_gone_one_says_so) {
    ScreenInstances open([](const std::string& name) -> std::unique_ptr<ScreenEngine> {
        if (name != "Screen") return nullptr;
        return screen(true);
    });
    CHECK(!open.answer("engine_open", {{"name", "Other"}}).has_value());
    json opened = *open.answer("engine_open", {{"name", "Screen"}});
    uint64_t id = opened["value"].get<uint64_t>();
    CHECK_EQ(*open.answer("engine_hears", {{"id", id}}), json::parse(R"({"value":{"pointer":true,"keys":true}})"));
    CHECK_EQ(open.count(), size_t(1));
    json drawn = *open.answer("engine_frame", {{"id", id}, {"ms", 0}, {"input", json::array()}});
    CHECK(drawn["value"]["shapes"].is_array());
    CHECK_EQ(*open.answer("engine_close", {{"id", id}}), json::parse(R"({"value":null})"));
    json gone = *open.answer("engine_frame", {{"id", id}, {"ms", 16}, {"input", json::array()}});
    CHECK_EQ(gone, json::parse(R"({"error":"The engine ended."})"));
    CHECK_EQ(open.count(), size_t(0));
}
