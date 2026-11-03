// Clarity.UI's commands answered by the service: see wire.h.

#include "service/wire.h"

#include <string.h>

#include "clarity/core.h"
#include "service/carried.h"

const ClarityCarried* clarity_carried_find(const ClarityCarried* files, size_t count, const char* name) {
    for (size_t i = 0; files && i < count; i++)
        if (files[i].name && strcmp(files[i].name, name) == 0) return &files[i];
    return NULL;
}

/// `args` with the names of the secret settings, which never go to the settings file.
static void with_secrets(const ClarityWire* wire, cJSON* args) {
    if (!wire || !wire->secrets) return;
    cJSON_DeleteItemFromObjectCaseSensitive(args, "secrets");
    cJSON_AddItemToObject(args, "secrets", cJSON_Parse(wire->secrets));
}

cJSON* clarity_wire_answer(ClarityService* service, const ClarityWire* wire, const char* command, cJSON* args) {
    // started: the main window and why the databases did not open, from the service's start.
    if (strcmp(command, "started") == 0) {
        cJSON* start = clarity_service_invoke(service, "start", args);
        const cJSON* value = cJSON_GetObjectItemCaseSensitive(start, "value");
        if (!cJSON_IsObject(value)) return start;
        cJSON* out = cJSON_CreateObject();
        cJSON* started = cJSON_AddObjectToObject(out, "value");
        cJSON_AddItemToObject(started, "main", cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(value, "main"), true));
        const cJSON* unconnected = cJSON_GetObjectItemCaseSensitive(value, "unconnected");
        cJSON_AddItemToObject(started, "unconnected", unconnected ? cJSON_Duplicate(unconnected, true) : cJSON_CreateNull());
        return out;
    }
    if (strcmp(command, "facts") == 0) {
        if (!wire || !wire->facts) return NULL;
        cJSON* out = cJSON_CreateObject();
        cJSON_AddItemToObject(out, "value", cJSON_Parse(wire->facts));
        return out;
    }
    // A stored file of a record, as base64.
    if (strcmp(command, "field_bytes") == 0) return clarity_service_invoke(service, "file_data", args);
    // A file of a store: base64, where the old web screen took a data URL.
    if (strcmp(command, "store_file") == 0) {
        cJSON* answer = clarity_service_invoke(service, "store_file", args);
        cJSON* value = cJSON_GetObjectItemCaseSensitive(answer, "value");
        const char* comma = cJSON_IsString(value) ? strchr(value->valuestring, ',') : NULL;
        if (comma && strncmp(value->valuestring, "data:", 5) == 0) cJSON_SetValuestring(value, clarity_format("%s", comma + 1));
        return answer;
    }
    if (strcmp(command, "read_settings") == 0 || strcmp(command, "keep_settings") == 0) with_secrets(wire, args);
    return clarity_service_invoke(service, command, args);
}
