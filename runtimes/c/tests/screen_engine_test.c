// Engines beside the screen on C, as Clarity.UI runs them in the program's
// process: one instance per canvas, against the cases every host shares
// (conformance/screen-engines.json).

#include <string.h>

#include "check.h"
#include "clarity/disk.h"
#include "engine/screen.h"
#include "tests.h"

static ClarityBytes engine_bytes(const char* file) {
    ClarityBytes bytes = {0};
    CHECK_OK(clarity_read_file(clarity_format("%s/engines/%s", CLARITY_CONFORMANCE, file), &bytes));
    return bytes;
}

static const char* const screen_uses[] = {"log"};

static ClarityScreenEngine* screen(bool opens, uint32_t seconds) {
    ClarityBytes bytes = engine_bytes("screen.wasm");
    ClarityScreenEngine* engine = NULL;
    CHECK_OK(clarity_screen_engine_load("Screen", bytes.data, bytes.size, 1, screen_uses, opens, (ClarityEngineLimits){seconds, 256}, &engine));
    return engine;
}

static bool opens_of(const cJSON* run) { return cJSON_IsTrue(cJSON_GetObjectItem(run, "opens")); }

/// The input as the conformance file writes it, as Clarity.UI sends it.
static cJSON* input_of(const cJSON* given) {
    cJSON* out = cJSON_CreateArray();
    const cJSON* i;
    if (!cJSON_IsArray(given)) given = NULL;
    cJSON_ArrayForEach(i, given) {
        cJSON* item = cJSON_CreateObject();
        cJSON* inner;
        if (cJSON_GetObjectItem(i, "pointer")) {
            inner = cJSON_AddObjectToObject(item, "pointer");
            cJSON_AddItemToObject(inner, "kind", cJSON_Duplicate(cJSON_GetObjectItem(i, "pointer"), true));
            cJSON_AddItemToObject(inner, "x", cJSON_Duplicate(cJSON_GetObjectItem(i, "x"), true));
            cJSON_AddItemToObject(inner, "y", cJSON_Duplicate(cJSON_GetObjectItem(i, "y"), true));
        } else if (cJSON_GetObjectItem(i, "hear")) {
            inner = cJSON_AddObjectToObject(item, "hear");
            cJSON_AddItemToObject(inner, "name", cJSON_Duplicate(cJSON_GetObjectItem(i, "hear"), true));
            const cJSON* values = cJSON_GetObjectItem(i, "values");
            cJSON_AddItemToObject(inner, "values", values ? cJSON_Duplicate(values, true) : cJSON_CreateArray());
        } else {
            inner = cJSON_AddObjectToObject(item, "key");
            cJSON_AddItemToObject(inner, "kind", cJSON_Duplicate(cJSON_GetObjectItem(i, "key"), true));
            cJSON_AddItemToObject(inner, "name", cJSON_Duplicate(cJSON_GetObjectItem(i, "name"), true));
        }
        cJSON_AddItemToArray(out, item);
    }
    return out;
}

/// Whether `got` equals `want`, both JSON.
static void check_same(const cJSON* got, const cJSON* want, const char* what) {
    if (!got || !want || !cJSON_Compare(got, want, 1))
        check_fail(__FILE__, __LINE__, clarity_format("%s: %s != %s", what, got ? cJSON_PrintUnformatted(got) : "(none)", want ? cJSON_PrintUnformatted(want) : "(none)"));
}

void each_frame_gives_the_canvas_its_shapes(void) {
    const cJSON* run;
    cJSON_ArrayForEach(run, cJSON_GetObjectItem(conformance("screen-engines.json"), "frames")) {
        ClarityScreenEngine* engine = screen(opens_of(run), 5);
        REQUIRE(engine != NULL);
        const cJSON* ticks = cJSON_GetObjectItem(run, "ticks");
        for (int i = 0; i < cJSON_GetArraySize(ticks); ++i) {
            cJSON* drawn = NULL;
            CHECK_OK(clarity_screen_engine_frame(engine, cJSON_GetArrayItem(ticks, i)->valuedouble, cJSON_CreateArray(), &drawn));
            check_same(cJSON_GetObjectItem(drawn, "shapes"), cJSON_GetArrayItem(cJSON_GetObjectItem(run, "draws"), i), cJSON_GetObjectItem(run, "name")->valuestring);
        }
        clarity_screen_engine_free(engine);
    }
}

