#include "formats/formats.h"

#include <string.h>

#include "clarity/core.h"
#include "clarity_formats.h"

static const ClarityFormatsApi* api(void) {
    static const ClarityFormatsApi* table;
    if (!table) table = clarity_formats_api();
    return table;
}

typedef int32_t (*FormatCall)(const char*, size_t, const uint8_t*, size_t, const char*, size_t, const char*, size_t, uint8_t**, size_t*);

static int32_t read_call(const char* f, size_t fn, const uint8_t* i, size_t in, const char* o, size_t on, const char* k, size_t kn, uint8_t** out, size_t* len) {
    return api()->read(f, fn, i, in, o, on, k, kn, out, len);
}

static int32_t write_call(const char* f, size_t fn, const uint8_t* v, size_t vn, const char* o, size_t on, const char* k, size_t kn, uint8_t** out, size_t* len) {
    return api()->write(f, fn, (const char*)v, vn, o, on, k, kn, out, len);
}

/* Runs a call into the library: what it made, copied into the arena, or NULL with its problem kept. */
static char* call(FormatCall run, const char* format, const char* input, const char* options, const char* kind, char* why, size_t why_size) {
    uint8_t* made = NULL;
    size_t len = 0;
    int32_t code = run(format, strlen(format), (const uint8_t*)input, strlen(input), options, strlen(options), kind, strlen(kind), &made, &len);
    if (code != CLARITY_FORMATS_OK) {
        api()->problem(why, why_size);
        return NULL;
    }
    char* copy = clarity_copy_n((const char*)made, len);
    api()->free(made, len);
    return copy;
}

bool clarity_formats_read(const char* format, const char* input, const char* options, const char* kind, cJSON** out) {
    char why[1024] = {0};
    char* fitted = call(read_call, format, input ? input : "", options ? options : "", kind, why, sizeof why);
    if (!fitted) return clarity_fail(CLARITY_VALUE, "%s", why[0] ? why : "what was read does not fit");
    cJSON* parsed = cJSON_Parse(fitted);
    if (!parsed) return clarity_fail(CLARITY_VALUE, "what was read does not fit");
    *out = parsed;
    return true;
}

const char* clarity_formats_write(const char* format, const char* value, const char* options, const char* kind) {
    char why[1024] = {0};
    char* written = call(write_call, format, value ? value : "", options ? options : "", kind, why, sizeof why);
    /* A kind the generator wrote always describes the value; should it not, the value as `_json` wrote it. */
    return written ? written : clarity_copy(value);
}

/* The table calls came with interface 2; an older library has no such fields. */
static bool tables_known(void) {
    return api()->interface >= 2 || clarity_fail(CLARITY_VALUE, "this Clarity.Formats library is too old to read a file as a table");
}

/* What the library made, copied into the arena, or its problem kept. */
static bool made(int32_t code, uint8_t* bytes, size_t len, ClarityBytes* out) {
    if (code != CLARITY_FORMATS_OK) {
        char why[1024] = {0};
        api()->problem(why, sizeof why);
        return clarity_fail(CLARITY_VALUE, "%s", why[0] ? why : "what was read does not fit");
    }
    *out = (ClarityBytes){(const uint8_t*)clarity_copy_n((const char*)bytes, len), len};
    api()->free(bytes, len);
    return true;
}

bool clarity_formats_read_table(const char* format, ClarityBytes input, const char* options, const char* fields, const char** out) {
    if (!tables_known()) return false;
    options = options ? options : "";
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->read_table(format, strlen(format), input.data, input.size, options, strlen(options), fields, strlen(fields), &bytes, &len);
    ClarityBytes table = {0};
    if (!made(code, bytes, len, &table)) return false;
    *out = (const char*)table.data;
    return true;
}

bool clarity_formats_write_table(const char* format, ClarityBytes original, const char* table, const char* options, const char* fields, ClarityBytes* out) {
    if (!tables_known()) return false;
    options = options ? options : "";
    uint8_t* bytes = NULL;
    size_t len = 0;
    int32_t code = api()->write_table(format, strlen(format), original.data, original.size, table, strlen(table), options, strlen(options), fields, strlen(fields), &bytes, &len);
    return made(code, bytes, len, out);
}
