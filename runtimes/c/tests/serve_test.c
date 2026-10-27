// The web target (DESIGN 7.3): the shell as a web server, which serves the
// renderer's page to a browser and answers its commands over HTTP with the
// same service the desktop window binds into its webview.

#include <stdlib.h>

#include "check.h"
#include "shell/serve.h"
#include "tests.h"

static const ClarityWindows nothing = {0};

static const char* answered(ClarityService* s, const char* page, const char* request) {
    size_t length = 0;
    const char* reply = clarity_serve_answer(s, page, request, strlen(request), &length);
    return reply && strlen(reply) == length ? reply : "";
}

static const char* post(const char* body) {
    return clarity_format("POST /clarity/invoke HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: %zu\r\n\r\n%s", strlen(body), body);
}

void the_server_serves_the_page_and_answers_its_commands(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    const char* page = "<html><head><title>Shop</title></head><body></body></html>";
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", page, db, &nothing};
    ClarityService* s = clarity_service_new(&options);
    // The page says where it sends its commands.
    const char* got = answered(s, page, "GET / HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(strncmp(got, "HTTP/1.1 200 OK\r\n", 17) == 0);
    CHECK(strstr(got, "Content-Type: text/html; charset=utf-8\r\n") != NULL);
    CHECK(strstr(got, "<script>window.clarityServer = \"/clarity/invoke\";</script></head>") != NULL);
    // A command and its arguments, answered as the desktop shell answers them.
    got = answered(s, page, post("{\"command\": \"start\", \"args\": {}}"));
    CHECK(strncmp(got, "HTTP/1.1 200 OK\r\n", 17) == 0);
    CHECK(strstr(got, "\r\n\r\n{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\"}}") != NULL);
    got = answered(s, page, post("{\"command\": \"nonsense\", \"args\": {}}"));
    CHECK(strstr(got, "\r\n\r\n{\"error\":\"there is no command nonsense\"}") != NULL);
    got = answered(s, page, post("not json"));
    CHECK(strstr(got, "\r\n\r\n{\"error\":") != NULL);
    // Nothing else is there.
    got = answered(s, page, "GET /secret HTTP/1.1\r\nHost: x\r\n\r\n");
    CHECK(strncmp(got, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
    clarity_service_free(s);
    clarity_db_close(db);
}

void the_server_listens_where_it_is_told(void) {
    char* none[] = {"app"};
    CHECK(strcmp(clarity_serve_address(1, none), "127.0.0.1:8080") == 0 || getenv("CLARITY_ADDRESS"));
    char* told[] = {"app", "--address", "0.0.0.0:9000"};
    CHECK(strcmp(clarity_serve_address(3, told), "0.0.0.0:9000") == 0);
}
