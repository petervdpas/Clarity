// The web target (DESIGN 7.3): Clarity.UI runs in the browser, and the
// server serves the page, the library's browser files, the program's files
// and its resources, and answers the library's commands over HTTP as the
// desktop answers them in-process.

#include <cstdlib>
#include <chrono>
#include <cstring>
#include <stdexcept>
#include <thread>

#include "clarity/text.hpp"
#include "clarity/watch.hpp"

#include "check.hpp"
#include "shell/serve.hpp"

using namespace clarity;

namespace {

struct Nothing : shell::Windows {
    WindowState open(Db&, const std::string& window, const nlohmann::json&) override {
        WindowState state;
        state.window = window;
        state.data = {{"name", "Ada"}};
        return state;
    }
    std::optional<std::string> handler(const std::string&, const std::string&) override { return std::nullopt; }
    Reply dispatch(Db&, const WindowState&, const std::string&) override { return {}; }
    Reply answer(Db&, const WindowState&, const std::string&, const nlohmann::json&) override { return {}; }
    Reply redraw(Db&, Reply reply) override { return reply; }
    std::vector<RefOption> options(Db&, const std::string&) override { return {}; }
    std::optional<Bytes> picture(Db&, const std::string&, int64_t, const std::string&) override { return std::nullopt; }
};

std::string post(const std::string& body) {
    return "POST /clarity/invoke HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: " + std::to_string(body.size()) + "\r\n\r\n" + body;
}

bool has(const std::string& text, const std::string& part) { return text.find(part) != std::string::npos; }

std::string body_of(const std::string& reply) {
    auto end = reply.find("\r\n\r\n");
    return end == std::string::npos ? "" : reply.substr(end + 4);
}

const char page[] = "<!doctype html><canvas id=\"clarity-ui\"></canvas><script src=\"clarity-ui-page.js\"></script>";
const char files_json[] = R"({"title":"Shop","windows":{}})";
const unsigned char wasm[] = {0x00, 'a', 's', 'm', 0x01, 0x00};
const unsigned char logo[] = {0x89, 'P', 'N', 'G'};
const shell::UiFile browser[] = {
    {"browser", "index.html", reinterpret_cast<const unsigned char*>(page), sizeof page - 1},
    {"browser", "clarity-ui.wasm", wasm, sizeof wasm},
    {"browser", "clarity-ui-page.js", reinterpret_cast<const unsigned char*>("page();"), 7},
    {"browser", "clarity/files.json", reinterpret_cast<const unsigned char*>(files_json), sizeof files_json - 1},
    {"browser", "clarity/resources/pictures/a logo.png", logo, sizeof logo},
    {nullptr, nullptr, nullptr, 0},
};

}  // namespace

