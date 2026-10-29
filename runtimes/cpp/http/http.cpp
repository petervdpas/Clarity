// HTTP calls through libcurl; see http.hpp.

#include "http/http.hpp"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <cstdlib>

#include <curl/curl.h>
#include <nlohmann/json.hpp>

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

namespace {

/// The address a `Link` header line names `rel="next"`; empty without one.
std::string next_link(std::string line) {
    std::string lower = line;
    std::transform(lower.begin(), lower.end(), lower.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    if (lower.rfind("link:", 0) != 0) return "";
    size_t at = 5;
    while (at < line.size()) {
        auto open = line.find('<', at);
        auto close = open == std::string::npos ? std::string::npos : line.find('>', open);
        if (close == std::string::npos) return "";
        auto end = line.find(',', close);
        std::string params;
        for (char c : lower.substr(close, end == std::string::npos ? std::string::npos : end - close)) {
            if (c != ' ') params += c;
        }
        if (params.find(";rel=\"next\"") != std::string::npos || params.find(";rel=next") != std::string::npos) return line.substr(open + 1, close - open - 1);
        if (end == std::string::npos) return "";
        at = end + 1;
    }
    return "";
}

/// Keeps the address a `Link` header names next, as curl hands each header line.
size_t heard(char* data, size_t size, size_t count, void* into) {
    auto found = next_link(std::string(data, size * count));
    if (!found.empty()) *static_cast<std::string*>(into) = found;
    return size * count;
}

/// A call as `http_call` makes it; `next` takes the address its `Link`
/// header names next, or empty.
HttpReply fetch(HttpMethod method, const std::string& url, const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, std::string* next);

}  // namespace

HttpReply http_call(HttpMethod method, const std::string& url, const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers) {
    return fetch(method, url, body, headers, nullptr);
}

namespace {

HttpReply fetch(HttpMethod method, const std::string& url, const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, std::string* next) {
    static Started started;
    std::string linked;
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
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, heard);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, &linked);
    switch (method) {
        case HttpMethod::Get:
            break;
        case HttpMethod::Post:
            curl_easy_setopt(curl, CURLOPT_POST, 1L);
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());
            curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, static_cast<long>(body.size()));
            break;
        case HttpMethod::Put:
        case HttpMethod::Patch:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, method == HttpMethod::Put ? "PUT" : "PATCH");
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
    if (next) *next = linked;
    return HttpReply{status, status >= 200 && status < 300, std::move(answer)};
}

}  // namespace

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

namespace {

/// A call as `http_call_at` makes it; `next` takes the address its `Link`
/// header names next, or empty.
HttpReply fetch_at(HttpMethod method, const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                   const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret, std::string* next);

/// The JSON at a dotted path of `value`: `meta.next`; `value` itself for "".
const nlohmann::json* json_at(const nlohmann::json& value, const std::string& path) {
    const nlohmann::json* at = &value;
    size_t start = 0;
    while (!path.empty() && start <= path.size()) {
        auto dot = path.find('.', start);
        auto step = path.substr(start, dot == std::string::npos ? std::string::npos : dot - start);
        if (!at->is_object() || !at->contains(step)) return nullptr;
        at = &(*at)[step];
        if (dot == std::string::npos) break;
        start = dot + 1;
    }
    return at;
}

}  // namespace

HttpReply http_call_at(HttpMethod method, const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                       const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret) {
    return fetch_at(method, base, path, query, body, headers, auth, secret, nullptr);
}

HttpReply http_pages(const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                     const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret, const HttpPaging& paging) {
    std::vector<std::string> items;
    std::optional<std::string> cursor;
    std::string link;
    for (int n = 0; n < HTTP_MOST_PAGES; ++n) {
        auto sent = query;
        switch (paging.kind) {
            case PagingKind::Page:
                sent.emplace_back(paging.param, std::to_string(n + 1));
                sent.emplace_back(paging.size_param, std::to_string(paging.size));
                break;
            case PagingKind::Offset:
                sent.emplace_back(paging.param, std::to_string(n * paging.size));
                sent.emplace_back(paging.size_param, std::to_string(paging.size));
                break;
            case PagingKind::Cursor:
                if (cursor) sent.emplace_back(paging.param, *cursor);
                break;
            case PagingKind::Link:
                break;
        }
        std::string next;
        // A next link is a whole address, with its own query.
        HttpReply reply = !link.empty() ? fetch_at(HttpMethod::Get, "", link, {}, std::string(), headers, auth, secret, &next)
                                        : fetch_at(HttpMethod::Get, base, path, sent, std::string(), headers, auth, secret, &next);
        if (!reply.ok) return reply;
        auto page = nlohmann::json::parse(reply.text, nullptr, false);
        const nlohmann::json* list = page.is_discarded() ? nullptr : json_at(page, paging.items);
        if (!list || !list->is_array()) {
            return none("page " + std::to_string(n + 1) + " of " + path + " has no list at " + (paging.items.empty() ? std::string("the top") : paging.items));
        }
        for (const auto& item : *list) items.push_back(item.dump());
        bool done = false;
        switch (paging.kind) {
            case PagingKind::Page:
            case PagingKind::Offset:
                done = static_cast<int>(list->size()) < paging.size;
                break;
            case PagingKind::Cursor: {
                const nlohmann::json* at = json_at(page, paging.next);
                cursor.reset();
                if (at && at->is_string() && !at->get<std::string>().empty()) cursor = at->get<std::string>();
                else if (at && at->is_number()) cursor = at->dump();
                done = !cursor;
                break;
            }
            case PagingKind::Link:
                link = next;
                done = link.empty();
                break;
        }
        if (done) {
            std::string text = "[";
            for (size_t i = 0; i < items.size(); ++i) text += (i ? "," : "") + items[i];
            return HttpReply{reply.status, true, text + "]"};
        }
    }
    return none("stopped after " + std::to_string(HTTP_MOST_PAGES) + " pages of " + path);
}

namespace {

HttpReply fetch_at(HttpMethod method, const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                   const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret, std::string* next) {
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
    return fetch(method, url, body, all, next);
}

}  // namespace

}  // namespace clarity
