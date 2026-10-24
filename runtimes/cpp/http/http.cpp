// HTTP calls through libcurl; see http.hpp.

#include "http/http.hpp"

#include <algorithm>
#include <cctype>

#include <curl/curl.h>

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

}  // namespace clarity
