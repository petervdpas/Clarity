// Runs every test of tests.h; the exit status tells whether one failed.

#include "check.h"
#include "tests.h"

#include <stdlib.h>

static int failures;

void check_fail(const char* file, int line, const char* what) {
    ++failures;
    fprintf(stderr, "%s:%d: %s\n", file, line, what);
}

cJSON* conformance(const char* name) {
    FILE* in = fopen(clarity_format("%s/%s", CLARITY_CONFORMANCE, name), "rb");
    if (!in) return NULL;
    ClarityBuffer text = {0};
    char chunk[4096];
    size_t got;
    while ((got = fread(chunk, 1, sizeof chunk, in)) > 0) clarity_buffer_add_n(&text, chunk, got);
    fclose(in);
    return cJSON_Parse(clarity_buffer_text(&text));
}

int check_json(const cJSON* got, const char* expected) {
    cJSON* want = cJSON_Parse(expected);
    return want && got && cJSON_Compare(got, want, 1);
}

#define LISTED(name) {#name, name},

static const struct {
    const char* name;
    void (*run)(void);
} tests[] = {CLARITY_TESTS(LISTED)};

int main(void) {
    clarity_start();
    size_t count = sizeof tests / sizeof tests[0];
    for (size_t i = 0; i < count; ++i) {
        int before = failures;
        tests[i].run();
        printf("%s %s\n", failures == before ? "ok  " : "FAIL", tests[i].name);
        clarity_scratch_reset();
    }
    printf("%zu tests, %d failures\n", count, failures);
    return failures == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
