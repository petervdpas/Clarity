// Engines hosted by the C++ runtime, against the cases every host shares
// (conformance/engines.json).

#include <array>
#include <atomic>
#include <fstream>
#include <memory>
#include <iterator>

#include "check.hpp"
#include "conformance.hpp"
#include "engine/engine.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::vector<uint8_t> read_bytes(const std::string& path) {
    std::ifstream in(std::string(CLARITY_CONFORMANCE) + "/" + path, std::ios::binary);
    return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

std::vector<uint8_t> hex(const std::string& text) {
    std::vector<uint8_t> out;
    for (size_t i = 0; i + 1 < text.size(); i += 2) out.push_back(static_cast<uint8_t>(std::stoi(text.substr(i, 2), nullptr, 16)));
    return out;
}

/// The limits every case runs under.
EngineLimits limits(const json& file) {
    return {file["limits"]["seconds"].get<uint32_t>(), file["limits"]["megabytes"].get<uint32_t>()};
}

/// The services the test engine uses.
std::vector<std::string> uses(const json& file) {
    return file["uses"].get<std::vector<std::string>>();
}

}  // namespace

TEST(every_engine_call_gives_what_every_host_gives) {
    auto file = conformance("engines.json");
    auto engine = Engine::load("Test", read_bytes(file["engine"]), {file["functions"].get<std::vector<std::string>>(), uses(file), limits(file)});
    for (const auto& call : file["calls"]) {
        json got;
        try {
            got = {{"value", engine.call(call["function"], call["args"])}};
        } catch (const RefusedError& e) {
            got = {{"error", e.what()}};
        }
        CHECK_EQ(got, call["gives"]);
    }
}

TEST(a_module_that_is_no_engine_is_refused_when_it_loads) {
    auto file = conformance("engines.json");
    for (const auto& refused : file["refused"]) {
        auto bytes = refused.contains("bytes") ? hex(refused["bytes"]) : read_bytes(file["engine"]);
        auto functions = refused.value("functions", std::vector<std::string>());
        try {
            Engine::load("Test", bytes, {functions, refused.value("uses", uses(file)), limits(file), EngineState::none, refused.value("open", false)});
            CHECK(false);
        } catch (const EngineError& e) {
            std::string message = e.what();
            CHECK(message.find(refused["message"].get<std::string>()) != std::string::npos);
            CHECK(message.rfind("engine Test ", 0) == 0);
        }
    }
}

TEST(a_value_comes_back_as_the_type_code_declares) {
    auto file = conformance("engines.json");
    auto engine = Engine::load("Test", read_bytes(file["engine"]), {{"add", "total"}, uses(file), limits(file)});
    CHECK_EQ(engine.call_as<int64_t>("add", json::array({2, 40})), int64_t(42));
    CHECK(engine.call_as<Decimal>("total", json::array({json::array({{{"price", "9.95"}, {"count", 3}}})})) == Decimal::parse("29.85"));
}

TEST(a_call_keeps_to_the_limits_of_its_engine) {
    auto file = conformance("engines.json");
    auto engine = Engine::load("Test", read_bytes(file["engine"]), {file["functions"].get<std::vector<std::string>>(), uses(file), limits(file)});
    for (const auto& call : file["limited"]) {
        if (call.contains("fails")) {
            try {
                engine.call(call["function"], call["args"]);
                CHECK(false);
            } catch (const EngineError& e) {
                CHECK(std::string(e.what()).rfind(call["fails"].get<std::string>(), 0) == 0);
            }
        } else {
            CHECK_EQ(json({{"value", engine.call(call["function"], call["args"])}}), call["gives"]);
        }
    }
}

TEST(an_engine_uses_the_services_it_declares) {
    auto file = conformance("engines.json");
    std::vector<std::string> lines;
    EngineServices services;
    services.clock = file["services"]["clock"].get<int64_t>();
    services.seed = file["services"]["seed"].get<uint64_t>();
    services.log = [&](const std::string& line) { lines.push_back(line); };
    std::string language = file["services"]["language"].get<std::string>();
    services.language = [language] { return std::optional(language); };
    auto engine = Engine::load("Test", read_bytes(file["engine"]), {file["functions"].get<std::vector<std::string>>(), uses(file), limits(file)}).with_services(services);
    for (const auto& call : file["served"]) {
        lines.clear();
        CHECK_EQ(json({{"value", engine.call(call["function"], call["args"])}}), call["gives"]);
        CHECK_EQ(json(lines), call.value("log", json::array()));
    }
}

