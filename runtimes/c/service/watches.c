// Long calls: see watches.h.

#include "service/watches.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/watch.h"

#if defined(_MSC_VER) && !defined(__clang__)
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

#if defined(_WIN32)
void clarity_gate_init(ClarityGate* gate) { InitializeSRWLock(gate); }
void clarity_gate_take(ClarityGate* gate) { AcquireSRWLockExclusive(gate); }
void clarity_gate_give(ClarityGate* gate) { ReleaseSRWLockExclusive(gate); }
#else
void clarity_gate_init(ClarityGate* gate) { pthread_mutex_init(gate, NULL); }
void clarity_gate_take(ClarityGate* gate) { pthread_mutex_lock(gate); }
void clarity_gate_give(ClarityGate* gate) { pthread_mutex_unlock(gate); }
#endif

/// A long call, by the number the screen gave it.
typedef struct Watch {
    ClarityWatches* watches;
    uint64_t id;
    ClarityEngineWatch watch;
    volatile int32_t stop;
    int64_t done, total;
    /// Whether an engine said how far it is; whether its handler runs now; whether end_watch came while it ran.
    bool told, running, ended;
    const ClarityEngineWatch* was;
    struct Watch* next;
} Watch;

struct ClarityWatches {
    ClarityGate gate;
    Watch* all;
};

/// The watch this thread's handler runs under.
static THREAD_LOCAL Watch* mine;

ClarityWatches* clarity_watches_new(void) {
    ClarityWatches* w = calloc(1, sizeof *w);
    if (w) clarity_gate_init(&w->gate);
    return w;
}

static void dropped(ClarityWatches* all, Watch* gone) {
    for (Watch** at = &all->all; *at; at = &(*at)->next) {
        if (*at == gone) {
            *at = gone->next;
            free(gone);
            return;
        }
    }
}

void clarity_watches_free(ClarityWatches* watches) {
    if (!watches) return;
    while (watches->all) dropped(watches, watches->all);
    free(watches);
}

static Watch* found(ClarityWatches* all, uint64_t id) {
    for (Watch* w = all->all; w; w = w->next)
        if (w->id == id) return w;
    return NULL;
}

static Watch* found_or_new(ClarityWatches* all, uint64_t id) {
    Watch* w = found(all, id);
    if (w) return w;
    w = calloc(1, sizeof *w);
    if (!w) return NULL;
    w->watches = all;
    w->id = id;
    w->next = all->all;
    all->all = w;
    return w;
}

/// An engine call says how far it is, on the handler's thread.
static void progressed(int64_t done, int64_t total, void* context) {
    Watch* w = context;
    clarity_gate_take(&w->watches->gate);
    w->done = done;
    w->total = total;
    w->told = true;
    clarity_gate_give(&w->watches->gate);
}

/// Enters `id` on this thread, holding the gate.
static void entered(ClarityWatches* all, uint64_t id) {
    Watch* w = found_or_new(all, id);
    if (!w) return;
    // A stop that came before the start holds: w->stop stays as it was.
    w->running = true;
    w->ended = false;
    w->watch.progress = progressed;
    w->watch.context = w;
    w->watch.stop = &w->stop;
    w->was = clarity_watching(&w->watch);
    mine = w;
}

/// Leaves this thread's watch, holding the gate; it goes when end_watch came meanwhile.
static void left(ClarityWatches* all) {
    Watch* w = mine;
    if (!w) return;
    clarity_watching(w->was);
    mine = NULL;
    w->running = false;
    if (w->ended) dropped(all, w);
}

void clarity_watches_enter(ClarityWatches* watches, uint64_t id) {
    clarity_gate_take(&watches->gate);
    entered(watches, id);
    clarity_gate_give(&watches->gate);
}

void clarity_watches_leave(ClarityWatches* watches) {
    clarity_gate_take(&watches->gate);
    left(watches);
    clarity_gate_give(&watches->gate);
}

bool clarity_watches_command(const char* command) {
    return strcmp(command, "start_watch") == 0 || strcmp(command, "watch_progress") == 0 || strcmp(command, "stop_watch") == 0 || strcmp(command, "end_watch") == 0;
}

static char* copied(const char* text) {
    size_t size = strlen(text) + 1;
    char* out = malloc(size);
    if (out) memcpy(out, text, size);
    return out;
}

char* clarity_watches_answer(ClarityWatches* all, const char* command, uint64_t id) {
    char text[96];
    snprintf(text, sizeof text, "{\"value\":null}");
    bool tells = strcmp(command, "watch_progress") == 0;
    clarity_gate_take(&all->gate);
    if (strcmp(command, "start_watch") == 0) {
        // On the handler's thread, just before it runs: its engine calls report here.
        entered(all, id);
    } else if (strcmp(command, "watch_progress") == 0) {
        Watch* w = found(all, id);
        if (w && w->told) snprintf(text, sizeof text, "{\"value\":[%lld,%lld]}", (long long)w->done, (long long)w->total);
    } else if (strcmp(command, "stop_watch") == 0) {
        // A stop may come before its start: it holds once the call starts.
        Watch* w = found_or_new(all, id);
        if (w) w->stop = 1;
    } else if (strcmp(command, "end_watch") == 0) {
        // It may come twice, and while the handler still runs on another thread.
        Watch* w = found(all, id);
        if (w && mine == w) {
            w->ended = true;
            left(all);
        } else if (w && w->running) {
            w->ended = true;
        } else if (w) {
            dropped(all, w);
        }
    }
    clarity_gate_give(&all->gate);
    return tells ? copied(text) : NULL;
}
