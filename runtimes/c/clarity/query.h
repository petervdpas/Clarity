// Queries: which records of an entity to read, and in which order.
// Generated code builds a query from Clarity's find; this file turns it
// into SQL with parameters, in the dialect of the database the records are
// in. Conditions and queries live in the scratch arena.

#pragma once

#include "clarity/dialect.h"
#include "clarity/value.h"

/// A comparison between a column and a value.
typedef enum { CLARITY_EQ, CLARITY_NOT_EQ, CLARITY_LT, CLARITY_LT_EQ, CLARITY_GT, CLARITY_GT_EQ } ClarityOp;

/// A step through a reference: the column that holds it, and the table and
/// key column of the records it refers to.
typedef struct {
    const char* column;
    const char* table;
    const char* key;
} ClarityHop;

/// A condition on the columns of one table.
typedef struct ClarityCond ClarityCond;

/// `column op value`.
ClarityCond* clarity_compare(const char* column, ClarityOp op, ClarityValue value);
/// A comparison of a decimal or money column, as numbers.
ClarityCond* clarity_number(const char* column, ClarityOp op, ClarityValue value);
/// The column's text contains `text`, ignoring case.
ClarityCond* clarity_contains(const char* column, const char* text);
/// Whether the JSON list kept in the column holds `item`, exactly: a list
/// field's `a.tags contains "live"`.
ClarityCond* clarity_holds(const char* column, ClarityValue item);
ClarityCond* clarity_one_of(const char* column, size_t count, const ClarityValue* values);
/// `clarity_one_of` for a decimal or money column.
ClarityCond* clarity_one_of_numbers(const char* column, size_t count, const ClarityValue* values);
/// A bool column that is true.
ClarityCond* clarity_is_true(const char* column);
/// A column with no value: an empty date, reference or picture (`none`).
ClarityCond* clarity_is_none(const char* column);
/// `cond` on the record `column` refers to, in `table` by `key`.
ClarityCond* clarity_through(const char* column, const char* table, const char* key, ClarityCond* cond);
ClarityCond* clarity_and(ClarityCond* a, ClarityCond* b);
ClarityCond* clarity_or(ClarityCond* a, ClarityCond* b);
ClarityCond* clarity_not(ClarityCond* a);

/// The records to read: a condition and an order. Without an order, records
/// come in the order they were stored.
typedef struct ClarityQuery ClarityQuery;

/// A query for every record.
ClarityQuery* clarity_query(void);
/// The query with its condition; gives `query` back.
ClarityQuery* clarity_where(ClarityQuery* query, ClarityCond* cond);
ClarityQuery* clarity_order_by(ClarityQuery* query, const char* column, bool descending);
/// `clarity_order_by` for a decimal or money column.
ClarityQuery* clarity_order_by_number(ClarityQuery* query, const char* column, bool descending);
/// Ordered by a column of the record the references `hops` lead to.
ClarityQuery* clarity_order_through(ClarityQuery* query, size_t hop_count, const ClarityHop* hops, const char* column, bool descending, bool numeric);

/// The SELECT for `table` in `d`, reading `columns`, and its parameters.
/// Records that sort the same come in the order of `key`.
const char* clarity_query_sql(const ClarityQuery* query, ClarityDialect d, const char* table, const char* key, const char* columns, ClarityValues* params);

/// `text contains part`, ignoring case: Clarity's `contains` outside a query.
bool clarity_text_contains(const char* text, const char* part);
