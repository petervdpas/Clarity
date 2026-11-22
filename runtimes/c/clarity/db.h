// The databases of an application: one per data source. Records are saved,
// deleted and read through here, with the entity's data events around a
// save or delete, and migration steps are applied here at startup.
//
// A record is a plain struct with its `id`; its entity describes it to the
// runtime (ClarityEntity): the table, the columns, how to write the record
// as stored values and read it back, and its data events. The records
// Clarity generates have one each, and typed calls around these, such as
// customer_save and customer_find.

#pragma once

#include "clarity/picture.h"
#include "clarity/record_file.h"
#include "clarity/query.h"
#include "clarity/store.h"

typedef struct ClarityDb ClarityDb;

/// A stored field a record needs a value in: its column (a file's content
/// column) and its label as written, a key as @key or the text itself.
typedef struct {
    const char* column;
    const char* label;
} ClarityRequired;

/// How the runtime reads and writes the records of one entity.
typedef struct ClarityEntity {
    /// The entity's name, as Clarity writes it: `Customer`.
    const char* name;
    const char* table;
    /// The key column; `id` unless the entity maps another.
    const char* key;
    /// The data source; "" for the first.
    const char* source;
    const char* const* columns;
    size_t column_count;
    const ClarityPictureColumn* pictures;
    size_t picture_count;
    /// sizeof the record's struct, and where its `int64_t id` is in it.
    size_t size;
    size_t id_at;
    /// Writes the record's fields, in the order of `columns`.
    void (*values)(const void* record, ClarityValue* out);
    /// Reads a record from its id and the values of `columns`.
    bool (*from_values)(int64_t id, const ClarityValue* values, void* record);
    /// The data events, NULL when the entity has none.
    bool (*before_save)(ClarityDb* db, void* record);
    bool (*after_save)(ClarityDb* db, void* record);
    bool (*before_delete)(ClarityDb* db, void* record);
    bool (*after_delete)(ClarityDb* db, void* record);
    /// Deletes the records that refer to record `id` with `cascade`, each as
    /// any record is deleted; NULL when nothing cascades from the entity.
    bool (*cascade)(ClarityDb* db, int64_t id);
    /// The required fields a save checks, in the entity's order; NULL when it has none.
    const ClarityRequired* required;
    size_t required_count;
    /// The column of the record's version (conformance/versions.json):
    /// `row_version` for an owned entity, the column an external entity
    /// names, or NULL for an entity whose saves are not checked; and where
    /// its `int64_t` is in the record (0 for a new record).
    const char* version;
    size_t version_at;
    /// Refuses a save or delete of a record someone saved since in the
    /// entity's own words (clarity_refuse, then false), about `current`, the
    /// record as it is stored now, with `db` to load what its fields refer
    /// to (`{current.changedBy.name}`); NULL for Clarity's savedSince.
    bool (*saved_since)(ClarityDb* db, const void* current);
} ClarityEntity;

/// Whether a record with these columns and values has a value in every
/// required field (no value and empty text are none; 0 and false are values).
/// False, refused with fieldRequired, at the first empty one in the entity's
/// order, its slot `field` the field's label in the person's language
/// (conformance/required.json).
bool clarity_check_required(const ClarityRequired* required, size_t required_count, const char* const* columns, const ClarityValue* values,
                            size_t column_count);

/// The database of one data source: its name, its kind and where it is (a
/// file for SQLite). `env:NAME` takes the location from that environment variable.
typedef struct {
    const char* name;
    ClarityDialect dialect;
    const char* at;
} ClaritySource;

/// A file of a file store.
typedef struct {
    const char* store;
    /// The path inside the store, with `/` between folders.
    const char* path;
    const char* name;
    /// In lower case, without the dot; "" without one.
    const char* extension;
    /// In bytes.
    int64_t size;
    /// When the file last changed, in UTC to the second.
    ClarityDateTime modified;
} ClarityFile;

typedef struct {
    ClarityFile* items;
    size_t count;
} ClarityFileList;

/// A database per data source.
bool clarity_db_open(const ClaritySource* sources, size_t count, ClarityDb** out);
/// One SQLite database, used for every record.
bool clarity_db_open_sqlite(const char* path, ClarityDb** out);
/// A SQLite database that lives in memory, for tests.
bool clarity_db_memory(ClarityDb** out);
void clarity_db_close(ClarityDb* db);

/// Runs SQL without parameters in the database of `source`, for tools and tests.
bool clarity_db_batch(ClarityDb* db, const char* source, const char* sql);
/// The rows SQL reads in the database of `source`, for tools and tests.
bool clarity_db_rows(ClarityDb* db, const char* source, const char* sql, const ClarityValues* params, ClarityRows* out);
bool clarity_db_dialect(ClarityDb* db, const char* source, ClarityDialect* out);
/// The number of migration steps applied so far to the default database.
bool clarity_schema_version(ClarityDb* db, int* out);
/// Whether the database of `source` has the `steps` migration steps the program was made
/// with: behind or ahead it fails with CLARITY_STEPS, whose message is the English line the
/// program stops with (docs/ledger/migrations.md, conformance/migrate-check.json). The program
/// does not migrate; clarity-migrate does.
bool clarity_check_steps(ClarityDb* db, const char* source, int steps);

