// Fitting a service's JSON to an entity; see json_read.h.

#include "clarity/json_read.h"

#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/core.h"

const char* clarity_json_of(const cJSON* value) {
    char* printed = cJSON_PrintUnformatted(value);
    return printed ? printed : "";
}

/// A name as names are matched: lower case, without underscores, hyphens and spaces.
static const char* plain(const char* name) {
    char* out = (char*)clarity_alloc(strlen(name) + 1);
    size_t n = 0;
    for (const char* c = name; *c; c++) {
        if (*c == '_' || *c == '-' || *c == ' ') continue;
        out[n++] = (char)tolower((unsigned char)*c);
    }
    out[n] = '\0';
    return out;
}

static const cJSON* found(const cJSON* object, const char* name) {
    const cJSON* item;
    cJSON_ArrayForEach(item, object) {
        if (item->string && strcmp(item->string, name) == 0) return item;
    }
    const char* wanted = plain(name);
    cJSON_ArrayForEach(item, object) {
        if (item->string && strcmp(plain(item->string), wanted) == 0) return item;
    }
    return NULL;
}

static const char* wanted(ClarityJsonKind kind) {
    switch (kind.is) {
        case CLARITY_JSON_TEXT: return "text";
        case CLARITY_JSON_INTEGER: return "an integer";
        case CLARITY_JSON_DECIMAL: return "a decimal";
        case CLARITY_JSON_BOOL: return "true or false";
        case CLARITY_JSON_DATE: return "a date";
        case CLARITY_JSON_DATE_TIME: return "a date and time";
        case CLARITY_JSON_VALUES: return "a list";
        case CLARITY_JSON_OTHER: return "something else";
        case CLARITY_JSON_ENUM:
            if (kind.count == 0) return "an enum value";
            if (kind.count == 1) return kind.values[0];
            {
                ClarityBuffer out = {0};
                for (size_t i = 0; i + 1 < kind.count; i++) clarity_buffer_addf(&out, "%s%s", i ? ", " : "", kind.values[i]);
                clarity_buffer_addf(&out, " or %s", kind.values[kind.count - 1]);
                return clarity_buffer_text(&out);
            }
    }
    return "something else";
}

static bool digits(const char* text, size_t length) {
    if (length == 0) return false;
    for (size_t i = 0; i < length; i++) {
        if (!isdigit((unsigned char)text[i])) return false;
    }
    return true;
}

static bool is_decimal(const char* text) {
    if (*text == '-') text++;
    const char* dot = strchr(text, '.');
    if (!dot) return digits(text, strlen(text));
    return digits(text, (size_t)(dot - text)) && digits(dot + 1, strlen(dot + 1));
}

static bool is_date(const char* text, size_t length) {
    return length == 10 && text[4] == '-' && text[7] == '-' && digits(text, 4) && digits(text + 5, 2) && digits(text + 8, 2);
}

/// A date and time as Clarity writes it, YYYY-MM-DDTHH:MM:SS, from the same
/// with or without seconds, a fraction of a second and a Z; NULL when it is none.
static const char* date_time(const char* text) {
    size_t length = strlen(text);
    if (length && text[length - 1] == 'Z') length--;
    const char* at = strpbrk(text, "T ");
    if (!at || (size_t)(at - text) > length || !is_date(text, (size_t)(at - text))) return NULL;
    const char* clock = at + 1;
    size_t clock_length = length - (size_t)(clock - text);
    const char* dot = memchr(clock, '.', clock_length);
    if (dot) clock_length = (size_t)(dot - clock);
    if (clock_length == 5 && clock[2] == ':' && digits(clock, 2) && digits(clock + 3, 2)) return clarity_format("%.10sT%.5s:00", text, clock);
    if (clock_length == 8 && clock[2] == ':' && clock[5] == ':' && digits(clock, 2) && digits(clock + 3, 2) && digits(clock + 6, 2)) return clarity_format("%.10sT%.8s", text, clock);
    return NULL;
}

/// A number as its shortest exact text.
static const char* number_text(double value) {
    char* text = clarity_format("%.15g", value);
    if (strtod(text, NULL) != value) text = clarity_format("%.17g", value);
    return text;
}

