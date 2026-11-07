// CLARITY_DATA moves every SQLite database of a program into a folder of its
// own, under its own file name: a run in the IDE or by hand (a live check on
// a copy) chooses its data without editing the generated program. The same
// as the Rust and C# runtimes.

#include <stdlib.h>

#include "check.h"
#include "clarity/db.h"
#include "clarity/disk.h"
#include "clarity/store.h"
#include "tests.h"

static const char* copy_folder;

static const char* copy_env(const char* name) { return strcmp(name, CLARITY_DATA_VARIABLE) == 0 ? copy_folder : NULL; }

void clarity_data_moves_an_sqlite_file_into_its_folder(void) {
    const char* at = "/home/someone/noughts/data/noughts.db";
    copy_folder = "/copy";
    CHECK_TEXT(clarity_data_folder_sqlite(at, copy_env), "/copy/noughts.db");
    CHECK_TEXT(clarity_data_folder_sqlite("C:\\shop\\shop.db", copy_env), "/copy/shop.db");
    // Unset or empty, the file is where the project put it.
    copy_folder = NULL;
    CHECK_TEXT(clarity_data_folder_sqlite(at, copy_env), at);
    copy_folder = "";
    CHECK_TEXT(clarity_data_folder_sqlite(at, copy_env), at);
    // A location from the environment or a secret setting is as it says, and so is SQLite's database in memory.
    copy_folder = "/copy";
    CHECK_TEXT(clarity_data_folder_sqlite("env:SHOP_DB", copy_env), "env:SHOP_DB");
    CHECK_TEXT(clarity_data_folder_sqlite("setting:shop/db", copy_env), "setting:shop/db");
    CHECK_TEXT(clarity_data_folder_sqlite(":memory:", copy_env), ":memory:");
}

static void set_data_folder(const char* folder) {
#ifdef _WIN32
    _putenv_s(CLARITY_DATA_VARIABLE, folder ? folder : "");
#else
    if (folder)
        setenv(CLARITY_DATA_VARIABLE, folder, 1);
    else
        unsetenv(CLARITY_DATA_VARIABLE);
#endif
}

// The process's variable, read when a database opens.
void a_database_opens_in_the_folder_clarity_data_names(void) {
    const char* folder = clarity_format("%s/clarity-data-c-%d", temp_folder(), rand());
    REQUIRE(clarity_make_folders(folder));
    set_data_folder(folder);
    const char* elsewhere = clarity_format("%s/elsewhere/shop.db", folder);
    ClaritySource source = {"main", CLARITY_SQLITE, elsewhere};
    ClarityDb* db = NULL;
    bool opened = clarity_db_open(&source, 1, &db);
    set_data_folder(NULL);
    CHECK(opened);
    if (opened) clarity_db_close(db);
    CHECK(clarity_exists(clarity_format("%s/shop.db", folder)));
    CHECK(!clarity_exists(elsewhere));
    remove_tree(folder);
}

static const cJSON* case_env;

static const char* case_env_get(const char* name) {
    const cJSON* value = cJSON_GetObjectItem(case_env, name);
    return cJSON_IsString(value) ? value->valuestring : NULL;
}

// Every runtime places an SQLite database alike (conformance/data-folder.json).
void a_database_opens_where_every_runtime_opens_it(void) {
    cJSON* cases = conformance("data-folder.json");
    REQUIRE(cases);
    const cJSON* item;
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(cases, "cases")) {
        case_env = cJSON_GetObjectItem(item, "env");
        CHECK_TEXT(clarity_data_folder_sqlite_on(cJSON_GetObjectItem(item, "os")->valuestring, cJSON_GetObjectItem(item, "location")->valuestring, case_env_get),
                   cJSON_GetObjectItem(item, "file")->valuestring);
    }
}
