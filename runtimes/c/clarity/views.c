#include "clarity/views.h"

#include <string.h>

#include "clarity/charts.h"
#include "clarity/tables.h"
#include "clarity/trees.h"

cJSON* clarity_data_at(const cJSON* data, const char* path) {
    const cJSON* at = data;
    const char* name = path ? path : "";
    if (!*name) return NULL;
    while (at && *name) {
        const char* dot = strchr(name, '.');
        size_t length = dot ? (size_t)(dot - name) : strlen(name);
        at = cJSON_IsObject(at) ? cJSON_GetObjectItemCaseSensitive(at, clarity_copy_n(name, length)) : NULL;
        name += length + (dot ? 1 : 0);
    }
    return (cJSON*)at;
}

/// Puts `value` as the field `name` of `object`.
static void put(cJSON* object, const char* name, cJSON* value) {
    if (cJSON_GetObjectItemCaseSensitive(object, name)) cJSON_ReplaceItemInObjectCaseSensitive(object, name, value);
    else cJSON_AddItemToObject(object, name, value);
}

static const char* text_of(const cJSON* view, const char* name, const char* otherwise) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(view, name);
    return cJSON_IsString(v) ? v->valuestring : otherwise;
}

/// The chosen row of `part`, as its place in the list; -1 for none.
static int chosen_of(const ClaritySelected* selected, size_t count, const char* part) {
    for (size_t i = 0; i < count; ++i)
        if (strcmp(selected[i].name, part) == 0) return selected[i].index;
    return -1;
}

/// The value `name` of each row, as `as` of an object of its own.
static cJSON* fields_of(const cJSON* list, const char* const* names, const char* const* as, size_t count) {
    cJSON* rows = cJSON_CreateArray();
    const cJSON* record;
    cJSON_ArrayForEach(record, list) {
        cJSON* row = cJSON_CreateObject();
        for (size_t i = 0; i < count; ++i) {
            const cJSON* value = names[i] ? cJSON_GetObjectItemCaseSensitive(record, names[i]) : NULL;
            cJSON_AddItemToObject(row, as[i], value ? cJSON_Duplicate(value, true) : cJSON_CreateNull());
        }
        cJSON_AddItemToArray(rows, row);
    }
    return rows;
}

static void fill_table(cJSON* view, const cJSON* list, int chosen) {
    int n = cJSON_GetArraySize(list);
    size_t* order = clarity_shown_order(list, cJSON_GetObjectItemCaseSensitive(view, "columns"), cJSON_GetObjectItemCaseSensitive(view, "order"));
    const cJSON* size = cJSON_GetObjectItemCaseSensitive(view, "page");
    cJSON* shown = cJSON_CreateArray();
    if (!cJSON_IsNumber(size) || size->valuedouble < 1) {
        // Without pages every row shows, in order.
        for (int i = 0; i < n; ++i) cJSON_AddItemToArray(shown, cJSON_CreateNumber((double)order[i]));
        cJSON_DeleteItemFromObjectCaseSensitive(view, "pages");
        put(view, "shown", shown);
        return;
    }
    const cJSON* at = cJSON_GetObjectItemCaseSensitive(view, "at");
    int64_t page = cJSON_IsNumber(at) ? (int64_t)at->valuedouble : 1;
    for (int i = 0; chosen >= 0 && i < n; ++i)
        if ((int)order[i] == chosen) page = clarity_page_with((int64_t)size->valuedouble, i);
    ClarityPage p = clarity_page_of(n, (int64_t)size->valuedouble, page);
    for (int64_t i = p.from; p.to > 0 && i <= p.to; ++i) cJSON_AddItemToArray(shown, cJSON_CreateNumber((double)order[i - 1]));
    cJSON* pages = cJSON_CreateObject();
    cJSON_AddNumberToObject(pages, "page", (double)p.page);
    cJSON_AddNumberToObject(pages, "pages", (double)p.pages);
    cJSON_AddNumberToObject(pages, "from", (double)p.from);
    cJSON_AddNumberToObject(pages, "to", (double)p.to);
    cJSON_AddNumberToObject(pages, "count", n);
    put(view, "at", cJSON_CreateNumber((double)p.page));
    put(view, "pages", pages);
    put(view, "shown", shown);
}

static bool holds(const cJSON* ids, const cJSON* id) {
    const cJSON* item;
    cJSON_ArrayForEach(item, ids) {
        if (cJSON_Compare(item, id, true)) return true;
    }
    return false;
}

