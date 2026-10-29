// HTTP calls through libcurl; see http.h.

#include "http/http.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>

#include <curl/curl.h>

#include "clarity/core.h"
#include "clarity/picture.h"
#include "clarity/settings.h"
#include "cjson/cJSON.h"

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

/// The address a `Link` header line names `rel="next"`, in the scratch arena; NULL without one.
static const char* next_link(const char* line) {
    // Header names are read in any case; strncasecmp is not on every system.
    const char* name = "link:";
    for (size_t i = 0; i < 5; i++) {
        if (tolower((unsigned char)line[i]) != name[i]) return NULL;
    }
    const char* part = line + 5;
    while (part && *part) {
        const char* open = strchr(part, '<');
        const char* close = open ? strchr(open, '>') : NULL;
        if (!close) return NULL;
        const char* end = strchr(close, ',');
        size_t length = end ? (size_t)(end - close) : strlen(close);
        const char* params = clarity_copy_n(close, length);
        ClarityBuffer plain = {0};
        for (const char* c = params; *c; c++) {
            if (*c != ' ') clarity_buffer_add_char(&plain, (char)tolower((unsigned char)*c));
        }
        const char* text = clarity_buffer_text(&plain);
        if (strstr(text, ";rel=\"next\"") || strstr(text, ";rel=next")) return clarity_copy_n(open + 1, (size_t)(close - open - 1));
        part = end ? end + 1 : NULL;
    }
    return NULL;
}

/// Keeps the address a `Link` header names next, as curl hands each header line.
static size_t heard(char* data, size_t size, size_t count, void* into) {
    const char* line = clarity_copy_n(data, size * count);
    const char* found = next_link(line);
    if (found) *(const char**)into = found;
    return size * count;
}

/// A call as `clarity_http_call` makes it; `next` takes the address its
/// `Link` header names next, or NULL.
static ClarityHttpReply fetch(ClarityHttpMethod method, const char* url, const char* body, const ClarityHttpHeader* headers, size_t count, const char** next);

ClarityHttpReply clarity_http_call(ClarityHttpMethod method, const char* url, const char* body, const ClarityHttpHeader* headers, size_t count) {
    return fetch(method, url, body, headers, count, NULL);
}

static ClarityHttpReply fetch(ClarityHttpMethod method, const char* url, const char* body, const ClarityHttpHeader* headers, size_t count, const char** next) {
    static bool started;
    const char* linked = NULL;
    if (next) *next = NULL;
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
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, heard);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, &linked);
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
        case CLARITY_HTTP_PATCH:
            curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, method == CLARITY_HTTP_PUT ? "PUT" : "PATCH");
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
    if (next) *next = linked;
    return reply;
}

const char* clarity_url_part(const char* text) {
    ClarityBuffer out = {0};
    for (const unsigned char* c = (const unsigned char*)(text ? text : ""); *c; c++) {
        if (isalnum(*c) || *c == '-' || *c == '.' || *c == '_' || *c == '~') clarity_buffer_add_char(&out, (char)*c);
        else clarity_buffer_addf(&out, "%%%02X", *c);
    }
    return clarity_buffer_text(&out);
}

/// A secret where the project file says it is: `env:NAME` reads that
/// environment variable, `setting:<program>/<name>` a secret setting from
/// the system's protected store (DESIGN 5.27); empty when it is not there.
static const char* secret_at(const char* location) {
    if (!location) return "";
    if (strncmp(location, "env:", 4) == 0) {
        const char* value = getenv(location + 4);
        return value ? value : "";
    }
    if (strncmp(location, "setting:", 8) == 0) {
        const char* at = location + 8;
        const char* slash = strchr(at, '/');
        if (!slash || !clarity_secret_store.get) return "";
        const char* value = clarity_secret_store.get(clarity_copy_n(at, (size_t)(slash - at)), slash + 1);
        return value ? value : "";
    }
    return location;
}

/// A call as `clarity_http_call_at` makes it; `next` takes the address its
/// `Link` header names next, or NULL.
static ClarityHttpReply fetch_at(ClarityHttpMethod method, const char* base, const char* path, const ClarityHttpHeader* query, size_t query_count, const char* body,
                                 const ClarityHttpHeader* headers, size_t count, ClarityAuthKind auth, const char* secret, const char** next);

ClarityHttpReply clarity_http_call_at(ClarityHttpMethod method, const char* base, const char* path, const ClarityHttpHeader* query, size_t query_count, const char* body,
                                      const ClarityHttpHeader* headers, size_t count, ClarityAuthKind auth, const char* secret) {
    return fetch_at(method, base, path, query, query_count, body, headers, count, auth, secret, NULL);
}

/// The JSON at a dotted path of `value`: `meta.next`; `value` itself for "".
static const cJSON* json_at(const cJSON* value, const char* path) {
    if (!path || !*path) return value;
    const char* step = path;
    while (value && step) {
        const char* dot = strchr(step, '.');
        const char* name = dot ? clarity_copy_n(step, (size_t)(dot - step)) : step;
        value = cJSON_IsObject(value) ? cJSON_GetObjectItemCaseSensitive(value, name) : NULL;
        step = dot ? dot + 1 : NULL;
    }
    return value;
}

