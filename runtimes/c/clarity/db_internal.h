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
    /// The folder, found once; NULL when a folder setting names it.
    char* folder;
    /// The folder setting of `program` that names the folder, read again
    /// at every use, and its default.
    char* program;
    char* setting;
    char* fallback;
} ClarityStore;

/// A row of a table as it was before a change and as the change left it
/// (DESIGN 5.29): its raw column values, kept outside the scratch arena.
typedef struct {
    const ClarityEntity* entity;
    int64_t id;
    /// Whether the row was there before the change, and is after it.
    bool had, has;
    ClarityValue* before;
    ClarityValue* after;
    /// The row's version in each state, for a versioned entity (0 where it
    /// was not there): undo then compares the version, not the whole row.
    int64_t before_version, after_version;
} ClarityChange;

typedef struct {
    ClarityChange* items;
    size_t count;
    size_t capacity;
} ClarityChanges;

/// A step undo takes back whole: what one handler run changed.
typedef struct {
    ClarityChanges changes;
    bool undone;
    /// Whether it is older than the steps kept, its changes freed.
    bool forgotten;
} ClarityDone;

struct ClarityDb {
    ClaritySourceDb* sources;
    size_t count;
    ClarityStore* stores;
    size_t store_count;
    /// Whether a step runs, and the changes it made so far.
    bool recording;
    ClarityChanges changes;
    /// Each step by its id, counted from 1; the changes of the last
    /// CLARITY_UNDO_STEPS kept.
    ClarityDone* steps;
    size_t step_count;
    /// The sources the running step holds a transaction in, by their place
    /// in `sources`: what one handler changes is kept or left whole.
    size_t* held;
    size_t held_count;
    /// Who signs in and who is signed in (auth/auth.c); NULL when the
    /// program names no people. `forget_people` frees it with the database.
    struct ClarityPeople* people;
    void (*forget_people)(struct ClarityPeople* people);
    /// The settings of the running handler: the window's data had them when
    /// the call began, and its code sets them before they are kept; a file
    /// store whose folder is a setting reads them first (conformance/settings.json, `now`).
    cJSON* held_settings;
};

/// A copy that outlives the scratch arena, to free.
char* clarity_lasting(const char* text);

/// The database of `source`; "" or NULL is the first.
bool clarity_source(ClarityDb* db, const char* source, ClaritySourceDb** out);
/// The folder of the file store `name`.
bool clarity_store_folder(ClarityDb* db, const char* name, const char** out);
