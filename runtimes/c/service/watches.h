// Long calls (DESIGN 5.12, clarity_ui.h "Long calls"): what the screen sees
// of a handler that runs long and how the person stops it, by the number
// the screen gave the call. The desktop asks from the screen's thread while
// the handler runs on another; the web server asks from one request while
// the handler's request is open. Nothing here needs the library.

#pragma once

#include <stdbool.h>
#include <stdint.h>

#if defined(_WIN32)
#include <windows.h>
typedef SRWLOCK ClarityGate;
#else
#include <pthread.h>
typedef pthread_mutex_t ClarityGate;
#endif

/// One thread at a time through a gate.
void clarity_gate_init(ClarityGate* gate);
void clarity_gate_take(ClarityGate* gate);
void clarity_gate_give(ClarityGate* gate);

/// The long calls being watched, by their numbers.
typedef struct ClarityWatches ClarityWatches;

ClarityWatches* clarity_watches_new(void);
void clarity_watches_free(ClarityWatches* watches);

/// Runs what this thread does next under watch `id`, until
/// clarity_watches_leave: the engine calls of the handler say how far they
/// are there, and a stop sent before kept stops them.
void clarity_watches_enter(ClarityWatches* watches, uint64_t id);
/// This thread's handler is done: its watch keeps how far it got until end_watch.
void clarity_watches_leave(ClarityWatches* watches);

/// Whether `command` is one of the watch commands: start_watch,
/// watch_progress, stop_watch or end_watch.
bool clarity_watches_command(const char* command);
/// Answers a watch command for call `id`: watch_progress `{"value": ...}`
/// as text the caller frees, the others none (NULL). start_watch enters the watch on this thread (the
/// desktop's), end_watch leaves it when it is this thread's.
char* clarity_watches_answer(ClarityWatches* watches, const char* command, uint64_t id);
