// The binding between a window on the screen and its handlers in the
// backend. The screen sends a window's state with each event; the handler
// runs on it and the reply carries the new state, the messages to show and
// the windows to open or close. The JSON is the other runtimes', so the same
// renderer serves every stack.

#pragma once

#include <map>
#include <cstdint>
#include <memory>
#include <functional>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/errors.hpp"
#include "clarity/text.hpp"

namespace clarity {

/// What a handler changed of a part; unset fields keep the design.
struct PartState {
    std::optional<bool> visible;
    std::optional<bool> enabled;
    std::optional<Text> caption;
    std::optional<Text> error;
    std::optional<bool> readonly;
    /// Whether a foldable panel or form is folded, as the person or a handler
    /// left it; none keeps the design (conformance/folding.json).
    std::optional<bool> folded;
    /// Whether a collapsible sidebar is folded to its button, and whether a
    /// maximizable one fills the window (conformance/sidebars.json).
    std::optional<bool> collapsed;
    // A canvas's grid in its units (0 for none) and whether code snaps to it, as a handler set them.
    std::optional<int64_t> grid;
    std::optional<bool> snap;
    // What a canvas's clear() fills it with, as a handler set it; empty is the theme's panel.
    std::optional<std::string> background;
    // A canvas's grid dots, as a handler set them: pixels across, and color (empty is the theme's border).
    std::optional<int64_t> dot_size;
    std::optional<std::string> dot_color;
    std::optional<bool> maximized;
    /// The open tab of a ribbon, counted from 1 (conformance/ribbons.json).
    std::optional<int64_t> tab;
    /// The step a wizard (a steps part) is on, counted from 1 (conformance/steps.json).
    std::optional<int64_t> step;
    /// The open section of an accordion, counted from 1, 0 for none (conformance/accordions.json).
    std::optional<int64_t> open;
    /// What an unbound text input holds; the screen sends it with each event.
    std::optional<std::string> text;
    /// Whether an unbound checkbox or switch is on.
    std::optional<bool> checked;
    /// A canvas's drawing: its shapes, in order (conformance/drawing.json).
    std::optional<nlohmann::json> drawing;
    /// Where a canvas was last clicked, in its units: x and y, and whatever
    /// else the screen tells about the pointer, kept as it came.
    std::optional<nlohmann::json> pointer;
    /// A custom part's properties by name, as code set them (conformance/custom-parts.json).
    std::optional<nlohmann::json> values;
};

/// A hear a handler gave a custom part: `arena.restart(2)`.
struct Hear {
    std::string part;
    std::string hear;
    nlohmann::json values = nlohmann::json::array();
};

/// One open window: its data by name, its parts' properties and the
/// selected row of each table or list.
struct WindowState {
    std::string window;
    nlohmann::json data = nlohmann::json::object();
    std::map<std::string, PartState> parts;
    std::map<std::string, int> selected;
    /// The number of this open window, its own while it is open: what belongs
    /// to it, such as an engine's state per window, goes by it.
    uint64_t instance = 0;
};

/// Has `listen` hear of every window that closes, by its number, for as long
/// as what it gives back is kept: an engine with state per window drops what
/// it had of it.
std::shared_ptr<void> on_window_closed(std::function<void(uint64_t instance)> listen);

/// Tells what listens that window `instance` has closed, once its own `on close` has run.
void window_closed(uint64_t instance);

/// How a message shows: info goes away by itself.
enum class Level { Info, Warning, Error };

struct Message {
    Level level;
    Text text;
};

/// `open Window with name = value, ...`
struct OpenRequest {
    std::string window;
    nlohmann::json with = nlohmann::json::object();
};

/// `ask ... then handler(values)` (DESIGN 5.14): a question the screen asks
/// in Clarity's own dialog when the handler has finished. Its answer runs
/// `handler` with `values` and, for a value, the answer last.
struct Ask {
    Text question;
    /// What it asks for, as the screen reads it: ask_confirm or ask_value.
    nlohmann::json form;
    std::string handler;
    nlohmann::json values = nlohmann::json::array();
};

/// A question with yes and no (none: OK and Cancel); the handler runs on yes.
nlohmann::json ask_confirm(std::optional<Text> yes, std::optional<Text> no);

/// A question for a value of `type` (text, integer, decimal, money, date,
/// datetime, duration, enum or record), of the enum or entity `of`. `only`
/// narrows the choices to the records' ids (or null); `start` is where a
/// typed value starts (or null).
nlohmann::json ask_value(const std::string& type, std::optional<std::string> of, nlohmann::json only, nlohmann::json start);

/// A dialog window to open with `with` (its data by name); the answer is what it closes with.
nlohmann::json ask_window(const std::string& window, nlohmann::json with);

/// What the screen does after a handler ran.
struct Reply {
    WindowState state;
    std::vector<Message> messages;
    /// The texts the handler put in text values, by the value's name: the
    /// screen fills them in the person's language and format.
    std::map<std::string, Text> shown;
    std::vector<OpenRequest> open;
    bool close = false;
    /// The questions to ask when the handler has finished, in order.
    std::vector<Ask> asks;
    /// What a dialog closes with (`close with value`), for the handler that asked for it.
    std::optional<nlohmann::json> result;
    /// The hears given to custom parts, in order.
    std::vector<Hear> hears;
    /// The text the handler copied (`copy text`), for the screen to put on the clipboard.
    std::optional<Text> copy;
};

/// A record a reference field can point at, as a dropdown shows it.
struct RefOption {
    int64_t id;
    std::string text;
    /// The fields a search dropdown shows of the record (U1c), by name.
    nlohmann::json fields = nlohmann::json::object();
};

/// What a handler can do to the screen.
class Ui {
public:
    explicit Ui(const WindowState& state) : parts_(state.parts), selected_(state.selected), instance_(state.instance) {}

