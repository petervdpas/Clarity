// The test server of conformance/http.json, for the C and C++ runtimes'
// tests: it answers the routes of the file on a free port of 127.0.0.1, one
// request at a time, on a thread of its own. Plain sockets, as C and C++
// both compile them; Windows through Winsock.

#pragma once

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
typedef SOCKET http_test_socket;
#define HTTP_TEST_CLOSE closesocket
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <pthread.h>
#include <sys/socket.h>
#include <unistd.h>
typedef int http_test_socket;
#define HTTP_TEST_CLOSE close
#endif

/// Reads up to the end of the request's headers, then its body by its
/// length; gives what was read, or 0 when the client went.
static size_t http_test_read(http_test_socket client, char* request, size_t room) {
    size_t got = 0;
    for (;;) {
        int n = (int)recv(client, request + got, (int)(room - 1 - got), 0);
        if (n <= 0) return got;
        got += (size_t)n;
        request[got] = '\0';
        const char* end = strstr(request, "\r\n\r\n");
        if (!end) continue;
        size_t length = 0;
        for (const char* line = request; line < end; line = strstr(line, "\r\n") + 2) {
            if (strncmp(line, "Content-Length:", 15) == 0 || strncmp(line, "content-length:", 15) == 0) length = (size_t)atol(line + 15);
        }
        if (got >= (size_t)(end + 4 - request) + length || got >= room - 1) return got;
    }
}

/// The value of header `name` in `request`, into `value`.
static void http_test_header(const char* request, const char* name, char* value, size_t room) {
    value[0] = '\0';
    const char* end = strstr(request, "\r\n\r\n");
    for (const char* line = strstr(request, "\r\n"); line && end && line < end; line = strstr(line + 2, "\r\n")) {
        const char* start = line + 2;
        size_t length = strlen(name);
        if (strncmp(start, name, length) == 0 && start[length] == ':') {
            const char* v = start + length + 1;
            while (*v == ' ') v++;
            size_t n = strcspn(v, "\r\n");
            if (n >= room) n = room - 1;
            memcpy(value, v, n);
            value[n] = '\0';
            return;
        }
    }
}

/// The server's own address, for the links it hands out.
static char http_test_address[64];

/// The number query parameter `name` holds in `query`, else `otherwise`.
static int http_test_number(const char* query, const char* name, int otherwise) {
    size_t length = strlen(name);
    for (const char* at = query; at && *at; at = strchr(at, '&') ? strchr(at, '&') + 1 : NULL) {
        if (strncmp(at, name, length) == 0 && at[length] == '=') return atoi(at + length + 1);
    }
    return otherwise;
}

/// The numbers from `from` up to `to`, and at most 5, as a JSON list, into `out`.
static void http_test_numbers(int from, int to, char* out, size_t room) {
    size_t used = (size_t)snprintf(out, room, "[");
    for (int n = from; n <= (to < 5 ? to : 5); n++) used += (size_t)snprintf(out + used, room - used, n == from ? "%d" : ",%d", n);
    snprintf(out + used, room - used, "]");
}

