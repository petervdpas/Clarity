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
    if (word == "DELETE") return HttpMethod::Delete;
    return HttpMethod::Get;
}

}  // namespace

TEST(every_http_call_gives_the_reply_the_cases_name) {
    json cases = conformance("http.json");
    std::string server = http_test_start();
    for (const auto& c : cases["cases"]) {
        std::string url = c.contains("url") ? c["url"].get<std::string>() : server + c["path"].get<std::string>();
        std::vector<std::pair<std::string, std::string>> headers;
        if (c.contains("headers")) {
            for (const auto& [name, value] : c["headers"].items()) headers.emplace_back(name, value.get<std::string>());
        }
        std::string body = c.contains("body") ? c["body"].get<std::string>() : std::string();
        HttpReply reply = http_call(method_of(c["method"].get<std::string>()), url, body, headers);
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
