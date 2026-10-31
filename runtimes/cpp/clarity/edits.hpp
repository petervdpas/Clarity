// The edit bus (DESIGN 5.29): per open window, the record of the person's
// edits, kept here in the runtime, not in the screen. Each edit is a message
// with the part, its scope (its form or panel, or the part itself), what it
// changed (each name of the window's data, or #parts.<name> for a part's own
// text or tick, with its value before and after) and the database steps its
// handler made. The screen sends an edit when the person ends one, and Ctrl+Z
// or Ctrl+Y as an undo; the runtime takes the steps back or does them again
// and answers with the window's state as it was. Every runtime keeps the
// record alike (conformance/edit-bus.json).

#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/ui.hpp"

namespace clarity {

/// One name an edit changed, with its value before and after; none when
/// the name was not there.
struct Changed {
    std::string name;
    std::optional<nlohmann::json> before;
    std::optional<nlohmann::json> after;
};

/// One edit of the person's.
struct Edit {
    std::string part;
    std::string scope;
    std::vector<Changed> changes;
    std::vector<int64_t> steps;
};

/// Takes database steps back (`back`) or does them again, in the order
/// given; it throws when the database refuses.
using TakeSteps = std::function<void(const std::vector<int64_t>& steps, bool back)>;

/// The record of one window's edits.
class EditBus {
public:
    /// The window's data as a reply that is no edit left it: the edits start
    /// from here. While an edit goes on (a drag), it waits for its end.
    void reset(const nlohmann::json& data);
    /// A copy of the window's data where the next edit starts: what a change
    /// the runtime refuses is taken back to; none before any reply.
    std::optional<nlohmann::json> baseline();
    /// An edit that goes on: `steps` are the database steps its handler
    /// made; it ends with edit.
    void drag(const std::string& part, const std::string& scope, const std::vector<int64_t>& steps);
    /// The end of an edit of `part`: `data` is the window's data now, with
    /// what its handler changed, `steps` the database steps that handler
    /// made. One that changed nothing and made no step is no message; a new
    /// edit ends what could be done again in its scope.
    void edit(const std::string& part, const std::string& scope, const nlohmann::json& data, const std::vector<int64_t>& steps);
    /// Takes back (`back`) or does again the last edit of `scope`, on `data`,
    /// the window's data now: the window's data after it, or none when the
    /// scope has no edit. When `take` throws, the edit stays where it was.
    std::optional<nlohmann::json> undo(const std::string& scope, const nlohmann::json& data, bool back, const TakeSteps& take);

private:
    std::mutex mutex_;
    nlohmann::json baseline_ = nlohmann::json::object();
    bool based_ = false;
    std::optional<Edit> going_;
    std::vector<Edit> done_;
    std::vector<Edit> undone_;
};

/// Whether a table's edit, its list before and after, kept to its `editable`
/// fields: the same rows in the same order, every other field as it was (a
/// new or a removed field is a change of it).
bool table_edit_fits(const nlohmann::json& before, const nlohmann::json& after, const std::vector<std::string>& editable);

/// The edit bus of the window numbered `instance`.
std::shared_ptr<EditBus> edits_of(uint64_t instance);
/// Forgets the edits of a window that closed.
void edits_closed(uint64_t instance);

/// What an edit of a window compares: its data, and each part's own text
/// and tick as #parts.<name>.
nlohmann::json edit_data(const WindowState& state);
/// `state` with `data` as an edit put it back: the window's data, and each
/// part's own text and tick.
WindowState with_edit_data(WindowState state, const nlohmann::json& data);

}  // namespace clarity
