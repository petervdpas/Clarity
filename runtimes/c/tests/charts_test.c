// The groups a chart shows, against the cases every runtime shares
// (conformance/charts.json, `groups`; `scale` and `slices` are the screen's drawing).

#include "check.h"
#include "clarity/charts.h"
#include "tests.h"

void a_chart_groups_its_rows_as_every_runtime_does(void) {
    cJSON* file = conformance("charts.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "groups")) {
        cJSON* got = clarity_chart_groups(cJSON_GetObjectItem(c, "rows"), cJSON_GetObjectItem(c, "by")->valuestring, cJSON_GetObjectItem(c, "measure")->valuestring,
                                          cJSON_GetObjectItem(c, "order"));
        if (!check_json(got, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "gives"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: gives %s", cJSON_GetObjectItem(c, "name")->valuestring, cJSON_PrintUnformatted(got)));
    }
    cJSON_Delete(file);
}
