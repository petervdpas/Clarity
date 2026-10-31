#include "clarity/canvas.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/picture.h"
#include "clarity/views.h"

static const int default_units[2] = {1000, 1000};

#define SVG_HEAD "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 %d %d\">"

static bool number_of(const cJSON* v, double* out) {
    if (cJSON_IsNumber(v)) {
        *out = v->valuedouble;
        return true;
    }
    if (cJSON_IsString(v) && *v->valuestring) {
        char* end;
        *out = strtod(v->valuestring, &end);
        return *end == '\0';
    }
    return false;
}

static double number_or(const cJSON* object, const char* name, double otherwise) {
    double n;
    return number_of(cJSON_GetObjectItemCaseSensitive(object, name), &n) ? n : otherwise;
}

static int64_t whole(double v) { return (int64_t)round(v); }

static const char* string_of(const cJSON* object, const char* name) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(object, name);
    return cJSON_IsString(v) ? v->valuestring : "";
}

static void put(cJSON* object, const char* name, cJSON* value) {
    if (cJSON_GetObjectItemCaseSensitive(object, name)) cJSON_ReplaceItemInObjectCaseSensitive(object, name, value);
    else cJSON_AddItemToObject(object, name, value);
}

static void put_number(cJSON* object, const char* name, int64_t n) { put(object, name, cJSON_CreateNumber((double)n)); }

/// Puts `value` at `path` of the window's data; an error when the record it is in is not there.
static bool put_at_path(cJSON* data, const char* path, cJSON* value) {
    const char* dot = strrchr(path, '.');
    cJSON* record = dot ? clarity_data_at(data, clarity_copy_n(path, (size_t)(dot - path))) : data;
    if (!*path || !cJSON_IsObject(record)) return clarity_fail(CLARITY_INVALID, "the window has no %s", path);
    put(record, dot ? dot + 1 : path, value);
    return true;
}

/// A place on the canvas: snapped to the grid when it snaps, else a whole number.
static int64_t placed(const ClarityCanvas* canvas, double v) {
    if (canvas->snap && canvas->grid > 0) return whole(v / (double)canvas->grid) * canvas->grid;
    return whole(v);
}

static void escaped(ClarityBuffer* out, const char* text) {
    for (const char* c = text; *c; ++c) {
        switch (*c) {
        case '&': clarity_buffer_add(out, "&amp;"); break;
        case '<': clarity_buffer_add(out, "&lt;"); break;
        case '>': clarity_buffer_add(out, "&gt;"); break;
        case '"': clarity_buffer_add(out, "&quot;"); break;
        default: clarity_buffer_add_char(out, *c);
        }
    }
}

static const char* escape(const char* text) {
    ClarityBuffer out = {0};
    escaped(&out, text);
    return clarity_buffer_text(&out);
}

static char* unescape_n(const char* text, size_t length) {
    static const char* const entities[][2] = {{"&lt;", "<"}, {"&gt;", ">"}, {"&quot;", "\""}, {"&amp;", "&"}};
    ClarityBuffer out = {0};
    for (size_t i = 0; i < length;) {
        size_t e = 0;
        for (; e < sizeof entities / sizeof *entities; ++e) {
            size_t n = strlen(entities[e][0]);
            if (i + n <= length && strncmp(text + i, entities[e][0], n) == 0) {
                clarity_buffer_add(&out, entities[e][1]);
                i += n;
                break;
            }
        }
        if (e == sizeof entities / sizeof *entities) clarity_buffer_add_char(&out, text[i++]);
    }
    return clarity_copy(clarity_buffer_text(&out));
}

