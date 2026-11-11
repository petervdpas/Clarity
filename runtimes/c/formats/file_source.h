// A data source that is a file in a format (docs/FORMATS-PLAN.md, "A file
// as a data source"): `datasource logins csv "imports/logins.csv" external`.
// The file is read through Clarity.Formats into one table of an in-memory
// SQLite database, so the generated code finds, saves and deletes as it
// does in any SQLite database. After each change that is kept (outside a
// transaction, or when it commits) the table is written back to a new file
// that then replaces the old one, so a crash never leaves half a file; the
// file's own column order and the columns no entity maps are kept. Carried
// with the formats part (clarity_use_formats in clarity.cmake).

#pragma once

#include "clarity/db.h"

/// What the generator says of a file source: its format and options (a
/// JSON object, "" for none), the entity's table and key column, and the
/// columns the entity maps with their kinds (JSON
/// `[{"name":"site","kind":"text"},...]`).
typedef struct {
    const char* name;
    const char* format;
    const char* path;
    const char* options;
    const char* table;
    const char* key;
    const char* fields;
    /// A readonly source never writes its file.
    bool readonly;
} ClarityFileSource;

/// Adds the file source `source->name`, its file found as an SQLite file's
/// location is (CLARITY_DATA, `env:`), read whole into its table. Fails,
/// `data source <name>: ...`, when the file cannot be read or does not fit.
bool clarity_db_add_file(ClarityDb* db, const ClarityFileSource* source);
