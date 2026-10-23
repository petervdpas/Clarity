// The part host (DESIGN 6.5) with a kind of part of the tests' own, which
// writes down every call the host makes, and no native windows.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "clarity_parts.h"

static int failures;

#define CHECK(cond)                                                    \
    do {                                                               \
        if (!(cond)) {                                                 \
            fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #cond); \
            ++failures;                                                \
        }                                                              \
    } while (0)

// -- The kind of part: calls written down in order ------------------------

static char calls[8192];
static int frames;
static ClarityPartHost* last_host;

static void called(const char* text) {
    strncat(calls, text, sizeof calls - strlen(calls) - 2);
    strcat(calls, ";");
}

static void* probe_create(void* parent, int width, int height, ClarityPartHost* host) {
    char line[128];
    snprintf(line, sizeof line, "create %dx%d%s", width, height, parent ? "" : " without a window");
    called(line);
    last_host = host;
    return malloc(1);
}

static void probe_resize(void* part, int width, int height) {
    (void)part;
    char line[64];
    snprintf(line, sizeof line, "resize %dx%d", width, height);
    called(line);
}

static void probe_frame(void* part) {
    (void)part;
    ++frames;
}

static void probe_destroy(void* part) {
    free(part);
    called("destroy");
}

static void probe_set(void* part, const char* name, const cJSON* value) {
    (void)part;
    char* text = cJSON_PrintUnformatted(value);
    char line[256];
    snprintf(line, sizeof line, "set %s %s", name, text);
    free(text);
    called(line);
}

static void probe_hear(void* part, const char* hear, const cJSON* values) {
    char* text = cJSON_PrintUnformatted(values);
    char line[256];
    snprintf(line, sizeof line, "hear %s %s", hear, text);
    free(text);
    called(line);
    // A hear the probe answers with a tell, as a library does.
    if (strcmp(hear, "ping") == 0) {
        cJSON* back = cJSON_CreateArray();
        cJSON_AddItemToArray(back, cJSON_CreateNumber(42));
        clarity_parts_tell(last_host, "pong", back);
    }
    (void)part;
}

const ClarityPartKind clarity_part_kinds[] = {
    {"Probe", probe_create, probe_resize, probe_frame, probe_destroy, probe_set, probe_hear},
    {NULL, NULL, NULL, NULL, NULL, NULL, NULL},
};

// -- What the stack hears --------------------------------------------------

static char told[1024];

static void on_told(uint64_t window, const char* part, const char* tell, const char* values, void* context) {
    (void)context;
    char line[256];
    snprintf(line, sizeof line, "%llu %s %s %s;", (unsigned long long)window, part, tell, values);
    strncat(told, line, sizeof told - strlen(told) - 1);
}

static void sleep_ms(long ms) {
    struct timespec t = {ms / 1000, (ms % 1000) * 1000000L};
    nanosleep(&t, NULL);
}

static void reset(void) {
    calls[0] = '\0';
    told[0] = '\0';
}

int main(void) {
    CHECK(clarity_parts_start(on_told, NULL));
    CHECK(strcmp(clarity_parts_problem(), "") == 0);

    // Values and hears given before the part is placed reach it once it is made, in order.
    reset();
    clarity_parts_set(1, "arena", "speed", "5");
    clarity_parts_hear(1, "arena", "restart", "[1]");
    clarity_parts_set(1, "arena", "value", "120");
    clarity_parts_place(1, "arena", "Probe", 99, 10, 20, 300, 200, true);
    clarity_parts_wait();
    CHECK(strcmp(calls, "create 300x200;set speed 5;hear restart [1];set value 120;") == 0);

    // Moving keeps its size; a new size resizes it; a value set later goes straight to it.
    reset();
    clarity_parts_place(1, "arena", "Probe", 99, 50, 60, 300, 200, true);
    clarity_parts_place(1, "arena", "Probe", 99, 50, 60, 400, 250, true);
    clarity_parts_set(1, "arena", "speed", "6");
    clarity_parts_wait();
    CHECK(strcmp(calls, "resize 400x250;set speed 6;") == 0);

    // A hear with its values; a tell comes back to the stack with the part's window and name.
    reset();
    clarity_parts_hear(1, "arena", "restart", "[2]");
    clarity_parts_hear(1, "arena", "ping", "[]");
    clarity_parts_wait();
    CHECK(strcmp(calls, "hear restart [2];hear ping [];") == 0);
    CHECK(strcmp(told, "1 arena pong [42];") == 0);

    // A shown part has frames; a hidden one has none.
    frames = 0;
    sleep_ms(120);
    clarity_parts_wait();
    CHECK(frames >= 3);
    clarity_parts_place(1, "arena", "Probe", 99, 50, 60, 400, 250, false);
    clarity_parts_wait();
    frames = 0;
    sleep_ms(80);
    clarity_parts_wait();
    CHECK(frames == 0);

    // The same part name in another window is another part.
    reset();
    clarity_parts_place(2, "arena", "Probe", 98, 0, 0, 100, 100, true);
    clarity_parts_wait();
    CHECK(strcmp(calls, "create 100x100;") == 0);

    // Closing a window ends its parts only.
    reset();
    clarity_parts_close(1);
    clarity_parts_set(2, "arena", "speed", "1");
    clarity_parts_wait();
    CHECK(strcmp(calls, "destroy;set speed 1;") == 0);

    // A kind the project does not have makes nothing.
    reset();
    clarity_parts_place(3, "map", "Nowhere", 97, 0, 0, 10, 10, true);
    clarity_parts_wait();
    CHECK(strcmp(calls, "") == 0);

    // Stopping ends what is left.
    reset();
    clarity_parts_stop();
    CHECK(strcmp(calls, "destroy;") == 0);

    if (failures == 0) printf("part host: all passed\n");
    return failures == 0 ? 0 : 1;
}
