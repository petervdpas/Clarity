// Who is using the program (docs/APP-CAPABILITIES-PLAN.md, step 4), through
// Clarity.Auth, the one implementation every stack calls. The generated
// program names its entity of people and its strategies once, after its
// databases open (clarity_db_add_signin); clarity_signin then finds the
// person by what Clarity.Auth says, checks them and makes them the current
// person. On the desktop and the command line that is one person for the
// program; served, each call is the person of its session cookie
// (clarity/people.h), so `me` is read per call (clarity_me_id). Refusals
// are Clarity's own words, by key (conformance/signin.json). Carried only
// when the project signs people in (clarity_use_auth in clarity.cmake).
//
// A save of a stored person that changes the secret of one of their
// strategies signs them out everywhere else when served (their own change),
// or everywhere (someone else's); nothing on the desktop or command line.
// After a sign-in, what Clarity.Auth says to store (a recovery code used
// up) is written back to the person's row.
//
// The first account: when the table of people has no rows and the strategy
// can make one (its catalog entry says `firstAccount`), the record is
// inserted with only the fields the strategy names (the match field, the
// secret as its hash); every other column takes its default from the
// table, so a required field without a default makes the insert fail.

#pragma once

#include <stdbool.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/db.h"
#include "clarity/people.h"

/// One strategy the project uses: its name in Clarity.Auth's catalog and
/// the columns of the table of people it names, in the catalog's order of
/// roles (`password email passwordHash` is {"email", "password_hash"}).
typedef struct {
    const char* name;
    const char* const* columns;
    size_t column_count;
} ClaritySigninStrategy;

/// The project's `signin` block as the program uses it. A zero
/// `session_seconds`, `tries` or `tries_seconds` takes Clarity's default:
/// 8 hours a session, 5 tries per 15 minutes.
typedef struct {
    /// The data source of the entity of people; "" or NULL for the first.
    const char* source;
    const char* table;
    /// Its key column; NULL for `id`.
    const char* key;
    /// The window of the kind `signin`.
    const char* page;
    /// Where people sign in: "desktop", "web", "cli".
    const char* const* targets;
    size_t target_count;
    const ClaritySigninStrategy* strategies;
    size_t strategy_count;
    /// How long a served session lasts.
    uint64_t session_seconds;
    /// Failed tries per key allowed within `tries_seconds`.
    uint32_t tries;
    uint64_t tries_seconds;
    /// The program's name, under which its secrets are kept (the remembered sign-in).
    const char* program;
    /// How long the desktop program keeps a sign-in between runs (`remember
    /// N days`) in the system's protected store (clarity_secret_store); 0
    /// without the line.
    uint64_t remember_seconds;
    /// Whether the sign-in part asks whether to remember (`remember N days
    /// when asked`): then only a sign-in whose input `remember` is "true"
    /// is remembered; else every one is.
    bool remember_asks;
} ClaritySigninSource;

/// The name of the remembered desktop sign-in among the program's secrets.
#define CLARITY_REMEMBERED "clarityRemembered"

/// Names the people of the program and how they sign in, after the
/// databases opened (and again after `connect` opens them anew). Copies
/// what it is given.
bool clarity_db_add_signin(ClarityDb* db, const ClaritySigninSource* source);

/// Signs a person in with `strategy` and its `inputs` (an object: `name`,
/// `password`, `token`, as the catalog names them): `*id` the record signed
/// in, or Clarity's refusal (signinRefused, signinTooManyTries). Failed
/// tries count per account and, served, per address.
bool clarity_signin(ClarityDb* db, const char* strategy, const cJSON* inputs, int64_t* id);
/// Signs the person out: served, the call's session is closed and its
/// cookie cleared; else nobody is signed in any more.
void clarity_signout(ClarityDb* db);

/// The record signed in for this call: served, the one of the call's
/// session; else the one of the program. 0 for nobody.
int64_t clarity_me_id(ClarityDb* db);
/// `me` in code: the record signed in now, read as `entity`; refused with
/// notAllowed when nobody is.
bool clarity_me(ClarityDb* db, const ClarityEntity* entity, void* record);

/// The sign-in window the program starts with, as Started.signin says it:
/// NULL when it names no people, when its people do not sign in where it
/// runs (web served, else desktop), or when someone is signed in. On the
/// desktop a sign-in remembered (`remember N days`) that still holds signs
/// its person in, else the `account` strategy signs the computer's user in
/// by itself when they have a record (no try counted).
const char* clarity_signin_first(ClarityDb* db);

/// `allow when <condition>`: false, refused with notAllowed, when it does not hold.
bool clarity_allow(bool holds);
/// `readable when` / `writable when` of an entity, checked on a find or a
/// save: false, refused with notYours, when it does not hold.
bool clarity_yours(bool holds);
/// Text assigned to a `hashed` field: what is stored, by the field's method
/// (`argon2id`, `sha256` for a token field, `sha256-lines` for recovery
/// codes), in the scratch arena. With a `minimum` (0 for none, the
/// strategy's `minimum N`) the text is checked first, refused with
/// secretTooShort (slot `minimum`) when it is shorter.
bool clarity_hashed(const char* method, const char* text, int64_t minimum, const char** out);
/// `newRecoveryCodes(count)` in code: the codes as lines of text, shown
/// once, in the scratch arena.
bool clarity_new_recovery_codes(int64_t count, const char** out);
/// `newToken()` in code, in the scratch arena.
bool clarity_new_token(const char** out);

/// What a command line gives to sign in with: `--user name` (the password
/// then typed), `--token value` (or the environment's CLARITY_TOKEN). NULL
/// for what it does not give.
typedef struct {
    const char* user;
    const char* token;
} ClarityCliSignin;

/// `--user name` and `--token value` among the program's arguments, the
/// token else from CLARITY_TOKEN.
ClarityCliSignin clarity_cli_signin_args(int argc, char** argv);
/// Signs the command line's person in before a command runs, when the
/// program's people sign in on `cli`: `--token` for `token`, `--user` with
/// the password `password(prompt)` gives (NULL for none; the program passes
/// clarity_read_password, which reads it without echo) for `password`, else the computer's user by
/// `account` when they have a record. `*signed_in` false and true returned
/// when the command line needs no sign-in; else `*id` the record signed in,
/// or Clarity's refusal.
bool clarity_signin_cli(ClarityDb* db, const ClarityCliSignin* given, const char* (*password)(const char* prompt), const char* prompt, int64_t* id, bool* signed_in);
/// A line typed on the terminal without echo, after `prompt`, in the
/// scratch arena; NULL when the program has no terminal.
const char* clarity_read_password(const char* prompt);
