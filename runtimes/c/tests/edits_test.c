// The edit bus (DESIGN 5.29) against the cases every runtime shares
// (conformance/edit-bus.json).

#include "check.h"
#include "clarity/edits.h"
#include "tests.h"

/// The database steps of a step of a case, in the scratch arena.
static int64_t* steps_of(const cJSON* given, size_t* count) {
    *count = (size_t)cJSON_GetArraySize(given);
    int64_t* steps = CLARITY_NEW(int64_t, *count);
    size_t i = 0;
    const cJSON* step;
    cJSON_ArrayForEach(step, given) steps[i++] = (int64_t)step->valuedouble;
    return steps;
}

/// What a take was given, and whether it refuses.
typedef struct {
    bool refuse;
    const int64_t* steps;
    size_t count;
} Taken;

static bool take(const int64_t* steps, size_t count, bool back, void* context) {
    (void)back;
    Taken* t = context;
    if (t->refuse) return clarity_fail(CLARITY_STORAGE, "changed since");
    t->steps = steps;
    t->count = count;
    return true;
}

void the_edit_bus_keeps_the_record_as_every_runtime_does(void) {
    cJSON* file = conformance("edit-bus.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        ClarityEdits* bus = clarity_edits_new();
        const cJSON* data = cJSON_GetObjectItem(c, "start");
        clarity_edits_reset(bus, data);
        int at = 0;
        const cJSON* s;
        cJSON_ArrayForEach(s, cJSON_GetObjectItem(c, "steps")) {
            ++at;
            const cJSON* made;
            size_t count;
            if ((made = cJSON_GetObjectItem(s, "edit"))) {
                const int64_t* steps = steps_of(cJSON_GetObjectItem(made, "steps"), &count);
                data = cJSON_GetObjectItem(made, "data");
                clarity_edits_edit(bus, cJSON_GetObjectItem(made, "part")->valuestring, cJSON_GetObjectItem(made, "scope")->valuestring, data, steps, count);
                continue;
            }
            if ((made = cJSON_GetObjectItem(s, "drag"))) {
                const int64_t* steps = steps_of(cJSON_GetObjectItem(made, "steps"), &count);
                data = cJSON_GetObjectItem(made, "data");
                clarity_edits_drag(bus, cJSON_GetObjectItem(made, "part")->valuestring, cJSON_GetObjectItem(made, "scope")->valuestring, steps, count);
                continue;
            }
            if ((made = cJSON_GetObjectItem(s, "reply"))) {
                data = cJSON_GetObjectItem(made, "data");
                clarity_edits_reset(bus, data);
                continue;
            }
            bool back = cJSON_GetObjectItem(s, "undo") != NULL;
            const char* scope = cJSON_GetObjectItem(s, back ? "undo" : "redo")->valuestring;
            Taken taken = {cJSON_GetObjectItem(s, "refused") != NULL, NULL, 0};
            bool found = false;
            cJSON* after = NULL;
            bool took = clarity_edits_undo(bus, scope, data, back, take, &taken, &found, &after);
            if (cJSON_GetObjectItem(s, "nothing")) {
                if (found) check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: something to take back", name, at));
                continue;
            }
            if (taken.refuse) {
                if (took) check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: not refused", name, at));
                continue;
            }
            if (!took || !found) {
                check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: nothing taken back", name, at));
                continue;
            }
            if (!cJSON_Compare(after, cJSON_GetObjectItem(s, "data"), 1))
                check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: data %s", name, at, cJSON_PrintUnformatted(after)));
            const int64_t* want = steps_of(cJSON_GetObjectItem(s, "steps"), &count);
            bool same = taken.count == count;
            for (size_t i = 0; same && i < count; ++i) same = taken.steps[i] == want[i];
            if (!same) check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: steps differ", name, at));
            data = after;
        }
        clarity_edits_free(bus);
    }
}

