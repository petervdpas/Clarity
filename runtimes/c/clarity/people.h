// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4), as the
// runtime's core sees it. Signing in itself is auth/auth.h, carried only by
// a program that names people who sign in (clarity_use_auth in
// clarity.cmake): when it adds its people to a database it hands the core
// these hooks, through which the service answers `started`, `signin` and
// `signout` and the web server finds the person of a session. Without them
// the program has no people and nobody signs in.
//
// A served call runs as the person of its session cookie: the web server
// enters the call (clarity_call_enter) on the thread that answers it, and
// signing in or out writes what changed of its session into it.

#pragma once

#include <stdbool.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

typedef struct ClarityDb ClarityDb;
typedef struct ClarityEntity ClarityEntity;

/// The name of the session cookie a served program sets.
#define CLARITY_SESSION_COOKIE "clarity_session"

/// What auth/auth.c does for the core.
typedef struct {
    /// The sign-in window the program starts with, NULL for none (Started.signin).
    const char* (*first)(ClarityDb* db);
    /// Signs a person in: their record's id, or Clarity's refusal.
    bool (*signin)(ClarityDb* db, const char* strategy, const cJSON* inputs, int64_t* id);
    void (*signout)(ClarityDb* db);
    /// The record a session token names, 0 for none.
    int64_t (*session)(const char* token);
    /// Closes a session, as signing in anew does with the one a call came with.
    void (*close)(const char* token);
    /// Whether `entity` is the program's people, whose saves `changed` hears of.
    bool (*watches)(ClarityDb* db, const ClarityEntity* entity);
    /// A save of stored person `id` changed these columns.
    void (*changed)(ClarityDb* db, const ClarityEntity* entity, int64_t id, const char* const* columns, size_t count);
} ClarityPeopleHooks;

/// Sets the hooks, for the program's whole life; NULL takes them away.
void clarity_people_set(const ClarityPeopleHooks* hooks);
/// The hooks; NULL when the program names no people.
const ClarityPeopleHooks* clarity_people(void);

/// The sign-in window of a program whose people sign in on the web: a
/// served call without a session reaches only it. NULL for none.
void clarity_people_set_web_page(const char* page);
/// The web sign-in window, in the scratch arena; NULL for none.
const char* clarity_people_web_page(void);

/// One served call: its person, the session and address it came with, and
/// what signing in or out changed of its session.
typedef struct {
    /// 0 for nobody.
    int64_t me;
    /// The session token the call came with ("" for none).
    char token[128];
    /// Where the call came from ("" when unknown).
    char address[64];
    /// The session a sign-in opened, and how long it lasts; "" for none.
    char opened[128];
    uint64_t opened_seconds;
    /// Whether the call signed out.
    bool closed;
} ClarityCall;

/// A served call from `address` ("" when unknown) with the request's
/// Cookie header (NULL for none): the person of its session, when the
/// session is one the program opened and it has not ended.
void clarity_call_open(ClarityCall* call, const char* cookie_header, const char* address);
/// The Set-Cookie header's value after the call, in the scratch arena: a
/// new session when it signed in and did not fail, the cookie cleared when
/// it signed out; NULL when its session did not change.
const char* clarity_call_cookie(const ClarityCall* call, bool failed);

/// The call this thread answers from now on; NULL when it answers none.
void clarity_call_enter(ClarityCall* call);
/// The served call this thread answers; NULL off one (the desktop, the command line).
ClarityCall* clarity_call(void);

/// Clarity's words when signing in did not work.
ClarityText* clarity_signin_refused(void);
/// Clarity's words when too many tries failed.
ClarityText* clarity_signin_too_many_tries(void);
/// Clarity's words when a chosen secret is shorter than `minimum`.
ClarityText* clarity_secret_too_short(int64_t minimum);
/// Clarity's words when `allow when` does not hold.
ClarityText* clarity_not_allowed(void);
/// Clarity's words when `readable when` or `writable when` does not hold.
ClarityText* clarity_not_yours(void);

/// The session token a request's Cookie header carries, copied into
/// `into`; false when it carries none (or one too long to be Clarity's).
bool clarity_session_of(const char* cookie_header, char* into, size_t size);

/// Whether a served call without a session may run while people sign in on
/// the web: the program's start, its language and facts, signing in and
/// out, and the sign-in window `page` itself.
bool clarity_open_without_session(const char* command, const cJSON* args, const char* page);