TEST(an_engine_with_state_keeps_its_instance_from_call_to_call) {
    auto file = conformance("engines.json");
    const auto& kept = file["kept"];
    CHECK_EQ(kept["state"].get<std::string>(), std::string("app"));
    auto engine = Engine::load("Test", read_bytes(file["engine"]),
                               {file["functions"].get<std::vector<std::string>>(), uses(file), limits(file), EngineState::per_app, kept["open"].get<bool>()});
    for (const auto& call : kept["calls"]) {
        if (call.contains("fails")) {
            try {
                engine.call(call["function"], call["args"]);
                CHECK(false);
            } catch (const EngineError& e) {
                CHECK(std::string(e.what()).rfind(call["fails"].get<std::string>(), 0) == 0);
            }
            continue;
        }
        json got;
        try {
            got = {{"value", engine.call(call["function"], call["args"])}};
        } catch (const RefusedError& e) {
            got = {{"error", e.what()}};
        }
        CHECK_EQ(got, call["gives"]);
    }
}

TEST(an_engine_with_state_per_window_keeps_an_instance_for_each_window) {
    auto file = conformance("engines.json");
    const auto& windows = file["windows"];
    CHECK_EQ(windows["state"].get<std::string>(), std::string("window"));
    auto engine = Engine::load("Test", read_bytes(file["engine"]),
                               {file["functions"].get<std::vector<std::string>>(), uses(file), limits(file), EngineState::per_window, windows["open"].get<bool>()});
    for (const auto& step : windows["steps"]) {
        if (step.contains("close")) {
            engine.close_window(step["close"].get<uint64_t>());
            continue;
        }
        CHECK_EQ(json({{"value", engine.call_in(step["window"].get<uint64_t>(), step["function"], json::array())}}), step["gives"]);
    }
}

// Progress of a long call and Stop, against the cases every host shares
// (conformance/progress.json).

TEST(a_long_engine_call_reports_how_far_it_is) {
    json file = conformance("progress.json");
    auto load = [&](const json& used) {
        return Engine::load(file["name"], read_bytes(file["engine"]), {file["functions"].get<std::vector<std::string>>(), used.get<std::vector<std::string>>(), limits(file)});
    };
    auto engine = load(file["uses"]);
    for (const auto& c : file["reported"]) {
        std::vector<std::array<int64_t, 2>> reports;
        EngineWatch watch{[&](int64_t done, int64_t total) { reports.push_back({done, total}); }, nullptr};
        CHECK(engine.call_watched(0, c["function"], c["args"], watch) == c["gives"]);
        auto expected = c["progress"].get<std::vector<std::array<int64_t, 2>>>();
        CHECK(reports == expected);
    }
    // Without `uses progress`, an engine that reports is refused.
    try {
        load(file["refused"]["uses"]);
        CHECK(false);
    } catch (const EngineError& e) {
        CHECK_EQ(std::string(e.what()), file["refused"]["message"].get<std::string>());
    }
}

TEST(stop_ends_a_long_engine_call) {
    json file = conformance("progress.json");
    auto engine = Engine::load(file["name"], read_bytes(file["engine"]), {file["functions"].get<std::vector<std::string>>(), file["uses"].get<std::vector<std::string>>(), limits(file)});
    for (const auto& c : file["stopped"]) {
        int64_t stop_at = c["stopAt"];
        auto stop = std::make_shared<std::atomic<bool>>(stop_at == 0);
        std::vector<std::array<int64_t, 2>> reports;
        EngineWatch watch{[&](int64_t done, int64_t total) {
                              reports.push_back({done, total});
                              if (done == stop_at) *stop = true;
                          },
                          stop};
        try {
            engine.call_watched(0, c["function"], c["args"], watch);
            CHECK(false);
        } catch (const EngineError& e) {
            CHECK_EQ(std::string(e.what()), c["fails"].get<std::string>());
        }
        auto first = c["firstReports"].get<std::vector<std::array<int64_t, 2>>>();
        CHECK(reports.size() >= first.size());
        CHECK(std::equal(first.begin(), first.end(), reports.begin()));
    }
}

// The shell watches a handler, not each engine call: a call made while a
// watch is current, as generated code makes it, reports to it and stops with it.
TEST(a_call_inside_a_watched_handler_is_watched) {
    json file = conformance("progress.json");
    auto engine = Engine::load(file["name"], read_bytes(file["engine"]), {file["functions"].get<std::vector<std::string>>(), file["uses"].get<std::vector<std::string>>(), limits(file)});
    std::vector<std::array<int64_t, 2>> reports;
    auto stop = std::make_shared<std::atomic<bool>>(false);
    EngineWatch watch{[&](int64_t done, int64_t total) { reports.push_back({done, total}); }, stop};
    {
        Watching watching(watch);
        CHECK(engine.call("work", json::array({2})) == json(2));
    }
    CHECK(reports.size() == 2);
    // Outside it, nothing is watched.
    engine.call("work", json::array({1}));
    CHECK(reports.size() == 2);
    *stop = true;
    Watching watching(watch);
    try {
        engine.call("work", json::array({5}));
        CHECK(false);
    } catch (const EngineError& e) {
        CHECK_EQ(std::string(e.what()), std::string("engine Long failed in work: it was stopped"));
    }
}
