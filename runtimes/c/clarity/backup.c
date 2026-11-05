// A backup of an SQLite database before migration steps change it; see backup.h.

#include "clarity/backup.h"

#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "clarity/disk.h"

const char* clarity_backup_stamp(int64_t seconds) {
    int64_t days = seconds / 86400, rest = seconds % 86400;
    if (rest < 0) {
        rest += 86400;
        days -= 1;
    }
    // The civil date of a day count (Howard Hinnant's days_from_civil, turned around).
    int64_t z = days + 719468;
    int64_t era = (z >= 0 ? z : z - 146096) / 146097;
    int64_t doe = z - era * 146097;
    int64_t yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    int64_t doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    int64_t mp = (5 * doy + 2) / 153;
    int64_t day = doy - (153 * mp + 2) / 5 + 1;
    int64_t month = mp < 10 ? mp + 3 : mp - 9;
    int64_t year = yoe + era * 400 + (month <= 2);
    return clarity_format("%04lld%02lld%02lldT%02lld%02lld%02lldZ", (long long)year, (long long)month, (long long)day, (long long)(rest / 3600),
                          (long long)(rest % 3600 / 60), (long long)(rest % 60));
}

const char* clarity_backup_file_name(const char* database, int64_t seconds, int step) {
    return clarity_format("%s-%s-before-migration-%04d.db", database, clarity_backup_stamp(seconds), step);
}

/// The name of a database file without its folder and extension: `shop` of `data/shop.db`.
static const char* stem_of(const char* path) {
    const char* slash = strrchr(path, '/');
    const char* back = strrchr(path, '\\');
    const char* base = slash && (!back || slash > back) ? slash + 1 : back ? back + 1 : path;
    const char* dot = strrchr(base, '.');
    return dot ? clarity_copy_n(base, (size_t)(dot - base)) : clarity_copy(base);
}

typedef struct {
    const char* when;
    int count;
    const char* file;
} Found;

/// A backup file of the database named `database`: its stamp, count within its second, and whether it is automatic.
static bool parse(const char* database, const char* file, Found* out, bool* automatic) {
    size_t n = strlen(database), length = strlen(file);
    if (length < n + 1 + 16 + 1 + 1 + 3 || strncmp(file, database, n) != 0 || file[n] != '-' || strcmp(file + length - 3, ".db") != 0) return false;
    const char* when = file + n + 1;
    if (when[15] != 'Z' || when[16] != '-') return false;
    const char* why = when + 17;
    char* reason = clarity_copy_n(why, (size_t)(file + length - 3 - why));
    int count = 1;
    char* under = strchr(reason, '_');
    if (under) {
        char* end = NULL;
        long c = strtol(under + 1, &end, 10);
        if (!end || *end || c < 1) return false;
        count = (int)c;
        *under = 0;
    }
    *out = (Found){clarity_copy_n(when, 16), count, file};
    *automatic = strcmp(reason, "manual") != 0;
    return true;
}

static int newest_first(const void* a, const void* b) {
    const Found *x = a, *y = b;
    int by_when = strcmp(y->when, x->when);
    return by_when ? by_when : y->count - x->count;
}

void clarity_prune_backups(const char* database) {
    const char* parent = clarity_parent(database);
    const char* folder = clarity_format("%s%sbackups", parent, *parent ? "/" : "");
    const char* name = stem_of(database);
    DIR* dir = opendir(folder);
    if (!dir) return;
    Found* all = NULL;
    size_t count = 0, capacity = 0;
    struct dirent* entry;
    while ((entry = readdir(dir))) {
        Found f;
        bool automatic = false;
        if (parse(name, entry->d_name, &f, &automatic) && automatic) {
            f.file = clarity_copy(entry->d_name);
            clarity_grow((void**)&all, &capacity, count, sizeof *all);
            all[count++] = f;
        }
    }
    closedir(dir);
    if (count > 1) qsort(all, count, sizeof *all, newest_first);
    for (size_t i = CLARITY_KEPT_BACKUPS; i < count; ++i) remove(clarity_format("%s/%s", folder, all[i].file));
}

/// The error of a backup that failed, as the refusal to migrate; the cause's message is copied before the error is written over.
static bool refused(void) {
    const char* why = clarity_copy(clarity_error()->message);
    return clarity_fail(CLARITY_STORAGE, "the database was not migrated, since it could not be backed up first: %s", why);
}

bool clarity_backup_before_migration(ClarityDriver* driver, int step) {
    if (driver->dialect != CLARITY_SQLITE) return true;
    ClarityRows rows = {0};
    if (!driver->rows(driver, "SELECT file FROM pragma_database_list WHERE name = 'main'", NULL, &rows)) return false;
    if (!rows.count || rows.items[0].values[0].kind != CLARITY_TEXT || !*rows.items[0].values[0].text) return true;
    const char* file = rows.items[0].values[0].text;
    const char* parent = clarity_parent(file);
    const char* folder = clarity_format("%s%sbackups", parent, *parent ? "/" : "");
    if (!clarity_make_folders(folder))
        return refused();
    const char* first = clarity_format("%s/%s", folder, clarity_backup_file_name(stem_of(file), (int64_t)time(NULL), step));
    const char* path = first;
    for (int n = 2; clarity_exists(path); ++n) path = clarity_format("%.*s_%d.db", (int)(strlen(first) - 3), first, n);
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_text_value(path));
    if (!driver->execute(driver, "VACUUM INTO ?1", &params, NULL))
        return refused();
    clarity_prune_backups(file);
    return true;
}
