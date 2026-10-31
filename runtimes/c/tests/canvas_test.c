// Canvas modes (DESIGN 5.31), against the cases every runtime shares
// (conformance/canvas-modes.json).

#include "check.h"
#include "clarity/canvas.h"
#include "tests.h"

static const char* text_or_empty(const cJSON* c, const char* name) {
    const cJSON* v = cJSON_GetObjectItem(c, name);
    return cJSON_IsString(v) ? v->valuestring : "";
}

/// The units a case names in `units`; false when it names none.
static bool units_of(const cJSON* c, int units[2]) {
    const cJSON* u = cJSON_GetObjectItem(c, "units");
    if (!cJSON_IsArray(u)) return false;
    units[0] = (int)cJSON_GetArrayItem(u, 0)->valuedouble;
    units[1] = (int)cJSON_GetArrayItem(u, 1)->valuedouble;
    return true;
}

void a_drawing_is_svg_text_as_every_runtime_writes_it(void) {
    cJSON* cases = conformance("canvas-modes.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "svg")) {
        const char* name = text_or_empty(c, "name");
        int units[2];
        units_of(c, units);
        const cJSON* shapes = cJSON_GetObjectItem(c, "shapes");
        const char* text = clarity_drawing_text(units, shapes);
        if (strcmp(text, text_or_empty(c, "text")) != 0) check_fail(__FILE__, __LINE__, clarity_format("%s: wrote\n%s", name, text));
        int read_units[2] = {1000, 1000};
        cJSON* read = clarity_drawing_shapes(text_or_empty(c, "text"), read_units);
        if (read_units[0] != units[0] || read_units[1] != units[1] || !check_json(read, cJSON_PrintUnformatted(shapes)))
            check_fail(__FILE__, __LINE__, clarity_format("%s: read %d %d %s", name, read_units[0], read_units[1], cJSON_PrintUnformatted(read)));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "read")) {
        const char* name = text_or_empty(c, "name");
        int units[2] = {1000, 1000}, want[2];
        cJSON* read = clarity_drawing_shapes(text_or_empty(c, "text"), units);
        if (units_of(c, want) && (units[0] != want[0] || units[1] != want[1]))
            check_fail(__FILE__, __LINE__, clarity_format("%s: units %d %d", name, units[0], units[1]));
        if (!check_json(read, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "shapes"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: read %s", name, cJSON_PrintUnformatted(read)));
    }
    cJSON_Delete(cases);
}

void a_canvas_acts_as_every_runtime_does(void) {
    cJSON* cases = conformance("canvas-modes.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "actions")) {
        const char* name = text_or_empty(c, "name");
        int units[2];
        const cJSON* at = cJSON_GetObjectItem(c, "at");
        const cJSON* grid = cJSON_GetObjectItem(c, "grid");
        ClarityCanvas canvas = {
            .mode = text_or_empty(c, "mode"),
            .list = text_or_empty(c, "list"),
            .at = {at ? cJSON_GetArrayItem(at, 0)->valuestring : "", at ? cJSON_GetArrayItem(at, 1)->valuestring : ""},
            .path = text_or_empty(c, "path"),
            .units = units_of(c, units) ? units : NULL,
            .grid = cJSON_IsNumber(grid) ? (int64_t)grid->valuedouble : 0,
            .snap = cJSON_IsTrue(cJSON_GetObjectItem(c, "snap")),
            .readonly = cJSON_IsTrue(cJSON_GetObjectItem(c, "readonly")),
        };
        cJSON* data = cJSON_Duplicate(cJSON_GetObjectItem(c, "data"), true);
        ClarityCanvasTouched touched = {0};
        ClarityCanvasDone done = clarity_canvas_event(data, &canvas, cJSON_GetObjectItem(c, "action"), &touched);
        if (done == CLARITY_CANVAS_FAILED) {
            check_fail(__FILE__, __LINE__, clarity_format("%s: %s", name, clarity_error()->message));
            continue;
        }
        if (cJSON_IsTrue(cJSON_GetObjectItem(c, "refused"))) {
            if (done != CLARITY_CANVAS_REFUSED) check_fail(__FILE__, __LINE__, clarity_format("%s: not refused", name));
            if (!check_json(data, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "data"))))
                check_fail(__FILE__, __LINE__, clarity_format("%s: a refused action changed the data", name));
            continue;
        }
        if (done != CLARITY_CANVAS_CHANGED) {
            check_fail(__FILE__, __LINE__, clarity_format("%s: refused", name));
            continue;
        }
        if (!check_json(data, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "after"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: data %s", name, cJSON_PrintUnformatted(data)));
        const cJSON* selected = cJSON_GetObjectItem(c, "selected");
        bool selects = cJSON_IsNumber(selected);
        if (selects != touched.selects || (selects && (int)selected->valuedouble != touched.selected))
            check_fail(__FILE__, __LINE__, clarity_format("%s: selected %d %d", name, touched.selects, touched.selected));
    }
    cJSON_Delete(cases);
}
