#include "clarity/core.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cjson/cJSON.h"
#include "clarity/text.h"

// The scratch arena: blocks of memory handed out from the front. A reset
// keeps the first block and frees the others.

enum { BLOCK = 64 * 1024, ALIGN = 16 };

typedef struct Block {
    struct Block* next;
    size_t size;
    size_t used;
    _Alignas(16) unsigned char data[];
} Block;

static Block* first;
static Block* current;

static Block* block(size_t size) {
    Block* b = malloc(sizeof(Block) + size);
    if (!b) {
        fputs("clarity: out of memory\n", stderr);
        abort();
    }
    b->next = NULL;
    b->size = size;
    b->used = 0;
    return b;
}

void* clarity_alloc(size_t size) {
    size = (size + ALIGN - 1) / ALIGN * ALIGN;
    if (size == 0) size = ALIGN;
    if (!first) first = current = block(BLOCK);
    if (current->used + size > current->size) {
        Block* next = block(size > BLOCK ? size : BLOCK);
        current->next = next;
        current = next;
    }
    void* out = current->data + current->used;
    current->used += size;
    memset(out, 0, size);
    return out;
}

void clarity_scratch_reset(void) {
    if (!first) return;
    Block* b = first->next;
    while (b) {
        Block* next = b->next;
        free(b);
        b = next;
    }
    first->next = NULL;
    first->used = 0;
    current = first;
}

size_t clarity_scratch_used(void) {
    size_t used = 0;
    for (Block* b = first; b; b = b->next) used += b->used;
    return used;
}

static void* json_alloc(size_t size) { return clarity_alloc(size); }

// JSON lives in the scratch arena too, and goes with it.
static void json_free(void* pointer) { (void)pointer; }

void clarity_start(void) {
    static bool started;
    if (started) return;
    started = true;
    cJSON_Hooks hooks = {json_alloc, json_free};
    cJSON_InitHooks(&hooks);
}

char* clarity_copy_n(const char* text, size_t length) {
    char* out = clarity_alloc(length + 1);
    if (text && length) memcpy(out, text, length);
    return out;
}

char* clarity_copy(const char* text) { return clarity_copy_n(text, text ? strlen(text) : 0); }

static char* vformat(const char* format, va_list args) {
    va_list again;
    va_copy(again, args);
    int length = vsnprintf(NULL, 0, format, args);
    char* out = clarity_alloc((size_t)(length < 0 ? 0 : length) + 1);
    if (length > 0) vsnprintf(out, (size_t)length + 1, format, again);
    va_end(again);
    return out;
}

char* clarity_format(const char* format, ...) {
    va_list args;
    va_start(args, format);
    char* out = vformat(format, args);
    va_end(args);
    return out;
}

void clarity_grow(void** items, size_t* capacity, size_t count, size_t item_size) {
    if (count < *capacity) return;
    size_t room = *capacity ? *capacity * 2 : 8;
    void* more = clarity_alloc(room * item_size);
    if (*items && count) memcpy(more, *items, count * item_size);
    *items = more;
    *capacity = room;
}

void clarity_buffer_add_n(ClarityBuffer* buffer, const char* text, size_t length) {
    if (buffer->length + length + 1 > buffer->capacity) {
        size_t room = buffer->capacity ? buffer->capacity : 64;
        while (room < buffer->length + length + 1) room *= 2;
        char* more = clarity_alloc(room);
        if (buffer->data) memcpy(more, buffer->data, buffer->length);
        buffer->data = more;
        buffer->capacity = room;
    }
    if (length) memcpy(buffer->data + buffer->length, text, length);
    buffer->length += length;
    buffer->data[buffer->length] = '\0';
}

void clarity_buffer_add(ClarityBuffer* buffer, const char* text) { clarity_buffer_add_n(buffer, text, text ? strlen(text) : 0); }

void clarity_buffer_add_char(ClarityBuffer* buffer, char c) { clarity_buffer_add_n(buffer, &c, 1); }

void clarity_buffer_addf(ClarityBuffer* buffer, const char* format, ...) {
    va_list args;
    va_start(args, format);
    char* text = vformat(format, args);
    va_end(args);
    clarity_buffer_add(buffer, text);
}

const char* clarity_buffer_text(const ClarityBuffer* buffer) { return buffer->data ? buffer->data : ""; }

// The last error, as errno keeps it.

static ClarityError last;

const ClarityError* clarity_error(void) { return &last; }

void clarity_set_error(const ClarityError* error) { last = *error; }

static void set(ClarityErrorKind kind, const char* message) {
    memset(&last, 0, sizeof last);
    last.kind = kind;
    snprintf(last.message, sizeof last.message, "%s", message);
}

bool clarity_fail(ClarityErrorKind kind, const char* format, ...) {
    const char* prefix = kind == CLARITY_STORAGE || kind == CLARITY_NOT_KEPT ? "storage: "
                         : kind == CLARITY_INVALID  ? "invalid value: "
                         : kind == CLARITY_CONTRACT ? "condition failed: "
                                                    : "";
    char message[sizeof last.message];
    size_t at = (size_t)snprintf(message, sizeof message, "%s", prefix);
    va_list args;
    va_start(args, format);
    vsnprintf(message + at, sizeof message - at, format, args);
    va_end(args);
    set(kind, message);
    return false;
}

bool clarity_not_found(const char* table, int64_t id) {
    char message[sizeof last.message];
    if (id) snprintf(message, sizeof message, "there is no %s %lld", table, (long long)id);
    else snprintf(message, sizeof message, "the %s is not set", table);
    set(CLARITY_NOT_FOUND, message);
    snprintf(last.table, sizeof last.table, "%s", table);
    last.id = id;
    return false;
}

bool clarity_in_use(const char* table, int64_t id) {
    char message[sizeof last.message];
    snprintf(message, sizeof message, "the %s %lld is still in use", table, (long long)id);
    set(CLARITY_IN_USE, message);
    snprintf(last.table, sizeof last.table, "%s", table);
    last.id = id;
    return false;
}

bool clarity_refuse(const ClarityText* text) {
    set(CLARITY_REFUSED, clarity_text_show(text));
    last.text = text;
    return false;
}

bool clarity_require(bool holds, const char* what) { return holds || clarity_fail(CLARITY_CONTRACT, "%s", what); }
