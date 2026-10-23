// The SQL of queries per database, against the cases every stack's runtime
// shares (conformance/queries.json).

#include "check.h"
#include "clarity/query.h"
#include "tests.h"

static ClarityOp op(const cJSON* value) {
    const char* name = value->valuestring;
    if (strcmp(name, "Eq") == 0) return CLARITY_EQ;
    if (strcmp(name, "NotEq") == 0) return CLARITY_NOT_EQ;
    if (strcmp(name, "Lt") == 0) return CLARITY_LT;
    if (strcmp(name, "LtEq") == 0) return CLARITY_LT_EQ;
    if (strcmp(name, "Gt") == 0) return CLARITY_GT;
    return CLARITY_GT_EQ;
}

static const char* text(const cJSON* in, const char* name) { return cJSON_GetObjectItemCaseSensitive(in, name)->valuestring; }

static const cJSON* item(const cJSON* in, const char* name) { return cJSON_GetObjectItemCaseSensitive(in, name); }

static ClarityValue sql_value(const cJSON* value) {
    const cJSON* inner = value->child;
    const char* kind = inner->string;
    if (strcmp(kind, "text") == 0) return clarity_text_value(inner->valuestring);
    if (strcmp(kind, "integer") == 0) return clarity_integer_value((int64_t)inner->valuedouble);
    if (strcmp(kind, "decimal") == 0) {
        ClarityDecimal d;
        clarity_decimal_parse(inner->valuestring, &d);
        return clarity_decimal_value(d);
    }
    if (strcmp(kind, "bool") == 0) return clarity_bool_value(cJSON_IsTrue(inner));
    if (strcmp(kind, "date") == 0) {
        ClarityDate d;
        clarity_date_parse(inner->valuestring, &d);
        return clarity_date_value(d);
    }
    return clarity_null();
}

/// A parameter as the cases write it.
static cJSON* case_value(ClarityValue v) {
    cJSON* out = cJSON_CreateObject();
    switch (v.kind) {
    case CLARITY_TEXT: cJSON_AddStringToObject(out, "text", v.text); break;
    case CLARITY_INTEGER: cJSON_AddNumberToObject(out, "integer", (double)v.integer); break;
    case CLARITY_DECIMAL: cJSON_AddStringToObject(out, "decimal", clarity_decimal_text(v.decimal)); break;
    case CLARITY_BOOL: cJSON_AddBoolToObject(out, "bool", v.flag); break;
    case CLARITY_DATE: cJSON_AddStringToObject(out, "date", clarity_date_text(v.date)); break;
    default: cJSON_AddNullToObject(out, "null"); break;
    }
    return out;
}

static size_t values(const cJSON* list, ClarityValue** out) {
    size_t count = (size_t)cJSON_GetArraySize(list);
    *out = CLARITY_NEW(ClarityValue, count);
    size_t i = 0;
    const cJSON* v;
    cJSON_ArrayForEach(v, list)(*out)[i++] = sql_value(v);
    return count;
}

