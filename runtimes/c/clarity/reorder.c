#include "clarity/reorder.h"

#include <string.h>

#include "clarity/trees.h"

/// Whether two parents are the same row's id; NULL is no parent.
static bool same_parent(const cJSON* a, const cJSON* b) {
    if (!a || !b) return !a && !b;
    return cJSON_Compare(a, b, true);
}

/// Each row's place at its level counted from 1, in `order`; `levels` by old place.
static int64_t* numbered(const int* levels, const size_t* order, size_t n) {
    // Counts by level: the top first, then under each place.
    int64_t* counts = CLARITY_NEW(int64_t, n + 1);
    int64_t* out = CLARITY_NEW(int64_t, n ? n : 1);
    for (size_t i = 0; i < n; ++i) out[i] = ++counts[levels[order[i]] + 1];
    return out;
}

static const cJSON* id_of(const cJSON* row) {
    static cJSON none = {.type = cJSON_NULL};
    const cJSON* id = cJSON_GetObjectItemCaseSensitive(row, "id");
    return id ? id : &none;
}

static const char* text_of(const cJSON* move, const char* name) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(move, name);
    return cJSON_IsString(v) ? v->valuestring : NULL;
}

static int place_of(const cJSON* move, const char* name) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(move, name);
    return cJSON_IsNumber(v) ? (int)v->valuedouble : -1;
}

bool clarity_reorder(const cJSON* rows, const cJSON* move, bool tree, ClarityReordered* out) {
    size_t n = cJSON_IsArray(rows) ? (size_t)cJSON_GetArraySize(rows) : 0;
    int r = place_of(move, "row");
    if (r < 0 || (size_t)r >= n) return false;
    const cJSON** ids = CLARITY_NEW(const cJSON*, n);
    const cJSON** parents = CLARITY_NEW(const cJSON*, n);
    size_t i = 0;
    const cJSON* row;
    cJSON_ArrayForEach(row, rows) {
        const cJSON* parent = tree ? cJSON_GetObjectItemCaseSensitive(row, "parent") : NULL;
        ids[i] = id_of(row);
        parents[i++] = parent && !cJSON_IsNull(parent) ? parent : NULL;
    }
    int* levels = clarity_tree_parents(ids, parents, n);
    // The rows at the level under `place` (-1 the top), in order.
    int* siblings = CLARITY_NEW(int, n);
    size_t sibling_count = 0;
    const cJSON* parent = parents[r];
    int anchor;
    bool after;
    const char* key = text_of(move, "key");
    if (key) {
        int k = 0;
        for (size_t j = 0; j < n; ++j) {
            if (levels[j] != levels[r]) continue;
            if ((int)j == r) k = (int)sibling_count;
            siblings[sibling_count++] = (int)j;
        }
        if (strcmp(key, "up") == 0 || strcmp(key, "down") == 0) {
            bool down = key[0] == 'd';
            int next = down ? k + 1 : k - 1;
            if (next < 0 || next >= (int)sibling_count) return false;
            anchor = siblings[next];
            after = down;
        } else if (!tree) {
            return false;
        } else if (strcmp(key, "left") == 0) {
            if (levels[r] == -1) return false;
            anchor = levels[r];
            after = true;
            parent = parents[levels[r]];
        } else if (strcmp(key, "right") == 0) {
            if (k == 0) return false;
            int before = siblings[k - 1];
            // The last of that row and the rows right under it, leaving out the moved row.
            anchor = before;
            for (size_t j = 0; j < n; ++j)
                if (levels[j] == before && (int)j != r && (int)j > anchor) anchor = (int)j;
            after = true;
            parent = ids[before];
        } else {
            return false;
        }
    } else {
        int t = place_of(move, "target");
        const char* where = text_of(move, "where");
        if (!where || t < 0 || (size_t)t >= n || t == r) return false;
        if (tree) {
            for (int p = t; p != -1; p = levels[p])
                if (p == r) return false;
        }
        if (strcmp(where, "into") == 0) {
            if (!tree) return false;
            anchor = t;
            for (size_t j = 0; j < n; ++j)
                if (levels[j] == t && (int)j != r && (int)j > anchor) anchor = (int)j;
            after = true;
            parent = ids[t];
        } else if (strcmp(where, "before") == 0 || strcmp(where, "after") == 0) {
            anchor = t;
            after = where[0] == 'a';
            parent = tree ? parents[t] : NULL;
        } else {
            return false;
        }
    }
    // The list without the moved row, then the row put in beside its anchor.
    size_t* order = CLARITY_NEW(size_t, n);
    size_t count = 0, at = 0;
    for (size_t j = 0; j < n; ++j) {
        if ((int)j == r) continue;
        if ((int)j == anchor) at = count + (after ? 1 : 0);
        order[count++] = j;
    }
    memmove(order + at + 1, order + at, (count - at) * sizeof *order);
    order[at] = (size_t)r;
    const cJSON** moved = CLARITY_NEW(const cJSON*, n);
    for (size_t j = 0; j < n; ++j) moved[j] = (int)j == r ? parent : parents[j];
    const cJSON** new_ids = CLARITY_NEW(const cJSON*, n);
    const cJSON** new_parents = CLARITY_NEW(const cJSON*, n);
    for (size_t j = 0; j < n; ++j) {
        new_ids[j] = ids[order[j]];
        new_parents[j] = moved[order[j]];
    }
    int* new_levels = clarity_tree_parents(new_ids, new_parents, n);
    // Levels by old place, so both lists are numbered alike.
    int* by_place = CLARITY_NEW(int, n);
    for (size_t j = 0; j < n; ++j) by_place[order[j]] = new_levels[j] == -1 ? -1 : (int)order[new_levels[j]];
    int64_t* numbers = numbered(by_place, order, n);
    size_t* places = CLARITY_NEW(size_t, n);
    for (size_t j = 0; j < n; ++j) places[j] = j;
    int64_t* before = numbered(levels, places, n);
    bool same = true;
    for (size_t j = 0; j < n && same; ++j)
        same = numbers[j] == before[order[j]] && same_parent(moved[order[j]], parents[order[j]]);
    if (same) return false;
    *out = (ClarityReordered){n, order, new_parents, numbers};
    return true;
}
