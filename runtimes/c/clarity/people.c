// Who is using the program, as the core sees it: see people.h.

#include "clarity/people.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/text.h"

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif

#if defined(_MSC_VER)
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

static const ClarityPeopleHooks* people;
static THREAD_LOCAL ClarityCall* current;

/// The web sign-in window behind its own lock: requests read it on their
/// threads while the program may name its people anew (after `connect`).
static char* web_page;
static char no_window[1];
#if defined(_WIN32)
static SRWLOCK web_lock = SRWLOCK_INIT;
static void web_take(void) { AcquireSRWLockExclusive(&web_lock); }
static void web_give(void) { ReleaseSRWLockExclusive(&web_lock); }
#else
static pthread_mutex_t web_lock = PTHREAD_MUTEX_INITIALIZER;
static void web_take(void) { pthread_mutex_lock(&web_lock); }
static void web_give(void) { pthread_mutex_unlock(&web_lock); }
#endif

void clarity_people_set(const ClarityPeopleHooks* hooks) { people = hooks; }

const ClarityPeopleHooks* clarity_people(void) { return people; }

void clarity_people_set_web_page(const char* page) {
    char* copy = NULL;
    if (page) {
        size_t length = strlen(page);
        copy = malloc(length + 1);
        // Out of memory, sign-in still applies: a page no window is named.
        if (copy) memcpy(copy, page, length + 1);
        else copy = no_window;
    }
    web_take();
    char* was = web_page;
    web_page = copy;
    web_give();
    if (was != no_window) free(was);
}

const char* clarity_people_web_page(void) {
    web_take();
    const char* page = web_page ? clarity_copy(web_page) : NULL;
    web_give();
    return page;
}

void clarity_call_enter(ClarityCall* call) { current = call; }

ClarityCall* clarity_call(void) { return current; }

void clarity_call_open(ClarityCall* call, const char* cookie_header, const char* address) {
    *call = (ClarityCall){0};
    if (address) snprintf(call->address, sizeof call->address, "%s", address);
    char token[sizeof call->token];
    if (!people || !people->session || !cookie_header || !clarity_session_of(cookie_header, token, sizeof token)) return;
    int64_t me = people->session(token);
    if (me <= 0) return;
    call->me = me;
    memcpy(call->token, token, sizeof token);
}

const char* clarity_call_cookie(const ClarityCall* call, bool failed) {
    if (call->opened[0] && !failed)
        return clarity_format("%s=%s; HttpOnly; SameSite=Strict; Path=/; Max-Age=%llu", CLARITY_SESSION_COOKIE, call->opened, (unsigned long long)call->opened_seconds);
    if (call->closed) return clarity_format("%s=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0", CLARITY_SESSION_COOKIE);
    return NULL;
}

/* A key of Clarity's own words; the model's translations hold its text in every language. */
static ClarityText* keyed(const char* key, const char* english) {
    ClarityText* text = clarity_key_text(key);
    text->text = english;
    return text;
}

ClarityText* clarity_signin_refused(void) { return keyed("signinRefused", "Signing in did not work. Check what you typed and try again."); }

ClarityText* clarity_signin_too_many_tries(void) { return keyed("signinTooManyTries", "Too many tries. Wait a few minutes and try again."); }

ClarityText* clarity_secret_too_short(int64_t minimum) {
    return clarity_text_with(keyed("secretTooShort", "This needs at least {minimum} characters."), "minimum", clarity_slot_integer(minimum, NULL));
}

ClarityText* clarity_not_allowed(void) { return keyed("notAllowed", "You are not allowed to do this."); }

ClarityText* clarity_not_yours(void) { return keyed("notYours", "This record is not yours."); }

bool clarity_session_of(const char* cookie_header, char* into, size_t size) {
    const char* name = CLARITY_SESSION_COOKIE;
    size_t name_length = strlen(name);
    for (const char* pair = cookie_header; pair && *pair;) {
        while (*pair == ' ' || *pair == '\t' || *pair == ';') pair++;
        const char* end = strchr(pair, ';');
        size_t length = end ? (size_t)(end - pair) : strlen(pair);
        const char* equals = memchr(pair, '=', length);
        if (equals) {
            size_t named = (size_t)(equals - pair);
            while (named > 0 && (pair[named - 1] == ' ' || pair[named - 1] == '\t')) named--;
            if (named == name_length && strncmp(pair, name, name_length) == 0) {
                const char* value = equals + 1;
                size_t value_length = length - (size_t)(value - pair);
                while (value_length > 0 && (*value == ' ' || *value == '\t')) value++, value_length--;
                while (value_length > 0 && (value[value_length - 1] == ' ' || value[value_length - 1] == '\t' || value[value_length - 1] == '\r')) value_length--;
                if (value_length == 0 || value_length >= size) return false;
                memcpy(into, value, value_length);
                into[value_length] = '\0';
                return true;
            }
        }
        if (!end) break;
        pair = end + 1;
    }
    return false;
}

/* The text `name` of an object, NULL when it has none. */
static const char* text_in(const cJSON* object, const char* name) {
    const cJSON* item = cJSON_GetObjectItemCaseSensitive(object, name);
    return cJSON_IsString(item) ? item->valuestring : NULL;
}

bool clarity_open_without_session(const char* command, const cJSON* args, const char* page) {
    static const char* const open[] = {"started", "set_language", "facts", "splash_status", "watches", "read_settings", "signin", "signout"};
    for (size_t i = 0; i < sizeof open / sizeof open[0]; i++)
        if (strcmp(command, open[i]) == 0) return true;
    const char* window = strcmp(command, "open_window") == 0 ? text_in(args, "window") : text_in(cJSON_GetObjectItemCaseSensitive(args, "state"), "window");
    return window && page && strcmp(window, page) == 0;
}