/// `given` fitted to `kind`, or NULL when it does not fit.
static cJSON* fit_value(const cJSON* given, ClarityJsonKind kind) {
    switch (kind.is) {
        case CLARITY_JSON_TEXT:
            if (cJSON_IsString(given)) return cJSON_CreateString(given->valuestring);
            if (cJSON_IsNumber(given) || cJSON_IsBool(given)) return cJSON_CreateString(clarity_json_of(given));
            return NULL;
        case CLARITY_JSON_INTEGER:
            if (cJSON_IsNumber(given) && floor(given->valuedouble) == given->valuedouble && fabs(given->valuedouble) < 9e15) return cJSON_CreateNumber(given->valuedouble);
            if (cJSON_IsString(given)) {
                char* end = NULL;
                const char* start = given->valuestring;
                while (isspace((unsigned char)*start)) start++;
                long long n = strtoll(start, &end, 10);
                while (end && isspace((unsigned char)*end)) end++;
                if (end != start && end && *end == '\0') return cJSON_CreateNumber((double)n);
            }
            return NULL;
        case CLARITY_JSON_DECIMAL:
            if (cJSON_IsNumber(given)) return cJSON_CreateString(number_text(given->valuedouble));
            if (cJSON_IsString(given) && is_decimal(given->valuestring)) return cJSON_CreateString(given->valuestring);
            return NULL;
        case CLARITY_JSON_BOOL:
            return cJSON_IsBool(given) ? cJSON_CreateBool(cJSON_IsTrue(given)) : NULL;
        case CLARITY_JSON_DATE:
            if (!cJSON_IsString(given)) return NULL;
            if (is_date(given->valuestring, strlen(given->valuestring))) return cJSON_CreateString(given->valuestring);
            {
                const char* moment = date_time(given->valuestring);
                return moment ? cJSON_CreateString(clarity_copy_n(moment, 10)) : NULL;
            }
        case CLARITY_JSON_DATE_TIME: {
            const char* moment = cJSON_IsString(given) ? date_time(given->valuestring) : NULL;
            return moment ? cJSON_CreateString(moment) : NULL;
        }
        case CLARITY_JSON_ENUM:
            if (!cJSON_IsString(given)) return NULL;
            for (size_t i = 0; i < kind.count; i++) {
                const char* a = kind.values[i];
                const char* b = given->valuestring;
                while (*a && *b && tolower((unsigned char)*a) == tolower((unsigned char)*b)) a++, b++;
                if (!*a && !*b) return cJSON_CreateString(kind.values[i]);
            }
            return NULL;
        case CLARITY_JSON_VALUES: {
            if (!cJSON_IsArray(given) || !kind.item) return NULL;
            cJSON* list = cJSON_CreateArray();
            const cJSON* item;
            cJSON_ArrayForEach(item, given) {
                cJSON* fitted = fit_value(item, *kind.item);
                if (!fitted) return NULL;
                cJSON_AddItemToArray(list, fitted);
            }
            return list;
        }
        case CLARITY_JSON_OTHER:
            return NULL;
    }
    return NULL;
}

static bool fit_record(const cJSON* given, const char* entity, const ClarityJsonField* fields, size_t count, const cJSON* base, const char* prefix, cJSON** out) {
    cJSON* record = base && cJSON_IsObject(base) ? cJSON_Duplicate(base, 1) : cJSON_CreateObject();
    for (size_t i = 0; i < count; i++) {
        const cJSON* value = found(given, fields[i].name);
        if (!value || cJSON_IsNull(value) || fields[i].kind.is == CLARITY_JSON_OTHER) continue;
        cJSON* fitted = fit_value(value, fields[i].kind);
        if (!fitted) return clarity_fail(CLARITY_VALUE, "%sfield '%s' of %s is %s, not %s", prefix, fields[i].name, entity, wanted(fields[i].kind), clarity_json_of(value));
        cJSON_DeleteItemFromObjectCaseSensitive(record, fields[i].name);
        cJSON_AddItemToObject(record, fields[i].name, fitted);
    }
    *out = record;
    return true;
}

bool clarity_json_fit(const char* text, const char* at, bool many, const char* entity, const ClarityJsonField* fields, size_t count, const cJSON* base, cJSON** out) {
    const cJSON* value = cJSON_Parse(text ? text : "");
    if (!value) return clarity_fail(CLARITY_VALUE, "the text is no JSON");
    if (at && *at) {
        const char* step = at;
        for (;;) {
            const char* dot = strchr(step, '.');
            size_t length = dot ? (size_t)(dot - step) : strlen(step);
            const char* name = clarity_copy_n(step, length);
            const cJSON* next = cJSON_IsObject(value) ? cJSON_GetObjectItemCaseSensitive(value, name) : NULL;
            if (!next) return clarity_fail(CLARITY_VALUE, "the JSON has no '%.*s'", (int)(step + length - at), at);
            value = next;
            if (!dot) break;
            step = dot + 1;
        }
    }
    if (many) {
        if (!cJSON_IsArray(value)) return clarity_fail(CLARITY_VALUE, "the JSON is not a list of %s", entity);
        cJSON* list = cJSON_CreateArray();
        int n = 0;
        const cJSON* item;
        cJSON_ArrayForEach(item, value) {
            n++;
            if (!cJSON_IsObject(item)) return clarity_fail(CLARITY_VALUE, "item %d of the list is not one %s", n, entity);
            cJSON* record = NULL;
            if (!fit_record(item, entity, fields, count, base, clarity_format("item %d: ", n), &record)) return false;
            cJSON_AddItemToArray(list, record);
        }
        *out = list;
        return true;
    }
    if (!cJSON_IsObject(value)) return clarity_fail(CLARITY_VALUE, "the JSON is not one %s", entity);
    return fit_record(value, entity, fields, count, base, "", out);
}
