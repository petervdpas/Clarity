#include "formats/file_source.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "clarity/db_internal.h"
#include "clarity/disk.h"
#include "clarity/json.h"
#include "formats/formats.h"

/// One table read from the file and kept in SQLite.
typedef struct {
    /// Its name in the file's tables: "" for the source's own, `lines` or `lines/notes` for records inside.
    char* name;
    char* table;
    char* key;
    /// The file's columns, in its order; for records inside, the second links a row to its row.
    char** columns;
    size_t column_count;
    /// Whether the second column is a link: the place of the row in the table it sits in.
    bool linked;
} FileTable;

typedef struct {
    ClarityDriver driver;
    /// The in-memory SQLite database that holds the tables.
    ClarityDriver* db;
    char* path;
    char* format;
    char* options;
    char* fields;
    /// The tables the file is read into: one, or with records inside (a mapping's `nested`) one per entity.
    FileTable* tables;
    size_t table_count;
    /// Whether the file holds records inside its records, so its tables go back as several.
    bool several;
    /// The file as it was last read or written, for formats that keep more than the table.
    uint8_t* original;
    size_t original_size;
    bool readonly;
    bool dirty;
    /// The file's size and modification time when it was last read or
    /// written: a change by another program refuses the next save.
    long long seen_size;
    long long seen_time;
    bool seen;
} FileDriver;

/* The file's size and modification time; false when it is not there. */
static bool stamp(const char* path, long long* size, long long* time) {
    struct stat info;
    if (stat(path, &info) != 0) return false;
    *size = (long long)info.st_size;
    *time = (long long)info.st_mtime;
    return true;
}

static void note_stamp(FileDriver* f) { f->seen = stamp(f->path, &f->seen_size, &f->seen_time); }

/* Refuses a change when another program changed the file since it was read
   or written: its size or modification time differ, and so do its bytes. */
static bool unchanged_on_disk(FileDriver* f) {
    if (f->readonly) return true;
    long long size = 0, time = 0;
    bool there = stamp(f->path, &size, &time);
    if (there == f->seen && (!there || (size == f->seen_size && time == f->seen_time))) return true;
    ClarityBytes bytes = {0};
    if (there && !clarity_read_file(f->path, &bytes)) return clarity_refuse(clarity_saved_since());
    if (bytes.size != f->original_size || (bytes.size && memcmp(bytes.data, f->original, bytes.size) != 0)) return clarity_refuse(clarity_saved_since());
    // The same bytes written again: no change.
    note_stamp(f);
    return true;
}

/* The keys of a table's rows, in their order (ascending: the key is the rowid). */
typedef struct {
    int64_t* keys;
    size_t count;
} Keys;

/* The table a nested table's rows sit in: `lines/notes` in `lines`, `lines` in the source's own (""). */
static size_t parent_of(const FileTable* tables, size_t at) {
    const char* name = tables[at].name;
    const char* slash = strrchr(name, '/');
    size_t length = slash ? (size_t)(slash - name) : 0;
    for (size_t i = 0; i < at; ++i)
        if (strlen(tables[i].name) == length && strncmp(tables[i].name, name, length) == 0) return i;
    return at;
}

/* The records field the table name `name` (`lines/notes`) leads to among `fields`. */
static const cJSON* records_field(const cJSON* fields, const char* name) {
    const cJSON* found = NULL;
    const char* step = name;
    while (*step) {
        const char* end = strchr(step, '/');
        size_t length = end ? (size_t)(end - step) : strlen(step);
        found = NULL;
        const cJSON* field;
        cJSON_ArrayForEach(field, fields) {
            const cJSON* field_name = cJSON_GetObjectItem(field, "name");
            if (cJSON_IsString(field_name) && strlen(field_name->valuestring) == length && strncmp(field_name->valuestring, step, length) == 0) {
                found = field;
                break;
            }
        }
        if (!found) return NULL;
        fields = cJSON_GetObjectItem(found, "fields");
        step = end ? end + 1 : step + length;
    }
    return found;
}

