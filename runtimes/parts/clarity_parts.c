// The part host (DESIGN 6.5): a queue of what the stack asked, done in
// order on a thread of the host's own, which also gives every shown part
// its frames. See clarity_parts.h.

#include "clarity_parts.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "platform.h"

/// How long between frames: about 60 a second.
#define FRAME_NS 16666667L

struct ClarityPartHost {
    uint64_t window;
    char* part;
};

/// A part of an open window.
typedef struct Part {
    ClarityPartHost host;
    const ClarityPartKind* kind;
    void* made;
    uintptr_t child;
    int x, y, width, height;
    bool shown;
    /// Values and hears given before the part was made, in order: an array of
    /// ["set", name, value] and ["hear", name, values].
    cJSON* waiting;
    struct Part* next;
} Part;

typedef enum { PLACE, SET, HEAR, CLOSE, WAIT, STOP } Ask;

/// One thing the stack asked.
typedef struct Asked {
    Ask ask;
    uint64_t window;
    char* part;
    char* name;
    cJSON* values;
    uintptr_t parent;
    int x, y, width, height;
    bool shown;
    struct Asked* next;
} Asked;

static pthread_t thread;
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t asked = PTHREAD_COND_INITIALIZER;
static pthread_cond_t done = PTHREAD_COND_INITIALIZER;
static Asked* first;
static Asked* last;
static unsigned long waits_asked, waits_done;
static bool running;

static ClarityPartsTold told;
static void* told_context;
static char problem[256];

/// Lives only on the part host's thread.
static Part* parts;

static char* copied(const char* text) {
    size_t n = strlen(text) + 1;
    char* out = malloc(n);
    memcpy(out, text, n);
    return out;
}

static void enqueue(Asked* a) {
    pthread_mutex_lock(&lock);
    if (last) {
        last->next = a;
    } else {
        first = a;
    }
    last = a;
    pthread_cond_signal(&asked);
    pthread_mutex_unlock(&lock);
}

static Asked* new_asked(Ask ask, uint64_t window, const char* part) {
    Asked* a = calloc(1, sizeof *a);
    a->ask = ask;
    a->window = window;
    a->part = part ? copied(part) : NULL;
    return a;
}

static void free_asked(Asked* a) {
    free(a->part);
    free(a->name);
    cJSON_Delete(a->values);
    free(a);
}

static Part* find(uint64_t window, const char* name) {
    for (Part* p = parts; p; p = p->next) {
        if (p->host.window == window && strcmp(p->host.part, name) == 0) return p;
    }
    return NULL;
}

/// The part of a window by name, made as a record when it is not there yet.
static Part* part_of(uint64_t window, const char* name) {
    Part* p = find(window, name);
    if (p) return p;
    p = calloc(1, sizeof *p);
    p->host.window = window;
    p->host.part = copied(name);
    p->waiting = cJSON_CreateArray();
    p->next = parts;
    parts = p;
    return p;
}

static const ClarityPartKind* kind_named(const char* name) {
    for (const ClarityPartKind* k = clarity_part_kinds; k->name; ++k) {
        if (strcmp(k->name, name) == 0) return k;
    }
    return NULL;
}

static void end(Part* p) {
    if (p->made) p->kind->destroy(p->made);
    if (p->child) clarity_platform_destroy(p->child);
    free(p->host.part);
    cJSON_Delete(p->waiting);
    free(p);
}

/// Whether CLARITY_PARTS_TRACE asks for every placement on standard error.
static bool tracing;

static void place(Asked* a) {
    const ClarityPartKind* kind = kind_named(a->name);
    if (tracing) {
        fprintf(stderr, "clarity parts: place %s (%s) of window %llu in %#lx at %d,%d %dx%d %s\n", a->part, a->name, (unsigned long long)a->window, (unsigned long)a->parent, a->x, a->y,
                a->width, a->height, a->shown ? "shown" : "hidden");
    }
    if (!kind) return;
    Part* p = part_of(a->window, a->part);
    if (!p->made) {
        p->kind = kind;
        p->child = clarity_platform_child(a->parent, a->x, a->y, a->width, a->height);
        clarity_platform_show(p->child, a->shown);
        p->x = a->x, p->y = a->y, p->width = a->width, p->height = a->height, p->shown = a->shown;
        p->made = kind->create((void*)p->child, a->width, a->height, &p->host);
        const cJSON* given;
        cJSON_ArrayForEach(given, p->waiting) {
            const char* name = cJSON_GetArrayItem(given, 1)->valuestring;
            const cJSON* values = cJSON_GetArrayItem(given, 2);
            if (strcmp(cJSON_GetArrayItem(given, 0)->valuestring, "set") == 0) {
                kind->set(p->made, name, values);
            } else {
                kind->hear(p->made, name, values);
            }
        }
        cJSON_Delete(p->waiting);
        p->waiting = cJSON_CreateArray();
        return;
    }
    bool resized = a->width != p->width || a->height != p->height;
    if (resized || a->x != p->x || a->y != p->y) clarity_platform_move(p->child, a->x, a->y, a->width, a->height);
    if (a->shown != p->shown) clarity_platform_show(p->child, a->shown);
    p->x = a->x, p->y = a->y, p->width = a->width, p->height = a->height, p->shown = a->shown;
    if (resized) p->kind->resize(p->made, a->width, a->height);
}

/// What the part is given before it is made waits, in order.
static void wait_for_part(Part* p, const char* what, const char* name, const cJSON* values) {
    cJSON* given = cJSON_CreateArray();
    cJSON_AddItemToArray(given, cJSON_CreateString(what));
    cJSON_AddItemToArray(given, cJSON_CreateString(name));
    cJSON_AddItemToArray(given, cJSON_Duplicate(values, true));
    cJSON_AddItemToArray(p->waiting, given);
}

