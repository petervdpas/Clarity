// HTTP calls (DESIGN 5.23) against the cases every runtime shares
// (conformance/http.json), on the test server of http_server.h.

#include "check.h"
#include "http/http.h"
#include "http_server.h"
#include "tests.h"

static ClarityHttpMethod method_of(const char* word) {
    if (strcmp(word, "POST") == 0) return CLARITY_HTTP_POST;
    if (strcmp(word, "PUT") == 0) return CLARITY_HTTP_PUT;
    if (strcmp(word, "DELETE") == 0) return CLARITY_HTTP_DELETE;
    return CLARITY_HTTP_GET;
}

void every_http_call_gives_the_reply_the_cases_name(void) {
    cJSON* cases = conformance("http.json");
    REQUIRE(cases);
    const char* server = http_test_start();
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "cases")) {
        const cJSON* url = cJSON_GetObjectItem(c, "url");
        const char* called = cJSON_IsString(url) ? url->valuestring : clarity_format("%s%s", server, cJSON_GetObjectItem(c, "path")->valuestring);
        const cJSON* body = cJSON_GetObjectItem(c, "body");
        ClarityHttpHeader headers[4];
        size_t count = 0;
        const cJSON* header;
        cJSON_ArrayForEach(header, cJSON_GetObjectItem(c, "headers")) {
            if (count < 4) headers[count++] = (ClarityHttpHeader){header->string, header->valuestring};
        }
        ClarityHttpReply reply = clarity_http_call(method_of(cJSON_GetObjectItem(c, "method")->valuestring), called, cJSON_IsString(body) ? body->valuestring : NULL, headers, count);
        CHECK_INT(reply.status, (int64_t)cJSON_GetObjectItem(c, "status")->valuedouble);
        CHECK(reply.ok == cJSON_IsTrue(cJSON_GetObjectItem(c, "ok")));
        const cJSON* text = cJSON_GetObjectItem(c, "text");
        if (cJSON_IsString(text)) CHECK_TEXT(reply.text, text->valuestring);
        const cJSON* starts = cJSON_GetObjectItem(c, "text_starts");
        if (cJSON_IsString(starts)) CHECK(strncmp(reply.text, starts->valuestring, strlen(starts->valuestring)) == 0);
    }
    cJSON_Delete(cases);
}

void a_url_names_its_host(void) {
    CHECK_TEXT(clarity_http_host("https://api.example.com/orders?x=1"), "api.example.com");
    CHECK_TEXT(clarity_http_host("http://127.0.0.1:1/"), "127.0.0.1");
    CHECK_TEXT(clarity_http_host("https://me:secret@example.com:8443"), "example.com");
    CHECK_TEXT(clarity_http_host("http://[::1]:8080/x"), "::1");
}