/* The place, from 1, of the row keyed `key` among `keys`; 0 for none. */
static int64_t place_of(const Keys* keys, int64_t key) {
    size_t low = 0, high = keys->count;
    while (low < high) {
        size_t middle = (low + high) / 2;
        if (keys->keys[middle] < key) low = middle + 1;
        else high = middle;
    }
    return low < keys->count && keys->keys[low] == key ? (int64_t)low + 1 : 0;
}

static FileDriver* file_of(ClarityDriver* driver) { return (FileDriver*)driver; }

static const char* quote(const char* name) { return clarity_quote(CLARITY_SQLITE, name); }

/* The last error's message without its kind before it. */
static const char* problem(void) {
    const char* message = clarity_error()->message;
    return strncmp(message, "storage: ", 9) == 0 ? message + 9 : message;
}

/* The last error again, as about data source `name`. Always false. */
static bool about(const char* name, const char* before) {
    const char* why = clarity_copy(problem());
    return clarity_fail(CLARITY_STORAGE, "data source %s: %s%s", name, before, why);
}

/* A name without `_`, `-` and spaces, in lower case, as Clarity.Formats matches columns. */
static const char* plain(const char* name) {
    ClarityBuffer out = {0};
    for (const char* c = name; *c; ++c)
        if (*c != '_' && *c != '-' && *c != ' ') clarity_buffer_add_char(&out, (char)tolower((unsigned char)*c));
    return clarity_buffer_text(&out);
}

/* The SQLite type a mapped column of `kind` gets, so its values are kept as the generated code reads them. */
static const char* column_type(const char* kind) {
    if (strcmp(kind, "integer") == 0 || strcmp(kind, "bool") == 0) return " INTEGER";
    const char* texts[] = {"text", "decimal", "date", "datetime", "enum", "values"};
    for (size_t i = 0; i < sizeof texts / sizeof *texts; ++i)
        if (strcmp(kind, texts[i]) == 0) return " TEXT";
    return "";
}

/* The type of file column `column`, by the kind of the field it maps. */
static const char* type_of(const cJSON* fields, const char* column) {
    const char* wanted = plain(column);
    const cJSON* field;
    cJSON_ArrayForEach(field, fields) {
        const cJSON* name = cJSON_GetObjectItem(field, "name");
        const cJSON* kind = cJSON_GetObjectItem(field, "kind");
        if (cJSON_IsString(name) && cJSON_IsString(kind) && strcmp(plain(name->valuestring), wanted) == 0) return column_type(kind->valuestring);
    }
    return "";
}

/* A cell read as SQLite keeps it: a whole number as an integer, other numbers as their text. */
static ClarityValue cell(const cJSON* value) {
    if (cJSON_IsNull(value)) return clarity_null();
    if (cJSON_IsBool(value)) return clarity_integer_value(cJSON_IsTrue(value) ? 1 : 0);
    if (cJSON_IsNumber(value)) {
        double n = value->valuedouble;
        if (n > -9.2e18 && n < 9.2e18 && (double)(int64_t)n == n) return clarity_integer_value((int64_t)n);
        return clarity_text_value(clarity_json_of(value));
    }
    if (cJSON_IsString(value)) return clarity_text_value(value->valuestring);
    return clarity_text_value(clarity_json_of(value));
}

/* A value of a row as a cell of the table written back. */
static void add_cell(ClarityBuffer* out, const ClarityValue* value) {
    switch (value->kind) {
    case CLARITY_INTEGER: clarity_buffer_addf(out, "%lld", (long long)value->integer); break;
    case CLARITY_REAL: {
        const char* short_form = clarity_format("%.15g", value->real);
        clarity_buffer_add(out, strtod(short_form, NULL) == value->real ? short_form : clarity_format("%.17g", value->real));
        break;
    }
    case CLARITY_TEXT: clarity_buffer_add(out, clarity_json_of(cJSON_CreateString(value->text))); break;
    default: clarity_buffer_add(out, "null");
    }
}

