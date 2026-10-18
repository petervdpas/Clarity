// The inside of ClarityDb, shared by the runtime's own files (db.c and
// files.c); generated code uses db.h only.

#pragma once

#include "clarity/db.h"

typedef struct {
    char* name;
    ClarityDriver* driver;
    /// How many saves and deletes with data events are running in it; the
    /// outermost one holds the transaction.
    int depth;
} ClaritySourceDb;

typedef struct {
    char* name;
    char* folder;
} ClarityStore;

struct ClarityDb {
    ClaritySourceDb* sources;
    size_t count;
    ClarityStore* stores;
    size_t store_count;
};

/// A copy that outlives the scratch arena, to free.
char* clarity_lasting(const char* text);

/// The database of `source`; "" or NULL is the first.
bool clarity_source(ClarityDb* db, const char* source, ClaritySourceDb** out);
/// The folder of the file store `name`.
bool clarity_store_folder(ClarityDb* db, const char* name, const char** out);
