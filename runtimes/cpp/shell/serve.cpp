// The web target (DESIGN 7.3): see serve.hpp. Plain sockets, as on every
// OS; Windows through Winsock.

#include "shell/serve.hpp"

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <string_view>
#include <system_error>
#include <thread>
#include <utility>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
using Socket = SOCKET;
#define SERVE_CLOSE closesocket
#define SERVE_INVALID INVALID_SOCKET
#else
#include <netdb.h>
#include <sys/socket.h>
#include <unistd.h>
using Socket = int;
#define SERVE_CLOSE close
#define SERVE_INVALID (-1)
#endif

namespace clarity::shell {

namespace {

/// The largest request the server reads: a command with a file in it.
constexpr size_t largest_request = 64u * 1024u * 1024u;

std::string reply(const std::string& status, const std::string& type, std::string_view body) {
    std::string out = "HTTP/1.1 " + status + "\r\nContent-Type: " + type + "\r\nContent-Length: " + std::to_string(body.size()) + "\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    out.append(body);
    return out;
}

/// The Content-Type of a file by its name.
std::string type_of(std::string_view name) {
    static const std::pair<std::string_view, std::string_view> types[] = {
        {".html", "text/html; charset=utf-8"}, {".js", "text/javascript"}, {".wasm", "application/wasm"}, {".json", "application/json"}, {".css", "text/css"},
        {".svg", "image/svg+xml"},           {".png", "image/png"},        {".jpg", "image/jpeg"},        {".jpeg", "image/jpeg"},       {".gif", "image/gif"},
        {".webp", "image/webp"},             {".ico", "image/x-icon"},     {".ttf", "font/ttf"},          {".otf", "font/otf"},          {".woff", "font/woff"},
        {".woff2", "font/woff2"},            {".wav", "audio/wav"},        {".mp3", "audio/mpeg"},        {".ogg", "audio/ogg"},         {".txt", "text/plain; charset=utf-8"},
    };
    for (const auto& [extension, type] : types)
        if (name.ends_with(extension)) return std::string(type);
    return "application/octet-stream";
}

int hex_digit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/// A path as it names a file: without its query, %XX read.
std::string decoded(const std::string& path) {
    std::string out;
    for (size_t i = 0; i < path.size() && path[i] != '?' && path[i] != '#'; ++i) {
        if (path[i] == '%' && i + 2 < path.size() && hex_digit(path[i + 1]) >= 0 && hex_digit(path[i + 2]) >= 0) {
            out += static_cast<char>(hex_digit(path[i + 1]) * 16 + hex_digit(path[i + 2]));
            i += 2;
        } else {
            out += path[i];
        }
    }
    return out;
}

/// Reads one request: its headers, then its body by its length; empty when the client went.
std::string read_request(Socket client) {
    std::string request;
    char chunk[64 * 1024];
    while (request.size() < largest_request) {
        int n = static_cast<int>(recv(client, chunk, sizeof chunk, 0));
        if (n <= 0) return {};
        request.append(chunk, static_cast<size_t>(n));
        auto end = request.find("\r\n\r\n");
        if (end == std::string::npos) continue;
        size_t length = 0;
        for (size_t line = 0; line < end; line = request.find("\r\n", line) + 2) {
            if (request.compare(line, 15, "Content-Length:") == 0 || request.compare(line, 15, "content-length:") == 0) length = std::strtoul(request.c_str() + line + 15, nullptr, 10);
        }
        if (request.size() >= end + 4 + length) return request;
    }
    return {};
}

void send_all(Socket client, const std::string& text) {
    size_t sent = 0;
    while (sent < text.size()) {
        int n = static_cast<int>(send(client, text.data() + sent, static_cast<int>(std::min<size_t>(text.size() - sent, 1 << 20)), 0));
        if (n <= 0) return;
        sent += static_cast<size_t>(n);
    }
}

/// The settings record of a window's state in `holder`'s "state"; null when it has none.
nlohmann::json window_settings(const nlohmann::json& holder) {
    if (!holder.is_object() || !holder.contains("state") || !holder["state"].is_object()) return nullptr;
    const auto& state = holder["state"];
    if (!state.contains("data") || !state["data"].is_object() || !state["data"].contains("settings")) return nullptr;
    return state["data"]["settings"].is_object() ? state["data"]["settings"] : nlohmann::json(nullptr);
}

/// The settings the answer's window has (its id left out), when one of
/// them differs from what the window had (`before`); null when none changed.
nlohmann::json settings_changed(const nlohmann::json& before, const std::string& answer) {
    auto answered = nlohmann::json::parse(answer, nullptr, false);
    auto after = answered.is_object() && answered.contains("value") ? window_settings(answered["value"]) : nlohmann::json(nullptr);
    if (!after.is_object() || !before.is_object()) return nullptr;
    after.erase("id");
    for (const auto& [name, value] : after.items())
        if (before.contains(name) && before[name] != value) return after;
    return nullptr;
}

}  // namespace

Page Server::page(const std::string& browser, const nlohmann::json& sent) {
    std::lock_guard lock(paging_);
    auto now = std::chrono::steady_clock::now();
    std::erase_if(pages_, [&](const auto& kept) { return kept.first != browser && now - kept.second.used > quiet_; });
    auto& kept = pages_[browser];
    kept.used = now;
    if (sent.is_object()) kept.page.settings = sent;
    return kept.page;
}

void Server::choose_language(const std::string& browser, std::optional<std::string> language) {
    std::lock_guard lock(paging_);
    pages_[browser].page.language = std::move(language);
}

nlohmann::json Server::keep_settings(const std::string& browser, const nlohmann::json& changed) {
    std::lock_guard lock(paging_);
    auto& settings = pages_[browser].page.settings;
    if (!settings.is_object()) settings = nlohmann::json::object();
    settings.update(changed);
    return settings;
}

std::string serve_answer(Server& server, const std::string& request) {
    auto end = request.find("\r\n\r\n");
    auto first = request.find(' ');
    auto second = first == std::string::npos ? first : request.find(' ', first + 1);
    if (end == std::string::npos || second == std::string::npos) return reply("400 Bad Request", "text/plain", "");
    std::string method = request.substr(0, first), path = decoded(request.substr(first + 1, second - first - 1));
    if (path == invoke_path) {
        if (method != "POST") return reply("405 Method Not Allowed", "text/plain", "send a command with POST");
        auto sent = nlohmann::json::parse(request.substr(end + 4), nullptr, false);
        if (!sent.is_object() || !sent.contains("command") || !sent["command"].is_string())
            return reply("200 OK", "application/json", nlohmann::json{{"error", "a command is JSON with its command and its args"}}.dump());
        auto command = sent["command"].get<std::string>();
        auto args = sent.contains("args") && sent["args"].is_object() ? sent["args"] : nlohmann::json::object();
        auto number = [&args](const char* name) { return args.contains(name) && args[name].is_number_unsigned() ? args[name].get<uint64_t>() : uint64_t(0); };
        // How far a long call is, and its Stop, beside the service.
        if (command == "watches") return reply("200 OK", "application/json", nlohmann::json{{"value", server.service().options().watches}}.dump());
        if (Watches::is_command(command)) return reply("200 OK", "application/json", server.watches().answer(command, number("id")).dump());
        // Engines beside the screen run in the program, each canvas's by its number.
        if (command.rfind("engine_", 0) == 0) {
            const auto& engines = server.service().options().screen_engines;
            auto answered = engines ? engines(command, args) : std::nullopt;
            return reply("200 OK", "application/json", answered ? answered->dump() : nlohmann::json{{"error", "there is no command " + command}}.dump());
        }
        // The page that asks (one id per tab), with its person's settings as it keeps them; its language is its own.
        std::string browser = sent.contains("browser") && sent["browser"].is_string() ? sent["browser"].get<std::string>() : "";
        Page page = server.page(browser, sent.contains("settings") ? sent["settings"] : nlohmann::json(nullptr));
        if (command == "set_language") {
            const auto& language = args.contains("language") ? args["language"] : nlohmann::json(nullptr);
            server.choose_language(browser, language.is_string() && !language.get<std::string>().empty() ? std::optional(language.get<std::string>()) : std::nullopt);
            return reply("200 OK", "application/json", nlohmann::json{{"value", nullptr}}.dump());
        }
        // What the window had: the state's settings, or else the page's.
        auto before = window_settings(args);
        if (before.is_null()) before = page.settings;
        // A handler's request that names its watch runs under it (the page sends no start_watch).
        uint64_t watch = command == "dispatch_event" || command == "answer_event" ? number("watch") : 0;
        if (watch) server.watches().enter(watch);
        std::optional<std::string> answer;
        {
            AsPage as(page);
            answer = wire_answer(server.service(), command, args);
        }
        if (watch) server.watches().leave();
        // Settings the command changed go back to this page alone, for it to keep.
        if (auto now = answer ? settings_changed(before, *answer) : nlohmann::json(nullptr); now.is_object()) {
            auto answered = nlohmann::json::parse(*answer);
            answered["settings"] = server.keep_settings(browser, now);
            answer = answered.dump();
        }
        // None reads as the service's error for a command it does not know, which the library takes as its default.
        return reply("200 OK", "application/json", answer ? *answer : nlohmann::json{{"error", "there is no command " + command}}.dump());
    }
    // The browser files by their paths below browser/; the page at /.
    if (method == "GET" && !path.empty() && path[0] == '/') {
        std::string name = path == "/" ? "index.html" : path.substr(1);
        for (const UiFile* file = server.service().options().browser_files; file && file->kind; ++file)
            if (name == file->name) return reply("200 OK", type_of(name), std::string_view(reinterpret_cast<const char*>(file->bytes), file->size));
    }
    return reply("404 Not Found", "text/plain", "");
}

std::string serve_address(int argc, char** argv, const std::string& project) {
    for (int i = 1; i + 1 < argc; i++)
        if (std::strcmp(argv[i], "--address") == 0) return argv[i + 1];
    const char* address = std::getenv("CLARITY_ADDRESS");
    return address && *address ? std::string(address) : project;
}

int serve(const Options& options, const std::string& address) {
#ifdef _WIN32
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) return 1;
#endif
    auto colon = address.rfind(':');
    if (colon == std::string::npos) {
        std::cerr << options.name << ": an address is host:port, not " << address << "\n";
        return 1;
    }
    std::string host = address.substr(0, colon), port = address.substr(colon + 1);
    addrinfo hints{};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_PASSIVE;
    addrinfo* found = nullptr;
    if (getaddrinfo(host.empty() ? nullptr : host.c_str(), port.c_str(), &hints, &found) != 0 || !found) {
        std::cerr << options.name << ": cannot listen on " << address << "\n";
        return 1;
    }
    Socket listener = socket(found->ai_family, found->ai_socktype, found->ai_protocol);
    int yes = 1;
    setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, reinterpret_cast<const char*>(&yes), sizeof yes);
    bool listening = listener != SERVE_INVALID && bind(listener, found->ai_addr, static_cast<int>(found->ai_addrlen)) == 0 && listen(listener, 16) == 0;
    freeaddrinfo(found);
    if (!listening) {
        std::cerr << options.name << ": cannot listen on " << address << "\n";
        return 1;
    }
    Server server(options);
    std::cerr << options.name << ": serving on http://" << address << "\n";
    // Requests side by side, each on a thread of its own: a long call's
    // request stays open while the page asks how far it is.
    for (;;) {
        Socket client = accept(listener, nullptr, nullptr);
        if (client == SERVE_INVALID) continue;
        auto answer_one = [&server, client] {
            std::string request = read_request(client);
            if (!request.empty()) send_all(client, serve_answer(server, request));
            SERVE_CLOSE(client);
        };
        try {
            std::thread(answer_one).detach();
        } catch (const std::system_error&) {
            answer_one();
        }
    }
}

}  // namespace clarity::shell
