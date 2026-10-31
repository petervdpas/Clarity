// The shell of C++ applications: a desktop window with a webview showing
// the web renderer every web stack ships, and the commands the renderer
// sends, answered by the program's windows. The commands and their JSON
// are the Tauri and Wails shells', so the same renderer serves every stack.

#pragma once

#include <stdexcept>

#include <atomic>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/watch.hpp"

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
    /// The values text field `field` of `entity` already has, which a typed
    /// dropdown offers (U1c).
    virtual std::vector<std::string> values(Db&, const std::string&, const std::string&) { return {}; }
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
    /// Why the databases did not open (DESIGN 5.27), with `db` an empty one in
    /// memory until `connect` opens them; none when they opened. What the
    /// screen shows first then: the app's `on unconnected` handler, or else
    /// the window that keeps the connection; empty for none.
    std::optional<Text> unconnected;
    std::string unconnected_handler;
    std::string unconnected_window;
    std::function<std::unique_ptr<Db>()> connect;
};

/// A desktop window, as the commands of the renderer in it see it.
class Desktop {
public:
    virtual ~Desktop() = default;
    virtual std::string name() const = 0;
    virtual void set_title(const std::string& title) = 0;
    virtual void close() = 0;
    /// The native window custom parts draw in (DESIGN 6.5): on Linux the
    /// X11 window of the webview; 0 where there is none.
    virtual uintptr_t part_parent() { return 0; }
    /// Opens the system's folder picker over the window, for a folder part:
    /// the folder chosen, or none when the person cancelled. A window
    /// without one says so.
    /// Whether the window has a screen to run scripts on, such as how far a
    /// long call is (DESIGN 5.12), and running one there, from any thread.
    virtual bool shows() const { return false; }
    virtual void show(const std::string& js) { (void)js; }
    virtual std::optional<std::string> choose_folder(const std::string& title) {
        (void)title;
        throw std::runtime_error("this window has no folder picker");
    }
};

/// Opens a link in the person's browser; whether it could.
bool open_in_browser(const std::string& address);
/// Writes a line into the application's log: the standard error.
void log_to_stderr(const std::string& line);

/// The part host's side of the commands of custom parts (DESIGN 6.5), which
/// `run` sets; a web server has none, and the commands do nothing.
struct PartsHost {
    std::function<void(uint64_t window, const std::string& part, const std::string& kind, uintptr_t parent, int x, int y, int width, int height, bool shown)> place;
    std::function<void(uint64_t window, const std::string& part, const std::string& name, const std::string& value)> set;
    std::function<void(uint64_t window, const std::string& part, const std::string& hear, const std::string& values)> hear;
    std::function<void(uint64_t window)> close;
};

/// Answers the renderer's commands.
class Service {
public:
    /// Text code makes is translated from the model's translations.
    explicit Service(Options options) : options_(std::move(options)) { load_translations(options_.model); }

    /// Runs a command: the same commands, with the same arguments, as the
    /// Tauri shell has. Answers `{"value": ...}` or `{"error": ...}`.
    nlohmann::json invoke(const std::string& command, const nlohmann::json& args, Desktop* caller = nullptr);
    /// Whether `command` runs a handler, which may make a long engine call:
    /// the desktop shell runs it off the screen's thread, so the window keeps painting.
    static bool watches(const std::string& command);
    /// Stops the handler that runs at its engine call's next look, from any
    /// thread: the person's Stop (DESIGN 5.12).
    void stop() { stop_->store(true); }

    /// Ends the program; set by `run`.
    std::function<void()> quit;
    /// The part host; set by `run`.
    PartsHost parts;
    /// Opens a link of a markdown part in the person's browser; whether it could.
    std::function<bool(const std::string& address)> open_link = open_in_browser;
    /// Writes a line of an engine beside the screen into the application's
    /// log: the standard error, where the backend's engines write.
    std::function<void(const std::string& line)> log_line = log_to_stderr;
    /// Opens a desktop window of its own, named `name`, for the opener, at
    /// `size` (its window's, or the standard one); set by `run`.
    std::function<void(Desktop& opener, const std::string& name, Size size)> open_desktop;
    /// A desktop window of its own closed: its opener is told, so it can reload its data.
    std::optional<std::string> closed(const std::string& name);
    /// The desktop window an open window's custom parts are in, by the open window's number.
    std::optional<std::string> parts_window(uint64_t instance);
    /// Whether the title bar's close of the main window goes to the screen:
    /// while the databases are not open, closing it connects again (DESIGN 5.27).
    bool closes_to_screen() const { return to_screen_; }

private:
    /// Runs a command, holding the lock.
    nlohmann::json answer(const std::string& command, const nlohmann::json& args, Desktop* caller);
    struct Apart {
        std::string window;
        nlohmann::json with;
        std::string opener;
    };
    Options options_;
    /// The databases `connect` opened, which `options_.db` then points at.
    std::unique_ptr<Db> connected_;
    std::mutex mutex_;
    /// The Stop flag of the handler that runs; each watched handler gets a new one.
    std::shared_ptr<std::atomic<bool>> stop_ = std::make_shared<std::atomic<bool>>(false);
    std::map<std::string, Apart> apart_;
    std::map<uint64_t, std::string> parts_windows_;
    int next_ = 0;
    /// Not behind mutex_: a command that holds it may wait on the desktop's thread, where the close arrives.
    std::atomic<bool> to_screen_ = false;

    Reply on_event(const WindowState& state, const std::string& event);
    /// Draws the window as its data is, running no handler.
    Reply drawn(const WindowState& state);
    /// Runs a handler, then draws the window's canvases anew.
    Reply dispatch(const WindowState& state, const std::string& handler);
    /// Runs a handler as one database step (DESIGN 5.29): the reply carries
    /// the step's id when it changed the database, and `steps` gets it.
    Reply stepped(const WindowState& state, const std::string& handler, std::vector<int64_t>* steps = nullptr);
    /// A change of `part` the runtime made to `state` (DESIGN 1.7), ended as
    /// edit_event ends one: its handler runs as one database step and, with
    /// a scope, the change goes on the window's edits; without a handler the
    /// window is drawn as it is.
    Reply edited(const WindowState& state, const std::string& part, const std::string& scope, const std::string& handler);
};

/// Shows the application's window and runs until its last window closes.
int run(const Options& options);

}  // namespace clarity::shell
