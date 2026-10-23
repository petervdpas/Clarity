#include "clarity/query.h"

#include <ctype.h>
#include <string.h>

#include "cjson/cJSON.h"

typedef enum { COMPARE, CONTAINS, HOLDS, ONE_OF, IS_TRUE, IS_NONE, THROUGH, AND, OR, NOT } Kind;

struct ClarityCond {
    Kind kind;
    const char* column;
    ClarityOp op;
    const ClarityValue* values;
    size_t count;
    bool numeric;
    const char* text;
    ClarityHop hop;
    ClarityCond* inner[2];
};

typedef struct {
    const ClarityHop* through;
    size_t hops;
    const char* column;
    bool descending;
    bool numeric;
} Key;

struct ClarityQuery {
    ClarityCond* filter;
    Key* order;
    size_t count;
    size_t capacity;
};

static ClarityCond* cond(Kind kind, const char* column) {
    ClarityCond* c = CLARITY_NEW(ClarityCond, 1);
    c->kind = kind;
    c->column = column;
    return c;
}

static const ClarityValue* kept(size_t count, const ClarityValue* values) {
    ClarityValue* out = CLARITY_NEW(ClarityValue, count);
    for (size_t i = 0; i < count; ++i) out[i] = values[i];
    return out;
}

ClarityCond* clarity_compare(const char* column, ClarityOp op, ClarityValue value) {
    ClarityCond* c = cond(COMPARE, column);
    c->op = op;
    c->values = kept(1, &value);
    c->count = 1;
    return c;
}

ClarityCond* clarity_number(const char* column, ClarityOp op, ClarityValue value) {
    ClarityCond* c = clarity_compare(column, op, value);
    c->numeric = true;
    return c;
}

ClarityCond* clarity_contains(const char* column, const char* text) {
    ClarityCond* c = cond(CONTAINS, column);
    c->text = clarity_copy(text);
    return c;
}

ClarityCond* clarity_holds(const char* column, ClarityValue item) {
    ClarityCond* c = cond(HOLDS, column);
    c->values = kept(1, &item);
    c->count = 1;
    return c;
}

// An item of a list as `holds` looks for it: SQLite compares the item
// itself, PostgreSQL a list of it as JSON, MySQL the item as JSON, and SQL
// Server its text. A date or a decimal is its text, as a list keeps it.
static ClarityValue list_item(ClarityDialect d, ClarityValue item) {
    const char* text = "";
    switch (item.kind) {
    case CLARITY_INTEGER:
        if (d == CLARITY_SQLITE) return item;
        return clarity_text_value(clarity_format(d == CLARITY_POSTGRES ? "[%lld]" : "%lld", (long long)item.integer));
    case CLARITY_TEXT: text = item.text ? item.text : ""; break;
    case CLARITY_DECIMAL: text = clarity_decimal_text(item.decimal); break;
    case CLARITY_DATE: text = clarity_date_text(item.date); break;
    default: break;
    }
    if (d == CLARITY_SQLITE || d == CLARITY_MSSQL) return clarity_text_value(text);
    // JSON lives in the scratch arena.
    const char* json = cJSON_PrintUnformatted(cJSON_CreateString(text));
    return clarity_text_value(d == CLARITY_POSTGRES ? clarity_format("[%s]", json) : json);
}

ClarityCond* clarity_one_of(const char* column, size_t count, const ClarityValue* values) {
    ClarityCond* c = cond(ONE_OF, column);
    c->values = kept(count, values);
    c->count = count;
    return c;
}

ClarityCond* clarity_one_of_numbers(const char* column, size_t count, const ClarityValue* values) {
    ClarityCond* c = clarity_one_of(column, count, values);
    c->numeric = true;
    return c;
}

ClarityCond* clarity_is_true(const char* column) { return cond(IS_TRUE, column); }
ClarityCond* clarity_is_none(const char* column) { return cond(IS_NONE, column); }

ClarityCond* clarity_through(const char* column, const char* table, const char* key, ClarityCond* inner) {
    ClarityCond* c = cond(THROUGH, NULL);
    c->hop = (ClarityHop){column, table, key};
    c->inner[0] = inner;
    return c;
}

static ClarityCond* both(Kind kind, ClarityCond* a, ClarityCond* b) {
    ClarityCond* c = cond(kind, NULL);
    c->inner[0] = a;
    c->inner[1] = b;
    return c;
}

ClarityCond* clarity_and(ClarityCond* a, ClarityCond* b) { return both(AND, a, b); }
ClarityCond* clarity_or(ClarityCond* a, ClarityCond* b) { return both(OR, a, b); }
ClarityCond* clarity_not(ClarityCond* a) { return both(NOT, a, NULL); }

static const char* op_sql(ClarityOp op) {
    switch (op) {
    case CLARITY_EQ: return "=";
    case CLARITY_NOT_EQ: return "!=";
    case CLARITY_LT: return "<";
    case CLARITY_LT_EQ: return "<=";
    case CLARITY_GT: return ">";
    default: return ">=";
    }
}

static const char* column_sql(ClarityDialect d, const char* column, bool numeric) {
    const char* quoted = clarity_quote(d, column);
    return numeric ? clarity_number_sql(d, quoted) : quoted;
}

static const char* param(ClarityDialect d, ClarityValues* params, ClarityValue value, bool numeric) {
    clarity_values_add(params, value);
    const char* mark = clarity_mark(d, (int)params->count);
    return numeric ? clarity_number_sql(d, mark) : mark;
}

