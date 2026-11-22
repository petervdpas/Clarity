// A backup of an SQLite database before migration steps change it
// (docs/ledger/backups.md): a copy beside it in backups/, made with VACUUM
// INTO, named <database>-<when>-before-migration-NNNN.db with the time in
// UTC, as Clarity and every runtime name it (conformance/backups.json). The
// newest ten automatic backups are kept; a manual one is the person's and
// stays.

#pragma once

#include <cstdint>
#include <string>

#include "clarity/store.hpp"

namespace clarity::backup {

/// How many automatic backups each database keeps.
inline constexpr int kept_automatic = 10;

/// A moment, in seconds since 1970, as a backup names it: `20261105T205905Z`.
std::string stamp(int64_t seconds);
/// The file of a backup of the database named `database` (its file's name
/// without .db) taken at `seconds` before migration step `step`.
std::string file_name(const std::string& database, int64_t seconds, int step);
/// Removes the automatic backups of the database file `database` beyond the
/// newest ten; manual ones, other databases' and other files stay.
void prune(const std::string& database);

}  // namespace clarity::backup