    /// The number of the open window the handler runs in.
    uint64_t instance() const { return instance_; }

    bool visible(const std::string& part) const;
    void set_visible(const std::string& part, bool value) { parts_[part].visible = value; }
    bool enabled(const std::string& part) const;
    void set_enabled(const std::string& part, bool value) { parts_[part].enabled = value; }
    /// The caption a handler set, or `design`.
    std::string caption(const std::string& part, const std::string& design) const;
    void set_caption(const std::string& part, Text value) { parts_[part].caption = std::move(value); }
    /// Starts an empty drawing on a canvas, before its draw handler runs.
    void begin_drawing(const std::string& part) { parts_[part].drawing = nlohmann::json::array(); }
    /// Adds a shape to a canvas's drawing: {"op": "line", ...}.
    void draw(const std::string& part, nlohmann::json shape) {
        auto& drawing = parts_[part].drawing;
        if (!drawing) drawing = nlohmann::json::array();
        drawing->push_back(std::move(shape));
    }
    /// Where a canvas was last clicked, across (`canvas.x`) and down (`canvas.y`); 0 before any click.
    int64_t x(const std::string& part) const { return pointer(part, "x"); }
    int64_t y(const std::string& part) const { return pointer(part, "y"); }
    /// Where a drag on a canvas started: `canvas.fromX`, `canvas.fromY`; 0 without one.
    int64_t from_x(const std::string& part) const { return pointer(part, "fromX"); }
    int64_t from_y(const std::string& part) const { return pointer(part, "fromY"); }
    /// The name of the link a person bent on a canvas (`canvas.bent`, "" without one), and its bend points now (`canvas.via`).
    std::string bent(const std::string& part) const;
    std::vector<int64_t> via(const std::string& part) const;
    /// The message a handler set on an input, or empty text.
    std::string error(const std::string& part) const;
    void set_error(const std::string& part, Text value) { parts_[part].error = std::move(value); }
    /// Whether an input is read-only: as a handler set it, or `design`.
    bool readonly(const std::string& part, bool design) const;
    void set_readonly(const std::string& part, bool value) { parts_[part].readonly = value; }
    /// Whether a foldable part is folded: as the person or a handler left it, or `design`.
    bool folded(const std::string& part, bool design) const;
    void set_folded(const std::string& part, bool value) { parts_[part].folded = value; }
    /// Whether a collapsible sidebar is folded to its button: as left, or `design`.
    bool collapsed(const std::string& part, bool design) const;
    void set_collapsed(const std::string& part, bool value) { parts_[part].collapsed = value; }
    /// A canvas's grid (`canvas.grid`) and whether code snaps to it (`canvas.snap`), as its line says until a handler sets them.
    int64_t grid(const std::string& part, int64_t design) const;
    void set_grid(const std::string& part, int64_t value) { parts_[part].grid = value; }
    bool snap(const std::string& part, bool design) const;
    void set_snap(const std::string& part, bool value) { parts_[part].snap = value; }
    /// What a canvas's clear() fills it with (`canvas.background`), as its line says until a handler sets it.
    std::string background(const std::string& part, const std::string& design) const;
    void set_background(const std::string& part, const std::string& value) { parts_[part].background = value; }
    /// A canvas's grid dots (`canvas.dotSize`, `canvas.dotColor`), as its line says until a handler sets them.
    int64_t dot_size(const std::string& part, int64_t design) const;
    void set_dot_size(const std::string& part, int64_t value) { parts_[part].dot_size = value; }
    std::string dot_color(const std::string& part, const std::string& design) const;
    void set_dot_color(const std::string& part, const std::string& value) { parts_[part].dot_color = value; }
    /// Whether a maximizable sidebar fills the window.
    bool maximized(const std::string& part) const;
    void set_maximized(const std::string& part, bool value) { parts_[part].maximized = value; }
    /// The open tab of a ribbon, counted from 1.
    int64_t tab(const std::string& part) const;
    void set_tab(const std::string& part, int64_t value) { parts_[part].tab = value; }
    /// The step a wizard is on, counted from 1.
    int64_t step(const std::string& part) const;
    void set_step(const std::string& part, int64_t value) { parts_[part].step = value; }
    /// The open section of an accordion, counted from 1; 0 when none is open.
    int64_t open_section(const std::string& part) const;
    void set_open_section(const std::string& part, int64_t value) { parts_[part].open = value; }
    /// The text of an unbound text input.
    std::string text(const std::string& part) const;
    void set_text(const std::string& part, std::string value) { parts_[part].text = std::move(value); }
    /// Whether an unbound checkbox or switch is on.
    bool checked(const std::string& part) const;
    void set_checked(const std::string& part, bool value) { parts_[part].checked = value; }

