// The web target (DESIGN 7.3): the shell as a web server, which serves the
// renderer's page to a browser and answers its commands over HTTP with the
// same service the desktop window binds into its webview.

#include <cstdlib>

#include "check.hpp"
#include "shell/serve.hpp"

using namespace clarity;

namespace {

struct Nothing : shell::Windows {
    WindowState open(Db&, const std::string&, const nlohmann::json&) override { return {}; }
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

}  // namespace

TEST(the_server_serves_the_page_and_answers_its_commands) {
    Db db = Db::memory();
    Nothing windows;
    shell::Options options;
    options.name = "shop";
    options.main = "Card";
    options.model = R"({"project":{}})";
    options.html = "<html><head><title>Shop</title></head><body></body></html>";
    options.db = &db;
    options.windows = &windows;
    shell::Service service(options);
    // The page says where it sends its commands.
    std::string got = shell::serve_answer(service, options.html, "GET / HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(got.rfind("HTTP/1.1 200 OK\r\n", 0) == 0);
    CHECK(has(got, "Content-Type: text/html; charset=utf-8\r\n"));
    CHECK(has(got, "<script>window.clarityServer = \"/clarity/invoke\";</script></head>"));
    // A command and its arguments, answered as the desktop shell answers them.
    got = shell::serve_answer(service, options.html, post(R"({"command": "start", "args": {}})"));
    CHECK(got.rfind("HTTP/1.1 200 OK\r\n", 0) == 0);
    CHECK(has(got, "\r\n\r\n{\"value\":{\"main\":\"Card\",\"model\":{\"project\":{}}}}"));
    got = shell::serve_answer(service, options.html, post(R"({"command": "nonsense", "args": {}})"));
    CHECK(has(got, "\r\n\r\n{\"error\":\"there is no command nonsense\"}"));
    got = shell::serve_answer(service, options.html, post("not json"));
    CHECK(has(got, "\r\n\r\n{\"error\":"));
    // Nothing else is there.
    got = shell::serve_answer(service, options.html, "GET /secret HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(got.rfind("HTTP/1.1 404 Not Found\r\n", 0) == 0);
}

TEST(the_server_listens_where_it_is_told) {
    char app[] = "app", flag[] = "--address", where[] = "0.0.0.0:9000";
    char* none[] = {app};
    CHECK(shell::serve_address(1, none, "127.0.0.1:9090") == "127.0.0.1:9090" || std::getenv("CLARITY_ADDRESS"));
    char* told[] = {app, flag, where};
    CHECK(shell::serve_address(3, told, "127.0.0.1:9090") == "0.0.0.0:9000");
}