/// How many undo steps are kept: the oldest goes when a newer one comes, so
/// a long session does not keep every row it ever changed.
#define CLARITY_UNDO_STEPS 100

/// Why a save or a delete is refused: someone saved the record since it was
/// read (conformance/versions.json), in Clarity's own words with their English.
ClarityText* clarity_saved_since(void);
/// Why an undo is refused: what it would take back was changed since.
ClarityText* clarity_undo_changed_since(void);
/// Why an undo is refused when its step is older than the ones kept.
ClarityText* clarity_undo_too_old(void);

/// Inserts a new record (and gives it its id) or updates a stored one,
/// with the entity's data events around it. A record someone saved since
/// it was read is refused (clarity_saved_since), changing nothing.
bool clarity_save(ClarityDb* db, const ClarityEntity* entity, void* record);
/// Saves a record regardless of its version (`save ... anyway`): what
/// someone saved since is written over, on purpose.
bool clarity_save_anyway(ClarityDb* db, const ClarityEntity* entity, void* record);
/// Reads a record again as it is stored now, with its version.
bool clarity_reload(ClarityDb* db, const ClarityEntity* entity, void* record);
/// Removes a stored record, with the entity's data events around it; one
/// someone saved since it was read is refused.
bool clarity_delete(ClarityDb* db, const ClarityEntity* entity, const void* record);
/// Runs a handler, `run(context)`, as one step, which undo takes back whole
/// (manual/code.handlers.md): its id, when it changed anything, is clarity_last_step.
/// Everything it changes is one transaction in each source it changes:
/// kept when it ends, left whole when it fails, so it leaves nothing
/// behind. A step inside a step is part of it. Without a database it just
/// runs. The last CLARITY_UNDO_STEPS are kept.
bool clarity_step(ClarityDb* db, bool (*run)(void* context), void* context);
/// The id of the last step a handler made, to undo it later by; 0 before any.
int64_t clarity_last_step(const ClarityDb* db);
/// Takes back step `id`: every row it changed as it was, with its own id.
/// Refused, changing nothing, when one of its rows changed since. `done`
/// (may be NULL) is false when it is undone already.
bool clarity_undo(ClarityDb* db, int64_t id, bool* done);
/// Does step `id` again after an undo; `done` is false when it is not undone.
bool clarity_redo(ClarityDb* db, int64_t id, bool* done);
/// Takes back (`back`) or does again the steps of one edit, in the order
/// given, as one: all of them or, refused, none. A step older than the
/// ones kept is refused (clarity_undo_too_old); `done` is false when none
/// of them is in the state to be replayed.
bool clarity_replay_steps(ClarityDb* db, const int64_t* ids, size_t count, bool back, bool* done);
/// The records a query finds, in its order, as an array in the scratch arena.
bool clarity_find(ClarityDb* db, const ClarityEntity* entity, const ClarityQuery* query, void** items, size_t* count);

/// How many owners one read of a stored list asks for (clarity_find_by_owners),
/// well below the fewest parameters a database takes in one statement (SQL Server's 2100).
#define CLARITY_OWNERS_PER_READ 500

/// The records of `entity` whose reference `column` points at one of the
/// `owner_count` ids of `owners`, in the order they were stored, without
/// their own lists: a stored list of many records at once (order.lines),
/// read CLARITY_OWNERS_PER_READ owners at a time. The items are in the scratch arena.
bool clarity_find_by_owners(ClarityDb* db, const ClarityEntity* entity, const char* column, size_t owner_count, const int64_t* owners, void** items,
                            size_t* count);

/// The items among `count` items of `size` bytes whose reference at
/// `reference_at` is `owner`, in their order, copied into the scratch arena
/// (`*out`): one record's share of a list read for many (clarity_find_by_owners).
size_t clarity_owned(const void* items, size_t count, size_t size, size_t reference_at, int64_t owner, void** out);
bool clarity_get(ClarityDb* db, const ClarityEntity* entity, int64_t id, void* record);
/// The record a reference field points at; 0 is a reference that is empty.
bool clarity_load(ClarityDb* db, const ClarityEntity* entity, int64_t reference, void* record);

