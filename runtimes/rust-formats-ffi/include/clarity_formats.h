/*
 * clarity_formats.h: the C interface of Clarity.Formats (docs/FORMATS-PLAN.md).
 * The library (libclarity_formats.so, clarity_formats.dll, libclarity_formats.dylib)
 * holds the data formats of code, one implementation for every stack: it
 * reads a format and fits it to what the program wants, and writes a value
 * in a format (conformance/formats-json-read.json, formats-json-write.json).
 *
 * Conventions
 * - Texts are UTF-8 as a pointer and a length, not ended by a zero.
 * - `kind` is JSON describing what is read or written: a record
 *   ({"kind":"record","entity":...,"fields":[...],"base":{...}}), a list of
 *   records ("records"), or a plain value ({"kind":"decimal"}, ...).
 * - `options` is a JSON object of the format's options ({"at":"data"}), or
 *   empty for none.
 * - `read` gives the fitted value as JSON, which the program decodes with its
 *   own record codec; `write` takes the value as the program's codec wrote it
 *   and gives the format's bytes. The library allocates the result into
 *   `*out` and `*out_len`; the program gives it back with `free`.
 * - A failing call gives a CLARITY_FORMATS_* code; `problem` copies what it was
 *   about, word for word as every stack says it. No panic crosses the interface.
 */

#ifndef CLARITY_FORMATS_H
#define CLARITY_FORMATS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_FORMATS_API __declspec(dllimport)
#else
#define CLARITY_FORMATS_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_FORMATS_INTERFACE 1

#define CLARITY_FORMATS_OK 0
/* What was read does not fit what the program wants, or is not in the format. */
#define CLARITY_FORMATS_NOT_FITTING 1
/* No format of that name is in the catalog. */
#define CLARITY_FORMATS_UNKNOWN_FORMAT 2
/* An option the format does not have, or of the wrong kind. */
#define CLARITY_FORMATS_BAD_OPTIONS 3
/* A fault: a kind that describes nothing, or one inside Clarity.Formats. */
#define CLARITY_FORMATS_FAILED 4

typedef struct ClarityFormatsApi {
    uint32_t interface;
    int32_t (*read)(const char* format, size_t format_len, const uint8_t* input, size_t input_len,
                    const char* options, size_t options_len, const char* kind, size_t kind_len,
                    uint8_t** out, size_t* out_len);
    int32_t (*write)(const char* format, size_t format_len, const char* value, size_t value_len,
                     const char* options, size_t options_len, const char* kind, size_t kind_len,
                     uint8_t** out, size_t* out_len);
    /* The catalog of formats and their options, as JSON. */
    int32_t (*catalog)(uint8_t** out, size_t* out_len);
    void (*free)(uint8_t* bytes, size_t len);
    /* What the last failing call on this thread was about, as UTF-8 into `into`; its code. */
    int32_t (*problem)(char* into, size_t size);
} ClarityFormatsApi;

/* The table, the same for the library's whole life. */
CLARITY_FORMATS_API const ClarityFormatsApi* clarity_formats_api(void);

#ifdef __cplusplus
}
#endif

#endif