/* One table's columns and rows as JSON, a link turned back into the place of its row among `parent`'s; fills `own` with the rows' keys. */
static bool add_table(ClarityBuffer* out, FileDriver* f, const FileTable* t, const Keys* parent, Keys* own) {
    ClarityBuffer names = {0};
    for (size_t i = 0; i < t->column_count; ++i) clarity_buffer_addf(&names, ", %s", quote(t->columns[i]));
    ClarityRows rows = {0};
    const char* select = clarity_format("SELECT %s%s FROM %s ORDER BY rowid", quote(t->key), clarity_buffer_text(&names), quote(t->table));
    if (!f->db->rows(f->db, select, NULL, &rows)) return false;
    own->keys = malloc((rows.count ? rows.count : 1) * sizeof *own->keys);
    if (!own->keys) return clarity_fail(CLARITY_STORAGE, "out of memory");
    own->count = 0;
    if (f->several) clarity_buffer_addf(out, "{\"name\":%s,", clarity_json_of(cJSON_CreateString(t->name)));
    else clarity_buffer_add_char(out, '{');
    clarity_buffer_add(out, "\"columns\":[");
    for (size_t i = 0; i < t->column_count; ++i) {
        if (i) clarity_buffer_add_char(out, ',');
        clarity_buffer_add(out, clarity_json_of(cJSON_CreateString(t->columns[i])));
    }
    clarity_buffer_add(out, "],\"rows\":[");
    for (size_t r = 0; r < rows.count; ++r) {
        const ClarityRow* row = &rows.items[r];
        own->keys[own->count++] = row->values[0].kind == CLARITY_INTEGER ? row->values[0].integer : 0;
        clarity_buffer_add(out, r ? ",[" : "[");
        for (size_t i = 1; i < row->count; ++i) {
            if (i > 1) clarity_buffer_add_char(out, ',');
            if (t->linked && i == 2) {
                int64_t place = parent && row->values[i].kind == CLARITY_INTEGER ? place_of(parent, row->values[i].integer) : 0;
                if (place) clarity_buffer_addf(out, "%lld", (long long)place);
                else clarity_buffer_add(out, "null");
            } else {
                add_cell(out, &row->values[i]);
            }
        }
        clarity_buffer_add_char(out, ']');
    }
    clarity_buffer_add(out, "]}");
    return true;
}

/* The tables as JSON, as Clarity.Formats writes them back: one as it is, several in `{"tables":[...]}`. */
static const char* tables_json(FileDriver* f) {
    Keys* keys = calloc(f->table_count ? f->table_count : 1, sizeof *keys);
    if (!keys) {
        clarity_fail(CLARITY_STORAGE, "out of memory");
        return NULL;
    }
    ClarityBuffer out = {0};
    if (f->several) clarity_buffer_add(&out, "{\"tables\":[");
    bool done = true;
    for (size_t i = 0; i < f->table_count && done; ++i) {
        if (i) clarity_buffer_add_char(&out, ',');
        size_t parent = parent_of(f->tables, i);
        done = add_table(&out, f, &f->tables[i], f->tables[i].linked && parent != i ? &keys[parent] : NULL, &keys[i]);
    }
    if (f->several) clarity_buffer_add(&out, "]}");
    for (size_t i = 0; i < f->table_count; ++i) free(keys[i].keys);
    free(keys);
    return done ? clarity_buffer_text(&out) : NULL;
}

/* Writes the tables back when a change is kept: to a new file beside the old one, which then replaces it. */
static bool keep(FileDriver* f) {
    if (!f->dirty || f->readonly || !clarity_sqlite_autocommit(f->db)) return true;
    f->dirty = false;
    const char* tables = tables_json(f);
    if (!tables) return false;
    ClarityBytes bytes = {0};
    if (!clarity_formats_write_table(f->format, (ClarityBytes){f->original, f->original_size}, tables, f->options, f->fields, &bytes))
        return clarity_fail(CLARITY_STORAGE, "%s", clarity_copy(problem()));
    char* folder = clarity_copy(f->path);
    char* slash = strrchr(folder, '/');
#ifdef _WIN32
    char* back = strrchr(folder, '\\');
    if (back > slash) slash = back;
#endif
    if (slash && slash != folder) {
        *slash = '\0';
        if (!clarity_make_folders(folder)) return false;
    }
    const char* written = clarity_format("%s.new", f->path);
    if (!clarity_write_file(written, bytes)) return false;
#ifdef _WIN32
    remove(f->path);
#endif
    if (rename(written, f->path) != 0) return clarity_fail(CLARITY_STORAGE, "cannot replace %s", f->path);
    uint8_t* kept = malloc(bytes.size ? bytes.size : 1);
    if (!kept) return clarity_fail(CLARITY_STORAGE, "out of memory");
    if (bytes.size) memcpy(kept, bytes.data, bytes.size);
    free(f->original);
    f->original = kept;
    f->original_size = bytes.size;
    note_stamp(f);
    return true;
}

