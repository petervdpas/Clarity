// The web target (DESIGN 7.3): see serve.h. Plain sockets, as on every OS;
// Windows through Winsock.

#include "service/serve.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "clarity/text.h"

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
typedef SOCKET serve_socket;
#define SERVE_CLOSE closesocket
#define SERVE_INVALID INVALID_SOCKET
#else
#include <netdb.h>
#include <pthread.h>
#include <sys/socket.h>
#include <unistd.h>
typedef int serve_socket;
#define SERVE_CLOSE close
#define SERVE_INVALID (-1)
#endif

/// The largest request the server reads: a command with a file in it.
#define LARGEST_REQUEST (64u * 1024u * 1024u)

static const char* reply(const char* status, const char* type, const char* body, size_t body_length, size_t* length) {
    const char* head = clarity_format("HTTP/1.1 %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n", status, type, body_length);
    ClarityBuffer out = {0};
    clarity_buffer_add_n(&out, head, strlen(head));
    clarity_buffer_add_n(&out, body, body_length);
    *length = out.length;
    return clarity_buffer_text(&out);
}

static bool ends_with(const char* text, const char* end) {
    size_t a = strlen(text), b = strlen(end);
    return a >= b && strcmp(text + a - b, end) == 0;
}

/// The Content-Type of a file by its name.
static const char* type_of(const char* name) {
    static const struct {
        const char* extension;
        const char* type;
    } types[] = {
        {".html", "text/html; charset=utf-8"}, {".js", "text/javascript"}, {".wasm", "application/wasm"}, {".json", "application/json"}, {".css", "text/css"},
        {".svg", "image/svg+xml"},           {".png", "image/png"},        {".jpg", "image/jpeg"},        {".jpeg", "image/jpeg"},       {".gif", "image/gif"},
        {".webp", "image/webp"},             {".ico", "image/x-icon"},     {".ttf", "font/ttf"},          {".otf", "font/otf"},          {".woff", "font/woff"},
        {".woff2", "font/woff2"},            {".wav", "audio/wav"},        {".mp3", "audio/mpeg"},        {".ogg", "audio/ogg"},         {".txt", "text/plain; charset=utf-8"},
    };
    for (size_t i = 0; i < sizeof types / sizeof types[0]; i++)
        if (ends_with(name, types[i].extension)) return types[i].type;
    return "application/octet-stream";
}

static int hex_digit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/// A path as it names a file: without its query, %XX read.
static const char* decoded(const char* path) {
    ClarityBuffer out = {0};
    for (const char* c = path; *c && *c != '?' && *c != '#'; c++) {
        if (*c == '%' && hex_digit(c[1]) >= 0 && hex_digit(c[2]) >= 0) {
            clarity_buffer_add_char(&out, (char)(hex_digit(c[1]) * 16 + hex_digit(c[2])));
            c += 2;
        } else {
            clarity_buffer_add_char(&out, *c);
        }
    }
    return out.length ? clarity_buffer_text(&out) : "";
}

static const char* file_reply(const ClarityCarried* file, size_t* length) {
    return reply("200 OK", type_of(file->name), (const char*)file->data, file->size, length);
}

/// One browser page (a tab): its person's settings as it last sent them and
/// the language its set_language chose, as JSON and text outside the
/// scratch arena, and when it last asked.
typedef struct {
    char* id;
    char* settings;
    char* language;
    uint64_t used;
} Page;

struct ClarityServer {
    const ClarityServeOptions* options;
    /// The pages, behind their own gate; one unasked for `quiet` ms is forgotten.
    ClarityGate paging;
    Page* pages;
    size_t page_count;
    uint64_t quiet;
    ClarityService* service;
    /// One command at a time on the service and its databases.
    ClarityGate turn;
    /// The long calls, answered beside the turn: the page asks how far one
    /// is, or stops it, while its handler's request holds the turn.
    ClarityWatches* watches;
};

ClarityServer* clarity_server_new(const ClarityServeOptions* options) {
    ClarityServer* server = calloc(1, sizeof *server);
    if (!server) return NULL;
    server->options = options;
    clarity_gate_init(&server->turn);
    clarity_gate_init(&server->paging);
    server->quiet = CLARITY_QUIET_MS;
    server->service = clarity_service_new(&options->service);
    server->watches = clarity_watches_new();
    if (server->service && server->watches) return server;
    clarity_server_free(server);
    return NULL;
}