void a_frame_that_fails_or_runs_too_long_ends_the_engine(void) {
    const cJSON* fails;
    cJSON_ArrayForEach(fails, cJSON_GetObjectItem(conformance("screen-engines.json"), "fails")) {
        ClarityScreenEngine* engine = screen(true, (uint32_t)cJSON_GetObjectItem(fails, "seconds")->valueint);
        REQUIRE(engine != NULL);
        const cJSON* tick;
        cJSON* drawn = NULL;
        cJSON_ArrayForEach(tick, cJSON_GetObjectItem(fails, "ticks")) CHECK_OK(clarity_screen_engine_frame(engine, tick->valuedouble, cJSON_CreateArray(), &drawn));
        const char* expected = cJSON_GetObjectItem(fails, "fails")->valuestring;
        CHECK(!clarity_screen_engine_frame(engine, cJSON_GetObjectItem(fails, "tick")->valuedouble, input_of(cJSON_GetObjectItem(fails, "input")), &drawn));
        CHECK_TEXT(clarity_error()->message, expected);
        // Ended, it draws no more.
        CHECK(!clarity_screen_engine_frame(engine, 16, cJSON_CreateArray(), &drawn));
        CHECK_TEXT(clarity_error()->message, expected);
        clarity_screen_engine_free(engine);
    }
}

void pointer_keys_and_hears_reach_the_engine_before_its_tick_and_it_tells(void) {
    cJSON* cases = conformance("screen-engines.json");
    const char* groups[] = {"input", "tells"};
    for (size_t g = 0; g < 2; ++g) {
        const cJSON* run;
        cJSON_ArrayForEach(run, cJSON_GetObjectItem(cases, groups[g])) {
            ClarityScreenEngine* engine = screen(opens_of(run), 5);
            REQUIRE(engine != NULL);
            CHECK(clarity_screen_engine_takes_pointer(engine) && clarity_screen_engine_takes_keys(engine));
            const cJSON* frame;
            cJSON_ArrayForEach(frame, cJSON_GetObjectItem(run, "frames")) {
                cJSON* drawn = NULL;
                CHECK_OK(clarity_screen_engine_frame(engine, cJSON_GetObjectItem(frame, "tick")->valuedouble, input_of(cJSON_GetObjectItem(frame, "input")), &drawn));
                check_same(cJSON_GetObjectItem(drawn, "shapes"), cJSON_GetObjectItem(frame, "draws"), cJSON_GetObjectItem(run, "name")->valuestring);
                if (cJSON_GetObjectItem(frame, "tells")) check_same(cJSON_GetObjectItem(drawn, "tells"), cJSON_GetObjectItem(frame, "tells"), cJSON_GetObjectItem(run, "name")->valuestring);
            }
            clarity_screen_engine_free(engine);
        }
    }
}

void an_engine_beside_the_screen_written_in_clarity_runs(void) {
    const cJSON* written = cJSON_GetObjectItem(conformance("screen-engines.json"), "written");
    const cJSON* file;
    cJSON_ArrayForEach(file, cJSON_GetObjectItem(written, "engines")) {
        ClarityBytes bytes = engine_bytes(file->valuestring);
        ClarityScreenEngine* engine = NULL;
        CHECK_OK(clarity_screen_engine_load(cJSON_GetObjectItem(written, "name")->valuestring, bytes.data, bytes.size, 0, NULL, false, (ClarityEngineLimits){5, 256}, &engine));
        REQUIRE(engine != NULL);
        const cJSON* frame;
        cJSON_ArrayForEach(frame, cJSON_GetObjectItem(written, "frames")) {
            cJSON* drawn = NULL;
            CHECK_OK(clarity_screen_engine_frame(engine, cJSON_GetObjectItem(frame, "tick")->valuedouble, input_of(cJSON_GetObjectItem(frame, "input")), &drawn));
            check_same(cJSON_GetObjectItem(drawn, "tells"), cJSON_GetObjectItem(frame, "tells"), file->valuestring);
            check_same(cJSON_GetObjectItem(drawn, "shapes"), cJSON_GetObjectItem(frame, "draws"), file->valuestring);
        }
        clarity_screen_engine_free(engine);
    }
}

