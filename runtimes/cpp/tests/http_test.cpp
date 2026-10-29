// HTTP calls (DESIGN 5.23) against the cases every runtime shares
// (conformance/http.json), on the test server the C runtime's tests use.

#include "../../c/tests/http_server.h"
#include "check.hpp"
#include "conformance.hpp"
#include "http/http.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

HttpMethod method_of(const std::string& word) {
    if (word == "POST") return HttpMethod::Post;
    if (word == "PUT") return HttpMethod::Put;
    if (word == "PATCH") return HttpMethod::Patch;
    if (word == "DELETE") return HttpMethod::Delete;
    return HttpMethod::Get;
}

}  // namespace

TEST(every_http_call_gives_the_reply_the_cases_name) {
    json cases = conformance("http.json");
    std::string server = http_test_start();
    for (const auto& [name, value] : cases["env"].items()) {
#ifdef _WIN32
        _putenv_s(name.c_str(), value.get<std::string>().c_str());
#else
        setenv(name.c_str(), value.get<std::string>().c_str(), 1);
#endif
    }
    for (const auto& c : cases["cases"]) {
        std::string url = c.contains("url") ? c["url"].get<std::string>() : server + c["path"].get<std::string>();
        std::vector<std::pair<std::string, std::string>> headers;
        if (c.contains("headers")) {
            for (const auto& [name, value] : c["headers"].items()) headers.emplace_back(name, value.get<std::string>());
        }
        std::string body = c.contains("body") ? c["body"].get<std::string>() : std::string();
        HttpReply reply = http_call(method_of(c["method"].get<std::string>()), url, body, headers);
        if (c.contains("base")) {
            std::string base = c["base"].get<std::string>();
            if (auto mark = base.find("{server}"); mark != std::string::npos) base.replace(mark, 8, server);
            std::vector<std::pair<std::string, std::string>> query;
            if (c.contains("query")) {
                for (const auto& pair : c["query"]) query.emplace_back(pair[0].get<std::string>(), pair[1].get<std::string>());
            }
            std::string kind = c.contains("auth") ? c["auth"]["kind"].get<std::string>() : "";
            HttpAuth auth = kind == "bearer" ? HttpAuth::Bearer : kind == "api-key" ? HttpAuth::ApiKey : kind == "basic" ? HttpAuth::Basic : HttpAuth::None;
            std::string secret = c.contains("auth") ? c["auth"]["secret"].get<std::string>() : std::string();
            if (c.contains("paging")) {
                const auto& p = c["paging"];
                std::string word = p["kind"].get<std::string>();
                PagingKind paged = word == "page" ? PagingKind::Page : word == "offset" ? PagingKind::Offset : word == "cursor" ? PagingKind::Cursor : PagingKind::Link;
                HttpPaging how{paged, p["param"].get<std::string>(), p["size_param"].get<std::string>(), p["size"].get<int>(), p["next"].get<std::string>(), p["items"].get<std::string>()};
                reply = http_pages(base, c["path"].get<std::string>(), query, headers, auth, secret, how);
            } else {
                reply = http_call_at(method_of(c["method"].get<std::string>()), base, c["path"].get<std::string>(), query, body, headers, auth, secret);
            }
        }
        CHECK_EQ(reply.status, c["status"].get<int64_t>());
        CHECK_EQ(reply.ok, c["ok"].get<bool>());
        if (c.contains("text")) CHECK_EQ(reply.text, c["text"].get<std::string>());
        if (c.contains("text_starts")) CHECK(reply.text.rfind(c["text_starts"].get<std::string>(), 0) == 0);
    }
}

TEST(a_url_names_its_host) {
    CHECK_EQ(http_host("https://api.example.com/orders?x=1"), std::string("api.example.com"));
    CHECK_EQ(http_host("http://127.0.0.1:1/"), std::string("127.0.0.1"));
    CHECK_EQ(http_host("https://me:secret@example.com:8443"), std::string("example.com"));
    CHECK_EQ(http_host("http://[::1]:8080/x"), std::string("::1"));
}
