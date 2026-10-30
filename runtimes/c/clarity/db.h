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

/// How the runtime reads and writes the records of one entity.
typedef struct {
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
} ClarityEntity;

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
bool clarity_db_dialect(ClarityDb* db, const char* source, ClarityDialect* out);
/// The number of migration steps applied so far to the default database.
bool clarity_schema_version(ClarityDb* db, int* out);
/// Applies the steps of `source` its database has not had yet, each on its
/// own; `applied` (may be NULL) gets how many ran.
bool clarity_migrate(ClarityDb* db, const char* source, const ClarityStep* steps, size_t count, int* applied);

/// Inserts a new record (and gives it its id) or updates a stored one,
/// with the entity's data events around it.
bool clarity_save(ClarityDb* db, const ClarityEntity* entity, void* record);
/// Removes a stored record, with the entity's data events around it.
bool clarity_delete(ClarityDb* db, const ClarityEntity* entity, const void* record);
/// The records a query finds, in its order, as an array in the scratch arena.
bool clarity_find(ClarityDb* db, const ClarityEntity* entity, const ClarityQuery* query, void** items, size_t* count);
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
/// A file as the picture of a field kept in `field_store` (NULL: in the
/// database): the file itself when it is in that store, else a copy.
bool clarity_picture_from(ClarityDb* db, const ClarityFile* file, const char* field_store, ClarityPicture** out);
/// A file of a store as the file of a file field, under its name: linked
/// when it is in the field's own store, else copied.
bool clarity_record_file_from(ClarityDb* db, const ClarityFile* file, const char* field_store, ClarityRecordFile** out);
/// The bytes of the file `path` of a store: a picture linked from a file
/// before its record is saved.
bool clarity_store_file(ClarityDb* db, const char* store, const char* path, ClarityBytes* out);
/// Another record's picture for a field kept in `to` (NULL: the database),
/// as code copies it (album.cover = other.cover); `*out` is NULL for none.
/// `value` is field `column` of record `id`, kept in `from`. A picture in
/// hand is copied as it is, a stored one is read first, and a file of a
/// store stays linked only in that same store.
bool clarity_copied_picture(ClarityDb* db, const ClarityEntity* entity, const ClarityPicture* value, int64_t id, const char* column, const char* from, const char* to, ClarityPicture** out);
/// Another record's file, with its name, as clarity_copied_picture copies a picture.
bool clarity_copied_file(ClarityDb* db, const ClarityEntity* entity, const ClarityRecordFile* value, int64_t id, const char* column, const char* from, const char* to, ClarityRecordFile** out);
