// Pictures in records: an image field holds no picture (NULL), the one the
// record has stored, a new one loaded but not yet saved, or a file of the
// field's own store. Reading records never carries the pictures themselves;
// the screen fetches each on its own. A picture is kept in the database, or
// as a file in a file store named by the hash of its bytes, the column
// holding that name. The other runtimes do the same (conformance/pictures.json).

#pragma once

#include "cjson/cJSON.h"
#include "clarity/dialect.h"
#include "clarity/value.h"

typedef enum {
    /// The record has one stored; it is fetched on its own.
    CLARITY_PICTURE_STORED,
    /// One loaded but not yet saved.
    CLARITY_PICTURE_NEW,
    /// A file of the field's own store, by its path there: saved as that path, nothing copied.
    CLARITY_PICTURE_IN_STORE,
} ClarityPictureState;

typedef struct {
    ClarityPictureState state;
    /// The bytes of a new picture.
    ClarityBytes bytes;
    /// The path of a file of the field's own store.
    const char* path;
} ClarityPicture;

ClarityPicture* clarity_picture_stored(void);
ClarityPicture* clarity_picture_new(ClarityBytes bytes);
ClarityPicture* clarity_picture_in_store(const char* path);
/// Whether two pictures are the same; NULL is no picture.
bool clarity_picture_same(const ClarityPicture* a, const ClarityPicture* b);

/// An image field's column, and the file store it keeps its pictures in;
/// without one (NULL) they are in the database.
typedef struct {
    const char* column;
    const char* store;
} ClarityPictureColumn;

/// A kind of picture every stack shows.
typedef struct {
    const char* extension;
    const char* media_type;
} ClarityPictureKind;

/// The kind of picture `bytes` are, told by their first bytes; false for no picture.
bool clarity_picture_kind(ClarityBytes bytes, ClarityPictureKind* out);
/// The kind of `bytes`, or an invalid-value error when they are no picture.
bool clarity_picture_checked(ClarityBytes bytes, ClarityPictureKind* out);
/// The name of a picture's file in a file store: the first 32 hex digits of
/// the SHA-256 of its bytes, and the extension of its kind.
bool clarity_picture_file_name(ClarityBytes bytes, const char** out);
/// A picture as the page shows it.
bool clarity_data_url(ClarityBytes bytes, const char** out);
/// What the select list reads of a picture kept in the database: whether there is one.
const char* clarity_presence_sql(ClarityDialect d, const char* column);
/// The file `path` (as a column holds it) in the store at `root`; a path
/// that would leave the store is refused.
bool clarity_in_store(const char* root, const char* path, const char** out);

const char* clarity_sha256_hex(ClarityBytes bytes);
const char* clarity_base64(ClarityBytes bytes);
ClarityBytes clarity_from_base64(const char* text);

/// The picture as it is stored: none, the one stored already (kept), a new one, or a file of its store.
ClarityValue clarity_picture_value(const ClarityPicture* picture);
/// `{"stored": true}`, `{"data": base64}`, `{"file": path}`, or null for none.
cJSON* clarity_picture_json(const ClarityPicture* picture);
ClarityPicture* clarity_picture_from_json(const cJSON* in);