TEST(the_server_serves_the_page_and_answers_its_commands) {
    Db db = Db::memory();
    Nothing windows;
    shell::Options options;
    options.name = "shop";
    options.main = "Card";
    options.model = R"({"project":{}})";
    options.db = &db;
    options.windows = &windows;
    options.browser_files = browser;
    options.facts = R"({"windows":{"Card":{"name":"text"}}})";
    shell::Server service(options);
    // The page at /, as it is: its script finds the server by itself.
    std::string got = shell::serve_answer(service, "GET / HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(got.rfind("HTTP/1.1 200 OK\r\n", 0) == 0);
    CHECK(has(got, "Content-Type: text/html; charset=utf-8\r\n"));
    CHECK(body_of(got) == page);
    // The browser files by their names and types; the wasm byte for byte, its zeros too.
    got = shell::serve_answer(service, "GET /clarity-ui.wasm HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(has(got, "Content-Type: application/wasm\r\n"));
    CHECK(has(got, "Content-Length: 6\r\n"));
    CHECK(body_of(got) == std::string(reinterpret_cast<const char*>(wasm), sizeof wasm));
    got = shell::serve_answer(service, "GET /clarity-ui-page.js?v=1 HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(has(got, "Content-Type: text/javascript\r\n"));
    got = shell::serve_answer(service, "GET /clarity/files.json HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(has(got, "Content-Type: application/json\r\n"));
    CHECK(body_of(got) == files_json);
    // A resource by its name, spaces written as %20.
    got = shell::serve_answer(service, "GET /clarity/resources/pictures/a%20logo.png HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(has(got, "Content-Type: image/png\r\n"));
    CHECK(body_of(got) == std::string(reinterpret_cast<const char*>(logo), sizeof logo));
    // Clarity.UI's commands, answered as the desktop answers them.
    got = shell::serve_answer(service, post(R"({"command": "started", "args": {}})"));
    CHECK(got.rfind("HTTP/1.1 200 OK\r\n", 0) == 0);
    CHECK(nlohmann::json::parse(body_of(got)) == nlohmann::json::parse(R"({"value":{"main":"Card","unconnected":null}})"));
    got = shell::serve_answer(service, post(R"({"command": "open_window", "args": {"window": "Card"}})"));
    CHECK(has(body_of(got), "\"name\":\"Ada\""));
    got = shell::serve_answer(service, post(R"({"command": "facts", "args": {}})"));
    CHECK(nlohmann::json::parse(body_of(got)) == nlohmann::json::parse(R"({"value":{"windows":{"Card":{"name":"text"}}}})"));
    // What the program does not answer is an error, which the library takes as its default.
    got = shell::serve_answer(service, post(R"({"command": "nonsense", "args": {}})"));
    CHECK(body_of(got) == R"({"error":"there is no command nonsense"})");
    got = shell::serve_answer(service, post("not json"));
    CHECK(body_of(got).rfind("{\"error\":", 0) == 0);
    got = shell::serve_answer(service, "GET /clarity/invoke HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(got.rfind("HTTP/1.1 405 Method Not Allowed\r\n", 0) == 0);
    // Nothing else is there.
    got = shell::serve_answer(service, "GET /secret HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(got.rfind("HTTP/1.1 404 Not Found\r\n", 0) == 0);
}

namespace {

/// `plan` runs long as an engine call does: it says how far it is, then waits until it is stopped.
struct Planner : Nothing {
    Reply dispatch(Db&, const WindowState& state, const std::string& handler) override {
        const EngineWatch* watch = current_watch();
        if (handler != "plan" || !watch) throw std::runtime_error("no watch");
        watch->progress(1, 10);
        for (int i = 0; i < 20000 && !watch->stop->load(); ++i) std::this_thread::sleep_for(std::chrono::milliseconds(1));
        if (watch->stop->load()) throw std::runtime_error("it was stopped");
        Reply reply;
        reply.state = state;
        return reply;
    }
};

std::string asked(shell::Server& server, const std::string& command) { return body_of(shell::serve_answer(server, post(command))); }

}  // namespace

// A long call over the wire (DESIGN 5.12): its handler's request names its
// watch and stays open while the page asks how far it is and stops it from
// requests of their own; a stop sent before its handler started holds.
TEST(a_long_call_over_the_wire_says_how_far_it_is_and_stops) {
    Db db = Db::memory();
    Planner windows;
    shell::Options options;
    options.name = "shop";
    options.main = "Card";
    options.model = R"({"project":{}})";
    options.db = &db;
    options.windows = &windows;
    options.watches = true;
    shell::Server server(options);
    CHECK(asked(server, R"({"command":"watches","args":{}})") == R"({"value":true})");
    CHECK(asked(server, R"({"command":"watch_progress","args":{"id":5}})") == R"({"value":null})");
    std::string planned;
    std::thread handler([&] { planned = asked(server, R"({"command":"dispatch_event","args":{"state":{"window":"Card","data":{}},"handler":"plan","watch":5}})"); });
    // How far it is, while its request is open.
    std::string progress;
    for (int i = 0; i < 5000 && progress != R"({"value":[1,10]})"; ++i) {
        progress = asked(server, R"({"command":"watch_progress","args":{"id":5}})");
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    CHECK(progress == R"({"value":[1,10]})");
    CHECK(asked(server, R"({"command":"stop_watch","args":{"id":5}})") == R"({"value":null})");
    handler.join();
    CHECK(has(planned, "it was stopped"));
    // It keeps how far it got until end_watch, which may come twice.
    CHECK(asked(server, R"({"command":"watch_progress","args":{"id":5}})") == R"({"value":[1,10]})");
    CHECK(asked(server, R"({"command":"end_watch","args":{"id":5}})") == R"({"value":null})");
    CHECK(asked(server, R"({"command":"end_watch","args":{"id":5}})") == R"({"value":null})");
    CHECK(asked(server, R"({"command":"watch_progress","args":{"id":5}})") == R"({"value":null})");
    // A stop before its handler started stops it when it starts.
    asked(server, R"({"command":"stop_watch","args":{"id":6}})");
    CHECK(has(asked(server, R"({"command":"dispatch_event","args":{"state":{"window":"Card","data":{}},"handler":"plan","watch":6}})"), "it was stopped"));
    asked(server, R"({"command":"end_watch","args":{"id":6}})");
}

TEST(the_server_listens_where_it_is_told) {
    char app[] = "app", flag[] = "--address", where[] = "0.0.0.0:9000";
    char* none[] = {app};
    CHECK(shell::serve_address(1, none, "127.0.0.1:9090") == "127.0.0.1:9090" || std::getenv("CLARITY_ADDRESS"));
    char* told[] = {app, flag, where};
    CHECK(shell::serve_address(3, told, "127.0.0.1:9090") == "0.0.0.0:9000");
}

namespace {

/// A card whose settings record starts from the asking page's: its open
/// handler writes what it saw, and flip changes a setting and quits.
struct PageCards : Nothing {
    WindowState open(Db&, const std::string& window, const nlohmann::json&) override {
        WindowState state;
        state.window = window;
        state.data = {{"settings", {{"id", 0}, {"showDone", true}, {"theme", ""}}}};
        return state;
    }
    std::optional<std::string> handler(const std::string&, const std::string& event) override {
        return event == "open" ? std::optional<std::string>("opened") : std::nullopt;
    }
    Reply dispatch(Db&, const WindowState& state, const std::string& handler) override {
        Ui ui(state);
        auto data = state.data;
        bool shows = data["settings"]["showDone"].get<bool>();
        if (handler == "opened") data["saw"] = shows;
        if (handler == "flip") {
            data["settings"]["showDone"] = !shows;
            ui.quit();
        }
        data["said"] = text_of("hi");
        return ui.reply(state.window, data);
    }
};

}  // namespace

// Each browser apart (DESIGN 7.9 B2): a request names its page and brings
// the person's settings as the page keeps them. A handler sees that page's
// settings and language, a page that sends none keeps what it had, settings
// a command changed and quit go back to that page alone, and a page unasked
// for longer than the quiet time is forgotten.
TEST(each_browser_has_its_own_settings_and_language) {
    Db db = Db::memory();
    PageCards windows;
    shell::Options options;
    options.name = "shop";
    options.main = "Card";
    options.model = R"({"project":{"locale":{"language":"en"},"translations":{"en":{"hi":"Hello"},"nl":{"hi":"Hallo"}}}})";
    options.db = &db;
    options.windows = &windows;
    options.browser_files = browser;
    shell::Server server(options);
    server.quiet(std::chrono::milliseconds(300));
    auto ask = [&](const std::string& body) { return nlohmann::json::parse(body_of(shell::serve_answer(server, post(body)))); };
    auto data = [](const nlohmann::json& answer) { return answer["value"]["state"]["data"]; };
    CHECK(ask(R"({"command":"set_language","args":{"language":"nl"},"browser":"a","settings":{"showDone":false}})") == nlohmann::json({{"value", nullptr}}));
    auto a = ask(R"({"command":"open_window","args":{"window":"Card"},"browser":"a","settings":{"showDone":false}})");
    CHECK(data(a)["saw"] == false && data(a)["said"] == "Hallo" && !a.contains("settings"));
    auto b = ask(R"({"command":"open_window","args":{"window":"Card"},"browser":"b"})");
    CHECK(data(b)["saw"] == true && data(b)["said"] == "Hello");
    // a's handler changes its settings: they come back to a alone, quit with them.
    auto flipped = ask(R"({"command":"dispatch_event","args":{"state":)" + a["value"]["state"].dump() + R"(,"handler":"flip"},"browser":"a"})");
    CHECK(flipped["settings"] == nlohmann::json({{"showDone", true}, {"theme", ""}}));
    CHECK(flipped["value"]["quit"] == true && data(flipped)["said"] == "Hallo");
    b = ask(R"({"command":"open_window","args":{"window":"Card"},"browser":"b"})");
    CHECK(!b.contains("settings") && !b["value"].contains("quit") && data(b)["said"] == "Hello");
    // a sends no settings: it keeps those it had, the changed ones.
    a = ask(R"({"command":"open_window","args":{"window":"Card"},"browser":"a"})");
    CHECK(data(a)["saw"] == true);
    // Unasked for longer than the quiet time, a page is forgotten: its language too.
    std::this_thread::sleep_for(std::chrono::milliseconds(400));
    ask(R"({"command":"open_window","args":{"window":"Card"},"browser":"b"})");
    a = ask(R"({"command":"open_window","args":{"window":"Card"},"browser":"a"})");
    CHECK(data(a)["said"] == "Hello");
    set_language(std::nullopt);
}
