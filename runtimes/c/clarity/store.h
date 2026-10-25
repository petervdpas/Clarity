// The connection to one database, behind the same few calls for every
// kind: run a statement, insert and get the new key, read rows, and apply a
// migration step. SQLite comes first; the server databases follow.

#pragma once

#include "clarity/dialect.h"
#include "clarity/text.h"
#include "clarity/value.h"

/// One row a query read: a value per column.
typedef struct {
    ClarityValue* values;
    size_t count;
} ClarityRow;

typedef struct {
    ClarityRow* items;
    size_t count;
    size_t capacity;
} ClarityRows;

/// The statements of one migration step.
typedef struct {
    const char* const* statements;
    size_t count;
} ClarityStep;

typedef struct ClarityDriver ClarityDriver;

/// A database connection; `params` may be NULL for none.
struct ClarityDriver {
    ClarityDialect dialect;
    /// Runs a statement; `changed` (may be NULL) gets the number of rows it changed.
    bool (*execute)(ClarityDriver* driver, const char* sql, const ClarityValues* params, int64_t* changed);
    /// Runs an INSERT from clarity_insert_sql and gives the new key.
    bool (*insert)(ClarityDriver* driver, const char* sql, const ClarityValues* params, int64_t* key);
    bool (*rows)(ClarityDriver* driver, const char* sql, const ClarityValues* params, ClarityRows* out);
    /// Runs SQL without parameters, such as a statement that creates a table.
    bool (*batch)(ClarityDriver* driver, const char* sql);
    /// Runs the statements of migration step `version` and records it, all or nothing.
    bool (*apply)(ClarityDriver* driver, int version, const ClarityStep* step);
    void (*close)(ClarityDriver* driver);
};

/// The statement that records migration step `version`.
const char* clarity_applied_sql(int version);
/// Where a database or store is: as written, or from the environment
/// variable `env:NAME` names, so passwords stay out of the program.
bool clarity_location(const char* text, const char** out);

/// Why the databases did not open, as the person reads it (DESIGN 5.27,
/// conformance/unconnected.json): in Clarity's own words, which the project
/// translates, with this runtime's message in the slot.
ClarityText* clarity_unconnected_reason(const ClarityError* error);
/// Connects to the database of a data source.
bool clarity_open_driver(ClarityDialect dialect, const char* at, ClarityDriver** out);
bool clarity_open_sqlite(const char* path, ClarityDriver** out);
