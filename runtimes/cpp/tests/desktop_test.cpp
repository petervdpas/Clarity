// The desktop program on Clarity.UI (DESIGN 7.9): the library's commands
// answered by the service, the desktop's own (started, facts, watches),
// and a program driven on a screen without a window, with a custom part.

#include <algorithm>
#include <cstdlib>
#include <filesystem>
#include <functional>
#include <map>
#include <thread>

#include "check.hpp"
#include "clarity/settings.hpp"
#include "clarity/watch.hpp"
#include "engine/engine.hpp"
#include "shell/desktop.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

/// What a long handler runs at its third day, as the person's Stop does while it runs.
std::function<void()> on_third_day;

class Shop final : public shell::Windows {
public:
    WindowState open(Db&, const std::string& window, const json& with) override {
        if (window != "Main") throw InvalidError("there is no window " + window);
        return opened(window, {{"name", "Ann"}, {"speed", 3}, {"reached", 0}}, with);
    }
    std::optional<std::string> handler(const std::string&, const std::string&) override { return std::nullopt; }
    Reply redraw(Db&, Reply reply) override { return reply; }
    Reply dispatch(Db&, const WindowState& state, const std::string& handler) override {
        Ui ui(state);
        json data = state.data;
        if (handler == "save") data["name"] = "Saved";
        // The built-in quit and keys, run by code.
        if (handler == "quitIt") ui.quit();
        if (handler == "keysNow") ui.show_keys();
        if (handler == "plan") {
            const EngineWatch* watch = current_watch();
            for (int64_t day = 1;; ++day) {
                watch->progress(day, 365);
                if (day == 3 && on_third_day) on_third_day();
                if (watch->stop->load()) throw EngineError("engine Planner failed in plan: it was stopped");
            }
        }
        return ui.reply(state.window, data);
    }
    // A custom part's tell runs its handler with the tell's values.
    Reply answer(Db&, const WindowState& state, const std::string& handler, const json& values) override {
        if (handler != "gaugeReached") throw InvalidError("Main has no handler " + handler + " to answer");
        json data = state.data;
        data["reached"] = values.at(0);
        return Ui(state).reply(state.window, data);
    }
    std::vector<RefOption> options(Db&, const std::string&) override { return {}; }
    std::optional<Bytes> picture(Db&, const std::string&, int64_t id, const std::string&) override {
        if (id == 1) return Bytes{1, 2, 3};
        return std::nullopt;
    }
};

const char* const main_markup =
    R"(<Window name="Main"><Textbox name="nameBox" at="1,1" span="4" bind="name" /><Button name="save" at="1,2" span="3" caption="Save" onclick="save" />)"
    R"(<Custom name="gauge" at="1,3" span="4" rows="3" part="CppGauge" bind="speed" tells='[{"tell":"reached","handler":"gaugeReached"}]' /></Window>)";

const unsigned char* bytes_of(const char* text) { return reinterpret_cast<const unsigned char*>(text); }

const shell::UiFile files[] = {
    {"window", "Main", bytes_of(main_markup), std::char_traits<char>::length(main_markup)},
    {nullptr, nullptr, nullptr, 0},
};

shell::Options options_of(Db& db, Shop& shop) {
    shell::Options options{"shop", "Shop", "Main", R"({"project":{}})", &db, &shop};
    options.ui_files = files;
    return options;
}

json called(shell::DesktopProgram& program, const std::string& command, const json& args = json::object()) {
    auto answered = program.call(command, args.dump());
    return answered ? json::parse(*answered) : json("none");
}

/// A custom part written in C++: it shows its value, and a click sets it to 5 and tells `reached`.
class Gauge final : public ui::CustomPart {
public:
    static inline std::vector<std::string> log;
    void set(std::string_view name, std::string_view value, ui::Cx&) override { log.push_back("set " + std::string(name) + " " + std::string(value)); }
    void hear(std::string_view, std::string_view, ui::Cx&) override {}
    void draw(ui::Canvas& canvas, double x, double y, double width, double height) override { canvas.fill(x, y, width, height, 0xFFFF0000); }
    void click(ui::Cx& cx) override {
        cx.change("5");
        cx.tell("reached", "[5]");
    }
};

}  // namespace

