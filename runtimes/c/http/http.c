// HTTP calls through libcurl; see http.h.

#include "http/http.h"

#include <ctype.h>
#include <string.h>

#include <curl/curl.h>

#include "clarity/core.h"

static size_t keep(char* data, size_t size, size_t count, void* into) {
    clarity_buffer_add_n((ClarityBuffer*)into, data, size * count);
    return size * count;
}

static ClarityHttpReply none(const char* why) {
    ClarityHttpReply reply = {0, false, why};
    return reply;
}

/// Whether `url` is an http or https address, whatever the case of its scheme.
static bool called(const char* url) {
    const char* end = strstr(url, "://");
    if (!end) return false;
    size_t length = (size_t)(end - url);
    char scheme[6] = {0};
    if (length == 0 || length >= sizeof scheme) return false;
    for (size_t i = 0; i < length; i++) scheme[i] = (char)tolower((unsigned char)url[i]);
    return strcmp(scheme, "http") == 0 || strcmp(scheme, "https") == 0;
}

const char* clarity_http_host(const char* url) {
    const char* rest = strstr(url, "://");
    rest = rest ? rest + 3 : url;
    size_t length = strcspn(rest, "/?#");
    const char* authority = clarity_copy_n(rest, length);
    const char* at = strrchr(authority, '@');
    const char* host = at ? at + 1 : authority;
    if (host[0] == '[') {
        const char* close = strchr(host, ']');
        return clarity_copy_n(host + 1, close ? (size_t)(close - host - 1) : strlen(host + 1));
    }
    return clarity_copy_n(host, strcspn(host, ":"));
}

ClarityHttpReply clarity_http_call(ClarityHttpMethod method, const char* url, const char* body, const ClarityHttpHeader* headers, size_t count) {
    static bool started;
    if (!started) {
        curl_global_init(CURL_GLOBAL_DEFAULT);
        started = true;
    }
    if (!url || !called(url)) return none(clarity_format("only http and https addresses are called, not %s", url ? url : ""));
    CURL* curl = curl_easy_init();
    if (!curl) return none(clarity_format("cannot reach %s: libcurl did not start", clarity_http_host(url)));
    struct curl_slist* sent = NULL;
    for (size_t i = 0; i < count; i++) sent = curl_slist_append(sent, clarity_format("%s: %s", headers[i].name, headers[i].value));
    ClarityBuffer answer = {0};
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, CLARITY_HTTP_TIMEOUT);
    curl_easy_setopt(curl, CURLOPT_NOSIGNAL, 1L);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, keep);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &answer);
    if (sent) curl_easy_setopt(curl, CURLOPT_HTTPHEADER, sent);
    const char* sending = body ? body : "";
    switch (method) {
        case CLARITY_HTTP_GET:
            break;
        case CLARITY_HTTP_POST:
            curl_easy_setopt(curl, CURLOPT_POST, 1L);
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, sending);
            curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)strlen(sending));
            break;
        case CLARITY_HTTP_PUT:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "PUT");
            curl_easy_setopt(curl, CURLOPT_POSTFIELDS, sending);
            curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)strlen(sending));
            break;
        case CLARITY_HTTP_DELETE:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "DELETE");
            break;
    }
    CURLcode done = curl_easy_perform(curl);
    long status = 0;
    if (done == CURLE_OK) curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    curl_slist_free_all(sent);
    curl_easy_cleanup(curl);
    if (done != CURLE_OK) return none(clarity_format("cannot reach %s: %s", clarity_http_host(url), curl_easy_strerror(done)));
    ClarityHttpReply reply = {status, status >= 200 && status < 300, clarity_buffer_text(&answer)};
    return reply;
}
