#include "clarity/dialect.h"

/// `name` between `open` and `close`, a `close` inside it doubled.
static const char* enclosed(const char* name, char open, char close) {
    ClarityBuffer out = {0};
    clarity_buffer_add_char(&out, open);
    for (const char* c = name; *c; ++c) {
        if (*c == close) clarity_buffer_add_char(&out, close);
        clarity_buffer_add_char(&out, *c);
    }
    clarity_buffer_add_char(&out, close);
    return clarity_buffer_text(&out);
}

const char* clarity_dialect_label(ClarityDialect d) {
    switch (d) {
    case CLARITY_SQLITE: return "SQLite";
    case CLARITY_POSTGRES: return "PostgreSQL";
    case CLARITY_MYSQL: return "MySQL";
    default: return "SQL Server";
    }
}

const char* clarity_quote(ClarityDialect d, const char* name) {
    switch (d) {
    case CLARITY_MYSQL: return enclosed(name, '`', '`');
    case CLARITY_MSSQL: return enclosed(name, '[', ']');
    default: return enclosed(name, '"', '"');
    }
}

const char* clarity_mark(ClarityDialect d, int n) {
    switch (d) {
    case CLARITY_SQLITE: return clarity_format("?%d", n);
    case CLARITY_POSTGRES: return clarity_format("$%d", n);
    case CLARITY_MYSQL: return "?";
    default: return clarity_format("@P%d", n);
    }
}

const char* clarity_number_sql(ClarityDialect d, const char* expr) {
    return d == CLARITY_SQLITE ? clarity_format("CAST(%s AS REAL)", expr) : expr;
}

const char* clarity_contains_sql(ClarityDialect d, const char* column, const char* mark) {
    // SQLite's LIKE ignores case already.
    if (d == CLARITY_SQLITE) return clarity_format("%s LIKE %s ESCAPE '!'", clarity_quote(d, column), mark);
    return clarity_format("LOWER(%s) LIKE LOWER(%s) ESCAPE '!'", clarity_quote(d, column), mark);
}

const char* clarity_escape_like(ClarityDialect d, const char* text) {
    ClarityBuffer out = {0};
    for (const char* c = text ? text : ""; *c; ++c) {
        if (*c == '!' || *c == '%' || *c == '_' || (*c == '[' && d == CLARITY_MSSQL)) clarity_buffer_add_char(&out, '!');
        clarity_buffer_add_char(&out, *c);
    }
    return clarity_buffer_text(&out);
}

const char* clarity_is_true_sql(ClarityDialect d, const char* column) {
    return clarity_format("%s%s", clarity_quote(d, column), d == CLARITY_POSTGRES ? " = TRUE" : " = 1");
}

const char* clarity_insert_sql(ClarityDialect d, const char* table, const char* key, const char* const* columns, size_t count) {
    const char* t = clarity_quote(d, table);
    const char* k = clarity_quote(d, key);
    if (count == 0) {
        switch (d) {
        case CLARITY_MYSQL: return clarity_format("INSERT INTO %s () VALUES ()", t);
        case CLARITY_SQLITE: return clarity_format("INSERT INTO %s DEFAULT VALUES", t);
        case CLARITY_POSTGRES: return clarity_format("INSERT INTO %s DEFAULT VALUES RETURNING %s", t, k);
        default: return clarity_format("INSERT INTO %s OUTPUT INSERTED.%s DEFAULT VALUES", t, k);
        }
    }
    ClarityBuffer names = {0}, marks = {0};
    for (size_t i = 0; i < count; ++i) {
        if (i) {
            clarity_buffer_add(&names, ", ");
            clarity_buffer_add(&marks, ", ");
        }
        clarity_buffer_add(&names, clarity_quote(d, columns[i]));
        clarity_buffer_add(&marks, clarity_mark(d, (int)i + 1));
    }
    const char* list = clarity_buffer_text(&names);
    const char* values = clarity_buffer_text(&marks);
    switch (d) {
    case CLARITY_POSTGRES: return clarity_format("INSERT INTO %s (%s) VALUES (%s) RETURNING %s", t, list, values, k);
    case CLARITY_MSSQL: return clarity_format("INSERT INTO %s (%s) OUTPUT INSERTED.%s VALUES (%s)", t, list, k, values);
    default: return clarity_format("INSERT INTO %s (%s) VALUES (%s)", t, list, values);
    }
}

const char* clarity_begin_sql(ClarityDialect d) {
    switch (d) {
    case CLARITY_MYSQL: return "START TRANSACTION";
    case CLARITY_MSSQL: return "BEGIN TRANSACTION";
    default: return "BEGIN";
    }
}

const char* clarity_commit_sql(ClarityDialect d) { return d == CLARITY_MSSQL ? "COMMIT TRANSACTION" : "COMMIT"; }
const char* clarity_rollback_sql(ClarityDialect d) { return d == CLARITY_MSSQL ? "ROLLBACK TRANSACTION" : "ROLLBACK"; }

const char* clarity_version_table_sql(ClarityDialect d) {
    if (d == CLARITY_MSSQL) return "IF OBJECT_ID(N'clarity_schema', N'U') IS NULL CREATE TABLE clarity_schema (version INT NOT NULL)";
    return "CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL)";
}
