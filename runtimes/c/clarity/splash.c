// What the splash screen says is loading; see splash.h.

#include "clarity/splash.h"

#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#include <windows.h>
static SRWLOCK gate = SRWLOCK_INIT;
#define TAKE() AcquireSRWLockExclusive(&gate)
#define GIVE() ReleaseSRWLockExclusive(&gate)
#else
#include <pthread.h>
static pthread_mutex_t gate = PTHREAD_MUTEX_INITIALIZER;
#define TAKE() pthread_mutex_lock(&gate)
#define GIVE() pthread_mutex_unlock(&gate)
#endif

static char* status = NULL;

static char* copied(const char* text) {
    size_t size = strlen(text) + 1;
    char* out = malloc(size);
    if (out) memcpy(out, text, size);
    return out;
}

void clarity_splash_set_status(const ClarityText* text) {
    // Written as JSON at once: the text's own memory may be the handler's scratch.
    char* json = text ? cJSON_PrintUnformatted(clarity_text_json(text)) : NULL;
    char* now = copied(json ? json : "\"\"");
    TAKE();
    char* before = status;
    status = now;
    GIVE();
    free(before);
}

char* clarity_splash_status(void) {
    TAKE();
    char* out = status ? copied(status) : NULL;
    GIVE();
    return out;
}
