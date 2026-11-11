// The web target (DESIGN 7.3): the application as a web server. Clarity.UI
// runs in the browser and asks the program over HTTP: the server serves the
// browser files the program carries (its browser/ folder: the page, the
// library's files, clarity/files.json and the resources below
// clarity/resources/), and answers each of the library's commands at
// CLARITY_INVOKE_PATH as the desktop answers them in-process
// (service/wire.h). Requests run side by side, each on a thread; commands
// take turns on the service and its database, except how far a long call
// is and its Stop (service/watches.h). The database is the server's. Each
// request names its page (`"browser"`, one id per tab) and brings the
// person's settings as the page keeps them (`"settings"`): its commands run
// with that page's settings and language (its own `set_language`), and
// settings a command changed go back in that page's answer alone (DESIGN
// 7.9 B2).

#pragma once

#include "service/carried.h"
#include "service/service.h"
#include "service/watches.h"
#include "service/wire.h"

/// Where the page sends Clarity.UI's commands.
#define CLARITY_INVOKE_PATH "/clarity/invoke"

typedef struct {
    /// What the service answers from: the program's windows, its databases, its model.
    ClarityServiceOptions service;
    /// The browser files, served by their names ("index.html" at /).
    const ClarityCarriedFiles* files;
    /// What the program adds to the service's answers: the facts.
    ClarityWire wire;
    /// Whether an engine says how far a long call is: the page then shows a
    /// bar with Stop, and names the call's watch in the handler's request.
    bool watches;
    /// Answers the library's commands of engines beside the screen
    /// (engine_open, engine_hears, engine_frame, engine_close; DESIGN 5.12)
    /// as {"value"} or {"error"}, beside the service's turn
    /// (clarity_screen_answer); NULL when the program has none.
    cJSON* (*screen_engines)(const char* command, const cJSON* args);
} ClarityServeOptions;

/// The server: the service, and the long calls it watches.
typedef struct ClarityServer ClarityServer;

/// A server over a new service; NULL when it did not start. `options` must outlast it.
ClarityServer* clarity_server_new(const ClarityServeOptions* options);
void clarity_server_free(ClarityServer* server);
ClarityService* clarity_server_service(ClarityServer* server);

/// How long a browser page may go unasked before the server forgets it, in
/// milliseconds: a tab closes without saying so.
#define CLARITY_QUIET_MS (30u * 60u * 1000u)
/// Sets how long a page may go unasked, as tests do.
void clarity_server_quiet(ClarityServer* server, uint64_t milliseconds);

/// The reply to one HTTP request, `length` bytes of it, in this thread's
/// scratch arena: a file the browser takes, or a command's answer. From any thread.
const char* clarity_serve_answer(ClarityServer* server, const char* request, size_t request_length, size_t* length);
/// clarity_serve_answer for a request from `address` (the client's, as
/// digits; "" when unknown), which failed sign-in tries count by. A request
/// carries the person's session in its `clarity_session` cookie, and a
/// reply that signs in or out sets it (clarity/people.h).
const char* clarity_serve_answer_from(ClarityServer* server, const char* request, size_t request_length, const char* address, size_t* length);

/// Where the server listens: `--address host:port`, else the environment's
/// CLARITY_ADDRESS, else `project`, where the project file says (its port).
const char* clarity_serve_address(int argc, char** argv, const char* project);

/// Runs the application as a web server on `address` until it stops; the
/// exit status.
int clarity_serve(const ClarityServeOptions* options, const char* address);