static void http_test_answer(http_test_socket client) {
    static char request[65536];
    size_t got = http_test_read(client, request, sizeof request);
    if (got == 0) return;
    char method[16] = {0}, path[256] = {0};
    sscanf(request, "%15s %255s", method, path);
    char* question = strchr(path, '?');
    char query[256] = {0};
    if (question) {
        snprintf(query, sizeof query, "%s", question + 1);
        *question = '\0';
    }
    const char* end = strstr(request, "\r\n\r\n");
    const char* body = end ? end + 4 : "";
    static char text[65536];
    char list[256];
    char link[256] = {0};
    int status = 200;
    int page = http_test_number(query, "page", 1);
    if (strcmp(path, "/paged") == 0) {
        int size = http_test_number(query, "size", 5);
        http_test_numbers((page - 1) * size + 1, page * size, text, sizeof text);
    } else if (strcmp(path, "/offset") == 0) {
        int offset = http_test_number(query, "offset", 0), limit = http_test_number(query, "limit", 5);
        http_test_numbers(offset + 1, offset + limit, list, sizeof list);
        snprintf(text, sizeof text, "{\"data\":%s}", list);
    } else if (strcmp(path, "/cursor") == 0) {
        int after = http_test_number(query, "after", 0);
        char next[32];
        if (after + 2 >= 5) snprintf(next, sizeof next, "null");
        else snprintf(next, sizeof next, "\"%d\"", after + 2);
        http_test_numbers(after + 1, after + 2, list, sizeof list);
        snprintf(text, sizeof text, "{\"data\":%s,\"meta\":{\"next\":%s}}", list, next);
    } else if (strcmp(path, "/linked") == 0) {
        if (page * 2 < 5) snprintf(link, sizeof link, "Link: <%s/linked?page=%d>; rel=\"next\"\r\n", http_test_address, page + 1);
        http_test_numbers((page - 1) * 2 + 1, page * 2, text, sizeof text);
    } else if (strcmp(path, "/broken") == 0) {
        if (page == 1) {
            snprintf(text, sizeof text, "[1,2]");
        } else {
            status = 500;
            snprintf(text, sizeof text, "down");
        }
    } else if (strcmp(path, "/endless") == 0) {
        snprintf(text, sizeof text, "[%d]", page);
    } else if (strcmp(path, "/hello") == 0) {
        snprintf(text, sizeof text, "hello");
    } else if (strcmp(path, "/echo") == 0) {
        snprintf(text, sizeof text, "%s %s", method, body);
    } else if (strcmp(path, "/header") == 0) {
        http_test_header(request, "X-Test", text, sizeof text);
    } else if (strcmp(path, "/query") == 0) {
        snprintf(text, sizeof text, "%s", query);
    } else if (strcmp(path, "/auth") == 0) {
        http_test_header(request, "Authorization", text, sizeof text);
        if (!text[0]) http_test_header(request, "X-Api-Key", text, sizeof text);
    } else {
        status = 404;
        snprintf(text, sizeof text, "no such page");
    }
    static char reply[70000];
    const char* reason = status == 200 ? "OK" : status == 500 ? "Internal Server Error" : "Not Found";
    int length = snprintf(reply, sizeof reply, "HTTP/1.1 %d %s\r\nContent-Type: text/plain; charset=utf-8\r\n%sContent-Length: %zu\r\nConnection: close\r\n\r\n%s",
                          status, reason, link, strlen(text), text);
    send(client, reply, length, 0);
}

#ifdef _WIN32
static DWORD WINAPI http_test_serve(LPVOID listening) {
#else
static void* http_test_serve(void* listening) {
#endif
    http_test_socket server = *(http_test_socket*)listening;
    for (;;) {
        http_test_socket client = accept(server, NULL, NULL);
#ifdef _WIN32
        if (client == INVALID_SOCKET) return 0;
#else
        if (client < 0) return NULL;
#endif
        http_test_answer(client);
        HTTP_TEST_CLOSE(client);
    }
}

/// Starts the server; gives its address, as `http://127.0.0.1:port`.
static const char* http_test_start(void) {
    static http_test_socket server;
    char* address = http_test_address;
#ifdef _WIN32
    WSADATA wsa;
    WSAStartup(MAKEWORD(2, 2), &wsa);
#endif
    server = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in at;
    memset(&at, 0, sizeof at);
    at.sin_family = AF_INET;
    at.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    at.sin_port = 0;
    bind(server, (struct sockaddr*)&at, sizeof at);
    listen(server, 8);
    socklen_t size = sizeof at;
    getsockname(server, (struct sockaddr*)&at, &size);
    snprintf(address, sizeof http_test_address, "http://127.0.0.1:%d", ntohs(at.sin_port));
#ifdef _WIN32
    CreateThread(NULL, 0, http_test_serve, &server, 0, NULL);
#else
    pthread_t thread;
    pthread_create(&thread, NULL, http_test_serve, &server);
    pthread_detach(thread);
#endif
    return address;
}
