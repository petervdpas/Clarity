/*
 * clarity_codec.h: what a codec library gives Clarity.Video
 * (docs/VIDEO-PLAN.md, "Codecs people add"). A project names the library in
 * its project file, `codec vp9 from "codecs/libvp9"`; Clarity carries it
 * beside the program and Clarity.Video loads it as the program starts. For
 * the formats it takes it wins over the system's decoder and openh264.
 *
 * The library exports one function, clarity_codec_api, giving this table.
 * Any language that makes a C library can write one: around libvpx, dav1d
 * or FFmpeg, in C, C++, Rust, Go (cgo) or C#'s native AOT.
 *
 * Conventions
 * - A frame is one sample as the MP4 file holds it.
 * - Pictures are RGBA, 4 bytes a pixel, row after row, no padding.
 * - Decoders are used from one thread at a time.
 */

#ifndef CLARITY_CODEC_H
#define CLARITY_CODEC_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_CODEC_INTERFACE 1

/* What `decode` did. */
#define CLARITY_CODEC_PICTURE 1
#define CLARITY_CODEC_WAITING 0
#define CLARITY_CODEC_ERROR (-1)

typedef struct ClarityCodecApi {
    /* CLARITY_CODEC_INTERFACE of the library. */
    uint32_t interface;
    /* Its name, as a project file names it: "vp9". */
    const char *name;
    /* The formats it decodes as MP4 names them, apart by commas: "vp09,av01". */
    const char *formats;
    /* A decoder for a track of `format`, `width` by `height`; `setup` is the
     * codec's setup from the file (for H.264 the parameter sets, each led by
     * its length in four bytes, big-endian), or NULL and 0. NULL when it
     * cannot make one. */
    void *(*decoder_new)(const char *format, uint32_t width, uint32_t height, const uint8_t *setup, size_t setup_len);
    void (*decoder_free)(void *decoder);
    /* Decodes `frame`; with a picture, writes it into `rgba` (`size` bytes,
     * width * height * 4 of the track) and its size into `width` and `height`. */
    int32_t (*decode)(void *decoder, const uint8_t *frame, size_t frame_len, uint8_t *rgba, size_t size, uint32_t *width, uint32_t *height);
} ClarityCodecApi;

const ClarityCodecApi *clarity_codec_api(void);

#ifdef __cplusplus
}
#endif

#endif
