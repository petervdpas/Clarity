// The binding between a window on the screen and its handlers in the
// backend. The screen sends a window's state with each event; the handler
// runs on it and the reply carries the new state, the messages to show and
// the windows to open or close. The JSON is the other runtimes', so the same
// renderer serves every stack.

#pragma once

#include <map>
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
    /// What an unbound text input holds; the screen sends it with each event.
    std::optional<std::string> text;
    /// Whether an unbound checkbox or switch is on.
    std::optional<bool> checked;
};

/// One open window: its data by name, its parts' properties and the
/// selected row of each table or list.
struct WindowState {
    std::string window;
    nlohmann::json data = nlohmann::json::object();
    std::map<std::string, PartState> parts;
    std::map<std::string, int> selected;
};

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

/// What the screen does after a handler ran.
struct Reply {
    WindowState state;
    std::vector<Message> messages;
    std::vector<OpenRequest> open;
    bool close = false;
};

/// A record a reference field can point at, as a dropdown shows it.
struct RefOption {
    int64_t id;
    std::string text;
};

/// What a handler can do to the screen.
class Ui {
public:
    explicit Ui(const WindowState& state) : parts_(state.parts), selected_(state.selected) {}

    bool visible(const std::string& part) const;
    void set_visible(const std::string& part, bool value) { parts_[part].visible = value; }
    bool enabled(const std::string& part) const;
    void set_enabled(const std::string& part, bool value) { parts_[part].enabled = value; }
    /// The caption a handler set, or `design`.
    std::string caption(const std::string& part, const std::string& design) const;
    void set_caption(const std::string& part, Text value) { parts_[part].caption = std::move(value); }
    /// The message a handler set on an input, or empty text.
    std::string error(const std::string& part) const;
    void set_error(const std::string& part, Text value) { parts_[part].error = std::move(value); }
    /// Whether an input is read-only: as a handler set it, or `design`.
    bool readonly(const std::string& part, bool design) const;
    void set_readonly(const std::string& part, bool value) { parts_[part].readonly = value; }
    /// The text of an unbound text input.
    std::string text(const std::string& part) const;
    void set_text(const std::string& part, std::string value) { parts_[part].text = std::move(value); }
    /// Whether an unbound checkbox or switch is on.
    bool checked(const std::string& part) const;
    void set_checked(const std::string& part, bool value) { parts_[part].checked = value; }

    /// The selected record of a table or list showing `rows`, as a copy to work on.
    template <typename R>
    R selected(const std::string& part, const std::vector<R>& rows) const {
        auto found = selected_.find(part);
        if (found == selected_.end() || found->second < 0 || size_t(found->second) >= rows.size())
            throw InvalidError("no row of " + part + " is selected");
        return rows[size_t(found->second)];
    }

    void message(Level level, Text text) { messages_.push_back({level, std::move(text)}); }
    /// Asks the screen to open `window` with its data filled by name, as the values are now.
    void open(const std::string& window, nlohmann::json with) { open_.push_back({window, std::move(with)}); }
    /// Asks the screen to close the window.
    void close() { close_ = true; }

    /// The reply for `window`, with its data after the handler.
    Reply reply(const std::string& window, nlohmann::json data) const;

private:
    std::map<std::string, PartState> parts_;
    std::map<std::string, int> selected_;
    std::vector<Message> messages_;
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
