// The web target (DESIGN 7.3): see serve.h. Plain sockets, as on every OS;
// Windows through Winsock.

#include "shell/serve.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
typedef SOCKET serve_socket;
#define SERVE_CLOSE closesocket
#define SERVE_INVALID INVALID_SOCKET
#else
#include <netdb.h>
#include <sys/socket.h>
#include <unistd.h>
typedef int serve_socket;
#define SERVE_CLOSE close
#define SERVE_INVALID (-1)
#endif

/// The largest request the server reads: a command with a file in it.
#define LARGEST_REQUEST (64u * 1024u * 1024u)

static const char* reply(const char* status, const char* type, const char* body, size_t body_length, size_t* length) {
    const char* head = clarity_format("HTTP/1.1 %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\n\r\n", status, type, body_length);
    ClarityBuffer out = {0};
    clarity_buffer_add_n(&out, head, strlen(head));
    clarity_buffer_add_n(&out, body, body_length);
    *length = out.length;
    return clarity_buffer_text(&out);
}

/// The page as the server gives it: it says where its commands go.
static const char* served(const char* page) {
    const char* head = strstr(page, "</head>");
    if (!head) return page;
    return clarity_format("%.*s<script>window.clarityServer = \"%s\";</script>%s", (int)(head - page), page, CLARITY_INVOKE_PATH, head);
}

const char* clarity_serve_answer(ClarityService* service, const char* page, const char* request, size_t request_length, size_t* length) {
    const char* end = strstr(request, "\r\n\r\n");
    char method[8] = {0}, path[256] = {0};
    if (!end || sscanf(request, "%7s %255s", method, path) != 2) return reply("400 Bad Request", "text/plain", "", 0, length);
    if (strcmp(path, "/") == 0 || strcmp(path, "/index.html") == 0) {
        const char* body = served(page);
        return reply("200 OK", "text/html; charset=utf-8", body, strlen(body), length);
    }
    if (strcmp(path, CLARITY_INVOKE_PATH) != 0) return reply("404 Not Found", "text/plain", "", 0, length);
    if (strcmp(method, "POST") != 0) {
        const char* body = "send a command with POST";
        return reply("405 Method Not Allowed", "text/plain", body, strlen(body), length);
    }
    const char* body = end + 4;
    cJSON* sent = cJSON_ParseWithLength(body, request_length - (size_t)(body - request));
    const cJSON* command = cJSON_GetObjectItem(sent, "command");
    const cJSON* args = cJSON_GetObjectItem(sent, "args");
    cJSON* answer;
    if (cJSON_IsString(command)) {
        cJSON* empty = cJSON_CreateObject();
        answer = clarity_service_invoke(service, command->valuestring, cJSON_IsObject(args) ? args : empty, NULL);
    } else {
        answer = cJSON_CreateObject();
        cJSON_AddStringToObject(answer, "error", "a command is JSON with its command and its args");
    }
    // cJSON takes its memory from the scratch arena, like the rest of the command.
    const char* printed = cJSON_PrintUnformatted(answer);
    return reply("200 OK", "application/json", printed, strlen(printed), length);
}

const char* clarity_serve_address(int argc, char** argv, const char* project) {
    for (int i = 1; i + 1 < argc; i++)
        if (strcmp(argv[i], "--address") == 0) return argv[i + 1];
    const char* address = getenv("CLARITY_ADDRESS");
    return address && *address ? address : project;
}

/// Reads one request: its headers, then its body by its length. NULL when the client went.
static char* read_request(serve_socket client, size_t* got) {
    size_t room = 64 * 1024;
    char* request = malloc(room);
    *got = 0;
    while (request) {
        if (*got + 1 >= room) {
            if (room >= LARGEST_REQUEST) break;
            room *= 2;
            char* bigger = realloc(request, room);
            if (!bigger) break;
            request = bigger;
        }
        int n = (int)recv(client, request + *got, (int)(room - 1 - *got), 0);
        if (n <= 0) break;
        *got += (size_t)n;
        request[*got] = '\0';
        const char* end = strstr(request, "\r\n\r\n");
        if (!end) continue;
        size_t length = 0;
        for (const char* line = request; line && line < end; line = strstr(line, "\r\n"), line = line ? line + 2 : NULL) {
            if (strncmp(line, "Content-Length:", 15) == 0 || strncmp(line, "content-length:", 15) == 0) length = (size_t)strtoul(line + 15, NULL, 10);
        }
        if (*got >= (size_t)(end + 4 - request) + length) return request;
    }
    free(request);
    return NULL;
}

static void send_all(serve_socket client, const char* text, size_t length) {
    while (length > 0) {
        int n = (int)send(client, text, (int)(length > 1 << 20 ? 1 << 20 : length), 0);
        if (n <= 0) return;
        text += n;
        length -= (size_t)n;
    }
}

int clarity_serve(const ClarityShellOptions* options, const char* address) {
#ifdef _WIN32
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) return 1;
#endif
    const char* colon = strrchr(address, ':');
    if (!colon) {
        fprintf(stderr, "%s: an address is host:port, not %s\n", options->name, address);
        return 1;
    }
    const char* host = clarity_format("%.*s", (int)(colon - address), address);
    struct addrinfo hints = {0}, *found = NULL;
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_PASSIVE;
    if (getaddrinfo(*host ? host : NULL, colon + 1, &hints, &found) != 0 || !found) {
        fprintf(stderr, "%s: cannot listen on %s\n", options->name, address);
        return 1;
    }
    serve_socket listener = socket(found->ai_family, found->ai_socktype, found->ai_protocol);
    int yes = 1;
    setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, (const char*)&yes, sizeof yes);
    if (listener == SERVE_INVALID || bind(listener, found->ai_addr, (int)found->ai_addrlen) != 0 || listen(listener, 16) != 0) {
        fprintf(stderr, "%s: cannot listen on %s\n", options->name, address);
        freeaddrinfo(found);
        return 1;
    }
    freeaddrinfo(found);
    ClarityService* service = clarity_service_new(options);
    fprintf(stderr, "%s: serving on http://%s\n", options->name, address);
    for (;;) {
        serve_socket client = accept(listener, NULL, NULL);
        if (client == SERVE_INVALID) continue;
        size_t got = 0;
        char* request = read_request(client, &got);
        if (request) {
            size_t length = 0;
            const char* answer = clarity_serve_answer(service, options->html, request, got, &length);
            send_all(client, answer, length);
            free(request);
        }
        SERVE_CLOSE(client);
        // The request is answered: its values are done with.
        clarity_scratch_reset();
    }
}