static bool execute(ClarityDriver* driver, const char* sql, const ClarityValues* params, int64_t* changed) {
    FileDriver* f = file_of(driver);
    int64_t count = 0;
    if (!unchanged_on_disk(f)) return false;
    if (!f->db->execute(f->db, sql, params, &count)) return false;
    if (changed) *changed = count;
    f->dirty = f->dirty || count > 0;
    return keep(f);
}

static bool insert(ClarityDriver* driver, const char* sql, const ClarityValues* params, int64_t* key) {
    FileDriver* f = file_of(driver);
    if (!unchanged_on_disk(f)) return false;
    if (!f->db->insert(f->db, sql, params, key)) return false;
    f->dirty = true;
    return keep(f);
}

static bool rows(ClarityDriver* driver, const char* sql, const ClarityValues* params, ClarityRows* out) {
    FileDriver* f = file_of(driver);
    return f->db->rows(f->db, sql, params, out);
}

/* A transaction's end keeps what it changed; a file another program
   changed meanwhile rolls it back instead. */
static bool batch(ClarityDriver* driver, const char* sql) {
    FileDriver* f = file_of(driver);
    if (f->dirty && strcmp(sql, clarity_commit_sql(CLARITY_SQLITE)) == 0 && !unchanged_on_disk(f)) {
        ClarityError refused = *clarity_error();
        f->db->batch(f->db, clarity_rollback_sql(CLARITY_SQLITE));
        f->dirty = false;
        clarity_set_error(&refused);
        return false;
    }
    return f->db->batch(f->db, sql) && keep(f);
}

/* A file source is external: Clarity writes no steps for it. */
static bool apply(ClarityDriver* driver, int version, const ClarityStep* step) {
    (void)driver;
    (void)version;
    (void)step;
    return true;
}

static void close_driver(ClarityDriver* driver) {
    FileDriver* f = file_of(driver);
    if (f->db) f->db->close(f->db);
    for (size_t t = 0; t < f->table_count; ++t) {
        FileTable* table = &f->tables[t];
        for (size_t i = 0; i < table->column_count; ++i) free(table->columns[i]);
        free(table->columns);
        free(table->name);
        free(table->table);
        free(table->key);
    }
    free(f->tables);
    free(f->path);
    free(f->format);
    free(f->options);
    free(f->fields);
    free(f->original);
    free(f);
}