ClarityHttpReply clarity_http_pages(const char* base, const char* path, const ClarityHttpHeader* query, size_t query_count, const ClarityHttpHeader* headers,
                                    size_t count, ClarityAuthKind auth, const char* secret, ClarityHttpPaging paging) {
    ClarityBuffer items = {0};
    clarity_buffer_add_char(&items, '[');
    bool first = true;
    const char* cursor = NULL;
    const char* link = NULL;
    for (int n = 0; n < CLARITY_HTTP_MOST_PAGES; n++) {
        ClarityHttpHeader* sent = CLARITY_NEW(ClarityHttpHeader, query_count + 2);
        size_t total = 0;
        for (size_t i = 0; i < query_count; i++) sent[total++] = query[i];
        switch (paging.kind) {
            case CLARITY_PAGING_PAGE:
                sent[total++] = (ClarityHttpHeader){paging.param, clarity_format("%d", n + 1)};
                sent[total++] = (ClarityHttpHeader){paging.size_param, clarity_format("%d", paging.size)};
                break;
            case CLARITY_PAGING_OFFSET:
                sent[total++] = (ClarityHttpHeader){paging.param, clarity_format("%d", n * paging.size)};
                sent[total++] = (ClarityHttpHeader){paging.size_param, clarity_format("%d", paging.size)};
                break;
            case CLARITY_PAGING_CURSOR:
                if (cursor) sent[total++] = (ClarityHttpHeader){paging.param, cursor};
                break;
            case CLARITY_PAGING_LINK:
                break;
        }
        const char* next = NULL;
        // A next link is a whole address, with its own query.
        ClarityHttpReply reply = link ? fetch_at(CLARITY_HTTP_GET, "", link, NULL, 0, NULL, headers, count, auth, secret, &next)
                                      : fetch_at(CLARITY_HTTP_GET, base, path, sent, total, NULL, headers, count, auth, secret, &next);
        if (!reply.ok) return reply;
        cJSON* page = cJSON_Parse(reply.text);
        const cJSON* list = page ? json_at(page, paging.items) : NULL;
        if (!cJSON_IsArray(list)) {
            cJSON_Delete(page);
            return none(clarity_format("page %d of %s has no list at %s", n + 1, path, paging.items && *paging.items ? paging.items : "the top"));
        }
        int got = 0;
        const cJSON* item;
        cJSON_ArrayForEach(item, list) {
            char* text = cJSON_PrintUnformatted(item);
            clarity_buffer_addf(&items, first ? "%s" : ",%s", text ? text : "null");
            cJSON_free(text);
            first = false;
            got++;
        }
        bool done = false;
        switch (paging.kind) {
            case CLARITY_PAGING_PAGE:
            case CLARITY_PAGING_OFFSET:
                done = got < paging.size;
                break;
            case CLARITY_PAGING_CURSOR: {
                const cJSON* at = json_at(page, paging.next);
                cursor = NULL;
                if (cJSON_IsString(at) && at->valuestring[0]) cursor = clarity_copy_n(at->valuestring, strlen(at->valuestring));
                else if (cJSON_IsNumber(at)) {
                    char* digits = cJSON_PrintUnformatted(at);
                    cursor = clarity_copy_n(digits, strlen(digits));
                    cJSON_free(digits);
                }
                done = cursor == NULL;
                break;
            }
            case CLARITY_PAGING_LINK:
                link = next;
                done = link == NULL;
                break;
        }
        cJSON_Delete(page);
        if (done) {
            clarity_buffer_add_char(&items, ']');
            ClarityHttpReply all = {reply.status, true, clarity_buffer_text(&items)};
            return all;
        }
    }
    return none(clarity_format("stopped after %d pages of %s", CLARITY_HTTP_MOST_PAGES, path));
}

static ClarityHttpReply fetch_at(ClarityHttpMethod method, const char* base, const char* path, const ClarityHttpHeader* query, size_t query_count, const char* body,
                                 const ClarityHttpHeader* headers, size_t count, ClarityAuthKind auth, const char* secret, const char** next) {
    ClarityBuffer url = {0};
    path = path ? path : "";
    if (base && *base && !strstr(path, "://")) {
        size_t length = strlen(base);
        while (length && base[length - 1] == '/') length--;
        clarity_buffer_add_n(&url, base, length);
        clarity_buffer_add_char(&url, '/');
        while (*path == '/') path++;
    }
    clarity_buffer_add(&url, path);
    for (size_t i = 0; i < query_count; i++) {
        bool first = i == 0 && !strchr(clarity_buffer_text(&url), '?');
        clarity_buffer_addf(&url, "%c%s=%s", first ? '?' : '&', clarity_url_part(query[i].name), clarity_url_part(query[i].value));
    }
    ClarityHttpHeader* all = CLARITY_NEW(ClarityHttpHeader, count + 1);
    for (size_t i = 0; i < count; i++) all[i] = headers[i];
    size_t total = count;
    const char* key = secret_at(secret);
    switch (auth) {
        case CLARITY_AUTH_BEARER:
            all[total++] = (ClarityHttpHeader){"Authorization", clarity_format("Bearer %s", key)};
            break;
        case CLARITY_AUTH_API_KEY:
            all[total++] = (ClarityHttpHeader){"X-Api-Key", key};
            break;
        case CLARITY_AUTH_BASIC: {
            ClarityBytes bytes = {(const uint8_t*)key, strlen(key)};
            all[total++] = (ClarityHttpHeader){"Authorization", clarity_format("Basic %s", clarity_base64(bytes))};
            break;
        }
        case CLARITY_AUTH_NONE:
            break;
    }
    return fetch(method, clarity_buffer_text(&url), body, all, total, next);
}
