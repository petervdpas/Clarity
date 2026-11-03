// The web target (DESIGN 7.3): the application as a web server. Clarity.UI
// runs in the browser and asks the program over HTTP: the server serves
// the browser files the program carries (Options::browser_files, its browser/
// folder: the page, the library's files, clarity/files.json and the
// resources below clarity/resources/), and answers each of the library's
// commands at invoke_path as the desktop answers them in-process
// (wire_answer). Requests run side by side, each on a thread of its own, so
// a long call's request stays open while the page asks how far it is. The
// database is the server's. Each request names its page (`"browser"`, one
// id per tab) and brings the person's settings as the page keeps them
// (`"settings"`): its commands run with that page's settings and language
// (its own set_language), and settings a command changed go back in that
// page's answer alone (DESIGN 7.9 B2).

#pragma once

#include <chrono>
#include <map>
#include <mutex>
#include <optional>
#include <string>

#include "shell/shell.hpp"

namespace clarity::shell {

/// Where the page sends Clarity.UI's commands.
inline constexpr const char* invoke_path = "/clarity/invoke";

/// The server: the service, and the long calls it watches beside it.
class Server {
public:
    explicit Server(Options options) : service_(std::move(options)) {}
    Service& service() { return service_; }
    Watches& watches() { return watches_; }

    /// How long a page may go unasked before the server forgets it: a tab closes without saying so.
    static constexpr std::chrono::milliseconds quiet_by_default = std::chrono::minutes(30);
    /// Sets how long a page may go unasked, as tests do.
    void quiet(std::chrono::milliseconds quiet) { quiet_ = quiet; }

    /// Page `browser` as it is now, its settings `sent` (as the page keeps
    /// them; null keeps those it had) taken; pages unasked for too long go.
    Page page(const std::string& browser, const nlohmann::json& sent);
    /// Page `browser`'s language, as its set_language chose.
    void choose_language(const std::string& browser, std::optional<std::string> language);
    /// Page `browser`'s settings with `changed` over them: what it keeps now.
    nlohmann::json keep_settings(const std::string& browser, const nlohmann::json& changed);

private:
    struct Kept {
        Page page;
        std::chrono::steady_clock::time_point used;
    };
    Service service_;
    Watches watches_;
    std::mutex paging_;
    std::map<std::string, Kept> pages_;
    std::chrono::milliseconds quiet_ = quiet_by_default;
};

/// The reply to one HTTP request: a browser file by its path (the page at
/// `/`), a command's answer at invoke_path. From any thread: commands take
/// turns on the service, except how far a long call is and its Stop.
std::string serve_answer(Server& server, const std::string& request);

/// Where the server listens: `--address host:port`, else the environment's
/// CLARITY_ADDRESS, else `project`, where the project file says (its port).
std::string serve_address(int argc, char** argv, const std::string& project);

/// Runs the application as a web server on `address` until it stops; the exit status.
int serve(const Options& options, const std::string& address);

}  // namespace clarity::shell