void clarity_server_free(ClarityServer* server) {
    if (!server) return;
    clarity_service_free(server->service);
    clarity_watches_free(server->watches);
    for (size_t i = 0; i < server->page_count; i++) {
        free(server->pages[i].id);
        free(server->pages[i].settings);
        free(server->pages[i].language);
    }
    free(server->pages);
    free(server);
}

void clarity_server_quiet(ClarityServer* server, uint64_t milliseconds) {
    if (server) server->quiet = milliseconds;
}

/// Milliseconds on a clock that only goes forward.
static uint64_t now_ms(void) {
#ifdef _WIN32
    return (uint64_t)GetTickCount64();
#else
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (uint64_t)now.tv_sec * 1000u + (uint64_t)now.tv_nsec / 1000000u;
#endif
}

/// A copy of `text` outside the scratch arena; NULL for NULL.
static char* lasting(const char* text) {
    if (!text) return NULL;
    char* copy = malloc(strlen(text) + 1);
    if (copy) strcpy(copy, text);
    return copy;
}

/// Page `id`, made when it is new, the pages unasked for too long gone;
/// holding the paging gate.
static Page* page_of(ClarityServer* server, const char* id) {
    uint64_t now = now_ms();
    size_t kept = 0;
    Page* found = NULL;
    for (size_t i = 0; i < server->page_count; i++) {
        Page page = server->pages[i];
        if (strcmp(page.id, id) != 0 && now - page.used > server->quiet) {
            free(page.id);
            free(page.settings);
            free(page.language);
            continue;
        }
        server->pages[kept] = page;
        if (strcmp(page.id, id) == 0) found = &server->pages[kept];
        kept++;
    }
    server->page_count = kept;
    if (!found) {
        Page* more = realloc(server->pages, (kept + 1) * sizeof *more);
        if (!more) return NULL;
        server->pages = more;
        found = &more[kept];
        *found = (Page){lasting(id), NULL, NULL, now};
        server->page_count++;
    }
    found->used = now;
    return found;
}

/// The settings record of a window's state in `holder`'s "state".
static const cJSON* settings_in(const cJSON* holder) {
    const cJSON* state = cJSON_GetObjectItemCaseSensitive(holder, "state");
    return cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(state, "data"), "settings");
}

/// The settings the answer's window has (its id left out), when one of them
/// differs from what the window had (`before`); NULL when none changed.
static cJSON* changed(const cJSON* before, const cJSON* answer) {
    const cJSON* after = settings_in(cJSON_GetObjectItemCaseSensitive(answer, "value"));
    if (!cJSON_IsObject(after) || !cJSON_IsObject(before)) return NULL;
    bool differs = false;
    const cJSON* setting;
    cJSON_ArrayForEach(setting, after) {
        if (!setting->string || strcmp(setting->string, "id") == 0) continue;
        const cJSON* was = cJSON_GetObjectItemCaseSensitive(before, setting->string);
        if (was && !cJSON_Compare(was, setting, true)) differs = true;
    }
    if (!differs) return NULL;
    cJSON* now = cJSON_Duplicate(after, true);
    cJSON_DeleteItemFromObjectCaseSensitive(now, "id");
    return now;
}

ClarityService* clarity_server_service(ClarityServer* server) { return server->service; }

static uint64_t number_of(const cJSON* args, const char* name) {
    const cJSON* id = cJSON_GetObjectItemCaseSensitive(args, name);
    return cJSON_IsNumber(id) && id->valuedouble > 0 ? (uint64_t)id->valuedouble : 0;
}

/// The answer to `command`, as JSON in the scratch arena, in `language` and
/// with the page's `settings` (none for the default and none).
static cJSON* answered(ClarityServer* server, const char* command, cJSON* args, const char* language, const cJSON* settings) {
    // How far a long call is, and its Stop, without waiting for the turn.
    if (clarity_watches_command(command)) {
        char* text = clarity_watches_answer(server->watches, command, number_of(args, "id"));
        cJSON* out = cJSON_Parse(text ? text : "{\"value\":null}");
        free(text);
        return out;
    }
    if (strcmp(command, "watches") == 0) {
        cJSON* out = cJSON_CreateObject();
        cJSON_AddBoolToObject(out, "value", server->options->watches);
        return out;
    }
    // Engines beside the screen run in the program, each canvas's by its number, without waiting for the turn.
    if (strncmp(command, "engine_", 7) == 0) return server->options->screen_engines ? server->options->screen_engines(command, args) : NULL;
    // A handler's request that names its watch runs under it (the page sends no start_watch).
    uint64_t watch = strcmp(command, "dispatch_event") == 0 || strcmp(command, "answer_event") == 0 ? number_of(args, "watch") : 0;
    clarity_gate_take(&server->turn);
    // The page's language and settings, set within the turn: commands take turns.
    clarity_set_language(language);
    clarity_service_page(server->service, settings);
    if (watch) clarity_watches_enter(server->watches, watch);
    cJSON* out = clarity_wire_answer(server->service, &server->options->wire, command, args);
    if (watch) clarity_watches_leave(server->watches);
    clarity_service_page(server->service, NULL);
    clarity_gate_give(&server->turn);
    return out;
}

