// The settings each person keeps for themselves (DESIGN 5.17): one JSON
// file per application in the folder for application settings of the
// person's system (conformance/settings.json, `file`). The screen decides
// which kept values still fit; this only reads and keeps the file.

#pragma once

#include "clarity/core.h"
#include "cjson/cJSON.h"

/// Reads a variable of the environment, or NULL; `context` is the caller's.
typedef const char* (*ClarityEnv)(const char* name, void* context);

/// Where `program` keeps its settings on system `os` ("windows", "macos",
/// or another), with `env` giving the environment, in the scratch arena;
/// NULL when it names no folder for application settings.
const char* clarity_settings_file_in(const char* os, ClarityEnv env, void* context, const char* program);
/// Where `program` keeps its settings on this machine, or NULL.
const char* clarity_settings_file(const char* program);
/// This machine's system, as the cases name it: "windows", "macos" or "linux".
const char* clarity_settings_os(void);
/// The folder of file store `store` when a folder setting names it, on
/// system `os`, in the scratch arena: the `kept` value when it is text that
/// is not empty, else `fallback` (the setting's default), else a folder
/// named after the store; a relative folder is in `settings_folder`, the
/// folder of the settings file.
const char* clarity_store_folder_in(const char* os, const char* settings_folder, const char* store, const cJSON* kept, const char* fallback);
/// The settings `program` kept, as a new object the caller deletes; empty
/// when it kept none, or they cannot be read.
cJSON* clarity_read_settings(const char* program);
/// Keeps `values` as the settings of `program`, written whole and then put in place.
bool clarity_keep_settings(const char* program, const cJSON* values);

/// The system's protected store of secret settings (DESIGN 5.27): `get`
/// gives a secret in the scratch arena or NULL, `set` keeps one ("" forgets
/// it). Empty in a program without secret settings; one with them calls
/// clarity_secrets_use (clarity/secrets) to fill it.
typedef struct {
    const char* (*get)(const char* program, const char* name);
    bool (*set)(const char* program, const char* name, const char* value);
} ClaritySecretStore;
extern ClaritySecretStore clarity_secret_store;

/// clarity_read_settings with the secrets named in `secrets` (an array of
/// names, or NULL), each from the store, beside the file's values.
cJSON* clarity_read_settings_with(const char* program, const cJSON* secrets);
/// clarity_keep_settings with those named in `secrets` kept in the store,
/// never in the file.
bool clarity_keep_settings_with(const char* program, const cJSON* values, const cJSON* secrets);
