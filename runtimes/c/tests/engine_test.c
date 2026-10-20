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
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, count, functions, used, uses, limits(file), &engine));
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
        CHECK_FAILS(CLARITY_ENGINE, clarity_engine_load("Test", data, size, count, functions, used, uses, limits(file), &engine));
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
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, count, functions, used, uses, limits(file), &engine));
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
    REQUIRE_OK(clarity_engine_load("Test", bytes.data, bytes.size, count, functions, used, uses, limits(file), &engine));
    const cJSON* services = cJSON_GetObjectItem(file, "services");
    clarity_engine_serve(engine, (ClarityEngineServices){.fixed_clock = true,
                                                         .clock = (int64_t)cJSON_GetObjectItem(services, "clock")->valuedouble,
                                                         .fixed_seed = true,
                                                         .seed = (uint64_t)cJSON_GetObjectItem(services, "seed")->valuedouble,
                                                         .log = keep_line});
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