    /// Forgets the selected row of a table or list, whose rows were read
    /// again or replaced: they are other records.
    void deselect(const std::string& part) { selected_.erase(part); }

    /// The selected record of a table or list showing `rows`, as a copy to work on.
    template <typename R>
    R selected(const std::string& part, const std::vector<R>& rows) const {
        auto found = selected_.find(part);
        if (found == selected_.end() || found->second < 0 || size_t(found->second) >= rows.size())
            throw InvalidError("no row of " + part + " is selected");
        return rows[size_t(found->second)];
    }

    void message(Level level, Text text) { messages_.push_back({level, std::move(text)}); }
    /// `copy text`: the screen puts it, as it shows, on the clipboard when the handler is done; the last copy counts.
    void copy(Text text) { copied_ = std::move(text); }
    /// The text value `data` shows `text`, translated and formatted by the screen.
    void set_shown(const std::string& data, Text text) { shown_[data] = std::move(text); }
    /// Asks the screen to open `window` with its data filled by name, as the values are now.
    void open(const std::string& window, nlohmann::json with) { open_.push_back({window, std::move(with)}); }
    /// Asks the screen to close the window.
    void close() { close_ = true; }
    /// Closes the dialog with `value`, which the handler that asked for it gets.
    void close_with(nlohmann::json value) {
        close_ = true;
        result_ = std::move(value);
    }
    /// Asks `question` when the handler has finished; the answer runs
    /// `handler` with `values` and, for a value, the answer.
    void ask(Text question, nlohmann::json form, std::string handler, nlohmann::json values) {
        asks_.push_back({std::move(question), std::move(form), std::move(handler), std::move(values)});
    }

    /// A property of a custom part as code set it; none when it was not set.
    std::optional<nlohmann::json> part_value(const std::string& part, const std::string& name) const;
    void set_part_value(const std::string& part, const std::string& name, nlohmann::json value) {
        auto& values = parts_[part].values;
        if (!values) values = nlohmann::json::object();
        (*values)[name] = std::move(value);
    }
    /// Gives a custom part a hear with its values, sent with the reply.
    void hear(std::string part, std::string hear, nlohmann::json values) {
        hears_.push_back({std::move(part), std::move(hear), std::move(values)});
    }

    /// The reply for `window`, with its data after the handler.
    Reply reply(const std::string& window, nlohmann::json data) const;

private:
    int64_t pointer(const std::string& part, const char* key) const;
    std::optional<nlohmann::json> result_;
    std::vector<Ask> asks_;
    std::vector<Hear> hears_;
    std::optional<Text> copied_;
    std::map<std::string, PartState> parts_;
    std::map<std::string, int> selected_;
    uint64_t instance_ = 0;
    std::vector<Message> messages_;
    std::map<std::string, Text> shown_;
    std::vector<OpenRequest> open_;
    bool close_ = false;
};

/// The state of `window` when it opens: its loaded data with `with` filled
/// in by name.
WindowState opened(const std::string& window, nlohmann::json data, const nlohmann::json& with);

void to_json(nlohmann::json& out, const PartState& p);
void from_json(const nlohmann::json& in, PartState& p);
void to_json(nlohmann::json& out, const WindowState& s);
void from_json(const nlohmann::json& in, WindowState& s);
void to_json(nlohmann::json& out, const Reply& r);
void to_json(nlohmann::json& out, const RefOption& o);

}  // namespace clarity
