#include "clarity/trees.h"

int* clarity_tree_parents(const cJSON** ids, const cJSON** parents, size_t n) {
    int* parent = CLARITY_NEW(int, n ? n : 1);
    for (size_t i = 0; i < n; ++i) {
        parent[i] = -1;
        if (!parents[i]) continue;
        for (size_t j = 0; j < n; ++j) {
            if (cJSON_Compare(ids[j], parents[i], true)) {
                parent[i] = (int)j;
                break;
            }
        }
    }
    // Break each loop at its row that comes first in the list.
    int* seen = CLARITY_NEW(int, n ? n : 1);
    for (size_t i = 0; i < n; ++i) {
        size_t count = 0;
        int p = (int)i;
        for (;;) {
            if (p == -1) break;
            size_t k = 0;
            while (k < count && seen[k] != p) ++k;
            if (k < count) break;
            seen[count++] = p;
            p = parent[p];
        }
        if (p == -1) continue;
        size_t from = 0;
        while (seen[from] != p) ++from;
        int first = seen[from];
        for (size_t k = from; k < count; ++k)
            if (seen[k] < first) first = seen[k];
        parent[first] = -1;
    }
    return parent;
}

/// The parent of each of `rows` ({id, parent}) as its place, and their ids.
static int* parents_of_rows(const cJSON* rows, const cJSON*** ids, size_t* n) {
    static cJSON none = {.type = cJSON_NULL};
    *n = (size_t)cJSON_GetArraySize(rows);
    *ids = CLARITY_NEW(const cJSON*, *n + 1);
    const cJSON** parents = CLARITY_NEW(const cJSON*, *n + 1);
    size_t i = 0;
    const cJSON* row;
    cJSON_ArrayForEach(row, rows) {
        const cJSON* id = cJSON_GetObjectItemCaseSensitive(row, "id");
        const cJSON* parent = cJSON_GetObjectItemCaseSensitive(row, "parent");
        (*ids)[i] = id ? id : &none;
        parents[i++] = parent && !cJSON_IsNull(parent) ? parent : NULL;
    }
    return clarity_tree_parents(*ids, parents, *n);
}

static bool is_open(const cJSON* open, const cJSON* id) {
    const cJSON* item;
    cJSON_ArrayForEach(item, open) {
        if (cJSON_Compare(item, id, true)) return true;
    }
    return false;
}

/// Adds the row at `place` to `out`, and the rows under it when it is open.
static void walk(cJSON* out, const cJSON** ids, const int* parent, size_t n, const cJSON* open, int place, int depth) {
    bool children = false;
    for (size_t i = 0; i < n && !children; ++i) children = parent[i] == place;
    cJSON* shown = cJSON_CreateArray();
    cJSON_AddItemToArray(shown, cJSON_CreateNumber(place));
    cJSON_AddItemToArray(shown, cJSON_CreateNumber(depth));
    cJSON_AddItemToArray(shown, cJSON_CreateBool(children));
    cJSON_AddItemToArray(out, shown);
    if (!children || !is_open(open, ids[place])) return;
    for (size_t i = 0; i < n; ++i)
        if (parent[i] == place) walk(out, ids, parent, n, open, (int)i, depth + 1);
}

cJSON* clarity_shown_tree(const cJSON* rows, const cJSON* open) {
    const cJSON** ids;
    size_t n;
    int* parent = parents_of_rows(rows, &ids, &n);
    cJSON* out = cJSON_CreateArray();
    for (size_t i = 0; i < n; ++i)
        if (parent[i] == -1) walk(out, ids, parent, n, open, (int)i, 0);
    return out;
}

cJSON* clarity_tree_above(const cJSON* rows, int at) {
    const cJSON** ids;
    size_t n;
    int* parent = parents_of_rows(rows, &ids, &n);
    cJSON* out = cJSON_CreateArray();
    if (at < 0 || (size_t)at >= n) return out;
    for (int p = parent[at]; p != -1; p = parent[p]) {
        // Top first: each row above goes before the ones found so far.
        cJSON* id = cJSON_Duplicate(ids[p], true);
        if (out->child) cJSON_InsertItemInArray(out, 0, id);
        else cJSON_AddItemToArray(out, id);
    }
    return out;
}
