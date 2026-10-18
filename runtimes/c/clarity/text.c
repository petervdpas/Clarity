#include "clarity/text.h"

#include <string.h>

static const char* named(const char* format) { return format && *format ? format : NULL; }

static ClaritySlot slot(const char* kind, cJSON* value) { return (ClaritySlot){kind, value, -1, NULL, NULL}; }

ClaritySlot clarity_slot_text(const char* value) { return slot("text", cJSON_CreateString(value ? value : "")); }

ClaritySlot clarity_slot_integer(int64_t value, const char* format) {
    ClaritySlot s = slot("integer", cJSON_CreateNumber((double)value));
    s.format = named(format);
    return s;
}

ClaritySlot clarity_slot_decimal(ClarityDecimal value, int scale, const char* format) {
    ClaritySlot s = slot("decimal", cJSON_CreateString(clarity_decimal_text(value)));
    s.scale = scale;
    s.format = named(format);
    return s;
}

ClaritySlot clarity_slot_money(ClarityDecimal value, const char* format) {
    ClaritySlot s = slot("money", cJSON_CreateString(clarity_decimal_text(value)));
    s.format = named(format);
    return s;
}

ClaritySlot clarity_slot_date(ClarityDate value, const char* format) {
    ClaritySlot s = slot("date", clarity_date_is_none(value) ? cJSON_CreateNull() : cJSON_CreateString(clarity_date_text(value)));
    s.format = named(format);
    return s;
}

ClaritySlot clarity_slot_date_time(ClarityDateTime value, const char* format) {
    ClaritySlot s = slot("dateTime", clarity_date_time_is_none(value) ? cJSON_CreateNull() : cJSON_CreateString(clarity_date_time_text(value)));
    s.format = named(format);
    return s;
}

ClaritySlot clarity_slot_duration(int64_t seconds) { return slot("duration", cJSON_CreateNumber((double)seconds)); }

ClaritySlot clarity_slot_bool(bool value) { return slot("bool", cJSON_CreateBool(value)); }

ClaritySlot clarity_slot_enum(const char* value, const char* enum_name) {
    ClaritySlot s = slot("enum", cJSON_CreateString(value ? value : ""));
    s.enum_name = enum_name;
    return s;
}

const char* clarity_slot_plain(const ClaritySlot* s) {
    const cJSON* v = s->value;
    if (!v || cJSON_IsNull(v)) return "";
    if (cJSON_IsBool(v)) return cJSON_IsTrue(v) ? "yes" : "no";
    if (cJSON_IsString(v)) return v->valuestring;
    if (cJSON_IsNumber(v)) return clarity_format("%lld", (long long)v->valuedouble);
    return cJSON_PrintUnformatted(v);
}

ClarityText* clarity_text(const char* text) {
    ClarityText* t = CLARITY_NEW(ClarityText, 1);
    t->text = clarity_copy(text);
    return t;
}

ClarityText* clarity_plain_text(const char* text) {
    ClarityBuffer escaped = {0};
    for (const char* c = text ? text : ""; *c; ++c) {
        if (*c == '{' || *c == '}') clarity_buffer_add_char(&escaped, *c);
        clarity_buffer_add_char(&escaped, *c);
    }
    return clarity_text(clarity_buffer_text(&escaped));
}

ClarityText* clarity_text_with(ClarityText* text, const char* path, ClaritySlot value) {
    for (size_t i = 0; i < text->count; ++i) {
        if (strcmp(text->paths[i], path) == 0) {
            text->slots[i] = value;
            return text;
        }
    }
    // Both arrays grow together; the slots keep the capacity.
    size_t paths_room = text->capacity;
    clarity_grow((void**)&text->paths, &paths_room, text->count, sizeof *text->paths);
    clarity_grow((void**)&text->slots, &text->capacity, text->count, sizeof *text->slots);
    text->paths[text->count] = clarity_copy(path);
    text->slots[text->count] = value;
    ++text->count;
    return text;
}