/* Creates the next of the file's tables and puts its rows in, a link to the place of a row as that row's key in `keys`. */
static bool fill_table(FileDriver* f, const ClarityFileSource* source, const cJSON* given, const cJSON* fields, Keys* keys) {
    size_t at = f->table_count;
    FileTable* t = &f->tables[at];
    f->table_count++;
    const cJSON* name = cJSON_GetObjectItem(given, "name");
    const cJSON* columns = cJSON_GetObjectItem(given, "columns");
    const cJSON* list = cJSON_GetObjectItem(given, "rows");
    if (!cJSON_IsArray(columns) || !cJSON_IsArray(list)) return clarity_fail(CLARITY_STORAGE, "data source %s: the table read is no table", source->name);
    t->name = clarity_lasting(cJSON_IsString(name) ? name->valuestring : "");
    if (!t->name) return clarity_fail(CLARITY_STORAGE, "out of memory");
    t->linked = t->name[0] != '\0';
    const cJSON* kinds = fields;
    if (t->linked) {
        const cJSON* field = records_field(fields, t->name);
        const cJSON* table = cJSON_GetObjectItem(field, "table");
        const cJSON* key = cJSON_GetObjectItem(field, "key");
        if (!cJSON_IsString(table) || !cJSON_IsString(key))
            return clarity_fail(CLARITY_STORAGE, "data source %s: the records inside '%s' are no field of the fields given", source->name, t->name);
        t->table = clarity_lasting(table->valuestring);
        t->key = clarity_lasting(key->valuestring);
        kinds = cJSON_GetObjectItem(field, "fields");
    } else {
        t->table = clarity_lasting(source->table);
        t->key = clarity_lasting(source->key);
    }
    size_t count = (size_t)cJSON_GetArraySize(columns);
    t->columns = calloc(count ? count : 1, sizeof *t->columns);
    if (!t->table || !t->key || !t->columns) return clarity_fail(CLARITY_STORAGE, "out of memory");
    bool keyed = false;
    const cJSON* column;
    cJSON_ArrayForEach(column, columns) {
        if (!cJSON_IsString(column)) return clarity_fail(CLARITY_STORAGE, "data source %s: the table read is no table", source->name);
        t->columns[t->column_count++] = clarity_lasting(column->valuestring);
        keyed = keyed || strcmp(column->valuestring, t->key) == 0;
    }
    ClarityBuffer create = {0};
    clarity_buffer_addf(&create, "CREATE TABLE %s (", quote(t->table));
    // The file has no key column: the row's place keys it, never written back.
    if (!keyed) clarity_buffer_addf(&create, "%s INTEGER PRIMARY KEY", quote(t->key));
    ClarityBuffer names = {0};
    ClarityBuffer marks = {0};
    for (size_t i = 0; i < t->column_count; ++i) {
        const char* column_name = t->columns[i];
        const char* type = strcmp(column_name, t->key) == 0 ? " INTEGER PRIMARY KEY" : t->linked && i == 1 ? " INTEGER" : type_of(kinds, column_name);
        clarity_buffer_addf(&create, "%s%s%s", i || !keyed ? ", " : "", quote(column_name), type);
        clarity_buffer_addf(&names, "%s%s", i ? ", " : "", quote(column_name));
        clarity_buffer_add(&marks, i ? ", ?" : "?");
    }
    clarity_buffer_add_char(&create, ')');
    if (!f->db->batch(f->db, clarity_buffer_text(&create))) return about(source->name, "");
    const char* insert_sql = clarity_format("INSERT INTO %s (%s) VALUES (%s)", quote(t->table), clarity_buffer_text(&names), clarity_buffer_text(&marks));
    size_t parent = parent_of(f->tables, at);
    const Keys* parent_keys = t->linked && parent != at ? &keys[parent] : NULL;
    size_t row_count = (size_t)cJSON_GetArraySize(list);
    keys[at].keys = malloc((row_count ? row_count : 1) * sizeof *keys[at].keys);
    if (!keys[at].keys) return clarity_fail(CLARITY_STORAGE, "out of memory");
    int number = 0;
    const cJSON* row;
    cJSON_ArrayForEach(row, list) {
        ++number;
        ClarityValues values = {0};
        const cJSON* value;
        cJSON_ArrayForEach(value, row) clarity_values_add(&values, cell(value));
        if (t->linked && values.count > 1) {
            // The place of the row it sits in, from 1, is that row's key.
            const ClarityValue place = values.items[1];
            bool found = parent_keys && place.kind == CLARITY_INTEGER && place.integer >= 1 && (size_t)place.integer <= parent_keys->count;
            values.items[1] = found ? clarity_integer_value(parent_keys->keys[place.integer - 1]) : clarity_null();
        }
        int64_t id = 0;
        if (!f->db->insert(f->db, insert_sql, &values, &id)) return about(source->name, clarity_format("row %d: ", number));
        keys[at].keys[keys[at].count++] = id;
    }
    return true;
}