/// One shape's line of a drawing; a line it does not read is its kept text.
static const char* shape_line(const cJSON* shape) {
    const char* op = string_of(shape, "op");
    const char* color = escape(string_of(shape, "color"));
#define N(name) (long long)whole(number_or(shape, name, 0))
    if (strcmp(op, "rect") == 0)
        return clarity_format("<rect x=\"%lld\" y=\"%lld\" width=\"%lld\" height=\"%lld\" fill=\"%s\"/>", N("x"), N("y"), N("width"), N("height"), color);
    if (strcmp(op, "line") == 0)
        return clarity_format("<line x1=\"%lld\" y1=\"%lld\" x2=\"%lld\" y2=\"%lld\" stroke=\"%s\" stroke-width=\"%lld\"/>", N("x1"), N("y1"), N("x2"), N("y2"), color, N("width"));
    if (strcmp(op, "ellipse") == 0)
        return clarity_format("<ellipse cx=\"%lld\" cy=\"%lld\" rx=\"%lld\" ry=\"%lld\" fill=\"%s\"/>", N("x"), N("y"), N("rx"), N("ry"), color);
    if (strcmp(op, "path") == 0) {
        const cJSON* points = cJSON_GetObjectItemCaseSensitive(shape, "points");
        int count = cJSON_IsArray(points) ? cJSON_GetArraySize(points) : 0;
        ClarityBuffer pairs = {0};
        for (int i = 0; i + 1 < count; i += 2) {
            double x = 0, y = 0;
            number_of(cJSON_GetArrayItem(points, i), &x);
            number_of(cJSON_GetArrayItem(points, i + 1), &y);
            clarity_buffer_addf(&pairs, "%s%lld,%lld", i ? " " : "", (long long)whole(x), (long long)whole(y));
        }
        return clarity_format("<polyline points=\"%s\" fill=\"none\" stroke=\"%s\" stroke-width=\"%lld\"/>", clarity_buffer_text(&pairs), color, N("width"));
    }
    if (strcmp(op, "text") == 0)
        return clarity_format("<text x=\"%lld\" y=\"%lld\" fill=\"%s\" font-size=\"%lld\">%s</text>", N("x"), N("y"), color, N("size"), escape(string_of(shape, "text")));
#undef N
    return string_of(shape, "text");
}

const char* clarity_drawing_text(const int units[2], const cJSON* shapes) {
    ClarityBuffer out = {0};
    clarity_buffer_addf(&out, SVG_HEAD "\n", units[0], units[1]);
    const cJSON* shape;
    cJSON_ArrayForEach(shape, shapes) {
        clarity_buffer_add(&out, shape_line(shape));
        clarity_buffer_add_char(&out, '\n');
    }
    clarity_buffer_add(&out, "</svg>\n");
    return clarity_buffer_text(&out);
}

/// An element of one line, `<name a="1" b="2"/>` or `<name a="1">content</name>`:
/// its attributes as an object of texts, NULL when the line is no such element.
static cJSON* svg_element(const char* line, const char** name, const char** content) {
    if (*line != '<') return NULL;
    const char* rest = line + 1;
    size_t end = strcspn(rest, " />");
    if (end == 0 || !rest[end]) return NULL;
    *name = clarity_copy_n(rest, end);
    *content = "";
    rest += end;
    cJSON* attrs = cJSON_CreateObject();
    for (;;) {
        while (*rest == ' ') ++rest;
        if (strncmp(rest, "/>", 2) == 0) return strcmp(rest, "/>") == 0 ? attrs : NULL;
        if (*rest == '>') {
            const char* close = clarity_format("</%s>", *name);
            size_t length = strlen(rest + 1), tail = strlen(close);
            if (length < tail || strcmp(rest + 1 + length - tail, close) != 0) return NULL;
            *content = unescape_n(rest + 1, length - tail);
            return attrs;
        }
        const char* eq = strstr(rest, "=\"");
        if (!eq || eq == rest) return NULL;
        const char* quote = strchr(eq + 2, '"');
        if (!quote) return NULL;
        put(attrs, clarity_copy_n(rest, (size_t)(eq - rest)), cJSON_CreateString(unescape_n(eq + 2, (size_t)(quote - eq - 2))));
        rest = quote + 1;
    }
}

static bool whole_text(const char* text, int64_t* out) {
    if (!*text) return false;
    char* end;
    errno = 0;
    long long n = strtoll(text, &end, 10);
    if (*end || errno) return false;
    *out = n;
    return true;
}

/// Copies the whole-number attributes named in pairs (attribute, field) into shape.
static bool picked(cJSON* shape, const cJSON* attrs, const char* const* pairs, size_t count) {
    for (size_t i = 0; i + 1 < count; i += 2) {
        int64_t n;
        if (!whole_text(string_of(attrs, pairs[i]), &n)) return false;
        put_number(shape, pairs[i + 1], n);
    }
    return true;
}

#define PICK(shape, attrs, ...) picked(shape, attrs, (const char* const[]){__VA_ARGS__}, sizeof((const char* const[]){__VA_ARGS__}) / sizeof(const char*))

