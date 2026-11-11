/*
 * clarity_auth.h: the C interface of Clarity.Auth (docs/APP-CAPABILITIES-PLAN.md,
 * step 4). The library (libclarity_auth.so, clarity_auth.dll,
 * libclarity_auth.dylib) holds the sign-in of the programs Clarity makes,
 * one implementation for every stack: the catalog of strategies, finding
 * and checking a person, hashes, tokens, failed tries and sessions.
 *
 * Conventions
 * - One function, `call`, takes an operation's name and its arguments as a
 *   JSON object and gives the result as JSON. The operations:
 *     catalog {}                                  -> [strategy, ...]
 *     find {"strategy","inputs":{...}}            -> {"role","equals"}
 *     check {"strategy","inputs","stored":{role: value}} -> bool
 *     firstAccount {"strategy","inputs"}          -> {role: value}
 *     hash {"method","text"}                      -> text
 *     newToken {}                                 -> text
 *     triesAllowed {"key","tries","window"}       -> bool
 *     triesFailed {"key"}, triesCleared {"key"}   -> null
 *     sessionOpen {"record","seconds"}            -> token
 *     sessionFind {"token"}                       -> record, 0 for none
 *     sessionClose {"token"}, sessionCloseAll {"record"} -> null
 *     rememberMake {"record","seconds","stored"} -> entry (desktop key store)
 *     rememberRecord {"entry"} -> record, rememberCheck {"entry","stored"} -> record or 0
 *   Tries and sessions take an optional "now" in seconds, for tests.
 * - Texts are UTF-8 as a pointer and a length, not ended by a zero. The
 *   library allocates the result into `*out` and `*out_len`; the program
 *   gives it back with `free`.
 * - A failing call gives a CLARITY_AUTH_* code; `problem` copies what it was
 *   about. No panic crosses the interface.
 */

#ifndef CLARITY_AUTH_H
#define CLARITY_AUTH_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_AUTH_API __declspec(dllimport)
#else
#define CLARITY_AUTH_API
#endif

/* The version of the table; it grows when a field is added at its end. */
#define CLARITY_AUTH_INTERFACE 1

#define CLARITY_AUTH_OK 0
/* No strategy of that name is in the catalog. */
#define CLARITY_AUTH_UNKNOWN_STRATEGY 1
/* An input or argument missing or of the wrong kind. */
#define CLARITY_AUTH_BAD_INPUT 2
/* A fault: an unknown operation, or one inside Clarity.Auth. */
#define CLARITY_AUTH_FAILED 3

typedef struct ClarityAuthApi {
    uint32_t interface;
    int32_t (*call)(const char* op, size_t op_len, const char* args, size_t args_len, uint8_t** out, size_t* out_len);
    void (*free)(uint8_t* bytes, size_t len);
    /* What the last failing call on this thread was about, as UTF-8 into `into`; its code. */
    int32_t (*problem)(char* into, size_t size);
} ClarityAuthApi;

/* The table, the same for the library's whole life. */
CLARITY_AUTH_API const ClarityAuthApi* clarity_auth_api(void);

#ifdef __cplusplus
}
#endif

#endif
