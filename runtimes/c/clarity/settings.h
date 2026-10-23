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
/// The settings `program` kept, as a new object the caller deletes; empty
/// when it kept none, or they cannot be read.
cJSON* clarity_read_settings(const char* program);
/// Keeps `values` as the settings of `program`, written whole and then put in place.
bool clarity_keep_settings(const char* program, const cJSON* values);
