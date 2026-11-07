/*
 * clarity_audio.h: the C interface of Clarity.Audio (docs/AUDIO-PLAN.md).
 * The library (libclarity_audio.so, clarity_audio.dll,
 * libclarity_audio.dylib) plays songs: mp3, flac, wav and ogg files, read
 * as they play, with pause, seek, the position and the length.
 *
 * A program does not call it itself. It hands the table to Clarity.UI once,
 * before its first window, and Clarity.UI's sound parts play through it:
 *
 *     clarity_ui_use_audio(clarity_audio_api());
 *
 * Conventions
 * - Players are used on the thread that made them (the window's thread).
 * - Text is UTF-8 as a pointer and a length in bytes; it need not end with NUL.
 * - A call that fails returns a CLARITY_AUDIO_* code other than
 *   CLARITY_AUDIO_OK; `problem` copies what it was about (a path, the
 *   decoder's reason). No panic crosses the interface.
 */

#ifndef CLARITY_AUDIO_H
#define CLARITY_AUDIO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_AUDIO_API __declspec(dllimport)
#else
#define CLARITY_AUDIO_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_AUDIO_INTERFACE 2

#define CLARITY_AUDIO_OK 0
/* No file at that path. */
#define CLARITY_AUDIO_MISSING 1
/* The file is there but could not be read. */
#define CLARITY_AUDIO_UNREADABLE 2
/* The data is no sound Clarity.Audio decodes. */
#define CLARITY_AUDIO_NOT_A_SOUND 3
/* The song cannot be moved to that point. */
#define CLARITY_AUDIO_CANNOT_SEEK 4
/* The computer has no speakers, or they did not open. */
#define CLARITY_AUDIO_NO_SPEAKERS 5
/* A NULL player, text that is no UTF-8, or a fault inside the library. */
#define CLARITY_AUDIO_FAILED 6

/* Which tag `tag` copies (interface 2). */
#define CLARITY_AUDIO_TITLE 0
#define CLARITY_AUDIO_ARTIST 1
#define CLARITY_AUDIO_ALBUM 2
/* The track number, as text ("7"; empty when the file does not say). */
#define CLARITY_AUDIO_TRACK 3

/* What `state` gives, as bits. */
#define CLARITY_AUDIO_PLAYING 1
#define CLARITY_AUDIO_PAUSED 2
#define CLARITY_AUDIO_ENDED 4

typedef struct ClarityAudioPlayer ClarityAudioPlayer;

typedef struct ClarityAudioApi {
    /* CLARITY_AUDIO_INTERFACE of the library. */
    uint32_t interface;
    /* A player on the speakers, opened the first time; NULL when they do not open (`problem` says why). */
    ClarityAudioPlayer *(*player_new)(void);
    void (*player_free)(ClarityAudioPlayer *player);
    int32_t (*play_file)(ClarityAudioPlayer *player, const char *path, size_t path_len);
    /* Sound data in memory, copied; `hint` is its kind ("mp3"), or NULL and 0. */
    int32_t (*play_bytes)(ClarityAudioPlayer *player, const uint8_t *bytes, size_t len, const char *hint, size_t hint_len);
    void (*pause)(ClarityAudioPlayer *player);
    void (*resume)(ClarityAudioPlayer *player);
    int32_t (*seek)(ClarityAudioPlayer *player, uint64_t milliseconds);
    void (*stop)(ClarityAudioPlayer *player);
    /* From 0 to 1. */
    void (*set_volume)(ClarityAudioPlayer *player, float volume);
    /* CLARITY_AUDIO_PLAYING, CLARITY_AUDIO_PAUSED, CLARITY_AUDIO_ENDED. */
    uint32_t (*state)(const ClarityAudioPlayer *player);
    uint64_t (*position_ms)(const ClarityAudioPlayer *player);
    /* The song's length, or -1 when its file does not say. */
    int64_t (*length_ms)(const ClarityAudioPlayer *player);
    /* Copies the last problem of this thread into `into` (at most `size` bytes,
     * ending with NUL when there is room); gives its whole length in bytes. */
    size_t (*problem)(char *into, size_t size);
    /* Interface 2: copies a tag of the song that plays (CLARITY_AUDIO_TITLE and
     * the others) into `into` as `problem` does; gives its whole length. */
    size_t (*tag)(const ClarityAudioPlayer *player, uint32_t which, char *into, size_t size);
} ClarityAudioApi;

/* The table, the same for the library's whole life. */
CLARITY_AUDIO_API const ClarityAudioApi *clarity_audio_api(void);

#ifdef __cplusplus
}
#endif

#endif