static const char* cond_sql(const ClarityCond* c, ClarityDialect d, ClarityValues* params) {
    switch (c->kind) {
    case COMPARE: {
        const char* column = column_sql(d, c->column, c->numeric);
        return clarity_format("%s %s %s", column, op_sql(c->op), param(d, params, c->values[0], c->numeric));
    }
    case CONTAINS: {
        const char* pattern = clarity_format("%%%s%%", clarity_escape_like(d, c->text));
        return clarity_contains_sql(d, c->column, param(d, params, clarity_text_value(pattern), false));
    }
    case HOLDS: return clarity_holds_sql(d, c->column, param(d, params, list_item(d, c->values[0]), false));
    case ONE_OF: {
        ClarityBuffer out = {0};
        clarity_buffer_add(&out, column_sql(d, c->column, c->numeric));
        clarity_buffer_add(&out, " IN (");
        for (size_t i = 0; i < c->count; ++i) {
            if (i) clarity_buffer_add(&out, ", ");
            clarity_buffer_add(&out, param(d, params, c->values[i], c->numeric));
        }
        clarity_buffer_add(&out, ")");
        return clarity_buffer_text(&out);
    }
    case IS_TRUE: return clarity_is_true_sql(d, c->column);
    case IS_NONE: return clarity_format("%s IS NULL", clarity_quote(d, c->column));
    // The inner condition names the columns of the referred table, which a
    // subquery's own FROM resolves first, even for a table that refers to itself.
    case THROUGH: {
        const char* column = clarity_quote(d, c->hop.column);
        const char* key = clarity_quote(d, c->hop.key);
        const char* table = clarity_quote(d, c->hop.table);
        return clarity_format("%s IN (SELECT %s FROM %s WHERE %s)", column, key, table, cond_sql(c->inner[0], d, params));
    }
    case AND:
    case OR: {
        const char* left = cond_sql(c->inner[0], d, params);
        const char* right = cond_sql(c->inner[1], d, params);
        return clarity_format("(%s %s %s)", left, c->kind == AND ? "AND" : "OR", right);
    }
    default: return clarity_format("NOT (%s)", cond_sql(c->inner[0], d, params));
    }
}

ClarityQuery* clarity_query(void) { return CLARITY_NEW(ClarityQuery, 1); }

ClarityQuery* clarity_where(ClarityQuery* query, ClarityCond* filter) {
    query->filter = filter;
    return query;
}

ClarityQuery* clarity_order_through(ClarityQuery* query, size_t hop_count, const ClarityHop* hops, const char* column, bool descending, bool numeric) {
    clarity_grow((void**)&query->order, &query->capacity, query->count, sizeof *query->order);
    ClarityHop* through = CLARITY_NEW(ClarityHop, hop_count);
    for (size_t i = 0; i < hop_count; ++i) through[i] = hops[i];
    query->order[query->count++] = (Key){through, hop_count, column, descending, numeric};
    return query;
}

ClarityQuery* clarity_order_by(ClarityQuery* query, const char* column, bool descending) {
    return clarity_order_through(query, 0, NULL, column, descending, false);
}

ClarityQuery* clarity_order_by_number(ClarityQuery* query, const char* column, bool descending) {
    return clarity_order_through(query, 0, NULL, column, descending, true);
}

// Through references, each step is a subquery on its own alias (t1, t2, ...)
// that finds the referred record by the column of the step before.
static const char* key_sql(ClarityDialect d, const char* table, const Key* key) {
    const char* direction = key->descending ? " DESC" : "";
    if (key->hops == 0) return clarity_format("%s%s", column_sql(d, key->column, key->numeric), direction);
    const char** aliases = CLARITY_NEW(const char*, key->hops);
    for (size_t i = 0; i < key->hops; ++i) aliases[i] = clarity_quote(d, clarity_format("t%zu", i + 1));
    const char* column = clarity_format("%s.%s", aliases[key->hops - 1], clarity_quote(d, key->column));
    const char* inner = key->numeric ? clarity_number_sql(d, column) : column;
    for (size_t i = key->hops; i-- > 0;) {
        const ClarityHop* hop = &key->through[i];
        const char* before = i == 0 ? clarity_quote(d, table) : aliases[i - 1];
        inner = clarity_format("(SELECT %s FROM %s %s WHERE %s.%s = %s.%s)", inner, clarity_quote(d, hop->table), aliases[i], aliases[i],
                               clarity_quote(d, hop->key), before, clarity_quote(d, hop->column));
    }
    return clarity_format("%s%s", inner, direction);
}

const char* clarity_query_sql(const ClarityQuery* query, ClarityDialect d, const char* table, const char* key, const char* columns, ClarityValues* params) {
    ClarityBuffer sql = {0};
    clarity_buffer_addf(&sql, "SELECT %s FROM %s", columns, clarity_quote(d, table));
    if (query && query->filter) clarity_buffer_addf(&sql, " WHERE %s", cond_sql(query->filter, d, params));
    clarity_buffer_add(&sql, " ORDER BY ");
    for (size_t i = 0; query && i < query->count; ++i) clarity_buffer_addf(&sql, "%s, ", key_sql(d, table, &query->order[i]));
    clarity_buffer_add(&sql, clarity_quote(d, key));
    return clarity_buffer_text(&sql);
}

bool clarity_text_contains(const char* text, const char* part) {
    size_t n = strlen(text ? text : ""), m = strlen(part ? part : "");
    if (m == 0) return true;
    for (size_t i = 0; i + m <= n; ++i) {
        size_t k = 0;
        while (k < m && tolower((unsigned char)text[i + k]) == tolower((unsigned char)part[k])) ++k;
        if (k == m) return true;
    }
    return false;
}
