// Engines hosted by the C runtime, against the cases every host shares
// (conformance/engines.json).

#include <string.h>

#include "check.h"
#include "clarity/disk.h"
#include "engine/engine.h"
#include "tests.h"

static uint8_t* hex(const char* text, size_t* size) {
    *size = strlen(text) / 2;
    uint8_t* bytes = clarity_alloc(*size + 1);
    for (size_t i = 0; i < *size; ++i) sscanf(text + 2 * i, "%2hhx", &bytes[i]);
    return bytes;
}

static ClarityBytes test_engine(const cJSON* file) {
    ClarityBytes bytes = {0};
    char path[512];
    snprintf(path, sizeof path, "%s/%s", CLARITY_CONFORMANCE, cJSON_GetObjectItem(file, "engine")->valuestring);
    CHECK(clarity_read_file(path, &bytes));
    return bytes;
}

/// The limits every case runs under.
static ClarityEngineLimits limits(const cJSON* file) {
    const cJSON* limits = cJSON_GetObjectItem(file, "limits");
    return (ClarityEngineLimits){(uint32_t)cJSON_GetObjectItem(limits, "seconds")->valueint, (uint32_t)cJSON_GetObjectItem(limits, "megabytes")->valueint};
}

static const char** names(const cJSON* list, size_t* count) {
    *count = (size_t)cJSON_GetArraySize(list);
    const char** out = clarity_alloc(sizeof(char*) * (*count + 1));
    size_t i = 0;
    const cJSON* name;
    cJSON_ArrayForEach(name, list) out[i++] = name->valuestring;
    return out;
}

void every_engine_call_gives_what_every_host_gives(void) {
    cJSON* file = conformance("engines.json");
    ClarityBytes bytes = test_engine(file);
    size_t count;
    const char** functions = names(cJSON_GetObjectItem(file, "functions"), &count);
    size_t used;
    const char** uses = names(cJSON_GetObjectItem(file, "uses"), &used);
    ClarityEngine* engine;
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, (ClarityEngineDeclaration){count, functions, used, uses, limits(file)}, &engine));
    const cJSON* call;
    cJSON_ArrayForEach(call, cJSON_GetObjectItem(file, "calls")) {
        cJSON* value = NULL;
        cJSON* got = cJSON_CreateObject();
        if (clarity_engine_call(engine, cJSON_GetObjectItem(call, "function")->valuestring, cJSON_Duplicate(cJSON_GetObjectItem(call, "args"), true), &value)) {
            cJSON_AddItemToObject(got, "value", value);
        } else {
            cJSON_AddStringToObject(got, "error", clarity_error()->message);
        }
        if (!cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true)) fprintf(stderr, "got %s\n", cJSON_PrintUnformatted(got));
        CHECK(cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true));
    }
    clarity_engine_free(engine);
}

void a_module_that_is_no_engine_is_refused_when_it_loads(void) {
    cJSON* file = conformance("engines.json");
    const cJSON* refused;
    cJSON_ArrayForEach(refused, cJSON_GetObjectItem(file, "refused")) {
        const cJSON* written = cJSON_GetObjectItem(refused, "bytes");
        ClarityBytes bytes = test_engine(file);
        size_t size = bytes.size;
        const uint8_t* data = bytes.data;
        if (written) data = hex(written->valuestring, &size);
        size_t count = 0;
        const cJSON* listed = cJSON_GetObjectItem(refused, "functions");
        const char** functions = listed ? names(listed, &count) : NULL;
        const cJSON* own = cJSON_GetObjectItem(refused, "uses");
        size_t used;
        const char** uses = names(own ? own : cJSON_GetObjectItem(file, "uses"), &used);
        ClarityEngine* engine;
        CHECK_FAILS(CLARITY_ENGINE, clarity_engine_load("Test", data, size, (ClarityEngineDeclaration){count, functions, used, uses, limits(file), .opens = cJSON_IsTrue(cJSON_GetObjectItem(refused, "open"))}, &engine));
        CHECK(strstr(clarity_error()->message, cJSON_GetObjectItem(refused, "message")->valuestring) != NULL);
        CHECK(strncmp(clarity_error()->message, "engine Test ", 12) == 0);
    }
}