static void fill_tree(cJSON* view, const cJSON* list, int chosen) {
    const char* names[] = {text_of(view, "key", "id"), text_of(view, "parent", NULL)};
    const char* as[] = {"id", "parent"};
    cJSON* rows = fields_of(list, names, as, 2);
    const cJSON* was = cJSON_GetObjectItemCaseSensitive(view, "open");
    cJSON* open = cJSON_IsArray(was) ? cJSON_Duplicate(was, true) : cJSON_CreateArray();
    const cJSON* above = chosen >= 0 ? clarity_tree_above(rows, chosen) : rows;
    const cJSON* id;
    if (chosen < 0) above = NULL;
    cJSON_ArrayForEach(id, above) {
        if (!holds(open, id)) cJSON_AddItemToArray(open, cJSON_Duplicate(id, true));
    }
    put(view, "shown", clarity_shown_tree(rows, open));
    put(view, "open", open);
}

static void fill_chart(cJSON* view, const cJSON* list) {
    const char* names[] = {text_of(view, "by", NULL), text_of(view, "of", NULL)};
    const char* as[] = {"by", "of"};
    put(view, "groups", clarity_chart_groups(fields_of(list, names, as, 2), text_of(view, "byKind", "text"), text_of(view, "measure", "count"),
                                             cJSON_GetObjectItemCaseSensitive(view, "values")));
}

void clarity_fill_views(ClarityWindowState* state, const ClaritySelected* sent, size_t sent_count) {
    if (!state->views) return;
    state->views = cJSON_Duplicate(state->views, true);
    cJSON* view;
    cJSON_ArrayForEach(view, state->views) {
        if (!cJSON_IsObject(view)) continue;
        const char* part = view->string;
        const cJSON* found = clarity_data_at(state->data, text_of(view, "list", ""));
        const cJSON* list = cJSON_IsArray(found) ? found : NULL;
        // A row a handler chose, or a window opened with, shows.
        int chosen = chosen_of(state->selected, state->selected_count, part);
        if (chosen == chosen_of(sent, sent_count, part)) chosen = -1;
        const char* kind = text_of(view, "kind", "");
        if (strcmp(kind, "table") == 0) fill_table(view, list, chosen);
        else if (strcmp(kind, "tree") == 0) fill_tree(view, list, chosen);
        else if (strcmp(kind, "chart") == 0) fill_chart(view, list);
    }
}

bool clarity_view_event(ClarityWindowState* state, const char* part, const cJSON* action) {
    // No action: the screen declared a view, which every reply fills.
    if (!action || !action->child) return true;
    cJSON* view = cJSON_GetObjectItemCaseSensitive(state->views, part);
    if (!cJSON_IsObject(view)) return clarity_fail(CLARITY_INVALID, "the window has no view %s", part);
    const cJSON* sort = cJSON_GetObjectItemCaseSensitive(action, "sort");
    const cJSON* page = cJSON_GetObjectItemCaseSensitive(action, "page");
    const cJSON* open = cJSON_GetObjectItemCaseSensitive(action, "open");
    const cJSON* close = cJSON_GetObjectItemCaseSensitive(action, "close");
    if (cJSON_IsString(sort)) {
        // A new order starts on the first page.
        put(view, "order", clarity_next_order(cJSON_GetObjectItemCaseSensitive(view, "order"), sort->valuestring));
        put(view, "at", cJSON_CreateNumber(1));
    } else if (cJSON_IsNumber(page)) {
        put(view, "at", cJSON_CreateNumber(page->valuedouble));
    } else if (open || close) {
        cJSON* ids = cJSON_GetObjectItemCaseSensitive(view, "open");
        if (!cJSON_IsArray(ids)) {
            ids = cJSON_CreateArray();
            put(view, "open", ids);
        }
        if (open && !holds(ids, open)) cJSON_AddItemToArray(ids, cJSON_Duplicate(open, true));
        for (int i = cJSON_GetArraySize(ids); close && i-- > 0;)
            if (cJSON_Compare(cJSON_GetArrayItem(ids, i), close, true)) cJSON_DeleteItemFromArray(ids, i);
    } else {
        return clarity_fail(CLARITY_INVALID, "a view takes sort, page, open or close, not %s", action ? cJSON_PrintUnformatted(action) : "nothing");
    }
    return true;
}
