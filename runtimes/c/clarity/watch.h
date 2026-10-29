// What the screen sees of a long engine call and how the person stops it
// (DESIGN 5.12, `uses progress`): the shell watches the handler it runs,
// and every engine call the handler makes reports to that watch.

#pragma once

#include <stdint.h>

/// What the screen sees of a long call and how the person stops it (`uses
/// progress`, conformance/progress.json): every report of how far it is
/// goes to `progress`, with `context`; once `*stop` is not 0 (set from any
/// thread) the call ends, failing with "it was stopped". Either may be NULL.
typedef struct {
    void (*progress)(int64_t done, int64_t total, void* context);
    void* context;
    const volatile int32_t* stop;
} ClarityEngineWatch;

/// Makes `watch` (or none) the watch of every engine call this thread makes,
/// as a shell does around a handler; gives the one that was.
const ClarityEngineWatch* clarity_watching(const ClarityEngineWatch* watch);
/// The watch of the handler running on this thread; NULL outside one.
const ClarityEngineWatch* clarity_current_watch(void);
