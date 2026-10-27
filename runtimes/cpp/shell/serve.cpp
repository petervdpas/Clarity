// The web target (DESIGN 7.3): see serve.hpp. Plain sockets, as on every
// OS; Windows through Winsock.

#include "shell/serve.hpp"

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <iostream>

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

std::string reply(const std::string& status, const std::string& type, const std::string& body) {
    return "HTTP/1.1 " + status + "\r\nContent-Type: " + type + "\r\nContent-Length: " + std::to_string(body.size()) + "\r\nConnection: close\r\n\r\n" + body;
}

/// The page as the server gives it: it says where its commands go.
std::string served(const std::string& page) {
    auto head = page.find("</head>");
    if (head == std::string::npos) return page;
    return page.substr(0, head) + "<script>window.clarityServer = \"" + invoke_path + "\";</script>" + page.substr(head);
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

}  // namespace

std::string serve_answer(Service& service, const std::string& page, const std::string& request) {
    auto end = request.find("\r\n\r\n");
    auto first = request.find(' ');
    auto second = first == std::string::npos ? first : request.find(' ', first + 1);
    if (end == std::string::npos || second == std::string::npos) return reply("400 Bad Request", "text/plain", "");
    std::string method = request.substr(0, first), path = request.substr(first + 1, second - first - 1);
    if (path == "/" || path == "/index.html") return reply("200 OK", "text/html; charset=utf-8", served(page));
    if (path != invoke_path) return reply("404 Not Found", "text/plain", "");
    if (method != "POST") return reply("405 Method Not Allowed", "text/plain", "send a command with POST");
    auto sent = nlohmann::json::parse(request.substr(end + 4), nullptr, false);
    nlohmann::json answer;
    if (sent.is_object() && sent.contains("command") && sent["command"].is_string()) {
        auto args = sent.contains("args") && sent["args"].is_object() ? sent["args"] : nlohmann::json::object();
        answer = service.invoke(sent["command"].get<std::string>(), args);
    } else {
        answer = {{"error", "a command is JSON with its command and its args"}};
    }
    return reply("200 OK", "application/json", answer.dump());
}

std::string serve_address(int argc, char** argv) {
    for (int i = 1; i + 1 < argc; i++)
        if (std::strcmp(argv[i], "--address") == 0) return argv[i + 1];
    const char* address = std::getenv("CLARITY_ADDRESS");
    return address && *address ? address : default_address;
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
    Service service(options);
    std::cerr << options.name << ": serving on http://" << address << "\n";
    for (;;) {
        Socket client = accept(listener, nullptr, nullptr);
        if (client == SERVE_INVALID) continue;
        std::string request = read_request(client);
        if (!request.empty()) send_all(client, serve_answer(service, options.html, request));
        SERVE_CLOSE(client);
    }
}

}  // namespace clarity::shell
