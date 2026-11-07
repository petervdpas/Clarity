/*
 * clarity_image.h: the C interface of Clarity.Image (docs/IMAGE-PLAN.md).
 * The library (libclarity_image.so, clarity_image.dll, libclarity_image.dylib)
 * holds the picture functions of code: resize, crop, rotate, flip, blend,
 * adjust and a picture's size, alike on every stack (conformance/picture-functions.json).
 *
 * Conventions
 * - A picture is its encoded bytes (PNG, JPEG or WebP) as a pointer and a length.
 * - A function that gives a picture writes a PNG the library allocated into
 *   `*out` and `*out_len`; the program gives it back with `free`.
 * - A failing call gives a CLARITY_IMAGE_* code; `problem` copies what it was
 *   about. No panic crosses the interface.
 */

#ifndef CLARITY_IMAGE_H
#define CLARITY_IMAGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_IMAGE_API __declspec(dllimport)
#else
#define CLARITY_IMAGE_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_IMAGE_INTERFACE 1

#define CLARITY_IMAGE_OK 0
/* The bytes are no PNG, JPEG or WebP picture. */
#define CLARITY_IMAGE_NOT_A_PICTURE 1
/* A size of 0, or more than 30000 pixels a side. */
#define CLARITY_IMAGE_BAD_SIZE 2
/* A fault inside Clarity.Image. */
#define CLARITY_IMAGE_FAILED 3

typedef struct ClarityImageApi {
    uint32_t interface;
    int32_t (*resize)(const uint8_t* picture, size_t len, int64_t width, int64_t height, uint8_t** out, size_t* out_len);
    int32_t (*crop)(const uint8_t* picture, size_t len, int64_t x, int64_t y, int64_t width, int64_t height, uint8_t** out, size_t* out_len);
    int32_t (*rotate)(const uint8_t* picture, size_t len, double degrees, uint8_t** out, size_t* out_len);
    int32_t (*flip)(const uint8_t* picture, size_t len, int32_t horizontal, uint8_t** out, size_t* out_len);
    int32_t (*blend)(const uint8_t* base, size_t base_len, const uint8_t* over, size_t over_len, int64_t x, int64_t y, int64_t opacity, uint8_t** out, size_t* out_len);
    int32_t (*adjust)(const uint8_t* picture, size_t len, double hue, double saturation, double brightness, double contrast, uint8_t** out, size_t* out_len);
    int32_t (*size)(const uint8_t* picture, size_t len, uint32_t* width, uint32_t* height);
    void (*free)(uint8_t* bytes, size_t len);
    /* What the last failing call on this thread was about, as UTF-8 into `into`; its code. */
    int32_t (*problem)(char* into, size_t size);
} ClarityImageApi;

/* The table, the same for the library's whole life. */
CLARITY_IMAGE_API const ClarityImageApi* clarity_image_api(void);

#ifdef __cplusplus
}
#endif

#endif
