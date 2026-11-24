// The connection to one database, behind the same few calls for every
// kind: run a statement, insert and get the new key, read rows, run SQL
// without parameters. SQLite comes first; the server databases follow. The
// program does not migrate (clarity-migrate does, docs/ledger/migrations.md).

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
    void (*close)(ClarityDriver* driver);
};

/// Where a database or store is: as written, or from the environment
/// variable `env:NAME` names, so passwords stay out of the program.
bool clarity_location(const char* text, const char** out);

/// The environment variable that names the folder a program's SQLite databases open in.
#define CLARITY_DATA_VARIABLE "CLARITY_DATA"
/// The file of an SQLite database at `location`, with `env` reading the
/// environment (the process's when NULL): in the folder CLARITY_DATA names,
/// under its own file name, or where the project put it when the variable
/// is unset or empty. A location from the environment (`env:`) or a secret
/// setting (`setting:`) is as it says, and so is SQLite's own `:memory:`.
/// The same as the Rust and C# runtimes.
const char* clarity_data_folder_sqlite(const char* location, const char* (*env)(const char* name));
/* clarity_data_folder_sqlite on system `os` ("windows", "macos" or another): a
 * data:<program>/<file> location opens in CLARITY_DATA's folder, else in the
 * person's folder for application data (conformance/data-folder.json). */
const char* clarity_data_folder_sqlite_on(const char* os, const char* location, const char* (*env)(const char* name));

/* The program's data folder (conformance/data-folder.json, folders): the
 * folder CLARITY_DATA names, else the person's folder for application data
 * under the program's name; "" when the environment names neither, which is
 * where the program runs. A relative file store and a relative folder
 * setting are in it, as a relative SQLite database is. `env` NULL reads the
 * process's environment. */
const char* clarity_data_folder(const char* program, const char* (*env)(const char* name));
/// clarity_data_folder on system `os` ("windows", "macos" or another).
const char* clarity_data_folder_on(const char* os, const char* program, const char* (*env)(const char* name));
/* The folder of a file store at `location` (conformance/data-folder.json,
 * stores): data:<program>/<name> (a relative folder in the project file) is
 * in the program's data folder under its own name; env:NAME is that
 * variable's value, CLARITY_NOT_SET while it is unset or empty; anything
 * else is as it is. */
bool clarity_data_folder_store(const char* location, const char* (*env)(const char* name), const char** out);
/// clarity_data_folder_store on system `os`.
bool clarity_data_folder_store_on(const char* os, const char* location, const char* (*env)(const char* name), const char** out);
/// `name` in `folder`; just `name` when there is no folder.
const char* clarity_in_folder(const char* folder, const char* name);

/// Why the databases did not open, as the person reads it (manual/data.settings.md,
/// conformance/unconnected.json): in Clarity's own words, which the project
/// translates, with this runtime's message in the slot.
ClarityText* clarity_unconnected_reason(const ClarityError* error);
/// Connects to the database of a data source.
bool clarity_open_driver(ClarityDialect dialect, const char* at, ClarityDriver** out);
bool clarity_open_sqlite(const char* path, ClarityDriver** out);
/// Whether a connection clarity_open_sqlite made is outside a transaction.
bool clarity_sqlite_autocommit(ClarityDriver* driver);