static void keep_line(const char* line, void* context) { cJSON_AddItemToArray(context, cJSON_CreateString(line)); }

void an_engine_beside_the_screen_logs_and_exports_its_frame_loop(void) {
    const cJSON* run;
    cJSON_ArrayForEach(run, cJSON_GetObjectItem(conformance("screen-engines.json"), "logs")) {
        cJSON* lines = cJSON_CreateArray();
        ClarityScreenEngine* engine = screen(opens_of(run), 5);
        REQUIRE(engine != NULL);
        clarity_screen_engine_serve(engine, (ClarityEngineServices){.log = keep_line, .log_context = lines});
        cJSON* drawn = NULL;
        CHECK_OK(clarity_screen_engine_frame(engine, 0, input_of(cJSON_GetObjectItem(run, "input")), &drawn));
        check_same(lines, cJSON_GetObjectItem(run, "lines"), cJSON_GetObjectItem(run, "name")->valuestring);
        clarity_screen_engine_free(engine);
    }
    ClarityBytes bytes = engine_bytes("test.wasm");
    const char* const uses[] = {"clock", "random", "log", "language"};
    ClarityScreenEngine* engine = NULL;
    CHECK(!clarity_screen_engine_load("Test", bytes.data, bytes.size, 4, uses, false, (ClarityEngineLimits){5, 256}, &engine));
    CHECK_TEXT(clarity_error()->message, "engine Test lacks clarity_tick, which an engine beside the screen needs");
}

static bool make_screen(const char* name, ClarityScreenEngine** out) {
    *out = NULL;
    if (strcmp(name, "Screen") != 0) return true;
    ClarityBytes bytes = engine_bytes("screen.wasm");
    return clarity_screen_engine_load("Screen", bytes.data, bytes.size, 1, screen_uses, true, (ClarityEngineLimits){5, 256}, out);
}

static cJSON* asked(ClarityScreenInstances* open, const char* command, const char* args) { return clarity_screen_answer(open, command, cJSON_Parse(args)); }

// Clarity.UI's commands: an instance by number, its frames, its end; a
// name the program has no engine of is left to the library.
void instances_go_by_number_and_a_gone_one_says_so(void) {
    ClarityScreenInstances* open = clarity_screen_instances_new(make_screen);
    REQUIRE(open != NULL);
    CHECK(asked(open, "engine_open", "{\"name\":\"Other\"}") == NULL);
    CHECK(asked(open, "dispatch_event", "{}") == NULL);
    cJSON* opened = asked(open, "engine_open", "{\"name\":\"Screen\"}");
    CHECK_JSON(opened, "{\"value\":1}");
    CHECK_JSON(asked(open, "engine_hears", "{\"id\":1}"), "{\"value\":{\"pointer\":true,\"keys\":true}}");
    CHECK_INT(clarity_screen_count(open), 1);
    cJSON* drawn = asked(open, "engine_frame", "{\"id\":1,\"ms\":0,\"input\":[]}");
    CHECK(cJSON_IsArray(cJSON_GetObjectItem(cJSON_GetObjectItem(drawn, "value"), "shapes")));
    CHECK_JSON(asked(open, "engine_close", "{\"id\":1}"), "{\"value\":null}");
    CHECK_JSON(asked(open, "engine_frame", "{\"id\":1,\"ms\":16,\"input\":[]}"), "{\"error\":\"The engine ended.\"}");
    CHECK_JSON(asked(open, "engine_hears", "{\"id\":1}"), "{\"value\":null}");
    CHECK_INT(clarity_screen_count(open), 0);
    clarity_screen_instances_free(open);
}