/// The shape of one line of a drawing: written back, it must be the line
/// again, or the line is kept as it is.
static cJSON* read_shape(const char* line) {
    const char *name, *content;
    cJSON* attrs = svg_element(line, &name, &content);
    cJSON* shape = cJSON_CreateObject();
    bool fits = false;
    if (attrs && strcmp(name, "rect") == 0) {
        cJSON_AddStringToObject(shape, "op", "rect");
        cJSON_AddStringToObject(shape, "color", string_of(attrs, "fill"));
        fits = PICK(shape, attrs, "x", "x", "y", "y", "width", "width", "height", "height");
    } else if (attrs && strcmp(name, "line") == 0) {
        cJSON_AddStringToObject(shape, "op", "line");
        cJSON_AddStringToObject(shape, "color", string_of(attrs, "stroke"));
        fits = PICK(shape, attrs, "x1", "x1", "y1", "y1", "x2", "x2", "y2", "y2", "stroke-width", "width");
    } else if (attrs && strcmp(name, "ellipse") == 0) {
        cJSON_AddStringToObject(shape, "op", "ellipse");
        cJSON_AddStringToObject(shape, "color", string_of(attrs, "fill"));
        fits = PICK(shape, attrs, "cx", "x", "cy", "y", "rx", "rx", "ry", "ry");
    } else if (attrs && strcmp(name, "polyline") == 0) {
        cJSON_AddStringToObject(shape, "op", "path");
        cJSON_AddStringToObject(shape, "color", string_of(attrs, "stroke"));
        fits = PICK(shape, attrs, "stroke-width", "width");
        cJSON* points = cJSON_AddArrayToObject(shape, "points");
        const char* at = string_of(attrs, "points");
        while (*at) {
            while (*at == ' ') ++at;
            if (!*at) break;
            size_t length = strcspn(at, " ");
            const char* pair = clarity_copy_n(at, length);
            const char* comma = strchr(pair, ',');
            int64_t x = 0, y = 0;
            if (!comma || !whole_text(clarity_copy_n(pair, (size_t)(comma - pair)), &x) || !whole_text(comma + 1, &y)) fits = false;
            cJSON_AddItemToArray(points, cJSON_CreateNumber((double)x));
            cJSON_AddItemToArray(points, cJSON_CreateNumber((double)y));
            at += length;
        }
    } else if (attrs && strcmp(name, "text") == 0) {
        cJSON_AddStringToObject(shape, "op", "text");
        cJSON_AddStringToObject(shape, "color", string_of(attrs, "fill"));
        cJSON_AddStringToObject(shape, "text", content);
        fits = PICK(shape, attrs, "x", "x", "y", "y", "font-size", "size");
    }
    if (fits && strcmp(shape_line(shape), line) == 0) return shape;
    cJSON* kept = cJSON_CreateObject();
    cJSON_AddStringToObject(kept, "op", "svg");
    cJSON_AddStringToObject(kept, "text", line);
    return kept;
}

cJSON* clarity_drawing_shapes(const char* text, int units[2]) {
    cJSON* shapes = cJSON_CreateArray();
    for (const char* at = text; *at;) {
        size_t length = strcspn(at, "\n");
        const char* line = clarity_copy_n(at, length);
        at += length + (at[length] ? 1 : 0);
        if (strspn(line, " \t\r") == length || strcmp(line, "</svg>") == 0) continue;
        if (strncmp(line, "<svg", 4) == 0) {
            int w, h;
            char end[3] = {0};
            if (sscanf(line, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 %d %d%2s", &w, &h, end) == 3 && strcmp(end, "\">") == 0) {
                units[0] = w;
                units[1] = h;
            }
            continue;
        }
        cJSON_AddItemToArray(shapes, read_shape(line));
    }
    return shapes;
}

/// The numbers of each shape a vector canvas keeps, which of them are
/// places, and what a left-out one is (conformance/drawing.json).
typedef struct {
    const char* op;
    const char* numbers[5];
    size_t places;
    double defaults[5];
} ShapeFields;

static const ShapeFields shape_fields[] = {
    {"rect", {"x", "y", "width", "height"}, 2, {0}},
    {"line", {"x1", "y1", "x2", "y2", "width"}, 4, {0, 0, 0, 0, 1}},
    {"ellipse", {"x", "y", "rx", "ry"}, 2, {0}},
    {"path", {"width"}, 0, {1}},
    {"text", {"x", "y", "size"}, 2, {0, 0, 10}},
};