/// Adds the file store `name` at a folder (made when it is not there
/// yet); `env:NAME` takes it from that environment variable.
bool clarity_add_store(ClarityDb* db, const char* name, const char* at);
/// Adds the file store `name` whose folder is the folder setting `setting`
/// of `program`, or `fallback` while it is not kept; read again at every
/// use, so a folder chosen while the application runs takes effect at once.
bool clarity_add_store_setting(ClarityDb* db, const char* name, const char* program, const char* setting, const char* fallback);
/// The settings the running handler has, as its window's data had them when
/// the call began (copied): a file store whose folder is a setting reads them
/// before what is kept.
void clarity_hold_settings(ClarityDb* db, const cJSON* settings);
/// A setting the running handler's code set, before the settings are kept.
void clarity_hold_setting(ClarityDb* db, const char* setting, const char* value);
/// The picture of field `column` of record `id`, read from the database
/// or from its file store; `out->data` is NULL when the record has none.
bool clarity_picture(ClarityDb* db, const ClarityEntity* entity, int64_t id, const char* column, ClarityBytes* out);

/// Every file of a store and its folders, but those whose name or folder
/// starts with a dot, sorted by path.
bool clarity_files(ClarityDb* db, const char* store, ClarityFileList* out);
/// The text a file holds.
bool clarity_read_text(ClarityDb* db, const ClarityFile* file, const char** out);
/// Writes `text` as the file `path` of a store, making its folders.
bool clarity_write_text(ClarityDb* db, const char* store, const char* path, const char* text);
/// The contents of a file as they are.
bool clarity_read_bytes(ClarityDb* db, const ClarityFile* file, ClarityBytes* out);
/// Writes `bytes` as they are as the file `path` of a store, making its folders.
bool clarity_write_bytes(ClarityDb* db, const char* store, const char* path, ClarityBytes bytes);

/// The store of a folder that is no store: "folder:" and its path.
#define CLARITY_FOLDER_STORE "folder:"
/// Whether a file's name (`Notes.MD`, `.env`) is one a folder takes: by its
/// extensions (`patterns`, written apart by spaces: `"*.md *.txt"`, ignoring
/// case; empty or `"*.*"`: any file), a hidden one (its name starting with a
/// dot) only with `hidden` (conformance/files.json).
bool clarity_folder_takes(const char* patterns, bool hidden, const char* name);
/// Whether a blob or file field that says its extensions (`patterns`, by
/// spaces) takes the file `name` (a name or a path): its last part matched
/// as a folder matches, a hidden one only with `hidden`; no file (an empty
/// name) is always taken. One of another type is refused in Clarity's own
/// words, fileTypeRefused (conformance/files.json, fieldFiles).
bool clarity_check_file_type(const char* field, const char* patterns, bool hidden, const char* name);
/// The files of a folder ("folder:" and its path) its extensions take, in
/// its folders too, sorted by path; hidden ones only with `hidden`.
bool clarity_files_in(ClarityDb* db, const char* folder, const char* patterns, bool hidden, ClarityFileList* out);
/// Writes `text` as the file `path` of a folder, when it takes the file.
bool clarity_write_text_in(ClarityDb* db, const char* folder, const char* patterns, bool hidden, const char* path, const char* text);
/// Writes `bytes` as the file `path` of a folder, when it takes the file.
bool clarity_write_bytes_in(ClarityDb* db, const char* folder, const char* patterns, bool hidden, const char* path, ClarityBytes bytes);
/// A file as the picture of a field kept in `field_store` (NULL: in the
/// database): the file itself when it is in that store, else a copy.
bool clarity_picture_from(ClarityDb* db, const ClarityFile* file, const char* field_store, ClarityPicture** out);
/// A file of a store as the file of a file field, under its name: linked
/// when it is in the field's own store, else copied.
bool clarity_record_file_from(ClarityDb* db, const ClarityFile* file, const char* field_store, ClarityRecordFile** out);
/// The bytes of the file `path` of a store: a picture linked from a file
/// before its record is saved.
bool clarity_store_file(ClarityDb* db, const char* store, const char* path, ClarityBytes* out);
/// Keeps a file a person picked in a web app in the file store `store` on
/// the server: in a folder named by its contents, as a blob's is, so nothing
/// is written over; `out` is its path on the server (step 5 of
/// docs/ledger/files-pictures.md).
bool clarity_store_upload(ClarityDb* db, const char* store, const char* name, ClarityBytes bytes, const char** out);
/// The file stores and their folders on this machine, by name, as
/// `[{"name", "folder"}]`; one whose folder cannot be found is left out.
cJSON* clarity_stores(ClarityDb* db);
/// Another record's picture for a field kept in `to` (NULL: the database),
/// as code copies it (album.cover = other.cover); `*out` is NULL for none.
/// `value` is field `column` of record `id`, kept in `from`. A picture in
/// hand is copied as it is, a stored one is read first, and a file of a
/// store stays linked only in that same store.
bool clarity_copied_picture(ClarityDb* db, const ClarityEntity* entity, const ClarityPicture* value, int64_t id, const char* column, const char* from, const char* to, ClarityPicture** out);
/// Another record's file, with its name, as clarity_copied_picture copies a picture.
bool clarity_copied_file(ClarityDb* db, const ClarityEntity* entity, const ClarityRecordFile* value, int64_t id, const char* column, const char* from, const char* to, ClarityRecordFile** out);
