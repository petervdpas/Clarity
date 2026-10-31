// A wizard's walk through its steps, against the cases every runtime
// shares (conformance/steps.json, `walk`); `step` and `set` are in values_test.c.

#include "check.h"
#include "clarity/steps.h"
#include "tests.h"

static bool holds(const cJSON* numbers, int64_t n) {
    const cJSON* item;
    cJSON_ArrayForEach(item, numbers) {
        if ((int64_t)item->valuedouble == n) return true;
    }
    return false;
}

void a_wizard_walks_its_steps_as_every_runtime_does(void) {
    cJSON* cases = conformance("steps.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "walk")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        int64_t count = (int64_t)cJSON_GetObjectItem(c, "steps")->valuedouble;
        const cJSON* start = cJSON_GetObjectItem(c, "start");
        int64_t step = start ? (int64_t)start->valuedouble : 1;
        const cJSON* handler = cJSON_GetObjectItem(c, "handler");
        const cJSON* jumps = cJSON_GetObjectItem(handler, "jumps");
        cJSON* ran = cJSON_CreateArray();
        const cJSON* button;
        cJSON_ArrayForEach(button, cJSON_GetObjectItem(c, "do")) {
            int64_t shown = clarity_shown_step(step, count);
            if (strcmp(button->valuestring, "back") == 0) {
                step = clarity_step_back(step, count);
            } else if (strcmp(button->valuestring, "finish") == 0) {
                cJSON_AddItemToArray(ran, cJSON_CreateString(clarity_format("finish %lld", (long long)shown)));
            } else {
                // The handler runs on the step shown; a refusal keeps the step.
                int64_t now = shown;
                if (!cJSON_IsNull(handler)) {
                    cJSON_AddItemToArray(ran, cJSON_CreateString(clarity_format("next %lld", (long long)shown)));
                    if (holds(cJSON_GetObjectItem(handler, "refuses"), shown)) continue;
                    if (jumps && (int64_t)cJSON_GetObjectItem(jumps, "from")->valuedouble == shown) now = (int64_t)cJSON_GetObjectItem(jumps, "to")->valuedouble;
                }
                step = clarity_step_after_next(shown, now, count);
            }
        }
        const cJSON* expect = cJSON_GetObjectItem(c, "expect");
        int64_t shown = clarity_shown_step(step, count);
        if (shown != (int64_t)cJSON_GetObjectItem(expect, "shown")->valuedouble)
            check_fail(__FILE__, __LINE__, clarity_format("%s: shows step %lld", name, (long long)shown));
        const char* names[2];
        size_t n = clarity_step_buttons(shown, count, names);
        cJSON* buttons = cJSON_CreateArray();
        for (size_t i = 0; i < n; ++i) cJSON_AddItemToArray(buttons, cJSON_CreateString(names[i]));
        if (!check_json(buttons, cJSON_PrintUnformatted(cJSON_GetObjectItem(expect, "buttons"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: buttons %s", name, cJSON_PrintUnformatted(buttons)));
        if (!check_json(ran, cJSON_PrintUnformatted(cJSON_GetObjectItem(expect, "ran"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: ran %s", name, cJSON_PrintUnformatted(ran)));
    }
    cJSON_Delete(cases);
}
