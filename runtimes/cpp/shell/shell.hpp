// The shell of C++ applications: a desktop window with a webview showing
// the web renderer every web stack ships, and the commands the renderer
// sends, answered by the program's windows. The commands and their JSON
// are the Tauri and Wails shells', so the same renderer serves every stack.

#pragma once

#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/db.hpp"
#include "clarity/ui.hpp"

namespace clarity::shell {

/// What the shell asks of the program's windows.
class Windows {
public:
    virtual ~Windows() = default;
    /// The state of `window` when it opens: its data loaded, with `with` filled in by name.
    virtual WindowState open(Db& db, const std::string& window, const nlohmann::json& with) = 0;
    /// The handler of a window's own event (open, close, return, start); the app's start has no window.
    virtual std::optional<std::string> handler(const std::string& window, const std::string& event) = 0;
    /// Runs a handler on a window's state.
    virtual Reply dispatch(Db& db, const WindowState& state, const std::string& handler) = 0;
    /// The records a reference field of `entity` can point at.
    virtual std::vector<RefOption> options(Db& db, const std::string& entity) = 0;
    /// The picture of image field `field` of record `id` of `entity`; none when it has none.
    virtual std::optional<Bytes> picture(Db& db, const std::string& entity, int64_t id, const std::string& field) = 0;
};

struct Options {
    /// The program's name; `title` the title of its window until a Clarity window gives its own.
    std::string name;
    std::string title;
    /// The window the application opens with.
    std::string main;
    /// The windows, frame and data dictionary the renderer draws from, as JSON.
    std::string model;
    /// The renderer's page, with its script, styles and pictures in it.
    std::string html;
    Db* db = nullptr;
    Windows* windows = nullptr;
};

/// A desktop window, as the commands of the renderer in it see it.
class Desktop {
public:
    virtual ~Desktop() = default;
    virtual std::string name() const = 0;
    virtual void set_title(const std::string& title) = 0;
    virtual void close() = 0;
};

/// Answers the renderer's commands.
class Service {
public:
    explicit Service(Options options) : options_(std::move(options)) {}

    /// Runs a command: the same commands, with the same arguments, as the
    /// Tauri shell has. Answers `{"value": ...}` or `{"error": ...}`.
    nlohmann::json invoke(const std::string& command, const nlohmann::json& args, Desktop* caller = nullptr);

    /// Ends the program; set by `run`.
    std::function<void()> quit;
    /// Opens a desktop window of its own, named `name`, for the opener; set by `run`.
    std::function<void(Desktop& opener, const std::string& name)> open_desktop;
    /// A desktop window of its own closed: its opener is told, so it can reload its data.
    std::optional<std::string> closed(const std::string& name);

private:
    struct Apart {
        std::string window;
        nlohmann::json with;
        std::string opener;
    };
    Options options_;
    std::mutex mutex_;
    std::map<std::string, Apart> apart_;
    int next_ = 0;

    Reply on_event(const WindowState& state, const std::string& event);
};

/// Shows the application's window and runs until its last window closes.
int run(const Options& options);

}  // namespace clarity::shell