void a_call_keeps_to_the_limits_of_its_engine(void) {
    cJSON* file = conformance("engines.json");
    ClarityBytes bytes = test_engine(file);
    size_t count;
    const char** functions = names(cJSON_GetObjectItem(file, "functions"), &count);
    size_t used;
    const char** uses = names(cJSON_GetObjectItem(file, "uses"), &used);
    ClarityEngine* engine;
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, (ClarityEngineDeclaration){count, functions, used, uses, limits(file)}, &engine));
    const cJSON* call;
    cJSON_ArrayForEach(call, cJSON_GetObjectItem(file, "limited")) {
        cJSON* value = NULL;
        bool answered = clarity_engine_call(engine, cJSON_GetObjectItem(call, "function")->valuestring, cJSON_Duplicate(cJSON_GetObjectItem(call, "args"), true), &value);
        const cJSON* fails = cJSON_GetObjectItem(call, "fails");
        if (fails) {
            CHECK(!answered);
            if (!answered && strncmp(clarity_error()->message, fails->valuestring, strlen(fails->valuestring)) != 0) fprintf(stderr, "got %s\n", clarity_error()->message);
            CHECK(!answered && strncmp(clarity_error()->message, fails->valuestring, strlen(fails->valuestring)) == 0);
        } else {
            CHECK(answered);
            cJSON* got = cJSON_CreateObject();
            if (answered) cJSON_AddItemToObject(got, "value", value);
            CHECK(cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true));
        }
    }
    clarity_engine_free(engine);
}

static cJSON* logged;

static void keep_line(const char* line, void* context) {
    (void)context;
    cJSON_AddItemToArray(logged, cJSON_CreateString(line));
}

void an_engine_uses_the_services_it_declares(void) {
    cJSON* file = conformance("engines.json");
    ClarityBytes bytes = test_engine(file);
    size_t count, used;
    const char** functions = names(cJSON_GetObjectItem(file, "functions"), &count);
    const char** uses = names(cJSON_GetObjectItem(file, "uses"), &used);
    ClarityEngine* engine;
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, (ClarityEngineDeclaration){count, functions, used, uses, limits(file)}, &engine));
    const cJSON* services = cJSON_GetObjectItem(file, "services");
    clarity_engine_serve(engine, (ClarityEngineServices){.fixed_clock = true,
                                                         .clock = (int64_t)cJSON_GetObjectItem(services, "clock")->valuedouble,
                                                         .fixed_seed = true,
                                                         .seed = (uint64_t)cJSON_GetObjectItem(services, "seed")->valuedouble,
                                                         .log = keep_line,
                                                         .fixed_language = true,
                                                         .language = cJSON_GetObjectItem(services, "language")->valuestring});
    const cJSON* call;
    cJSON_ArrayForEach(call, cJSON_GetObjectItem(file, "served")) {
        logged = cJSON_CreateArray();
        cJSON* value = NULL;
        CHECK(clarity_engine_call(engine, cJSON_GetObjectItem(call, "function")->valuestring, cJSON_Duplicate(cJSON_GetObjectItem(call, "args"), true), &value));
        cJSON* got = cJSON_CreateObject();
        cJSON_AddItemToObject(got, "value", value);
        if (!cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true)) fprintf(stderr, "got %s\n", cJSON_PrintUnformatted(got));
        CHECK(cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true));
        const cJSON* log = cJSON_GetObjectItem(call, "log");
        cJSON* none = cJSON_CreateArray();
        CHECK(cJSON_Compare(logged, log ? log : none, true));
        cJSON_Delete(none);
        cJSON_Delete(logged);
    }
    clarity_engine_free(engine);
}

