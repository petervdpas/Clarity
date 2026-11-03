// The service of C++ applications: the commands of Clarity's screen (the
// web server's wire, DESIGN 7.3), answered by the program's windows. The
// desktop program on Clarity.UI (desktop.hpp) and the web server
// (serve.hpp) both answer through it.

#pragma once

#include <cstddef>
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

/// What the service asks of the program's windows.
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
    /// handler (conformance/drawing.json). The service redraws after a
    /// window opens and after every handler.
    virtual Reply redraw(Db& db, Reply reply) = 0;
    /// The records a reference field of `entity` can point at.
    virtual std::vector<RefOption> options(Db& db, const std::string& entity) = 0;
    /// The values text field `field` of `entity` already has, which a typed
    /// dropdown offers (U1c).
    virtual std::vector<std::string> values(Db&, const std::string&, const std::string&) { return {}; }
    /// The picture of image field `field` of record `id` of `entity`; none when it has none.
    virtual std::optional<Bytes> picture(Db& db, const std::string& entity, int64_t id, const std::string& field) = 0;
};

/// A window's size in the screen's units.
struct Size {
    int width = 0;
    int height = 0;
};

/// The size a window opens at: `size`, or the standard size when it says none.
Size window_size(Size size);

/// A file the desktop program carries for Clarity.UI (DESIGN 7.9): kind
/// "window", "app", "theme" or "resource", its name and its bytes. A list
/// of them ends with one whose kind is null.
struct UiFile {
    const char* kind;
    const char* name;
    const unsigned char* bytes;
    size_t size;
};

struct Options {
    /// The program's name; `title` the title of its main window.
    std::string name;
    std::string title;
    /// The window the application opens with.
    std::string main;
    /// The windows, frame and data dictionary the screen draws from, as JSON.
    std::string model;
    /// The web screen's page, with its script, styles and pictures in it (the web target).
    std::string html;
    Db* db = nullptr;
    Windows* windows = nullptr;
    /// The size the main window opens at; zero for the standard size.
    Size size;
    /// Why the databases did not open (DESIGN 5.27), with `db` an empty one in
    /// memory until `connect` opens them; none when they opened. What the
    /// screen shows first then: the app's `on unconnected` handler, or else
    /// the window that keeps the connection; empty for none.
    std::optional<Text> unconnected;
    std::string unconnected_handler;
    std::string unconnected_window;
    std::function<std::unique_ptr<Db>()> connect;
    /// The desktop's files (windows and app frames as markup, themes,
    /// resources), ended by one without a kind; none on the web.
    const UiFile* ui_files = nullptr;
    /// What the program knows of its fields and types, as JSON (the
    /// runtime's facts, DESIGN 7.9); empty for none.
    std::string facts;
    /// Whether an engine says how far a long call is (DESIGN 5.12): the
    /// desktop then runs handlers on a worker thread with Stop.
    bool watches = false;
    /// Registers the program's custom parts with Clarity.UI; none without.
    std::function<void()> register_parts;
    /// Engines beside the screen on the desktop (DESIGN 5.12): answers
    /// Clarity.UI's engine commands (engine_open, engine_hears, engine_frame,
    /// engine_close), from several threads; none for no engine.
    std::function<std::optional<nlohmann::json>(const std::string& command, const nlohmann::json& args)> screen_engines;
};

/// Opens a link in the person's browser; whether it could.
bool open_in_browser(const std::string& address);
/// Writes a line into the application's log: the standard error.
void log_to_stderr(const std::string& line);

/// Answers the screen's commands, one at a time.
class Service {
public:
    /// Text code makes is translated from the model's translations.
    explicit Service(Options options) : options_(std::move(options)) { load_translations(options_.model); }

    /// Runs a command of the web server's wire. Answers `{"value": ...}` or `{"error": ...}`.
    nlohmann::json invoke(const std::string& command, const nlohmann::json& args);

    /// Opens a link of a markdown part in the person's browser; whether it could.
    std::function<bool(const std::string& address)> open_link = open_in_browser;
    /// Writes a line of an engine beside the screen into the application's
    /// log: the standard error, where the backend's engines write.
    std::function<void(const std::string& line)> log_line = log_to_stderr;

    const Options& options() const { return options_; }

private:
    /// Runs a command, holding the lock.
    nlohmann::json answer(const std::string& command, const nlohmann::json& args);
    Options options_;
    /// The databases `connect` opened, which `options_.db` then points at.
    std::unique_ptr<Db> connected_;
    std::mutex mutex_;

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
    /// What the person changed, refused (DESIGN 1.7): the window's data as
    /// its edit baseline had it, no handler run, no edit kept.
    Reply taken_back(const WindowState& state);
};

}  // namespace clarity::shell
