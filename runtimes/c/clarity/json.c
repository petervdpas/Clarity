#include "clarity/json.h"

cJSON* clarity_json_text(const char* value) { return cJSON_CreateString(value ? value : ""); }
cJSON* clarity_json_integer(int64_t value) { return cJSON_CreateNumber((double)value); }
cJSON* clarity_json_decimal(ClarityDecimal value) { return cJSON_CreateString(clarity_decimal_text(value)); }
cJSON* clarity_json_bool(bool value) { return cJSON_CreateBool(value); }

cJSON* clarity_json_date(ClarityDate value) {
    return clarity_date_is_none(value) ? cJSON_CreateNull() : cJSON_CreateString(clarity_date_text(value));
}

cJSON* clarity_json_date_time(ClarityDateTime value) {
    return clarity_date_time_is_none(value) ? cJSON_CreateNull() : cJSON_CreateString(clarity_date_time_text(value));
}

cJSON* clarity_json_reference(int64_t id) { return id ? cJSON_CreateNumber((double)id) : cJSON_CreateNull(); }

const cJSON* clarity_json_given(const cJSON* in, const char* name) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(in, name);
    return v && !cJSON_IsNull(v) ? v : NULL;
}

static bool unfit(const char* name, const char* what) { return clarity_fail(CLARITY_INVALID, "%s is no %s", name, what); }

bool clarity_json_get_text(const cJSON* in, const char* name, const char** out) {
    const cJSON* v = clarity_json_given(in, name);
    if (!v) return true;
    if (!cJSON_IsString(v)) return unfit(name, "text");
    *out = clarity_copy(v->valuestring);
    return true;
}

bool clarity_json_get_integer(const cJSON* in, const char* name, int64_t* out) {
    const cJSON* v = clarity_json_given(in, name);
    if (!v) return true;
    if (!cJSON_IsNumber(v)) return unfit(name, "whole number");
    *out = (int64_t)v->valuedouble;
    return true;
}

bool clarity_json_get_decimal(const cJSON* in, const char* name, ClarityDecimal* out) {
    const cJSON* v = clarity_json_given(in, name);
    if (!v) return true;
    if (cJSON_IsString(v)) return clarity_decimal_parse(v->valuestring, out);
    if (!cJSON_IsNumber(v)) return unfit(name, "number");
    // A number that is whole stays exact; another goes through its text.
    if (v->valuedouble == (double)(int64_t)v->valuedouble) {
        *out = clarity_decimal_of((int64_t)v->valuedouble);
        return true;
    }
    return clarity_decimal_parse(clarity_format("%.15g", v->valuedouble), out);
}

bool clarity_json_get_bool(const cJSON* in, const char* name, bool* out) {
    const cJSON* v = clarity_json_given(in, name);
    if (!v) return true;
    if (!cJSON_IsBool(v)) return unfit(name, "yes or no");
    *out = cJSON_IsTrue(v);
    return true;
}

bool clarity_json_get_date(const cJSON* in, const char* name, ClarityDate* out) {
    const cJSON* v = clarity_json_given(in, name);
    if (!v) return true;
    if (!cJSON_IsString(v) || !clarity_date_parse(v->valuestring, out))
        return clarity_fail(CLARITY_INVALID, "'%s' is no date", cJSON_IsString(v) ? v->valuestring : name);
    return true;
}

bool clarity_json_get_date_time(const cJSON* in, const char* name, ClarityDateTime* out) {
    const cJSON* v = clarity_json_given(in, name);
    if (!v) return true;
    if (!cJSON_IsString(v) || !clarity_date_time_parse(v->valuestring, out))
        return clarity_fail(CLARITY_INVALID, "'%s' is no date and time", cJSON_IsString(v) ? v->valuestring : name);
    return true;
}

bool clarity_json_get_reference(const cJSON* in, const char* name, int64_t* out) { return clarity_json_get_integer(in, name, out); }

bool clarity_json_get_picture(const cJSON* in, const char* name, ClarityPicture** out) {
    const cJSON* v = clarity_json_given(in, name);
    if (v) *out = clarity_picture_from_json(v);
    return true;
}

cJSON* clarity_json_bytes(ClarityBytes value) { return cJSON_CreateString(clarity_base64(value)); }

bool clarity_json_get_bytes(const cJSON* in, const char* name, ClarityBytes* out) {
    const cJSON* item = cJSON_GetObjectItemCaseSensitive(in, name);
    if (!cJSON_IsString(item)) return clarity_fail(CLARITY_INVALID, "%s is no bytes", name);
    *out = clarity_from_base64(item->valuestring);
    return true;
}
