// The SQL dialects of the databases a data source can be. Statements that
// read and write records are written here, so the rest of the runtime works
// the same on every database. The other runtimes have the same rules;
// conformance/queries.json keeps them equal. Text comes from the scratch arena.

#pragma once

#include "clarity/core.h"

typedef enum { CLARITY_SQLITE, CLARITY_POSTGRES, CLARITY_MYSQL, CLARITY_MSSQL } ClarityDialect;

/// The database's name, for messages.
const char* clarity_dialect_label(ClarityDialect d);
/// A table or column name as SQL needs it, so names like order work.
const char* clarity_quote(ClarityDialect d, const char* name);
/// The mark of parameter `n`, counted from 1.
const char* clarity_mark(ClarityDialect d, int n);
/// A decimal as a comparison reads it: SQLite stores decimals as text.
const char* clarity_number_sql(ClarityDialect d, const char* expr);
/// `column` containing the LIKE pattern `mark`, ignoring case; wildcards are escaped with !.
const char* clarity_contains_sql(ClarityDialect d, const char* column, const char* mark);
/// Whether the JSON list kept in `column` holds the item `mark` (a list field).
const char* clarity_holds_sql(ClarityDialect d, const char* column, const char* mark);
/// Text that LIKE matches as it is.
const char* clarity_escape_like(ClarityDialect d, const char* text);
/// A bool column that is true.
const char* clarity_is_true_sql(ClarityDialect d, const char* column);
/// The INSERT of a new record, giving its key back on PostgreSQL and SQL Server.
const char* clarity_insert_sql(ClarityDialect d, const char* table, const char* key, const char* const* columns, size_t count);
const char* clarity_begin_sql(ClarityDialect d);
const char* clarity_commit_sql(ClarityDialect d);
const char* clarity_rollback_sql(ClarityDialect d);
/// Creates the table that remembers how many migration steps a database has had.
const char* clarity_version_table_sql(ClarityDialect d);
