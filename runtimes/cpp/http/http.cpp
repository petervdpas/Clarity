// HTTP calls through libcurl; see http.hpp.

#include "http/http.hpp"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <cstdlib>

#include <curl/curl.h>

#include "clarity/settings.hpp"

namespace clarity {

namespace {

size_t keep(char* data, size_t size, size_t count, void* into) {
    static_cast<std::string*>(into)->append(data, size * count);
    return size * count;
}

HttpReply none(std::string why) {
    return HttpReply{0, false, std::move(why)};
}

/// Whether `url` is an http or https address, whatever the case of its scheme.
bool called(const std::string& url) {
    auto end = url.find("://");
    if (end == std::string::npos) return false;
    std::string scheme = url.substr(0, end);
    std::transform(scheme.begin(), scheme.end(), scheme.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return scheme == "http" || scheme == "https";
}

/// libcurl readied once for the program.
struct Started {
    Started() { curl_global_init(CURL_GLOBAL_DEFAULT); }
};

}  // namespace

std::string http_host(const std::string& url) {
    auto scheme = url.find("://");
    std::string rest = scheme == std::string::npos ? url : url.substr(scheme + 3);
    std::string authority = rest.substr(0, rest.find_first_of("/?#"));
    auto at = authority.rfind('@');
    std::string host = at == std::string::npos ? authority : authority.substr(at + 1);
    if (!host.empty() && host[0] == '[') return host.substr(1, host.find(']') - 1);
    return host.substr(0, host.find(':'));
}

HttpReply http_call(HttpMethod method, const std::string& url, const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers) {
    static Started started;
    if (!called(url)) return none("only http and https addresses are called, not " + url);
    CURL* curl = curl_easy_init();
    if (!curl) return none("cannot reach " + http_host(url) + ": libcurl did not start");
    curl_slist* sent = nullptr;
    for (const auto& [name, value] : headers) sent = curl_slist_append(sent, (name + ": " + value).c_str());
    std::string answer;
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, HTTP_TIMEOUT);
    curl_easy_setopt(curl, CURLOPT_NOSIGNAL, 1L);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, keep);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &answer);
    if (sent) curl_easy_setopt(curl, CURLOPT_HTTPHEADER, sent);
    switch (method) {
        case HttpMethod::Get:
            break;
        case HttpMethod::Post:
            curl_easy_setopt(curl, CURLOPT_POST, 1L);
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());
            curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, static_cast<long>(body.size()));
            break;
        case HttpMethod::Put:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "PUT");
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());
            curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, static_cast<long>(body.size()));
            break;
        case HttpMethod::Delete:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "DELETE");
            break;
    }
    CURLcode done = curl_easy_perform(curl);
    long status = 0;
    if (done == CURLE_OK) curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    curl_slist_free_all(sent);
    curl_easy_cleanup(curl);
    if (done != CURLE_OK) return none("cannot reach " + http_host(url) + ": " + curl_easy_strerror(done));
    return HttpReply{status, status >= 200 && status < 300, std::move(answer)};
}

std::string url_part(const std::string& text) {
    std::string out;
    for (unsigned char c : text) {
        if (std::isalnum(c) || c == '-' || c == '.' || c == '_' || c == '~') {
            out += static_cast<char>(c);
        } else {
            char hex[4];
            std::snprintf(hex, sizeof hex, "%%%02X", c);
            out += hex;
        }
    }
    return out;
}

namespace {

/// A secret where the project file says it is: `env:NAME` reads that
/// environment variable, `setting:<program>/<name>` a secret setting from
/// the system's protected store (DESIGN 5.27); empty when it is not there.
std::string secret_at(const std::string& location) {
    if (location.rfind("env:", 0) == 0) {
        const char* value = std::getenv(location.substr(4).c_str());
        return value ? value : "";
    }
    if (location.rfind("setting:", 0) == 0) {
        auto at = location.substr(8);
        auto slash = at.find('/');
        if (slash == std::string::npos || !secret_store.get) return "";
        return secret_store.get(at.substr(0, slash), at.substr(slash + 1)).value_or("");
    }
    return location;
}

std::string base64_of(const std::string& text) {
    static const char* digits = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::string out;
    size_t i = 0;
    for (; i + 2 < text.size(); i += 3) {
        uint32_t n = (uint32_t(uint8_t(text[i])) << 16) | (uint32_t(uint8_t(text[i + 1])) << 8) | uint8_t(text[i + 2]);
        out += {digits[n >> 18], digits[(n >> 12) & 63], digits[(n >> 6) & 63], digits[n & 63]};
    }
    if (i + 1 == text.size()) {
        uint32_t n = uint32_t(uint8_t(text[i])) << 16;
        out += {digits[n >> 18], digits[(n >> 12) & 63], '=', '='};
    } else if (i + 2 == text.size()) {
        uint32_t n = (uint32_t(uint8_t(text[i])) << 16) | (uint32_t(uint8_t(text[i + 1])) << 8);
        out += {digits[n >> 18], digits[(n >> 12) & 63], digits[(n >> 6) & 63], '='};
    }
    return out;
}

}  // namespace

HttpReply http_call_at(HttpMethod method, const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                       const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret) {
    std::string url = path;
    if (!base.empty() && path.find("://") == std::string::npos) {
        std::string trimmed = base;
        while (!trimmed.empty() && trimmed.back() == '/') trimmed.pop_back();
        url = trimmed + "/" + path.substr(path.find_first_not_of('/') == std::string::npos ? path.size() : path.find_first_not_of('/'));
    }
    for (size_t i = 0; i < query.size(); ++i) {
        url += (i == 0 && url.find('?') == std::string::npos ? "?" : "&") + url_part(query[i].first) + "=" + url_part(query[i].second);
    }
    auto all = headers;
    switch (auth) {
        case HttpAuth::Bearer: all.emplace_back("Authorization", "Bearer " + secret_at(secret)); break;
        case HttpAuth::ApiKey: all.emplace_back("X-Api-Key", secret_at(secret)); break;
        case HttpAuth::Basic: all.emplace_back("Authorization", "Basic " + base64_of(secret_at(secret))); break;
        case HttpAuth::None: break;
    }
    return http_call(method, url, body, all);
}

}  // namespace clarity