void an_engine_with_state_keeps_its_instance_from_call_to_call(void) {
    cJSON* file = conformance("engines.json");
    ClarityBytes bytes = test_engine(file);
    size_t count, used;
    const char** functions = names(cJSON_GetObjectItem(file, "functions"), &count);
    const char** uses = names(cJSON_GetObjectItem(file, "uses"), &used);
    const cJSON* kept = cJSON_GetObjectItem(file, "kept");
    CHECK(strcmp(cJSON_GetObjectItem(kept, "state")->valuestring, "app") == 0);
    ClarityEngineDeclaration declaration = {count, functions, used, uses, limits(file), CLARITY_ENGINE_PER_APP, cJSON_IsTrue(cJSON_GetObjectItem(kept, "open"))};
    ClarityEngine* engine;
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, declaration, &engine));
    const cJSON* call;
    cJSON_ArrayForEach(call, cJSON_GetObjectItem(kept, "calls")) {
        cJSON* value = NULL;
        bool answered = clarity_engine_call(engine, cJSON_GetObjectItem(call, "function")->valuestring, cJSON_Duplicate(cJSON_GetObjectItem(call, "args"), true), &value);
        const cJSON* fails = cJSON_GetObjectItem(call, "fails");
        if (fails) {
            CHECK(!answered && strncmp(clarity_error()->message, fails->valuestring, strlen(fails->valuestring)) == 0);
            continue;
        }
        cJSON* got = cJSON_CreateObject();
        if (answered) {
            cJSON_AddItemToObject(got, "value", value);
        } else {
            cJSON_AddStringToObject(got, "error", clarity_error()->message);
        }
        if (!cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true)) fprintf(stderr, "got %s\n", cJSON_PrintUnformatted(got));
        CHECK(cJSON_Compare(got, cJSON_GetObjectItem(call, "gives"), true));
    }
    clarity_engine_free(engine);
}

void an_engine_with_state_per_window_keeps_an_instance_for_each_window(void) {
    cJSON* file = conformance("engines.json");
    ClarityBytes bytes = test_engine(file);
    size_t count, used;
    const char** functions = names(cJSON_GetObjectItem(file, "functions"), &count);
    const char** uses = names(cJSON_GetObjectItem(file, "uses"), &used);
    const cJSON* windows = cJSON_GetObjectItem(file, "windows");
    CHECK(strcmp(cJSON_GetObjectItem(windows, "state")->valuestring, "window") == 0);
    ClarityEngineDeclaration declaration = {count, functions, used, uses, limits(file), CLARITY_ENGINE_PER_WINDOW, cJSON_IsTrue(cJSON_GetObjectItem(windows, "open"))};
    ClarityEngine* engine;
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, declaration, &engine));
    const cJSON* step;
    cJSON_ArrayForEach(step, cJSON_GetObjectItem(windows, "steps")) {
        const cJSON* closed = cJSON_GetObjectItem(step, "close");
        if (closed) {
            clarity_engine_close_window(engine, (uint64_t)closed->valuedouble);
            continue;
        }
        cJSON* value = NULL;
        uint64_t window = (uint64_t)cJSON_GetObjectItem(step, "window")->valuedouble;
        CHECK(clarity_engine_call_in(engine, window, cJSON_GetObjectItem(step, "function")->valuestring, cJSON_CreateArray(), &value));
        cJSON* got = cJSON_CreateObject();
        cJSON_AddItemToObject(got, "value", value);
        CHECK(cJSON_Compare(got, cJSON_GetObjectItem(step, "gives"), true));
    }
    clarity_engine_free(engine);
}

// Progress of a long call and Stop, against the cases every host shares
// (conformance/progress.json).

typedef struct {
    int64_t reports[64][2];
    size_t count;
    int64_t stop_at;
    volatile int32_t stop;
} Watched;

static void watched(int64_t done, int64_t total, void* context) {
    Watched* w = context;
    if (w->count < 64) {
        w->reports[w->count][0] = done;
        w->reports[w->count][1] = total;
    }
    w->count++;
    if (done == w->stop_at) w->stop = 1;
}

static ClarityEngine* long_engine(const cJSON* file, const cJSON* used_list) {
    ClarityBytes bytes = test_engine(file);
    size_t count;
    const char** functions = names(cJSON_GetObjectItem(file, "functions"), &count);
    size_t used;
    const char** uses = names(used_list, &used);
    ClarityEngine* engine = NULL;
    clarity_engine_load(cJSON_GetObjectItem(file, "name")->valuestring, bytes.data, bytes.size, (ClarityEngineDeclaration){count, functions, used, uses, limits(file)}, &engine);
    return engine;
}

