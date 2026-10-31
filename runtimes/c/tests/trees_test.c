// The rows a tree shows, against the cases every runtime shares
// (conformance/trees.json).

#include "check.h"
#include "clarity/trees.h"
#include "tests.h"

void a_tree_shows_its_rows_as_every_runtime_does(void) {
    cJSON* file = conformance("trees.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        cJSON* got = clarity_shown_tree(cJSON_GetObjectItem(c, "rows"), cJSON_GetObjectItem(c, "open"));
        if (!check_json(got, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "shows"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: shows %s", cJSON_GetObjectItem(c, "name")->valuestring, cJSON_PrintUnformatted(got)));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "reveal")) {
        CHECK_JSON(clarity_tree_above(cJSON_GetObjectItem(c, "rows"), (int)cJSON_GetObjectItem(c, "at")->valuedouble), cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "opens")));
    }
    cJSON_Delete(file);
}
