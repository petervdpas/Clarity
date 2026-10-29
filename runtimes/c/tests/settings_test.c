// Where an application keeps its settings, as every runtime finds it
// (conformance/settings.json, `file`), and keeping them.

#include <stdio.h>
#include <stdlib.h>
#ifndef _WIN32
#include <unistd.h>
#endif

#include "check.h"
#include "clarity/db.h"
#include "clarity/disk.h"
#include "clarity/settings.h"
#include "tests.h"

static const char* from_case(const char* name, void* context) {
    const cJSON* value = cJSON_GetObjectItemCaseSensitive((const cJSON*)context, name);
    return cJSON_IsString(value) ? value->valuestring : NULL;
}

void the_settings_file_is_where_every_runtime_puts_it(void) {
    cJSON* cases = conformance("settings.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "file")) {
        const char* found = clarity_settings_file_in(cJSON_GetObjectItem(c, "os")->valuestring, from_case, cJSON_GetObjectItem(c, "env"), cJSON_GetObjectItem(c, "program")->valuestring);
        const cJSON* wanted = cJSON_GetObjectItem(c, "file");
        if (cJSON_IsNull(wanted)) CHECK(found == NULL);
        else CHECK_TEXT(found, wanted->valuestring);
    }
    cJSON_Delete(cases);
}

void settings_kept_are_read_back(void) {
    // A folder of the test's own, not the user's settings.
    const char* folder = clarity_format("%s/clarity-settings-c-%d", temp_folder(), rand());
#ifdef _WIN32
    const char* was = getenv("APPDATA") ? clarity_format("%s", getenv("APPDATA")) : "";
    _putenv_s("APPDATA", folder);
#else
    setenv("XDG_CONFIG_HOME", folder, 1);
#endif
    const char* program = "clarity-settings-test";
    cJSON* none = clarity_read_settings(program);
    CHECK(cJSON_GetArraySize(none) == 0);
    cJSON_Delete(none);
    cJSON* values = cJSON_Parse("{\"pageSize\":20,\"language\":\"nl\"}");
    REQUIRE(clarity_keep_settings(program, values));
    cJSON* kept = clarity_read_settings(program);
    CHECK(cJSON_Compare(kept, values, 1));
    cJSON_Delete(kept);
    cJSON_Delete(values);
    // What the test kept goes, so the next run starts without it.
    remove_tree(folder);
#ifdef _WIN32
    _putenv_s("APPDATA", was);
#endif
}

void a_folder_setting_is_where_every_runtime_puts_a_store(void) {
    cJSON* cases = conformance("settings.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "folders")) {
        const char* found = clarity_store_folder_in(cJSON_GetObjectItem(c, "os")->valuestring, cJSON_GetObjectItem(c, "settingsFolder")->valuestring,
                                                    cJSON_GetObjectItem(c, "store")->valuestring, cJSON_GetObjectItem(c, "kept"), cJSON_GetObjectItem(c, "default")->valuestring);
        CHECK_TEXT(found, cJSON_GetObjectItem(c, "folder")->valuestring);
    }
}

// A store whose folder is a setting (filestore photos "setting:photosFolder")
// keeps its files where the person's settings say, read again at every use,
// so choosing another folder takes effect at once.
void a_store_keeps_its_files_in_a_folder_setting(void) {
    const char* home = clarity_format("%s/clarity-folder-c-%d", temp_folder(), rand());
#ifdef _WIN32
    const char* was = getenv("APPDATA") ? clarity_format("%s", getenv("APPDATA")) : "";
    _putenv_s("APPDATA", home);
#elif defined(__APPLE__)
    const char* was = clarity_format("%s", getenv("HOME"));
    setenv("HOME", home, 1);
#else
    setenv("XDG_CONFIG_HOME", home, 1);
#endif
    const char* program = "clarity-folder-test";
    const char* settings_folder = clarity_parent(clarity_settings_file(program));
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_add_store_setting(db, "photos", program, "photosFolder", "photos"));
    // Not chosen yet: the default, in the settings' folder.
    CHECK_OK(clarity_write_text(db, "photos", "first.txt", "first"));
    CHECK(clarity_exists(clarity_format("%s/photos/first.txt", settings_folder)));
    // Chosen while the program runs: the next file goes there.
    const char* chosen = clarity_format("%s/chosen", home);
    cJSON* values = cJSON_CreateObject();
    cJSON_AddStringToObject(values, "photosFolder", chosen);
    REQUIRE(clarity_keep_settings(program, values));
    cJSON_Delete(values);
    CHECK_OK(clarity_write_text(db, "photos", "second.txt", "second"));
    CHECK(clarity_exists(clarity_format("%s/second.txt", chosen)));
    clarity_db_close(db);
    remove_tree(home);
#ifdef _WIN32
    _putenv_s("APPDATA", was);
#elif defined(__APPLE__)
    setenv("HOME", was, 1);
#endif
}
