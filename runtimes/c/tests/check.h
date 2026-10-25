// A test harness of a few lines, so the runtime's tests need nothing but
// the compiler. CHECK and its kin report a failure and go on; REQUIRE stops
// the test. Every test is listed in tests.h.

#pragma once

#include <stdio.h>
#include <string.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

void check_fail(const char* file, int line, const char* what);
/// The case file `name` of the repository's conformance folder, which every stack's runtime shares.
cJSON* conformance(const char* name);
/// Whether JSON equals the JSON written in `expected`, ignoring the order of names.
int check_json(const cJSON* got, const char* expected);

/// The folder for a test's scratch files: TMPDIR, else TEMP (Windows), else /tmp.
const char* temp_folder(void);

/// Removes a folder and everything in it, if it is there.
void remove_tree(const char* path);

#define CHECK(cond)                                                       \
    do {                                                                  \
        if (!(cond)) check_fail(__FILE__, __LINE__, "failed: " #cond);    \
    } while (0)

#define REQUIRE(cond)                                                     \
    do {                                                                  \
        if (!(cond)) {                                                    \
            check_fail(__FILE__, __LINE__, "required: " #cond);           \
            return;                                                       \
        }                                                                 \
    } while (0)

#define CHECK_TEXT(a, b)                                                                                           \
    do {                                                                                                           \
        const char* check_a = (a);                                                                                 \
        const char* check_b = (b);                                                                                 \
        if (!check_a || !check_b || strcmp(check_a, check_b) != 0)                                                 \
            check_fail(__FILE__, __LINE__, clarity_format("%s == %s: '%s' != '%s'", #a, #b, check_a ? check_a : "(null)", check_b ? check_b : "(null)")); \
    } while (0)

#define CHECK_INT(a, b)                                                                                                   \
    do {                                                                                                                  \
        long long check_a = (long long)(a);                                                                               \
        long long check_b = (long long)(b);                                                                               \
        if (check_a != check_b) check_fail(__FILE__, __LINE__, clarity_format("%s == %s: %lld != %lld", #a, #b, check_a, check_b)); \
    } while (0)

/// A call that must work; its error is reported when it does not.
#define CHECK_OK(call)                                                                                        \
    do {                                                                                                      \
        if (!(call)) check_fail(__FILE__, __LINE__, clarity_format("%s: %s", #call, clarity_error()->message)); \
    } while (0)

#define REQUIRE_OK(call)                                                                                        \
    do {                                                                                                        \
        if (!(call)) {                                                                                          \
            check_fail(__FILE__, __LINE__, clarity_format("%s: %s", #call, clarity_error()->message));           \
            return;                                                                                             \
        }                                                                                                       \
    } while (0)

/// A call that must fail with an error of `kind`.
#define CHECK_FAILS(expected, call)                                                                                   \
    do {                                                                                                          \
        if (call) check_fail(__FILE__, __LINE__, "did not fail: " #call);                                         \
        else if (clarity_error()->kind != (expected))                                                                 \
            check_fail(__FILE__, __LINE__, clarity_format("%s failed otherwise: %s", #call, clarity_error()->message)); \
    } while (0)

#define CHECK_JSON(got, expected)                                                                                          \
    do {                                                                                                                   \
        const cJSON* check_got = (got);                                                                                    \
        if (!check_json(check_got, expected))                                                                              \
            check_fail(__FILE__, __LINE__, clarity_format("%s: %s != %s", #got, cJSON_PrintUnformatted(check_got), expected)); \
    } while (0)