static void set(Asked* a) {
    Part* p = part_of(a->window, a->part);
    if (!a->values) return;
    if (p->made) {
        p->kind->set(p->made, a->name, a->values);
    } else {
        wait_for_part(p, "set", a->name, a->values);
    }
}

static void hear(Asked* a) {
    Part* p = part_of(a->window, a->part);
    if (!cJSON_IsArray(a->values)) {
        cJSON_Delete(a->values);
        a->values = cJSON_CreateArray();
    }
    if (p->made) {
        p->kind->hear(p->made, a->name, a->values);
    } else {
        wait_for_part(p, "hear", a->name, a->values);
    }
}

static void close_window(uint64_t window) {
    Part** at = &parts;
    while (*at) {
        Part* p = *at;
        if (p->host.window == window) {
            *at = p->next;
            end(p);
        } else {
            at = &p->next;
        }
    }
}

static void frame_all(void) {
    for (Part* p = parts; p; p = p->next) {
        if (p->made && p->shown) p->kind->frame(p->made);
    }
}

static void add_ns(struct timespec* t, long ns) {
    t->tv_nsec += ns;
    while (t->tv_nsec >= 1000000000L) {
        t->tv_nsec -= 1000000000L;
        t->tv_sec += 1;
    }
}

static bool before(const struct timespec* a, const struct timespec* b) {
    return a->tv_sec < b->tv_sec || (a->tv_sec == b->tv_sec && a->tv_nsec < b->tv_nsec);
}

static void* run(void* unused) {
    (void)unused;
    struct timespec next_frame;
    clock_gettime(CLOCK_REALTIME, &next_frame);
    add_ns(&next_frame, FRAME_NS);
    for (;;) {
        pthread_mutex_lock(&lock);
        while (!first) {
            if (pthread_cond_timedwait(&asked, &lock, &next_frame) != 0) break;
        }
        Asked* taken = first;
        first = last = NULL;
        pthread_mutex_unlock(&lock);

        bool stop = false;
        unsigned long waits = 0;
        while (taken) {
            Asked* a = taken;
            taken = a->next;
            switch (a->ask) {
                case PLACE: place(a); break;
                case SET: set(a); break;
                case HEAR: hear(a); break;
                case CLOSE: close_window(a->window); break;
                case WAIT: ++waits; break;
                case STOP: stop = true; break;
            }
            free_asked(a);
        }
        struct timespec now;
        clock_gettime(CLOCK_REALTIME, &now);
        if (!before(&now, &next_frame)) {
            frame_all();
            next_frame = now;
            add_ns(&next_frame, FRAME_NS);
        }
        clarity_platform_flush();
        if (waits || stop) {
            pthread_mutex_lock(&lock);
            waits_done += waits;
            pthread_cond_broadcast(&done);
            pthread_mutex_unlock(&lock);
        }
        if (stop) break;
    }
    while (parts) {
        Part* p = parts;
        parts = p->next;
        end(p);
    }
    clarity_platform_close();
    return NULL;
}

bool clarity_parts_start(ClarityPartsTold on_told, void* context) {
    if (running) return true;
    problem[0] = '\0';
    tracing = getenv("CLARITY_PARTS_TRACE") != NULL;
    if (!clarity_platform_open(problem, sizeof problem)) return false;
    told = on_told;
    told_context = context;
    if (pthread_create(&thread, NULL, run, NULL) != 0) {
        snprintf(problem, sizeof problem, "the part host's thread could not start");
        clarity_platform_close();
        return false;
    }
    running = true;
    return true;
}

const char* clarity_parts_problem(void) {
    return problem;
}

void clarity_parts_stop(void) {
    if (!running) return;
    enqueue(new_asked(STOP, 0, NULL));
    pthread_join(thread, NULL);
    running = false;
}

void clarity_parts_place(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown) {
    if (!running) return;
    Asked* a = new_asked(PLACE, window, part);
    a->name = copied(kind);
    a->parent = parent;
    a->x = x, a->y = y, a->width = width, a->height = height, a->shown = shown;
    enqueue(a);
}

void clarity_parts_set(uint64_t window, const char* part, const char* name, const char* value) {
    if (!running) return;
    Asked* a = new_asked(SET, window, part);
    a->name = copied(name);
    a->values = cJSON_Parse(value);
    enqueue(a);
}

void clarity_parts_hear(uint64_t window, const char* part, const char* hear, const char* values) {
    if (!running) return;
    Asked* a = new_asked(HEAR, window, part);
    a->name = copied(hear);
    a->values = cJSON_Parse(values);
    enqueue(a);
}

void clarity_parts_close(uint64_t window) {
    if (!running) return;
    enqueue(new_asked(CLOSE, window, NULL));
}

void clarity_parts_wait(void) {
    if (!running) return;
    pthread_mutex_lock(&lock);
    unsigned long mine = ++waits_asked;
    pthread_mutex_unlock(&lock);
    enqueue(new_asked(WAIT, 0, NULL));
    pthread_mutex_lock(&lock);
    while (waits_done < mine) pthread_cond_wait(&done, &lock);
    pthread_mutex_unlock(&lock);
}

void clarity_parts_tell(ClarityPartHost* host, const char* tell, cJSON* values) {
    if (!cJSON_IsArray(values)) {
        cJSON_Delete(values);
        values = cJSON_CreateArray();
    }
    char* text = cJSON_PrintUnformatted(values);
    if (told) told(host->window, host->part, tell, text, told_context);
    free(text);
    cJSON_Delete(values);
}