/// A shape as a vector canvas keeps it: whole numbers, places snapped. NULL
/// and an error when it is no shape a canvas keeps.
static cJSON* fit_shape(const cJSON* shape, const ClarityCanvas* canvas) {
    const char* op = string_of(shape, "op");
    const ShapeFields* fields = NULL;
    for (size_t i = 0; i < sizeof shape_fields / sizeof *shape_fields; ++i)
        if (strcmp(shape_fields[i].op, op) == 0) fields = &shape_fields[i];
    if (!fields) {
        clarity_fail(CLARITY_INVALID, "a vector canvas keeps rect, line, ellipse, path and text, not \"%s\"", op);
        return NULL;
    }
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "op", op);
    for (size_t i = 0; i < 5 && fields->numbers[i]; ++i) {
        double n = number_or(shape, fields->numbers[i], fields->defaults[i]);
        put_number(out, fields->numbers[i], i < fields->places ? placed(canvas, n) : whole(n));
    }
    cJSON_AddStringToObject(out, "color", string_of(shape, "color"));
    if (strcmp(op, "path") == 0) {
        cJSON* points = cJSON_AddArrayToObject(out, "points");
        const cJSON* p;
        cJSON_ArrayForEach(p, cJSON_GetObjectItemCaseSensitive(shape, "points")) {
            double n = 0;
            number_of(p, &n);
            cJSON_AddItemToArray(points, cJSON_CreateNumber((double)placed(canvas, n)));
        }
    }
    if (strcmp(op, "text") == 0) cJSON_AddStringToObject(out, "text", string_of(shape, "text"));
    return out;
}

/// Moves shape so its first place is at x, y, the rest with it; false when
/// it is there already or is a line the runtime does not read.
static bool moved_shape(cJSON* shape, int64_t x, int64_t y) {
    const char* op = string_of(shape, "op");
#define N(name) whole(number_or(shape, name, 0))
    int64_t dx, dy;
    if (strcmp(op, "rect") == 0 || strcmp(op, "ellipse") == 0 || strcmp(op, "text") == 0) {
        dx = x - N("x"), dy = y - N("y");
        put_number(shape, "x", x);
        put_number(shape, "y", y);
    } else if (strcmp(op, "line") == 0) {
        dx = x - N("x1"), dy = y - N("y1");
        put_number(shape, "x2", N("x2") + dx);
        put_number(shape, "y2", N("y2") + dy);
        put_number(shape, "x1", x);
        put_number(shape, "y1", y);
    } else if (strcmp(op, "path") == 0) {
        cJSON* points = cJSON_GetObjectItemCaseSensitive(shape, "points");
        if (!cJSON_IsArray(points) || cJSON_GetArraySize(points) < 2) return false;
        dx = x - whole(cJSON_GetArrayItem(points, 0)->valuedouble);
        dy = y - whole(cJSON_GetArrayItem(points, 1)->valuedouble);
        int i = 0;
        cJSON* p;
        cJSON_ArrayForEach(p, points) {
            double n = 0;
            number_of(p, &n);
            cJSON_SetNumberValue(p, (double)(whole(n) + (i++ % 2 == 0 ? dx : dy)));
        }
    } else {
        return false;
    }
#undef N
    return dx != 0 || dy != 0;
}

static bool same_id(const cJSON* a, const cJSON* b) {
    if (cJSON_IsNumber(a) && cJSON_IsNumber(b)) return a->valuedouble == b->valuedouble;
    return a && b && cJSON_Compare(a, b, true);
}

static bool same_number(const cJSON* value, int64_t n) {
    double got;
    return number_of(value, &got) && got == (double)n;
}

static ClarityCanvasDone fails(void) { return CLARITY_CANVAS_FAILED; }

static ClarityCanvasDone moved_object(cJSON* data, const ClarityCanvas* canvas, const cJSON* move, ClarityCanvasTouched* touched) {
    if (!cJSON_IsObject(move)) return clarity_fail(CLARITY_INVALID, "an object is moved"), fails();
    cJSON* list = clarity_data_at(data, canvas->list);
    if (!cJSON_IsArray(list)) return clarity_fail(CLARITY_INVALID, "the window has no list %s", canvas->list), fails();
    const cJSON* id = cJSON_GetObjectItemCaseSensitive(move, "id");
    int place = -1;
    cJSON* record;
    cJSON_ArrayForEach(record, list) {
        ++place;
        if (!cJSON_IsObject(record) || !same_id(cJSON_GetObjectItemCaseSensitive(record, "id"), id)) continue;
        int64_t x = placed(canvas, number_or(move, "x", 0)), y = placed(canvas, number_or(move, "y", 0));
        if (same_number(cJSON_GetObjectItemCaseSensitive(record, canvas->at[0]), x) && same_number(cJSON_GetObjectItemCaseSensitive(record, canvas->at[1]), y))
            return CLARITY_CANVAS_REFUSED;
        put_number(record, canvas->at[0], x);
        put_number(record, canvas->at[1], y);
        *touched = (ClarityCanvasTouched){true, place};
        return CLARITY_CANVAS_CHANGED;
    }
    return CLARITY_CANVAS_REFUSED;
}

/// A place from 1 an action names; 0 when it names none.
static int index_of(const cJSON* v) {
    double n;
    return number_of(v, &n) && n >= 1 ? (int)n : 0;
}