void a_long_engine_call_reports_how_far_it_is(void) {
    cJSON* file = conformance("progress.json");
    ClarityEngine* engine = long_engine(file, cJSON_GetObjectItem(file, "uses"));
    REQUIRE(engine);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "reported")) {
        Watched w = {.stop_at = -1};
        ClarityEngineWatch watch = {watched, &w, &w.stop};
        cJSON* got = NULL;
        CHECK_OK(clarity_engine_call_watched(engine, 0, cJSON_GetObjectItem(c, "function")->valuestring, cJSON_Duplicate(cJSON_GetObjectItem(c, "args"), 1), &watch, &got));
        CHECK(got && got->valuedouble == cJSON_GetObjectItem(c, "gives")->valuedouble);
        const cJSON* expected = cJSON_GetObjectItem(c, "progress");
        CHECK(w.count == (size_t)cJSON_GetArraySize(expected));
        for (size_t i = 0; i < w.count && i < 64; ++i) {
            CHECK(w.reports[i][0] == (int64_t)cJSON_GetArrayItem(cJSON_GetArrayItem(expected, (int)i), 0)->valuedouble);
            CHECK(w.reports[i][1] == (int64_t)cJSON_GetArrayItem(cJSON_GetArrayItem(expected, (int)i), 1)->valuedouble);
        }
    }
    clarity_engine_free(engine);
    // Without `uses progress`, an engine that reports is refused.
    const cJSON* refused = cJSON_GetObjectItem(file, "refused");
    CHECK(!long_engine(file, cJSON_GetObjectItem(refused, "uses")));
    CHECK_TEXT(clarity_error()->message, cJSON_GetObjectItem(refused, "message")->valuestring);
}

void stop_ends_a_long_engine_call(void) {
    cJSON* file = conformance("progress.json");
    ClarityEngine* engine = long_engine(file, cJSON_GetObjectItem(file, "uses"));
    REQUIRE(engine);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "stopped")) {
        int64_t stop_at = (int64_t)cJSON_GetObjectItem(c, "stopAt")->valuedouble;
        Watched w = {.stop_at = stop_at == 0 ? -1 : stop_at, .stop = stop_at == 0};
        ClarityEngineWatch watch = {watched, &w, &w.stop};
        cJSON* got = NULL;
        CHECK_FAILS(CLARITY_ENGINE, clarity_engine_call_watched(engine, 0, cJSON_GetObjectItem(c, "function")->valuestring, cJSON_Duplicate(cJSON_GetObjectItem(c, "args"), 1), &watch, &got));
        CHECK_TEXT(clarity_error()->message, cJSON_GetObjectItem(c, "fails")->valuestring);
        const cJSON* first = cJSON_GetObjectItem(c, "firstReports");
        CHECK(w.count >= (size_t)cJSON_GetArraySize(first));
        for (int i = 0; i < cJSON_GetArraySize(first); ++i) CHECK(w.reports[i][0] == (int64_t)cJSON_GetArrayItem(cJSON_GetArrayItem(first, i), 0)->valuedouble);
    }
    clarity_engine_free(engine);
}

// The shell watches a handler, not each engine call: a call made while a
// watch is current, as generated code makes it, reports to it and stops with it.
void a_call_inside_a_watched_handler_is_watched(void) {
    cJSON* file = conformance("progress.json");
    ClarityEngine* engine = long_engine(file, cJSON_GetObjectItem(file, "uses"));
    REQUIRE(engine);
    Watched w = {.stop_at = -1};
    ClarityEngineWatch watch = {watched, &w, &w.stop};
    const ClarityEngineWatch* was = clarity_watching(&watch);
    cJSON* got = NULL;
    CHECK_OK(clarity_engine_call_in(engine, 0, "work", cJSON_Parse("[2]"), &got));
    CHECK(w.count == 2);
    clarity_watching(was);
    // Outside it, nothing is watched.
    CHECK_OK(clarity_engine_call_in(engine, 0, "work", cJSON_Parse("[1]"), &got));
    CHECK(w.count == 2);
    w.stop = 1;
    clarity_watching(&watch);
    CHECK_FAILS(CLARITY_ENGINE, clarity_engine_call_in(engine, 0, "work", cJSON_Parse("[5]"), &got));
    CHECK_TEXT(clarity_error()->message, "engine Long failed in work: it was stopped");
    clarity_watching(NULL);
    clarity_engine_free(engine);
}
