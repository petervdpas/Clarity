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
    /// Runs the handler the answer to a question goes to (DESIGN 5.14),
    /// with the values written for it and the answer last.
    virtual Reply answer(Db& db, const WindowState& state, const std::string& handler, const nlohmann::json& values) = 0;
    /// Draws the canvases of the window `reply` shows anew, each by its draw
    /// handler (conformance/drawing.json). The shell redraws after a window
    /// opens and after every handler.
    virtual Reply redraw(Db& db, Reply reply) = 0;
    /// The records a reference field of `entity` can point at.
    virtual std::vector<RefOption> options(Db& db, const std::string& entity) = 0;
    /// The picture of image field `field` of record `id` of `entity`; none when it has none.
    virtual std::optional<Bytes> picture(Db& db, const std::string& entity, int64_t id, const std::string& field) = 0;
};

/// A desktop window's size in pixels.
struct Size {
    int width = 0;
    int height = 0;
};

/// The size a desktop window opens at: `size`, or the standard size when it says none.
Size window_size(Size size);

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
    /// The size the main window's desktop window opens at; zero for the standard size.
    Size size;
};

/// A desktop window, as the commands of the renderer in it see it.
class Desktop {
public:
    virtual ~Desktop() = default;
    virtual std::string name() const = 0;
    virtual void set_title(const std::string& title) = 0;
    virtual void close() = 0;
};

/// Opens a link in the person's browser; whether it could.
bool open_in_browser(const std::string& address);

/// Answers the renderer's commands.
class Service {
public:
    /// Text code makes is translated from the model's translations.
    explicit Service(Options options) : options_(std::move(options)) { load_translations(options_.model); }

    /// Runs a command: the same commands, with the same arguments, as the
    /// Tauri shell has. Answers `{"value": ...}` or `{"error": ...}`.
    nlohmann::json invoke(const std::string& command, const nlohmann::json& args, Desktop* caller = nullptr);

    /// Ends the program; set by `run`.
    std::function<void()> quit;
    /// Opens a link of a markdown part in the person's browser; whether it could.
    std::function<bool(const std::string& address)> open_link = open_in_browser;
    /// Opens a desktop window of its own, named `name`, for the opener, at
    /// `size` (its window's, or the standard one); set by `run`.
    std::function<void(Desktop& opener, const std::string& name, Size size)> open_desktop;
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
    /// Runs a handler, then draws the window's canvases anew.
    Reply dispatch(const WindowState& state, const std::string& handler);
};

/// Shows the application's window and runs until its last window closes.
int run(const Options& options);

}  // namespace clarity::shell