const char* clarity_text_show(const ClarityText* text) {
    if (!text) return "";
    ClarityBuffer out = {0};
    const char* t = text->text;
    size_t length = strlen(t);
    size_t i = 0;
    while (i < length) {
        char c = t[i];
        if ((c == '{' || c == '}') && i + 1 < length && t[i + 1] == c) {
            // {{ and }} are braces.
            clarity_buffer_add_char(&out, c);
            i += 2;
        } else if (c == '{') {
            const char* end = strchr(t + i + 1, '}');
            size_t stop = end ? (size_t)(end - t) : length;
            const char* path = clarity_copy_n(t + i + 1, stop - i - 1);
            for (size_t k = 0; k < text->count; ++k)
                if (strcmp(text->paths[k], path) == 0) clarity_buffer_add(&out, clarity_slot_plain(&text->slots[k]));
            i = stop + 1;
        } else {
            clarity_buffer_add_char(&out, c);
            ++i;
        }
    }
    return clarity_buffer_text(&out);
}

cJSON* clarity_slot_json(const ClaritySlot* s) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "kind", s->kind);
    cJSON_AddItemToObject(out, "value", s->value ? cJSON_Duplicate(s->value, true) : cJSON_CreateNull());
    if (s->scale >= 0) cJSON_AddNumberToObject(out, "scale", s->scale);
    if (s->format) cJSON_AddStringToObject(out, "format", s->format);
    if (s->enum_name) cJSON_AddStringToObject(out, "enum", s->enum_name);
    return out;
}

bool clarity_slot_from_json(const cJSON* in, ClaritySlot* out) {
    const cJSON* kind = cJSON_GetObjectItemCaseSensitive(in, "kind");
    if (!cJSON_IsString(kind)) return clarity_fail(CLARITY_INVALID, "a slot has no kind");
    ClaritySlot s = slot(clarity_copy(kind->valuestring), NULL);
    const cJSON* value = cJSON_GetObjectItemCaseSensitive(in, "value");
    s.value = value ? cJSON_Duplicate(value, true) : cJSON_CreateNull();
    const cJSON* scale = cJSON_GetObjectItemCaseSensitive(in, "scale");
    if (cJSON_IsNumber(scale)) s.scale = (int)scale->valuedouble;
    const cJSON* format = cJSON_GetObjectItemCaseSensitive(in, "format");
    if (cJSON_IsString(format)) s.format = clarity_copy(format->valuestring);
    const cJSON* enum_name = cJSON_GetObjectItemCaseSensitive(in, "enum");
    if (cJSON_IsString(enum_name)) s.enum_name = clarity_copy(enum_name->valuestring);
    *out = s;
    return true;
}

cJSON* clarity_text_json(const ClarityText* text) {
    if (!text) return cJSON_CreateString("");
    if (text->count == 0) return cJSON_CreateString(clarity_text_show(text));
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "text", text->text);
    cJSON* values = cJSON_AddObjectToObject(out, "values");
    for (size_t i = 0; i < text->count; ++i) cJSON_AddItemToObject(values, text->paths[i], clarity_slot_json(&text->slots[i]));
    return out;
}

bool clarity_text_from_json(const cJSON* in, ClarityText** out) {
    if (cJSON_IsString(in)) {
        *out = clarity_plain_text(in->valuestring);
        return true;
    }
    const cJSON* text = cJSON_GetObjectItemCaseSensitive(in, "text");
    if (!cJSON_IsString(text)) return clarity_fail(CLARITY_INVALID, "text to show has no text");
    ClarityText* t = clarity_text(text->valuestring);
    const cJSON* value;
    cJSON_ArrayForEach(value, cJSON_GetObjectItemCaseSensitive(in, "values")) {
        ClaritySlot s;
        if (!clarity_slot_from_json(value, &s)) return false;
        clarity_text_with(t, value->string, s);
    }
    *out = t;
    return true;
}
