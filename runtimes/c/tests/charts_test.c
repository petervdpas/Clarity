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

// The numbers of a list a chart draws, as every renderer takes them
// (app/src/lib/ui/trend.ts; the cases have none of their own).
void a_chart_of_numbers_takes_its_last_points(void) {
    cJSON* list = cJSON_Parse("[1, \"2.5\", \" 3 \", \"\", null, \"many\", true, 4]");
    CHECK_JSON(clarity_numbers_of(list, NULL), "[1,2.5,3,4]");
    CHECK_JSON(clarity_numbers_of(list, cJSON_Parse("null")), "[1,2.5,3,4]");
    CHECK_JSON(clarity_numbers_of(list, cJSON_Parse("2")), "[3,4]");
    CHECK_JSON(clarity_numbers_of(list, cJSON_Parse("10")), "[1,2.5,3,4]");
    CHECK_JSON(clarity_numbers_of(cJSON_Parse("{\"a\":1}"), NULL), "[]");
    CHECK_JSON(clarity_numbers_of(NULL, NULL), "[]");
}
