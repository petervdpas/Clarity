// A test's database set up as clarity-migrate leaves it (docs/ledger/migrations.md):
// the program does not migrate, it checks (clarity_check_steps).

#pragma once

#include <stdbool.h>
#include <stddef.h>

#include "clarity/db.h"

/// The statements of one migration step.
typedef struct {
    const char* const* statements;
    size_t count;
} ClarityStep;

/// Runs the steps of `source` its database has not had yet and counts each in
/// clarity_schema, with references unchecked on SQLite while they run (a step
/// may build a table again that others refer to); `applied` (may be NULL) gets
/// how many ran.
bool apply_steps(ClarityDb* db, const char* source, const ClarityStep* steps, size_t count, int* applied);
