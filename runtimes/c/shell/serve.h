// The web target (DESIGN 7.3): the application as a web server. It serves
// the renderer's page to a browser, telling the page where to send its
// commands, and answers each command with the same service the desktop
// window binds into its webview. One request at a time; the database is
// the server's.

#pragma once

#include "shell/shell.h"

/// Where a page the server serves sends its commands.
#define CLARITY_INVOKE_PATH "/clarity/invoke"
/// Where the server listens when nothing says otherwise.
#define CLARITY_DEFAULT_ADDRESS "127.0.0.1:8080"

/// The reply to one HTTP request, `length` bytes of it, in the scratch
/// arena: `page` at `/`, a command's answer at CLARITY_INVOKE_PATH.
const char* clarity_serve_answer(ClarityService* service, const char* page, const char* request, size_t request_length, size_t* length);

/// Where the server listens: `--address host:port`, else the environment's
/// CLARITY_ADDRESS, else CLARITY_DEFAULT_ADDRESS.
const char* clarity_serve_address(int argc, char** argv);

/// Runs the application as a web server on `address` until it stops; the
/// exit status.
int clarity_serve(const ClarityShellOptions* options, const char* address);
