// Files in records (U1c): a file field holds no file (NULL), the one the
// record has stored, a new one chosen but not yet saved, or a file of its
// file store; always with its name. Like a picture, reading records never
// carries the file itself; the screen fetches it on its own to open it or
// save a copy. In the database the bytes are in the field's column and the
// name in the column beside it; in a file store the file keeps its name in
// a folder named by its contents (conformance/record-files.json).

#pragma once

#include "cjson/cJSON.h"
#include "clarity/picture.h"
#include "clarity/settings.h"
#include "clarity/value.h"

/// The file of a file field that has one, and its name.
typedef struct {
    const char* name;
    ClarityPicture* content;
} ClarityRecordFile;

ClarityRecordFile* clarity_record_file(const char* name, ClarityPicture* content);
/// The name a chosen file is kept under: the last part of what was chosen,
/// trimmed; "file" when nothing is left.
const char* clarity_clean_name(const char* chosen);
/// The path of a file in a file store: a folder named by the first 32 hex
/// digits of the SHA-256 of its bytes, and in it the file under its name.
const char* clarity_store_path(ClarityBytes bytes, const char* name);
/// The name a path in a store shows: its last part.
const char* clarity_name_of(const char* path);
/// The extension of a file's name, in lower case without its dot; "" without one.
const char* clarity_extension_of(const char* name);
/// The name of a record's file, and its extension; "" when it has none.
const char* clarity_record_file_name(const ClarityRecordFile* file);
const char* clarity_record_file_extension(const ClarityRecordFile* file);
/// The folder a copy is saved in on system `os`, with `env` giving the environment; NULL when it names no home.
const char* clarity_downloads_in(const char* os, ClarityEnv env, void* context);
/// The name a copy gets where `taken` names are in use: the name itself, else " (2)", " (3)" and on before its extension.
const char* clarity_free_name(const char* name, bool (*taken)(const char* name, void* context), void* context);
/// Saves a copy of a file in the person's Downloads folder, under a name free there; the file it wrote.
bool clarity_save_copy(const char* name, ClarityBytes bytes, const char** out);
/// Writes a copy of a file in a folder of its own under the system's temporary folder, to open it; the file.
bool clarity_temporary_copy(const char* name, ClarityBytes bytes, const char** out);

/// What a file field stores in its own column (as a picture does), and in the column of its name ("" for none).
ClarityValue clarity_file_content_value(const ClarityRecordFile* file);
ClarityValue clarity_file_name_value(const ClarityRecordFile* file);
/// A record's file, from whether it has one (as for a picture) and its name.
bool clarity_get_file(const char* column, ClarityValue content, ClarityValue name, ClarityRecordFile** out);
/// `{"name", "stored": true}`, `{"name", "data"}`, `{"name", "file"}`, or null for none.
cJSON* clarity_file_json(const ClarityRecordFile* file);
bool clarity_json_get_file(const cJSON* in, const char* key, ClarityRecordFile** out);