/* Creates a table for each of the file's tables and puts their rows in. */
static bool fill(FileDriver* f, const ClarityFileSource* source, const cJSON* read) {
    const cJSON* several = cJSON_GetObjectItem(read, "tables");
    f->several = cJSON_IsArray(several);
    size_t count = f->several ? (size_t)cJSON_GetArraySize(several) : 1;
    f->tables = calloc(count ? count : 1, sizeof *f->tables);
    Keys* keys = calloc(count ? count : 1, sizeof *keys);
    if (!f->tables || !keys) {
        free(keys);
        return clarity_fail(CLARITY_STORAGE, "out of memory");
    }
    cJSON* fields = cJSON_Parse(source->fields ? source->fields : "");
    bool done = true;
    if (!f->several) {
        done = fill_table(f, source, read, fields, keys);
    } else {
        const cJSON* given;
        cJSON_ArrayForEach(given, several) {
            if (!(done = fill_table(f, source, given, fields, keys))) break;
        }
    }
    for (size_t i = 0; i < count; ++i) free(keys[i].keys);
    free(keys);
    cJSON_Delete(fields);
    return done;
}

/* Reads the file at `path` (already found as an SQLite file's location is) into its table. */
static bool open_file(const ClarityFileSource* source, const char* path, ClarityDriver** out) {
    ClarityBytes original = {0};
    /* A file not there yet is a new one, made at the first save; a readonly source needs it. */
    FILE* there = fopen(path, "rb");
    if (there) fclose(there);
    if ((there || source->readonly) && !clarity_read_file(path, &original)) return about(source->name, "");
    const char* text = NULL;
    if (!clarity_formats_read_table(source->format, original, source->options, source->fields ? source->fields : "", &text)) return about(source->name, "");
    cJSON* table = cJSON_Parse(text);
    if (!table) return clarity_fail(CLARITY_STORAGE, "data source %s: the table read is no table", source->name);
    FileDriver* f = calloc(1, sizeof *f);
    if (!f) return clarity_fail(CLARITY_STORAGE, "out of memory");
    f->driver = (ClarityDriver){CLARITY_SQLITE, execute, insert, rows, batch, apply, close_driver};
    f->path = clarity_lasting(path);
    f->format = clarity_lasting(source->format);
    f->options = clarity_lasting(source->options ? source->options : "");
    f->fields = clarity_lasting(source->fields ? source->fields : "");
    f->readonly = source->readonly;
    f->original = malloc(original.size ? original.size : 1);
    if (f->original && original.size) memcpy(f->original, original.data, original.size);
    f->original_size = original.size;
    if (f->path) note_stamp(f);
    if (!f->path || !f->format || !f->options || !f->fields || !f->original) {
        close_driver(&f->driver);
        return clarity_fail(CLARITY_STORAGE, "out of memory");
    }
    if (!clarity_open_sqlite(":memory:", &f->db) || !fill(f, source, table)) {
        ClarityError error = *clarity_error();
        close_driver(&f->driver);
        clarity_set_error(&error);
        return false;
    }
    *out = &f->driver;
    return true;
}

bool clarity_db_add_file(ClarityDb* db, const ClarityFileSource* source) {
    const char* path = NULL;
    if (!clarity_location(clarity_data_folder_sqlite(source->path, NULL), &path)) return false;
    ClarityDriver* driver = NULL;
    if (!open_file(source, path, &driver)) return false;
    // A source of the same name is replaced.
    for (size_t i = 0; i < db->count; ++i) {
        if (strcmp(db->sources[i].name, source->name) != 0) continue;
        db->sources[i].driver->close(db->sources[i].driver);
        db->sources[i].driver = driver;
        db->sources[i].depth = 0;
        return true;
    }
    ClaritySourceDb* grown = realloc(db->sources, (db->count + 1) * sizeof *grown);
    char* name = clarity_lasting(source->name);
    if (!grown || !name) {
        if (grown) db->sources = grown;
        free(name);
        driver->close(driver);
        return clarity_fail(CLARITY_STORAGE, "out of memory");
    }
    db->sources = grown;
    db->sources[db->count++] = (ClaritySourceDb){name, driver, 0};
    return true;
}