static ClarityCond* cond(const cJSON* value) {
    const cJSON* inner = value->child;
    const char* kind = inner->string;
    ClarityValue* list;
    if (strcmp(kind, "compare") == 0) return clarity_compare(text(inner, "column"), op(item(inner, "op")), sql_value(item(inner, "value")));
    if (strcmp(kind, "number") == 0) return clarity_number(text(inner, "column"), op(item(inner, "op")), sql_value(item(inner, "value")));
    if (strcmp(kind, "contains") == 0) return clarity_contains(text(inner, "column"), text(inner, "text"));
    if (strcmp(kind, "holds") == 0) return clarity_holds(text(inner, "column"), sql_value(item(inner, "value")));
    if (strcmp(kind, "matches") == 0) {
        const cJSON* listed = item(inner, "columns");
        size_t n = (size_t)cJSON_GetArraySize(listed);
        const char** columns = CLARITY_NEW(const char*, n ? n : 1);
        size_t i = 0;
        const cJSON* column;
        cJSON_ArrayForEach(column, listed) columns[i++] = column->valuestring;
        return clarity_matches(text(inner, "table"), text(inner, "key"), n, columns, text(inner, "text"));
    }
    if (strcmp(kind, "oneOf") == 0) {
        size_t n = values(item(inner, "values"), &list);
        return clarity_one_of(text(inner, "column"), n, list);
    }
    if (strcmp(kind, "oneOfNumbers") == 0) {
        size_t n = values(item(inner, "values"), &list);
        return clarity_one_of_numbers(text(inner, "column"), n, list);
    }
    if (strcmp(kind, "isTrue") == 0) return clarity_is_true(inner->valuestring);
    if (strcmp(kind, "isNone") == 0) return clarity_is_none(inner->valuestring);
    if (strcmp(kind, "and") == 0) return clarity_and(cond(cJSON_GetArrayItem(inner, 0)), cond(cJSON_GetArrayItem(inner, 1)));
    if (strcmp(kind, "or") == 0) return clarity_or(cond(cJSON_GetArrayItem(inner, 0)), cond(cJSON_GetArrayItem(inner, 1)));
    if (strcmp(kind, "not") == 0) return clarity_not(cond(inner));
    return clarity_through(text(inner, "column"), text(inner, "table"), text(inner, "key"), cond(item(inner, "filter")));
}

static ClarityQuery* query(const cJSON* value) {
    ClarityQuery* q = clarity_query();
    if (item(value, "filter")) clarity_where(q, cond(item(value, "filter")));
    const cJSON* key;
    cJSON_ArrayForEach(key, item(value, "order")) {
        const cJSON* through = item(key, "through");
        size_t n = (size_t)cJSON_GetArraySize(through);
        ClarityHop* hops = CLARITY_NEW(ClarityHop, n);
        size_t i = 0;
        const cJSON* h;
        cJSON_ArrayForEach(h, through) hops[i++] = (ClarityHop){text(h, "column"), text(h, "table"), text(h, "key")};
        clarity_order_through(q, n, hops, text(key, "column"), cJSON_IsTrue(item(key, "descending")), cJSON_IsTrue(item(key, "numeric")));
    }
    return q;
}

void every_query_case_gives_its_sql_on_every_database(void) {
    cJSON* file = conformance("queries.json");
    const cJSON* cases = item(file, "cases");
    REQUIRE(cJSON_GetArraySize(cases) > 0);
    const struct {
        const char* word;
        ClarityDialect dialect;
    } dialects[] = {{"sqlite", CLARITY_SQLITE}, {"postgres", CLARITY_POSTGRES}, {"mysql", CLARITY_MYSQL}, {"mssql", CLARITY_MSSQL}};
    const cJSON* c;
    cJSON_ArrayForEach(c, cases) {
        ClarityQuery* q = query(item(c, "query"));
        for (size_t k = 0; k < 4; ++k) {
            ClarityDialect d = dialects[k].dialect;
            ClarityBuffer columns = {0};
            clarity_buffer_add(&columns, clarity_quote(d, text(c, "key")));
            const cJSON* column;
            cJSON_ArrayForEach(column, item(c, "columns")) clarity_buffer_addf(&columns, ", %s", clarity_quote(d, column->valuestring));
            ClarityValues params = {0};
            const char* sql = clarity_query_sql(q, d, text(c, "table"), text(c, "key"), clarity_buffer_text(&columns), &params);
            const cJSON* expect = item(item(c, "expect"), dialects[k].word);
            CHECK_TEXT(sql, text(expect, "sql"));
            cJSON* got = cJSON_CreateArray();
            for (size_t i = 0; i < params.count; ++i) cJSON_AddItemToArray(got, case_value(params.items[i]));
            CHECK_JSON(got, cJSON_PrintUnformatted(item(expect, "params")));
        }
    }
}
