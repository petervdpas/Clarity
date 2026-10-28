// The web target (DESIGN 7.3): the application as a web server. It serves
// the renderer's page to a browser, telling the page where to send its
// commands, and answers each command with the same service the desktop
// window binds into its webview. One request at a time; the database is
// the server's.

#pragma once

#include <string>

#include "shell/shell.hpp"

namespace clarity::shell {

/// Where a page the server serves sends its commands.
inline constexpr const char* invoke_path = "/clarity/invoke";

/// The reply to one HTTP request: `page` at `/`, a command's answer at invoke_path.
std::string serve_answer(Service& service, const std::string& page, const std::string& request);

/// Where the server listens: `--address host:port`, else the environment's
/// CLARITY_ADDRESS, else `project`, where the project file says (its port).
std::string serve_address(int argc, char** argv, const std::string& project);

/// Runs the application as a web server on `address` until it stops; the exit status.
int serve(const Options& options, const std::string& address);

}  // namespace clarity::shell