// A part's own text and tick travel with the window's data, and come back from it.
void a_parts_own_text_and_tick_are_edit_data(void) {
    ClarityWindowState state = {0};
    REQUIRE(clarity_window_state_from_json(
        cJSON_Parse("{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"},\"parts\":{\"pinBox\":{\"text\":\"12\"},\"okBox\":{\"checked\":true},\"title\":{}}}"), &state));
    cJSON* data = clarity_edit_data(&state);
    CHECK_JSON(data, "{\"name\":\"Ada\",\"#parts.pinBox\":{\"text\":\"12\"},\"#parts.okBox\":{\"checked\":true}}");
    ClarityWindowState back = clarity_with_edit_data(&state, cJSON_Parse("{\"name\":\"Ann\",\"#parts.pinBox\":{\"text\":\"\"}}"));
    CHECK_JSON(clarity_window_state_json(&back),
               "{\"window\":\"Card\",\"instance\":0,\"data\":{\"name\":\"Ann\"},\"parts\":{\"pinBox\":{\"text\":\"\"},\"okBox\":{},\"title\":{}},\"selected\":{}}");
}

/// A window state of a case (`data` and `parts`), in the scratch arena.
static ClarityWindowState state_of(const cJSON* given) {
    cJSON* json = cJSON_Duplicate(given, true);
    cJSON_AddStringToObject(json, "window", "Case");
    ClarityWindowState state = {0};
    if (!clarity_window_state_from_json(json, &state)) check_fail(__FILE__, __LINE__, "a case's state does not read");
    return state;
}

// The edit bus on whole window states (conformance/edit-bus.json, `states`):
// what code set on parts is compared and put back as data is.
void the_edit_bus_puts_back_what_code_set_on_parts(void) {
    cJSON* file = conformance("edit-bus.json");
    REQUIRE(file);
    REQUIRE(cJSON_GetArraySize(cJSON_GetObjectItem(file, "states")) > 0);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "states")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        ClarityEdits* bus = clarity_edits_new();
        ClarityWindowState state = state_of(cJSON_GetObjectItem(c, "start"));
        clarity_edits_reset(bus, clarity_edit_data(&state));
        int at = 0;
        const cJSON* s;
        cJSON_ArrayForEach(s, cJSON_GetObjectItem(c, "steps")) {
            ++at;
            const cJSON* made;
            if ((made = cJSON_GetObjectItem(s, "edit"))) {
                state = state_of(cJSON_GetObjectItem(made, "state"));
                clarity_edits_edit(bus, cJSON_GetObjectItem(made, "part")->valuestring, cJSON_GetObjectItem(made, "scope")->valuestring,
                                   clarity_edit_data(&state), NULL, 0);
                continue;
            }
            if ((made = cJSON_GetObjectItem(s, "reply"))) {
                state = state_of(cJSON_GetObjectItem(made, "state"));
                clarity_edits_reset(bus, clarity_edit_data(&state));
                continue;
            }
            bool back = cJSON_GetObjectItem(s, "undo") != NULL;
            const char* scope = cJSON_GetObjectItem(s, back ? "undo" : "redo")->valuestring;
            Taken taken = {false, NULL, 0};
            bool found = false;
            cJSON* after = NULL;
            if (!clarity_edits_undo(bus, scope, clarity_edit_data(&state), back, take, &taken, &found, &after) || !found) {
                check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: nothing taken back", name, at));
                continue;
            }
            state = clarity_with_edit_data(&state, after);
            cJSON* got = clarity_window_state_json(&state);
            const cJSON* want = cJSON_GetObjectItem(s, "state");
            if (!cJSON_Compare(cJSON_GetObjectItem(got, "data"), cJSON_GetObjectItem(want, "data"), 1))
                check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: data %s", name, at, cJSON_PrintUnformatted(got)));
            const cJSON* part;
            cJSON_ArrayForEach(part, cJSON_GetObjectItem(want, "parts")) {
                const cJSON* has = cJSON_GetObjectItem(cJSON_GetObjectItem(got, "parts"), part->string);
                const cJSON* field;
                cJSON_ArrayForEach(field, part) {
                    const cJSON* value = cJSON_GetObjectItem(has, field->string);
                    if (strcmp(field->string, "values") == 0) {
                        const cJSON* v;
                        cJSON_ArrayForEach(v, field) {
                            if (!cJSON_Compare(cJSON_GetObjectItem(value, v->string), v, 1))
                                check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: %s.values.%s in %s", name, at, part->string, v->string, cJSON_PrintUnformatted(has)));
                        }
                        continue;
                    }
                    bool same = cJSON_IsNull(field) ? value == NULL : value && cJSON_Compare(value, field, 1);
                    if (!same) check_fail(__FILE__, __LINE__, clarity_format("%s, step %d: %s.%s in %s", name, at, part->string, field->string, cJSON_PrintUnformatted(has)));
                }
            }
        }
        clarity_edits_free(bus);
    }
}
