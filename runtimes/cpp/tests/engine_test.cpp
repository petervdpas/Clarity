// Engines hosted by the C++ runtime, against the cases every host shares
// (conformance/engines.json).

#include <fstream>
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

}  // namespace

TEST(every_engine_call_gives_what_every_host_gives) {
    auto file = conformance("engines.json");
    auto engine = Engine::load("Test", read_bytes(file["engine"]), file["functions"].get<std::vector<std::string>>());
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
            Engine::load("Test", bytes, functions);
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
    auto engine = Engine::load("Test", read_bytes(file["engine"]), {"add", "total"});
    CHECK_EQ(engine.call_as<int64_t>("add", json::array({2, 40})), int64_t(42));
    CHECK(engine.call_as<Decimal>("total", json::array({json::array({{{"price", "9.95"}, {"count", 3}}})})) == Decimal::parse("29.85"));
}
