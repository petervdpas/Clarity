// A test's database set up as clarity-migrate leaves it (docs/ledger/migrations.md):
// the program does not migrate, it checks (Db::check_steps).

#pragma once

#include <string>
#include <vector>

#include "clarity/db.hpp"
#include "clarity/dialect.hpp"

namespace clarity {

/// The statements of one migration step.
using Step = std::vector<std::string>;

/// Runs the steps of `source` its database has not had yet and counts each in
/// clarity_schema, with references unchecked on SQLite while they run (a step
/// may build a table again that others refer to); gives how many ran.
inline int apply_steps(Db& db, const std::string& source, const std::vector<Step>& steps) {
    Dialect dialect = db.dialect(source);
    db.batch(source, version_table(dialect));
    auto rows = db.rows(source, "SELECT MAX(version) FROM clarity_schema", {});
    int done = rows.empty() || !std::holds_alternative<int64_t>(rows[0][0]) ? 0 : int(std::get<int64_t>(rows[0][0]));
    bool sqlite = dialect == Dialect::Sqlite;
    if (sqlite) db.batch(source, "PRAGMA foreign_keys = OFF");
    try {
        for (int i = done; i < int(steps.size()); ++i) {
            for (const auto& statement : steps[size_t(i)]) db.batch(source, statement);
            db.batch(source, "INSERT INTO clarity_schema (version) VALUES (" + std::to_string(i + 1) + ")");
        }
    } catch (...) {
        if (sqlite) db.batch(source, "PRAGMA foreign_keys = ON");
        throw;
    }
    if (sqlite) db.batch(source, "PRAGMA foreign_keys = ON");
    return std::max(0, int(steps.size()) - done);
}

inline int apply_steps(Db& db, const std::vector<Step>& steps) { return apply_steps(db, "", steps); }

}  // namespace clarity
