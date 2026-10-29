#include "clarity/watch.h"

#if defined(_MSC_VER) && !defined(__clang__)
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

static THREAD_LOCAL const ClarityEngineWatch* current;

const ClarityEngineWatch* clarity_watching(const ClarityEngineWatch* watch) {
    const ClarityEngineWatch* was = current;
    current = watch;
    return was;
}

const ClarityEngineWatch* clarity_current_watch(void) { return current; }