/// The answer to `command` of page `browser`, as that page: with its
/// settings (`sent`, as the page keeps them; none keeps those it had) and
/// its language, which set_language chooses for it alone. The answer carries
/// the page's settings ("settings") when the command changed them.
static cJSON* answered_as(ClarityServer* server, const char* browser, const cJSON* sent, const char* command, cJSON* args) {
    clarity_gate_take(&server->paging);
    Page* page = page_of(server, browser);
    if (!page) {
        clarity_gate_give(&server->paging);
        return answered(server, command, args, NULL, NULL);
    }
    if (cJSON_IsObject(sent)) {
        free(page->settings);
        page->settings = lasting(cJSON_PrintUnformatted(sent));
    }
    if (strcmp(command, "set_language") == 0) {
        const cJSON* language = cJSON_GetObjectItemCaseSensitive(args, "language");
        free(page->language);
        page->language = cJSON_IsString(language) && *language->valuestring ? lasting(language->valuestring) : NULL;
        clarity_gate_give(&server->paging);
        cJSON* out = cJSON_CreateObject();
        cJSON_AddNullToObject(out, "value");
        return out;
    }
    cJSON* settings = page->settings ? cJSON_Parse(page->settings) : cJSON_CreateObject();
    const char* language = page->language ? clarity_format("%s", page->language) : NULL;
    clarity_gate_give(&server->paging);
    // What the window had: the state's settings, or else the page's.
    const cJSON* before = settings_in(args);
    if (!cJSON_IsObject(before)) before = settings;
    cJSON* out = answered(server, command, args, language, settings);
    cJSON* now = out ? changed(before, out) : NULL;
    if (now) {
        clarity_gate_take(&server->paging);
        page = page_of(server, browser);
        cJSON* kept = page && page->settings ? cJSON_Parse(page->settings) : NULL;
        if (!cJSON_IsObject(kept)) kept = cJSON_CreateObject();
        const cJSON* setting;
        cJSON_ArrayForEach(setting, now) {
            cJSON_DeleteItemFromObjectCaseSensitive(kept, setting->string);
            cJSON_AddItemToObject(kept, setting->string, cJSON_Duplicate(setting, true));
        }
        if (page) {
            free(page->settings);
            page->settings = lasting(cJSON_PrintUnformatted(kept));
        }
        clarity_gate_give(&server->paging);
        cJSON_AddItemToObject(out, "settings", kept);
    }
    return out;
}

/// A command's answer: none reads as the service's error for a command it
/// does not know, which the library takes as its default.
static const char* invoked(ClarityServer* server, const char* body, size_t body_length, size_t* length) {
    cJSON* sent = cJSON_ParseWithLength(body, body_length);
    const cJSON* command = cJSON_GetObjectItem(sent, "command");
    cJSON* args = cJSON_GetObjectItem(sent, "args");
    cJSON* answer;
    if (cJSON_IsString(command)) {
        // The page that asks (one id per tab) and its person's settings as it keeps them.
        const cJSON* browser = cJSON_GetObjectItem(sent, "browser");
        answer = answered_as(server, cJSON_IsString(browser) ? browser->valuestring : "", cJSON_GetObjectItem(sent, "settings"), command->valuestring, cJSON_IsObject(args) ? args : cJSON_CreateObject());
        if (!answer) {
            answer = cJSON_CreateObject();
            cJSON_AddStringToObject(answer, "error", clarity_format("there is no command %s", command->valuestring));
        }
    } else {
        answer = cJSON_CreateObject();
        cJSON_AddStringToObject(answer, "error", "a command is JSON with its command and its args");
    }
    // cJSON takes its memory from the scratch arena, like the rest of the command.
    const char* printed = cJSON_PrintUnformatted(answer);
    return reply("200 OK", "application/json", printed, strlen(printed), length);
}

