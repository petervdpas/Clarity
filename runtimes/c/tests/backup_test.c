// A backup before migration steps change a database (docs/BACKUP-PLAN.md),
// named and pruned as every stack does it (conformance/backups.json).

#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "check.h"
#include "clarity/backup.h"
#include "clarity/db.h"
#include "clarity/disk.h"
#include "tests.h"

static int by_name(const void* a, const void* b) { return strcmp(*(const char* const*)a, *(const char* const*)b); }

/// The files in `folder`, by name; their count in `count`.
static const char** files_in(const char* folder, size_t* count) {
    const char** all = NULL;
    size_t capacity = 0;
    *count = 0;
    DIR* dir = opendir(folder);
    if (!dir) return NULL;
    struct dirent* entry;
    while ((entry = readdir(dir))) {
        if (entry->d_name[0] == '.') continue;
        clarity_grow((void**)&all, &capacity, *count, sizeof *all);
        all[(*count)++] = clarity_copy(entry->d_name);
    }
    closedir(dir);
    if (*count > 1) qsort(all, *count, sizeof *all, by_name);
    return all;
}

void backups_are_named_and_pruned_as_every_stack_does(void) {
    cJSON* cases = conformance("backups.json");
    REQUIRE(cases);
    const cJSON* item;
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(cases, "stamps")) {
        CHECK_TEXT(clarity_backup_stamp((int64_t)cJSON_GetObjectItem(item, "seconds")->valuedouble), cJSON_GetObjectItem(item, "stamp")->valuestring);
    }
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(cases, "names")) {
        CHECK_TEXT(clarity_backup_file_name(cJSON_GetObjectItem(item, "database")->valuestring, (int64_t)cJSON_GetObjectItem(item, "seconds")->valuedouble,
                                            cJSON_GetObjectItem(item, "step")->valueint),
                   cJSON_GetObjectItem(item, "file")->valuestring);
    }
    CHECK_INT(cJSON_GetObjectItem(cases, "kept")->valueint, CLARITY_KEPT_BACKUPS);
    const cJSON* prune = cJSON_GetObjectItem(cases, "prune");
    const char* dir = clarity_format("%s/clarity-c-prune", temp_folder());
    remove_tree(dir);
    REQUIRE_OK(clarity_make_folders(clarity_format("%s/backups", dir)));
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(prune, "files")) {
        FILE* out = fopen(clarity_format("%s/backups/%s", dir, item->valuestring), "wb");
        if (out) fclose(out);
    }
    clarity_prune_backups(clarity_format("%s/%s.db", dir, cJSON_GetObjectItem(prune, "database")->valuestring));
    size_t count = 0;
    const char** left = files_in(clarity_format("%s/backups", dir), &count);
    const cJSON* wanted = cJSON_GetObjectItem(prune, "left");
    CHECK_INT((int)count, cJSON_GetArraySize(wanted));
    for (size_t i = 0; i < count && (int)i < cJSON_GetArraySize(wanted); ++i) CHECK_TEXT(left[i], cJSON_GetArrayItem(wanted, (int)i)->valuestring);
    remove_tree(dir);
    cJSON_Delete(cases);
}

void a_database_with_a_schema_is_backed_up_before_new_steps(void) {
    const char* dir = clarity_format("%s/clarity-c-backup", temp_folder());
    remove_tree(dir);
    REQUIRE_OK(clarity_make_folders(dir));
    const char* file = clarity_format("%s/shop.db", dir);
    const char* first_sql[] = {"CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '')"};
    const char* second_sql[] = {"ALTER TABLE item ADD COLUMN price INTEGER NOT NULL DEFAULT 0"};
    ClarityStep steps[2] = {{first_sql, 1}, {second_sql, 1}};
    ClaritySource source = {"main", CLARITY_SQLITE, file};
    ClarityDb* db;
    int applied = 0;
    REQUIRE_OK(clarity_db_open(&source, 1, &db));
    CHECK_OK(clarity_migrate(db, "", steps, 1, &applied));
    clarity_db_close(db);
    CHECK(!clarity_exists(clarity_format("%s/backups", dir)));
    REQUIRE_OK(clarity_db_open(&source, 1, &db));
    CHECK_OK(clarity_migrate(db, "", steps, 2, &applied));
    CHECK_INT(applied, 1);
    clarity_db_close(db);
    size_t count = 0;
    const char** backups = files_in(clarity_format("%s/backups", dir), &count);
    REQUIRE(count == 1);
    CHECK(strncmp(backups[0], "shop-", 5) == 0);
    CHECK(strstr(backups[0], "-before-migration-0002.db") != NULL);
    ClaritySource copy = {"main", CLARITY_SQLITE, clarity_format("%s/backups/%s", dir, backups[0])};
    REQUIRE_OK(clarity_db_open(&copy, 1, &db));
    int version = 0;
    CHECK_OK(clarity_schema_version(db, &version));
    CHECK_INT(version, 1);
    clarity_db_close(db);
    remove_tree(dir);
}