static ClarityCanvasDone vector_changed(cJSON* data, const ClarityCanvas* canvas, const cJSON* action, ClarityCanvasTouched* touched) {
    int units[2] = {canvas->units ? canvas->units[0] : default_units[0], canvas->units ? canvas->units[1] : default_units[1]};
    const cJSON* was = clarity_data_at(data, canvas->path);
    const char* text = cJSON_IsString(was) ? was->valuestring : "";
    cJSON* shapes = clarity_drawing_shapes(text, units);
    int count = cJSON_GetArraySize(shapes), selected = 0;
    const cJSON* add = cJSON_GetObjectItemCaseSensitive(action, "add");
    const cJSON* change = cJSON_GetObjectItemCaseSensitive(action, "change");
    const cJSON* move = cJSON_GetObjectItemCaseSensitive(action, "move");
    const cJSON* gone = cJSON_GetObjectItemCaseSensitive(action, "delete");
    if (add) {
        cJSON* fitted = fit_shape(add, canvas);
        if (!fitted) return fails();
        cJSON_AddItemToArray(shapes, fitted);
        selected = count + 1;
    } else if (change) {
        int index = index_of(cJSON_GetObjectItemCaseSensitive(change, "index"));
        if (index < 1 || index > count) return CLARITY_CANVAS_REFUSED;
        cJSON* fitted = fit_shape(cJSON_GetObjectItemCaseSensitive(change, "shape"), canvas);
        if (!fitted) return fails();
        cJSON_ReplaceItemInArray(shapes, index - 1, fitted);
        selected = index;
    } else if (move) {
        int index = index_of(cJSON_GetObjectItemCaseSensitive(move, "index"));
        if (index < 1 || index > count) return CLARITY_CANVAS_REFUSED;
        if (!moved_shape(cJSON_GetArrayItem(shapes, index - 1), placed(canvas, number_or(move, "x", 0)), placed(canvas, number_or(move, "y", 0))))
            return CLARITY_CANVAS_REFUSED;
        selected = index;
    } else if (gone) {
        int index = index_of(gone);
        if (index < 1 || index > count) return CLARITY_CANVAS_REFUSED;
        cJSON_DeleteItemFromArray(shapes, index - 1);
    } else {
        return clarity_fail(CLARITY_INVALID, "a vector canvas adds, changes, moves or deletes a shape"), fails();
    }
    const char* drawn = clarity_drawing_text(units, shapes);
    if (strcmp(drawn, text) == 0) return CLARITY_CANVAS_REFUSED;
    if (!put_at_path(data, canvas->path, cJSON_CreateString(drawn))) return fails();
    *touched = (ClarityCanvasTouched){selected > 0, selected - 1};
    return CLARITY_CANVAS_CHANGED;
}

ClarityCanvasDone clarity_canvas_event(cJSON* data, const ClarityCanvas* canvas, const cJSON* action, ClarityCanvasTouched* touched) {
    *touched = (ClarityCanvasTouched){false, 0};
    if (canvas->readonly) return CLARITY_CANVAS_REFUSED;
    const char* mode = canvas->mode ? canvas->mode : "";
    if (strcmp(mode, "objects") == 0) return moved_object(data, canvas, cJSON_GetObjectItemCaseSensitive(action, "move"), touched);
    if (strcmp(mode, "paint") == 0) {
        // The picture as an image field holds a new one (conformance/pictures.json), kept as it comes.
        const cJSON* stroke = cJSON_GetObjectItemCaseSensitive(action, "stroke");
        if (!stroke) return clarity_fail(CLARITY_INVALID, "a painted canvas is stroked"), fails();
        const cJSON* was = clarity_data_at(data, canvas->path);
        if (was ? cJSON_Compare(was, stroke, true) : cJSON_IsNull(stroke)) return CLARITY_CANVAS_REFUSED;
        // A stroke that is no picture is refused: the record never holds it.
        const cJSON* painted = cJSON_GetObjectItemCaseSensitive(stroke, "data");
        ClarityPictureKind kind;
        if (!cJSON_IsString(painted) || !clarity_picture_kind(clarity_from_base64(painted->valuestring), &kind)) return CLARITY_CANVAS_REFUSED;
        if (!put_at_path(data, canvas->path, cJSON_Duplicate(stroke, true))) return fails();
        return CLARITY_CANVAS_CHANGED;
    }
    if (strcmp(mode, "vector") == 0) return vector_changed(data, canvas, action, touched);
    return clarity_fail(CLARITY_INVALID, "a canvas's mode is objects, paint or vector, not \"%s\"", mode), fails();
}
