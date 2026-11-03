/*
 * A C program against clarity_ui.h and the library, built with the system's
 * C compiler by tests/c_header.rs: it proves the header matches what the
 * library has. It makes a window over a part host written in C, clicks its
 * button, types in its box, draws it into its own pixels and checks what
 * failing calls say. It prints "ok" and returns 0 when all held.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity_ui.h"

#define CHECK(what)                                                     \
    do {                                                                \
        if (!(what)) {                                                  \
            char *why = clarity_last_error();                           \
            fprintf(stderr, "line %d: %s (%s)\n", __LINE__, #what,      \
                    why ? why : "no error");                            \
            clarity_string_free(why);                                   \
            return 1;                                                   \
        }                                                               \
    } while (0)

#define TEXT(s) (s), strlen(s)

/* The window model the host answers from: one value and what happened. */
static char name_value[64] = "\"Ann\"";
static int runs = 0;
static int writes = 0;
static int released = 0;

static void host_read(void *user, const char *path, size_t len, ClarityAnswer *value) {
    (void)user;
    if (len == 4 && memcmp(path, "name", 4) == 0)
        clarity_answer_set(value, (const uint8_t *)name_value, strlen(name_value));
}

static void host_write(void *user, const char *part, size_t part_len, const char *path, size_t path_len, const char *value, size_t value_len) {
    (void)user; (void)part; (void)part_len; (void)path; (void)path_len;
    if (value_len < sizeof name_value) {
        memcpy(name_value, value, value_len);
        name_value[value_len] = 0;
    }
    writes++;
}

static void host_state(void *user, const char *name, size_t len, ClarityAnswer *state) {
    (void)user; (void)name; (void)len; (void)state;
}

static void host_set_state(void *user, const char *name, size_t len, const char *state, size_t state_len) {
    (void)user; (void)name; (void)len; (void)state; (void)state_len;
}

static int32_t host_run(void *user, const char *part, size_t part_len, const char *call, size_t call_len) {
    (void)user; (void)call; (void)call_len;
    if (part && part_len == 4 && memcmp(part, "save", 4) == 0)
        runs++;
    return 1;
}

static void host_text(void *user, const char *written, size_t len, ClarityAnswer *text) {
    (void)user;
    clarity_answer_set(text, (const uint8_t *)written, len);
}

static uint64_t host_changes(void *user) {
    (void)user;
    return (uint64_t)writes;
}

static void host_release(void *user) {
    (void)user;
    released = 1;
}

int main(void) {
    CHECK(clarity_ui_interface() == CLARITY_UI_INTERFACE);
    CHECK(strlen(clarity_ui_version()) > 0);

    /* A failing call says why, and the message is the caller's to free. */
    ClarityFiles *files = NULL;
    CHECK(clarity_files_new(&files) == CLARITY_OK);
    CHECK(clarity_files_model(files, TEXT("{ no")) == CLARITY_INVALID);
    char *why = clarity_last_error();
    CHECK(why != NULL && strstr(why, "does not read") != NULL);
    clarity_string_free(why);
    CHECK(clarity_files_window(NULL, TEXT("Main"), TEXT("<Window />")) == CLARITY_INVALID);
    clarity_files_free(files);

    ClarityPartHostCallbacks host;
    memset(&host, 0, sizeof host);
    host.read = host_read;
    host.write = host_write;
    host.state = host_state;
    host.set_state = host_set_state;
    host.run = host_run;
    host.text = host_text;
    host.changes = host_changes;
    host.release = host_release;

    const char *markup =
        "<Window name=\"Main\">"
        "<Textbox name=\"nameBox\" at=\"1,1\" span=\"4\" bind=\"name\" />"
        "<Button name=\"save\" at=\"1,2\" span=\"3\" caption=\"Save\" onclick=\"save\" />"
        "</Window>";
    ClarityScreen *screen = NULL;
    CHECK(clarity_screen_new(TEXT(markup), NULL, 0, &host, &screen) == CLARITY_OK);
    CHECK(clarity_screen_resize(screen, 400, 300, 1.0f) == CLARITY_OK);

    double x, y, w, h;
    CHECK(clarity_screen_part_place(screen, TEXT("save"), &x, &y, &w, &h) == CLARITY_OK);
    CHECK(w > 0 && h > 0);
    CHECK(clarity_screen_click(screen, x + w / 2, y + h / 2) == CLARITY_OK);
    CHECK(runs == 1);
    CHECK(clarity_screen_part_place(screen, TEXT("nothing"), &x, &y, &w, &h) == CLARITY_NOT_FOUND);

    CHECK(clarity_screen_focus(screen, TEXT("nameBox")) == CLARITY_OK);
    char *focused = NULL;
    CHECK(clarity_screen_focused(screen, &focused) == CLARITY_OK);
    CHECK(focused != NULL && strcmp(focused, "nameBox") == 0);
    clarity_string_free(focused);
    CHECK(clarity_screen_key(screen, TEXT("End"), 0, NULL) == CLARITY_OK);
    CHECK(clarity_screen_type(screen, TEXT(" B")) == CLARITY_OK);
    int32_t taken = 0;
    CHECK(clarity_screen_key(screen, TEXT("Tab"), 0, &taken) == CLARITY_OK);
    CHECK(strcmp(name_value, "\"Ann B\"") == 0);

    int32_t width = 400, height = 300;
    uint8_t *pixels = calloc((size_t)width * height, 4);
    CHECK(pixels != NULL);
    CHECK(clarity_screen_draw(screen, pixels, width, height, (size_t)width * 4) == CLARITY_OK);
    CHECK(pixels[3] == 255);
    free(pixels);

    char *parts = NULL;
    CHECK(clarity_screen_parts_json(screen, &parts) == CLARITY_OK);
    CHECK(strstr(parts, "\"save\"") != NULL);
    clarity_string_free(parts);

    clarity_screen_free(screen);
    CHECK(released == 1);

    printf("ok\n");
    return 0;
}
