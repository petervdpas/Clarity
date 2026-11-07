/*
 * clarity_video.h: the C interface of Clarity.Video (docs/VIDEO-PLAN.md).
 * The library (libclarity_video.so, clarity_video.dll,
 * libclarity_video.dylib) opens a video and decodes its pictures on a
 * thread of its own, by the first decoder that takes it: a codec the
 * program added, the system's own, or Cisco's openh264 beside the program.
 *
 * A program does not call it itself. It hands the table to Clarity.UI once,
 * before its first window, and Clarity.UI's video parts play through it:
 *
 *     clarity_ui_use_video(clarity_video_api());
 *
 * Conventions
 * - Text is UTF-8 as a pointer and a length in bytes; it need not end with NUL.
 * - Times are milliseconds from the video's start.
 * - A failing call gives a CLARITY_VIDEO_* code; `problem` copies what it was
 *   about. No panic crosses the interface.
 */

#ifndef CLARITY_VIDEO_H
#define CLARITY_VIDEO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_VIDEO_API __declspec(dllimport)
#else
#define CLARITY_VIDEO_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_VIDEO_INTERFACE 2

#define CLARITY_VIDEO_OK 0
/* No file at that path. */
#define CLARITY_VIDEO_MISSING 1
/* The file is there but could not be read. */
#define CLARITY_VIDEO_UNREADABLE 2
/* The file is no video Clarity.Video reads. */
#define CLARITY_VIDEO_NOT_A_VIDEO 3
/* No codec takes the video's format. */
#define CLARITY_VIDEO_NO_CODEC 4
/* The codec could not be loaded. */
#define CLARITY_VIDEO_CODEC_MISSING 5
/* A frame could not be decoded. */
#define CLARITY_VIDEO_UNDECODABLE 6
/* A NULL handle, text that is no UTF-8, or a fault inside the library. */
#define CLARITY_VIDEO_FAILED 7

/* What `picture_at` did. */
#define CLARITY_VIDEO_NO_PICTURE 0
#define CLARITY_VIDEO_SAME_PICTURE 1
#define CLARITY_VIDEO_NEW_PICTURE 2

typedef struct ClarityVideoFrames ClarityVideoFrames;

typedef struct ClarityVideoApi {
    /* CLARITY_VIDEO_INTERFACE of the library. */
    uint32_t interface;
    /* Opens the video at `path` and starts decoding; NULL when it cannot (`problem` with NULL says why). */
    ClarityVideoFrames *(*open)(const char *path, size_t path_len);
    void (*free)(ClarityVideoFrames *frames);
    /* Width and height in pixels. */
    void (*size)(const ClarityVideoFrames *frames, uint32_t *width, uint32_t *height);
    /* The video's length, 0 when the file does not say. */
    uint64_t (*length_ms)(const ClarityVideoFrames *frames);
    /* How long one frame shows. */
    uint64_t (*frame_ms)(const ClarityVideoFrames *frames);
    /* The picture to show at `at_ms`: its time goes to `time_ms`. When that is
     * `known_ms` (the time of the picture the caller holds) it gives
     * CLARITY_VIDEO_SAME_PICTURE and copies nothing; else it copies the RGBA
     * pixels (width * height * 4 bytes, row after row) into `into` when `size`
     * holds them and gives CLARITY_VIDEO_NEW_PICTURE. Pass UINT64_MAX as
     * `known_ms` when the caller holds none. */
    int32_t (*picture_at)(const ClarityVideoFrames *frames, uint64_t at_ms, uint64_t known_ms, uint8_t *into, size_t size, uint64_t *time_ms);
    /* 1 when `at_ms` is past the last picture, else 0. */
    int32_t (*ended_at)(const ClarityVideoFrames *frames, uint64_t at_ms);
    /* Why the pictures stopped coming (frames), or why the last open on this
     * thread failed (NULL): its code, the detail copied into `into` as text
     * (at most `size` bytes, ending with NUL when there is room). */
    int32_t (*problem)(const ClarityVideoFrames *frames, char *into, size_t size);
    /* Interface 2: loads the codec library `file` (clarity_codec.h) where
     * Clarity carries it (beside the program, its `codecs` folder, .NET's
     * native folder, beside this library) and registers it, so it plays the
     * formats it takes. CLARITY_VIDEO_OK, or CLARITY_VIDEO_CODEC_MISSING and
     * `problem` with NULL says why. */
    int32_t (*add_codec)(const char *file, size_t file_len);
} ClarityVideoApi;

/* The table, the same for the library's whole life. */
CLARITY_VIDEO_API const ClarityVideoApi *clarity_video_api(void);

#ifdef __cplusplus
}
#endif

#endif