TEST(the_desktop_answers_the_librarys_commands) {
    Db db = Db::memory();
    Shop shop;
    shell::Options options = options_of(db, shop);
    shell::DesktopProgram program(options);
    // How it starts: the main window, nothing unconnected; the model stays with the files.
    CHECK_EQ(called(program, "started"), json::parse(R"({"value":{"main":"Main","unconnected":null}})"));
    // Without facts, engines beside the screen or a command it knows: the library's default.
    CHECK_EQ(called(program, "facts"), json("none"));
    CHECK_EQ(called(program, "engine_open", {{"name", "Board"}}), json("none"));
    CHECK_EQ(called(program, "nonsense"), json("none"));
    CHECK_EQ(called(program, "watches"), json::parse(R"({"value":false})"));
    // A record's file goes as base64.
    CHECK_EQ(called(program, "field_bytes", {{"entity", "Card"}, {"id", 1}, {"field", "photo"}}), json::parse(R"({"value":"AQID"})"));
    CHECK_EQ(called(program, "field_bytes", {{"entity", "Card"}, {"id", 2}, {"field", "photo"}}), json::parse(R"({"value":null})"));
    // The service's commands go through as they are.
    json opened = called(program, "open_window", {{"window", "Main"}, {"with", json::object()}});
    CHECK_EQ(opened["value"]["state"]["data"]["name"], json("Ann"));
    // Arguments that do not read are an error, not a fault.
    auto bad = program.call("open_window", "{not json");
    REQUIRE(bad.has_value());
    CHECK(json::parse(*bad).contains("error"));

    options.facts = R"({"windows":{"Main":{"name":"text"}},"records":{},"fields":{},"enums":{}})";
    shell::DesktopProgram knowing(options);
    CHECK_EQ(called(knowing, "facts")["value"]["windows"]["Main"]["name"], json("text"));
    // Engines beside the screen: the program's own answers, by their commands.
    options.screen_engines = [](const std::string& command, const json& args) -> std::optional<json> {
        if (command == "engine_open" && args.value("name", "") == "Board") return json{{"value", 7}};
        return std::nullopt;
    };
    shell::DesktopProgram engines(options);
    CHECK_EQ(called(engines, "engine_open", {{"name", "Board"}}), json::parse(R"({"value":7})"));
    CHECK_EQ(called(engines, "engine_open", {{"name", "Other"}}), json("none"));
}

// A long call (DESIGN 5.12): the handler runs on a worker thread between
// start_watch and end_watch; the screen's thread asks how far it is and
// stops it while the handler holds the program.
TEST(a_long_call_is_watched_and_stopped_from_the_screens_thread) {
    Db db = Db::memory();
    Shop shop;
    shell::Options options = options_of(db, shop);
    options.watches = true;
    shell::DesktopProgram program(options);
    CHECK_EQ(called(program, "watches"), json::parse(R"({"value":true})"));
    CHECK_EQ(called(program, "watch_progress", {{"id", 7}}), json::parse(R"({"value":null})"));
    json progress;
    on_third_day = [&] {
        std::thread screen([&] {
            progress = called(program, "watch_progress", {{"id", 7}});
            called(program, "stop_watch", {{"id", 7}});
        });
        screen.join();
    };
    json answer;
    std::thread worker([&] {
        called(program, "start_watch", {{"id", 7}});
        answer = called(program, "dispatch_event", {{"state", {{"window", "Main"}, {"data", {{"name", "Ann"}}}}}, {"handler", "plan"}});
        called(program, "end_watch", {{"id", 7}});
    });
    worker.join();
    on_third_day = nullptr;
    CHECK_EQ(progress, json::parse(R"({"value":[3,365]})"));
    CHECK_EQ(answer, json::parse(R"({"error":"engine Planner failed in plan: it was stopped"})"));
    called(program, "end_watch", {{"id", 7}});
    CHECK_EQ(called(program, "watch_progress", {{"id", 7}}), json::parse(R"({"value":null})"));

    // A Stop that comes before its start_watch holds once the call starts.
    called(program, "stop_watch", {{"id", 8}});
    std::thread early([&] {
        called(program, "start_watch", {{"id", 8}});
        answer = called(program, "dispatch_event", {{"state", {{"window", "Main"}, {"data", json::object()}}}, {"handler", "plan"}});
        called(program, "end_watch", {{"id", 8}});
    });
    early.join();
    CHECK_EQ(answer, json::parse(R"({"error":"engine Planner failed in plan: it was stopped"})"));
    // Outside a watch, a handler's thread has none.
    CHECK(current_watch() == nullptr);
}

