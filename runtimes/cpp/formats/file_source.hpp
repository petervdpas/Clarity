// A data source that is a file in a format (docs/FORMATS-PLAN.md, "A file
// as a data source"): `datasource logins csv "imports/logins.csv" external`.
// The file is read through Clarity.Formats into one table of an in-memory
// SQLite database, so the generated code finds, saves and deletes as it
// does in any SQLite database. After each change that is kept (outside a
// transaction, or when it commits) the table is written back to a new file
// that then replaces the old one, so a crash never leaves half a file; the
// file's own column order and the columns no entity maps are kept. Carried
// with the formats part (clarity_use_formats in clarity.cmake); Db::add_file
// adds one.

#pragma once

#include <string>

#include "clarity/db.hpp"

namespace clarity {

/// What the generator says of a file source: its format and options (a
/// JSON object, empty for none), the entity's table and key column, and the
/// columns the entity maps with their kinds (JSON
/// `[{"name":"site","kind":"text"},...]`).
struct FileSource {
    std::string name;
    std::string format;
    std::string path;
    std::string options;
    std::string table;
    std::string key;
    std::string fields;
    /// A readonly source never writes its file.
    bool readonly = false;
};

}  // namespace clarity
