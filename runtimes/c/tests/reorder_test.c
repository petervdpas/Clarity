// Reordering (DESIGN 5.30), against the cases every runtime shares
// (conformance/reorder.json).

#include "check.h"
#include "clarity/reorder.h"
#include "tests.h"

static void check_cases(const cJSON* cases, bool tree) {
    const cJSON* c;
    cJSON_ArrayForEach(c, cases) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        ClarityReordered got;
        bool moved = clarity_reorder(cJSON_GetObjectItem(c, "rows"), cJSON_GetObjectItem(c, "move"), tree, &got);
        if (cJSON_IsTrue(cJSON_GetObjectItem(c, "refused"))) {
            if (moved) check_fail(__FILE__, __LINE__, clarity_format("%s: moved", name));
            continue;
        }
        if (!moved) {
            check_fail(__FILE__, __LINE__, clarity_format("%s: refused", name));
            continue;
        }
        cJSON* order = cJSON_CreateArray();
        cJSON* parents = cJSON_CreateArray();
        cJSON* numbers = cJSON_CreateArray();
        for (size_t i = 0; i < got.count; ++i) {
            cJSON_AddItemToArray(order, cJSON_CreateNumber((double)got.order[i]));
            cJSON_AddItemToArray(parents, got.parents[i] ? cJSON_Duplicate(got.parents[i], true) : cJSON_CreateNull());
            cJSON_AddItemToArray(numbers, cJSON_CreateNumber((double)got.numbers[i]));
        }
        if (!check_json(order, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "order"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: order %s", name, cJSON_PrintUnformatted(order)));
        if (!check_json(numbers, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "numbers"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: numbers %s", name, cJSON_PrintUnformatted(numbers)));
        if (tree && !check_json(parents, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "parents"))))
            check_fail(__FILE__, __LINE__, clarity_format("%s: parents %s", name, cJSON_PrintUnformatted(parents)));
    }
}

void every_reorder_case_moves_as_every_runtime_does(void) {
    cJSON* cases = conformance("reorder.json");
    REQUIRE(cases);
    check_cases(cJSON_GetObjectItem(cases, "lists"), false);
    check_cases(cJSON_GetObjectItem(cases, "trees"), true);
    cJSON_Delete(cases);
}