// The program driven on a screen without a window (DESIGN 1.6): its main
// window opens from its files, a click runs its handler, and a custom part
// written in C++ draws, changes its value and tells its window.
TEST(a_driven_desktop_program_runs_its_windows_and_custom_parts) {
    REQUIRE(ui::fits());
    Db db = Db::memory();
    Shop shop;
    shell::Options options = options_of(db, shop);
    ui::register_part("CppGauge", [] { return std::make_unique<Gauge>(); });
    shell::DesktopProgram program(options);
    ui::Files made = shell::desktop_files(options);
    ui::Driven driven(program, made);
    json read = json::parse(driven.step(R"({"do":"read"})"));
    CHECK_EQ(read["data"]["name"], json("Ann"));
    CHECK(std::find(Gauge::log.begin(), Gauge::log.end(), "set value 3") != Gauge::log.end());
    json clicked = json::parse(driven.step(R"({"do":"click","part":"save"})"));
    CHECK(!clicked.contains("error"));
    read = json::parse(driven.step(R"({"do":"read"})"));
    CHECK_EQ(read["data"]["name"], json("Saved"));
    // The gauge's click: its value goes to what it binds, its tell to its handler.
    driven.step(R"({"do":"click","part":"gauge"})");
    read = json::parse(driven.step(R"({"do":"read"})"));
    CHECK_EQ(read["data"]["speed"], json(5));
    CHECK_EQ(read["data"]["reached"], json(5));
    // A step that cannot be done answers why.
    json missing = json::parse(driven.step(R"({"do":"click","part":"nothing"})"));
    CHECK(missing.contains("error"));
}

TEST(a_failing_call_of_the_library_throws_its_message) {
    ui::Files files;
    bool threw = false;
    try {
        files.model("{not json");
    } catch (const ui::Error& e) {
        threw = true;
        CHECK_EQ(e.status(), CLARITY_INVALID);
        CHECK(std::string(e.what()).find("model") != std::string::npos);
    }
    CHECK(threw);
}

// The built-in quit and keys run by a handler reach the library in its reply,
// once: the next reply says neither.
TEST(code_quits_and_opens_the_keys_window_through_the_layer) {
    Db db = Db::memory();
    Shop shop;
    shell::Options options = options_of(db, shop);
    shell::DesktopProgram program(options);
    auto run = [&](const char* handler) { return called(program, "dispatch_event", {{"state", {{"window", "Main"}, {"data", json::object()}}}, {"handler", handler}})["value"]; };
    json quit = run("quitIt");
    CHECK_EQ(quit.value("quit", false), true);
    CHECK(!quit.contains("keys"));
    json keys = run("keysNow");
    CHECK_EQ(keys.value("keys", false), true);
    CHECK(!keys.contains("quit"));
    json plain = run("save");
    CHECK(!plain.contains("quit") && !plain.contains("keys"));
}

