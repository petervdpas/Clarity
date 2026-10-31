// The rows a table shows, against the cases every runtime shares
// (conformance/table-order.json and table-pages.json).

#include "check.h"
#include "clarity/tables.h"
#include "tests.h"

void a_table_shows_its_rows_in_the_order_every_runtime_does(void) {
    cJSON* file = conformance("table-order.json");
    REQUIRE(file);
    const cJSON* rows = cJSON_GetObjectItem(file, "rows");
    const cJSON* columns = cJSON_GetObjectItem(file, "columns");
    size_t n = (size_t)cJSON_GetArraySize(rows);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        bool sortable = cJSON_IsTrue(cJSON_GetObjectItem(c, "sortable"));
        cJSON* order = cJSON_Duplicate(cJSON_GetObjectItem(c, "order"), true);
        cJSON* events = cJSON_CreateArray();
        int chosen = -1;
        const cJSON* step;
        cJSON_ArrayForEach(step, cJSON_GetObjectItem(c, "steps")) {
            const char* what = cJSON_GetArrayItem(step, 0)->valuestring;
            const cJSON* arg = cJSON_GetArrayItem(step, 1);
            if (strcmp(what, "sort") == 0) {
                // A header click sorts, and selects nothing.
                if (sortable) order = clarity_next_order(order, arg->valuestring);
            } else {
                chosen = (int)clarity_shown_order(rows, columns, order)[(int)arg->valuedouble];
                cJSON_AddItemToArray(events, cJSON_CreateString(clarity_format("select %d", chosen)));
            }
        }
        size_t* shown = clarity_shown_order(rows, columns, order);
        cJSON* got = cJSON_CreateArray();
        for (size_t i = 0; i < n; ++i) cJSON_AddItemToArray(got, cJSON_CreateNumber((double)shown[i]));
        const cJSON* expect = cJSON_GetObjectItem(c, "expect");
        if (!check_json(got, cJSON_PrintUnformatted(cJSON_GetObjectItem(expect, "shown"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: shows %s", name, cJSON_PrintUnformatted(got)));
        if (!check_json(events, cJSON_PrintUnformatted(cJSON_GetObjectItem(expect, "events"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: events %s", name, cJSON_PrintUnformatted(events)));
        const cJSON* want = cJSON_GetObjectItem(expect, "chosen");
        if (want && (int)want->valuedouble != chosen) check_fail(__FILE__, __LINE__, clarity_format("%s: chose %d", name, chosen));
    }
    cJSON_Delete(file);
}

void a_table_pages_its_rows_as_every_runtime_does(void) {
    cJSON* file = conformance("table-pages.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "pages")) {
        int64_t count = (int64_t)cJSON_GetObjectItem(c, "count")->valuedouble;
        ClarityPage page = clarity_page_of(count, (int64_t)cJSON_GetObjectItem(c, "size")->valuedouble, (int64_t)cJSON_GetObjectItem(c, "page")->valuedouble);
        CHECK_INT(page.page, cJSON_GetObjectItem(c, "shows")->valuedouble);
        CHECK_INT(page.pages, cJSON_GetObjectItem(c, "pages")->valuedouble);
        CHECK_INT(page.from, cJSON_GetObjectItem(c, "from")->valuedouble);
        CHECK_INT(page.to, cJSON_GetObjectItem(c, "to")->valuedouble);
        CHECK_TEXT(clarity_page_text(page, count), cJSON_GetObjectItem(c, "text")->valuestring);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "selected")) {
        CHECK_INT(clarity_page_with((int64_t)cJSON_GetObjectItem(c, "size")->valuedouble, (int64_t)cJSON_GetObjectItem(c, "at")->valuedouble),
                  cJSON_GetObjectItem(c, "page")->valuedouble);
    }
    cJSON_Delete(file);
}
