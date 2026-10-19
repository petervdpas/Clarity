// What every part of the C runtime stands on: memory for the values of one
// command, and the error of the last call that failed.
//
// Memory: text, lists, records read from the database and JSON come from
// the scratch arena. It lasts until the shell has answered the command they
// belong to, when clarity_scratch_reset() empties it, so generated code
// never frees anything. The program runs one command at a time.
//
// Errors: a call that can fail returns false and leaves its error, read
// with clarity_error(), as errno does. The kinds are the other runtimes'.

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#if defined(__GNUC__)
#define CLARITY_PRINTF(at, from) __attribute__((format(printf, at, from)))
#else
#define CLARITY_PRINTF(at, from)
#endif

/// Readies the runtime: cJSON takes its memory from the scratch arena.
/// Opening a database calls it; calling it again does nothing.
void clarity_start(void);

/// `size` zeroed bytes from the scratch arena.
void* clarity_alloc(size_t size);
/// `count` items of `type`, zeroed, from the scratch arena.
#define CLARITY_NEW(type, count) ((type*)clarity_alloc(sizeof(type) * (size_t)(count)))
/// A copy of `text` in the scratch arena; NULL gives "".
char* clarity_copy(const char* text);
char* clarity_copy_n(const char* text, size_t length);
/// Text written as printf writes it, in the scratch arena.
char* clarity_format(const char* format, ...) CLARITY_PRINTF(1, 2);
/// Ends a command: everything the scratch arena holds is gone.
void clarity_scratch_reset(void);
/// How many bytes the scratch arena hands out now, for tests.
size_t clarity_scratch_used(void);

/// Bytes, such as a picture's.
typedef struct {
    const uint8_t* data;
    size_t size;
} ClarityBytes;

/// Text being written piece by piece, in the scratch arena. Zeroed it is
/// empty; `data` always ends with a NUL.
typedef struct {
    char* data;
    size_t length;
    size_t capacity;
} ClarityBuffer;

void clarity_buffer_add(ClarityBuffer* buffer, const char* text);
void clarity_buffer_add_n(ClarityBuffer* buffer, const char* text, size_t length);
void clarity_buffer_add_char(ClarityBuffer* buffer, char c);
void clarity_buffer_addf(ClarityBuffer* buffer, const char* format, ...) CLARITY_PRINTF(2, 3);
/// The text written so far; "" when nothing was.
const char* clarity_buffer_text(const ClarityBuffer* buffer);

/// Room for one more item in a growing array of the scratch arena.
void clarity_grow(void** items, size_t* capacity, size_t count, size_t item_size);

/// What went wrong.
typedef enum {
    /// The database or the disk failed.
    CLARITY_STORAGE = 1,
    /// A value does not fit: a wrong date, a reference to nothing, a file that is no picture.
    CLARITY_INVALID,
    /// A record that is not stored, or a reference that is empty.
    CLARITY_NOT_FOUND,
    /// A record that others still refer to cannot be deleted.
    CLARITY_IN_USE,
    /// `refuse`: the text the person reads, with its slots.
    CLARITY_REFUSED,
    /// A `require` or `ensure` that did not hold.
    CLARITY_CONTRACT,
    /// A statement broke a reference between records; the database turns
    /// it into CLARITY_INVALID or CLARITY_IN_USE.
    CLARITY_REFERENCE,
    /// An engine could not load, or failed while it ran (DESIGN 5.11).
    CLARITY_ENGINE,
} ClarityErrorKind;

typedef struct ClarityText ClarityText;

typedef struct {
    ClarityErrorKind kind;
    /// As the other runtimes word it: `storage: database is locked`.
    char message[1024];
    /// The text of a refusal.
    const ClarityText* text;
    /// The table and id of a record not found or in use; id 0 for a reference that is empty.
    char table[128];
    int64_t id;
} ClarityError;

/// The error of the last call that failed.
const ClarityError* clarity_error(void);
/// Puts back an error kept from before, such as one a rollback must not hide.
void clarity_set_error(const ClarityError* error);
/// Leaves an error of `kind`, its message written as printf writes it. Always false.
bool clarity_fail(ClarityErrorKind kind, const char* format, ...) CLARITY_PRINTF(2, 3);
/// There is no record `id` of `table`; id 0: the reference is empty. Always false.
bool clarity_not_found(const char* table, int64_t id);
/// The record `id` of `table` is still referred to. Always false.
bool clarity_in_use(const char* table, int64_t id);
/// `refuse text`. Always false.
bool clarity_refuse(const ClarityText* text);
/// A `require` or `ensure` line: false when it does not hold.
bool clarity_require(bool holds, const char* what);