const char* clarity_serve_answer(ClarityServer* server, const char* request, size_t request_length, size_t* length) {
    const char* end = strstr(request, "\r\n\r\n");
    char method[8] = {0}, target[1024] = {0};
    if (!end || sscanf(request, "%7s %1023s", method, target) != 2) return reply("400 Bad Request", "text/plain", "", 0, length);
    const char* path = decoded(target);
    if (strcmp(path, CLARITY_INVOKE_PATH) == 0) {
        if (strcmp(method, "POST") != 0) {
            const char* body = "send a command with POST";
            return reply("405 Method Not Allowed", "text/plain", body, strlen(body), length);
        }
        return invoked(server, end + 4, request_length - (size_t)(end + 4 - request), length);
    }
    if (strcmp(method, "GET") == 0) {
        // The browser files by their paths below browser/; the page at /.
        const ClarityCarriedFiles* files = server->options->files;
        const ClarityCarried* found = files && path[0] == '/' ? clarity_carried_find(files->browser, files->browser_count, strcmp(path, "/") == 0 ? "index.html" : path + 1) : NULL;
        if (found) return file_reply(found, length);
    }
    return reply("404 Not Found", "text/plain", "", 0, length);
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

/// One client's request, answered on a thread of its own.
typedef struct {
    ClarityServer* server;
    serve_socket client;
} Connection;

/// Reads the request, sends the answer, closes and frees the connection.
static void answer_one(Connection* connection) {
    size_t got = 0;
    char* request = read_request(connection->client, &got);
    if (request) {
        size_t length = 0;
        const char* answer = clarity_serve_answer(connection->server, request, got, &length);
        send_all(connection->client, answer, length);
        free(request);
    }
    SERVE_CLOSE(connection->client);
    free(connection);
    // The request is answered, and its thread ends: its values are done with.
    clarity_scratch_release();
}

#ifdef _WIN32
static DWORD WINAPI answer_thread(void* arg) {
    answer_one(arg);
    return 0;
}

static bool started(Connection* connection) {
    HANDLE thread = CreateThread(NULL, 0, answer_thread, connection, 0, NULL);
    if (!thread) return false;
    CloseHandle(thread);
    return true;
}
#else
static void* answer_thread(void* arg) {
    answer_one(arg);
    return NULL;
}

static bool started(Connection* connection) {
    pthread_t thread;
    if (pthread_create(&thread, NULL, answer_thread, connection) != 0) return false;
    pthread_detach(thread);
    return true;
}
#endif

int clarity_serve(const ClarityServeOptions* options, const char* address) {
#ifdef _WIN32
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) return 1;
#endif
    const char* colon = strrchr(address, ':');
    if (!colon) {
        fprintf(stderr, "%s: an address is host:port, not %s\n", options->service.name, address);
        return 1;
    }
    const char* host = clarity_format("%.*s", (int)(colon - address), address);
    struct addrinfo hints = {0}, *found = NULL;
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_PASSIVE;
    if (getaddrinfo(*host ? host : NULL, colon + 1, &hints, &found) != 0 || !found) {
        fprintf(stderr, "%s: cannot listen on %s\n", options->service.name, address);
        return 1;
    }
    serve_socket listener = socket(found->ai_family, found->ai_socktype, found->ai_protocol);
    int yes = 1;
    setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, (const char*)&yes, sizeof yes);
    if (listener == SERVE_INVALID || bind(listener, found->ai_addr, (int)found->ai_addrlen) != 0 || listen(listener, 16) != 0) {
        fprintf(stderr, "%s: cannot listen on %s\n", options->service.name, address);
        freeaddrinfo(found);
        return 1;
    }
    freeaddrinfo(found);
    ClarityServer* server = clarity_server_new(options);
    if (!server) {
        fprintf(stderr, "%s: the service did not start\n", options->service.name);
        return 1;
    }
    fprintf(stderr, "%s: serving on http://%s\n", options->service.name, address);
    // Requests side by side, each on a thread of its own: a long call's
    // request stays open while the page asks how far it is.
    for (;;) {
        serve_socket client = accept(listener, NULL, NULL);
        if (client == SERVE_INVALID) continue;
        Connection* connection = malloc(sizeof *connection);
        if (!connection) {
            SERVE_CLOSE(client);
            continue;
        }
        connection->server = server;
        connection->client = client;
        if (!started(connection)) {
            answer_one(connection);
        }
    }
}
