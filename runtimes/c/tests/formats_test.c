// The data formats of code on the C runtime (conformance/formats-runtime.json):
// read through Clarity.Formats and written back, the kind and options passed
// and the messages given word for word. A record's own `_from_json` and
// `_json`, which the generator writes, are built and run with every
// generated program that reads or writes a format.

#include "check.h"
#include "clarity/json.h"
#include "formats/formats.h"
#include "tests.h"

void formats_read_and_write_as_every_stack_does(void) {
    cJSON* file = conformance("formats-runtime.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        const char* kind = clarity_json_of(cJSON_GetObjectItem(c, "kind"));
        const cJSON* given = cJSON_GetObjectItem(c, "options");
        const char* options = given ? clarity_json_of(given) : "";
        const char* input = cJSON_GetObjectItem(c, "input")->valuestring;
        cJSON* fitted = NULL;
        bool read = clarity_formats_read("json", input, options, kind, &fitted);
        const cJSON* written = cJSON_GetObjectItem(c, "written");
        if (written) {
            if (!read) {
                check_fail(__FILE__, __LINE__, name);
                continue;
            }
            CHECK_TEXT(clarity_formats_write("json", clarity_json_of(fitted), "", kind), written->valuestring);
        } else {
            CHECK(!read);
            CHECK_TEXT(clarity_error()->message, cJSON_GetObjectItem(c, "error")->valuestring);
        }
    }
}