namespace {

/// A window whose data is the person's settings record; flip says what it saw and turns showDone over.
class Flipping final : public shell::Windows {
public:
    std::string saw, tokens;
    WindowState open(Db&, const std::string& window, const json& with) override {
        return opened(window, {{"settings", {{"id", 0}, {"showDone", true}, {"theme", ""}, {"apiToken", ""}}}}, with);
    }
    std::optional<std::string> handler(const std::string&, const std::string&) override { return std::nullopt; }
    Reply redraw(Db&, Reply reply) override { return reply; }
    Reply dispatch(Db&, const WindowState& state, const std::string&) override {
        json data = state.data;
        bool shown = data["settings"]["showDone"].get<bool>();
        saw += shown ? "t" : "f";
        tokens += data["settings"]["apiToken"].get<std::string>();
        data["settings"]["showDone"] = !shown;
        return Ui(state).reply(state.window, data);
    }
    Reply answer(Db&, const WindowState& state, const std::string&, const json&) override { return Ui(state).reply(state.window, state.data); }
    std::vector<RefOption> options(Db&, const std::string&) override { return {}; }
    std::optional<Bytes> picture(Db&, const std::string&, int64_t, const std::string&) override { return std::nullopt; }
};

const char* const flip_markup = R"(<Window name="Main"><Button name="flip" at="1,1" span="3" caption="Flip" onclick="flip" /></Window>)";

const shell::UiFile flip_files[] = {
    {"window", "Main", bytes_of(flip_markup), std::char_traits<char>::length(flip_markup)},
    {nullptr, nullptr, nullptr, 0},
};

}  // namespace

// A window starts from the settings the person kept, and settings a handler
// changes are kept: the next run's handler sees them.
TEST(a_window_starts_from_the_kept_settings_and_a_handlers_change_is_kept) {
    REQUIRE(ui::fits());
    auto folder = std::filesystem::temp_directory_path() / "clarity-desktop-settings-cpp";
    std::filesystem::remove_all(folder);
#ifdef _WIN32
    const char* had = std::getenv("APPDATA");
    std::string was = had ? had : "";
    _putenv_s("APPDATA", folder.string().c_str());
#else
    setenv("XDG_CONFIG_HOME", folder.string().c_str(), 1);
#endif
    Db db = Db::memory();
    Flipping flipping;
    auto run = [&] {
        shell::Options options{"clarity-desktop-settings-test", "Flip", "Main", R"({"project":{}})", &db, &flipping};
        options.ui_files = flip_files;
        shell::DesktopProgram program(options);
        ui::Files made = shell::desktop_files(options);
        ui::Driven driven(program, made);
        driven.step(R"({"do":"click","part":"flip"})");
    };
    run();
    CHECK_EQ(read_settings("clarity-desktop-settings-test").value("showDone", true), false);
    run();
    CHECK_EQ(flipping.saw, std::string("tf"));
    std::filesystem::remove_all(folder);
#ifdef _WIN32
    _putenv_s("APPDATA", was.c_str());
#endif
}

// A secret setting a handler reads has its value from the protected store,
// never from the settings file.
TEST(a_secret_setting_a_handler_reads_comes_from_the_store) {
    REQUIRE(ui::fits());
    auto folder = std::filesystem::temp_directory_path() / "clarity-desktop-secrets-cpp";
    std::filesystem::remove_all(folder);
#ifdef _WIN32
    const char* had = std::getenv("APPDATA");
    std::string was = had ? had : "";
    _putenv_s("APPDATA", folder.string().c_str());
#else
    setenv("XDG_CONFIG_HOME", folder.string().c_str(), 1);
#endif
    SecretStore before = secret_store;
    std::map<std::string, std::string> store{{"apiToken", "s3cret"}};
    secret_store.get = [&](const std::string&, const std::string& name) -> std::optional<std::string> {
        auto found = store.find(name);
        return found == store.end() ? std::nullopt : std::optional<std::string>(found->second);
    };
    secret_store.set = [&](const std::string&, const std::string& name, const std::string& value) { store[name] = value; };
    Db db = Db::memory();
    Flipping flipping;
    {
        shell::Options options{"clarity-desktop-secrets-test", "Flip", "Main", R"({"project":{}})", &db, &flipping};
        options.ui_files = flip_files;
        options.secrets = json::array({"apiToken"});
        shell::DesktopProgram program(options);
        ui::Files made = shell::desktop_files(options);
        ui::Driven driven(program, made);
        driven.step(R"({"do":"click","part":"flip"})");
    }
    CHECK_EQ(flipping.tokens, std::string("s3cret"));
    CHECK(!read_settings("clarity-desktop-secrets-test").contains("apiToken"));
    secret_store = before;
    std::filesystem::remove_all(folder);
#ifdef _WIN32
    _putenv_s("APPDATA", was.c_str());
#endif
}
