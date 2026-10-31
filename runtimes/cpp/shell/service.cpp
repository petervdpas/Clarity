// The service of the shell: it answers the renderer's commands, for the
// desktop window (shell.cpp) and for a browser (serve.cpp). It needs nothing
// of the desktop toolkit: what belongs to a desktop window it asks of the
// hooks `run` sets, and a web server sets none.

#include "shell/shell.hpp"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <set>

#if defined(_WIN32)
#include <windows.h>
#include <shellapi.h>
#else
#include <spawn.h>
extern char** environ;
#endif

#include "clarity/dropdown_options.hpp"
#include "clarity/edits.hpp"
#include "clarity/list_fields.hpp"
#include "clarity/paths.hpp"
#include "clarity/ranges.hpp"
#include "clarity/reorder.hpp"
#include "clarity/settings.hpp"
#include "clarity/steps.hpp"
#include "clarity/views.hpp"

namespace clarity::shell {

using nlohmann::json;

namespace {

json failed(const std::exception& e) {
    // A refusal keeps its slots, so the screen fills them in the person's format.
    if (auto refused = dynamic_cast<const RefusedError*>(&e)) return {{"error", refused->text}};
    return {{"error", e.what()}};
}

json value(json v) { return {{"value", std::move(v)}}; }

std::string text_of(const json& args, const char* key) { return args.contains(key) && args[key].is_string() ? args[key].get<std::string>() : ""; }

/// A number as text, as a range reads it: none for null or empty text.
std::optional<std::string> number_text(const json& v) {
    if (v.is_string()) return v.get<std::string>().empty() ? std::nullopt : std::optional(v.get<std::string>());
    if (v.is_number()) return v.dump();
    return std::nullopt;
}

std::optional<std::string> number_of(const json& args, const char* key) { return args.contains(key) ? number_text(args[key]) : std::nullopt; }

/// A window's reply, which is where its edits start (DESIGN 5.29): what a
/// handler changed is never the person's edit. Its views show what its
/// lists are now (DESIGN 1.7); `sent`, for a view_event, the chosen rows
/// the screen sent.
json answered(Reply reply, const std::map<std::string, int>* sent = nullptr) {
    fill_views(reply.state, sent);
    edits_of(reply.state.instance)->reset(edit_data(reply.state));
    return value(reply);
}

/// The reply keeps the views of the window a handler ran on, when it gave none of its own.
Reply with_views(Reply reply, const WindowState& state) {
    if (reply.state.views.empty() && reply.state.window == state.window) reply.state.views = state.views;
    return reply;
}

}  // namespace

void log_to_stderr(const std::string& line) { std::fprintf(stderr, "%s\n", line.c_str()); }

bool open_in_browser(const std::string& address) {
#if defined(_WIN32)
    return reinterpret_cast<INT_PTR>(ShellExecuteA(nullptr, "open", address.c_str(), nullptr, nullptr, SW_SHOWNORMAL)) > 32;
#else
#if defined(__APPLE__)
    const char* opener = "open";
#else
    const char* opener = "xdg-open";
#endif
    char* argv[] = {const_cast<char*>(opener), const_cast<char*>(address.c_str()), nullptr};
    pid_t pid;
    return posix_spawnp(&pid, opener, nullptr, nullptr, argv, environ) == 0;
#endif
}

namespace {

/// Whether a link of a markdown part may open: a web or a mail address, as on every stack.
bool link_opens(const std::string& address) {
    return address.rfind("http://", 0) == 0 || address.rfind("https://", 0) == 0 || address.rfind("mailto:", 0) == 0;
}

}  // namespace

Reply Service::on_event(const WindowState& state, const std::string& event) {
    if (auto handler = options_.windows->handler(state.window, event)) return dispatch(state, *handler);
    return drawn(state);
}

Reply Service::drawn(const WindowState& state) { return options_.windows->redraw(*options_.db, Ui(state).reply(state.window, state.data)); }

Reply Service::dispatch(const WindowState& state, const std::string& handler) {
    return options_.windows->redraw(*options_.db, with_views(options_.windows->dispatch(*options_.db, state, handler), state));
}

Reply Service::stepped(const WindowState& state, const std::string& handler, std::vector<int64_t>* steps) {
    Db& db = *options_.db;
    int64_t before = db.last_step();
    Reply reply;
    db.step([&] { reply = dispatch(state, handler); });
    if (db.last_step() != before) {
        reply.step = db.last_step();
        if (steps) steps->push_back(reply.step);
    }
    return reply;
}

Reply Service::edited(const WindowState& state, const std::string& part, const std::string& scope, const std::string& handler) {
    std::vector<int64_t> steps;
    Reply reply = handler.empty() ? drawn(state) : stepped(state, handler, &steps);
    if (!scope.empty()) edits_of(state.instance)->edit(part, scope, edit_data(reply.state), steps);
    return reply;
}

bool Service::watches(const std::string& command) {
    static const std::set<std::string> watched{"start_app", "open_window", "close_window", "return_to", "dispatch_event", "answer_event", "edit_event",
                                                "reorder_event", "item_event", "range_event", "step_event"};
    return watched.contains(command);
}

json Service::invoke(const std::string& command, const json& args, Desktop* caller) {
    // The person stops the handler that runs, which holds the lock.
    if (command == "stop_call") {
        stop();
        return value(nullptr);
    }
    std::lock_guard lock(mutex_);
    if (!watches(command) || !caller || !caller->shows()) return answer(command, args, caller);
    // The handler's engine calls say how far they are: to the screen at most
    // ten times a second, and that it ended once it did.
    stop_ = std::make_shared<std::atomic<bool>>(false);
    std::optional<std::chrono::steady_clock::time_point> last;
    EngineWatch watch{[&](int64_t done, int64_t total) {
                          auto now = std::chrono::steady_clock::now();
                          if (last && now - *last < std::chrono::milliseconds(100) && done < total) return;
                          last = now;
                          caller->show("window.dispatchEvent(new CustomEvent(\"clarity-progress\", {detail: {\"done\":" + std::to_string(done) + ",\"total\":" + std::to_string(total) + "}}))");
                      },
                      stop_};
    json answered;
    {
        Watching watching(watch);
        answered = answer(command, args, caller);
    }
    if (last) caller->show("window.dispatchEvent(new CustomEvent(\"clarity-progress\", {detail: null}))");
    return answered;
}

json Service::answer(const std::string& command, const json& args, Desktop* caller) {
    Db& db = *options_.db;
    try {
        WindowState state = args.contains("state") ? args["state"].get<WindowState>() : WindowState{};
        json with = args.value("with", json::object());
        if (command == "start") {
            json started = {{"model", json::parse(options_.model)}, {"main", options_.main}};
            auto or_null = [](const std::string& text) { return text.empty() ? json(nullptr) : json(text); };
            if (options_.unconnected) {
                started["unconnected"] = {{"handler", or_null(options_.unconnected_handler)}, {"window", or_null(options_.unconnected_window)}, {"problem", json(*options_.unconnected)}};
                to_screen_ = true;
            }
            return value(started);
        }
        // The databases again, as when the person has kept their connection.
        if (command == "connect") {
            if (!options_.connect) return value(nullptr);
            try {
                connected_ = options_.connect();
            } catch (const std::exception& e) {
                return {{"error", json(unconnected_reason(e))}};
            }
            options_.db = connected_.get();
            options_.unconnected.reset();
            to_screen_ = false;
            return value(nullptr);
        }
        // The settings the person kept (DESIGN 5.17); the screen takes back those that still fit.
        if (command == "read_settings") return value(read_settings_with(options_.name, args.value("secrets", json(nullptr))));
        // The person's settings, as the screen has them now.
        if (command == "keep_settings") {
            const auto& settings = args.contains("settings") ? args["settings"] : json(nullptr);
            if (!settings.is_object()) throw InvalidError("keep_settings takes the settings as an object");
            // Its secret settings go to the system's protected store, never the file (DESIGN 5.27).
            keep_settings_with(options_.name, settings, args.value("secrets", json(nullptr)));
            return value(nullptr);
        }
        if (command == "open_window") return answered(on_event(options_.windows->open(db, text_of(args, "window"), with), "open"));
        if (command == "close_window") {
            auto reply = on_event(state, "close");
            // The window has closed: what it had of an engine goes, and its edits.
            window_closed(state.instance);
            edits_closed(state.instance);
            return value(reply);
        }
        if (command == "return_to") return answered(on_event(state, "return"));
        if (command == "start_app") return answered(on_event(WindowState{}, "start"));
        // One undo step (DESIGN 5.29): the reply carries its id when the handler changed the database.
        if (command == "dispatch_event") return answered(stepped(state, text_of(args, "handler")));
        if (command == "edit_event") {
            // The person ended an edit of a part (DESIGN 5.29): its handler runs as
            // one database step, and the edit goes on the window's record with
            // what it changed. A drag goes on until its drop.
            std::string handler = text_of(args, "handler");
            std::vector<int64_t> steps;
            Reply reply = handler.empty() ? drawn(state) : stepped(state, handler, &steps);
            auto bus = edits_of(state.instance);
            if (args.value("ends", false)) bus->edit(text_of(args, "part"), text_of(args, "scope"), edit_data(reply.state), steps);
            else bus->drag(text_of(args, "part"), text_of(args, "scope"), steps);
            return answered(reply);
        }
        // The person's actions the runtime decides (DESIGN 1.7): it changes the
        // window's data, and the change ends as an edit does.
        std::string part = text_of(args, "part");
        std::string scope = text_of(args, "scope");
        std::string handler = text_of(args, "handler");
        if (command == "reorder_event") {
            // A row moved (DESIGN 5.30): the list's new order, each row's number
            // in `by` and, in a tree, its parent; the moved row is chosen.
            std::string list = text_of(args, "list"), by = text_of(args, "by"), parent = text_of(args, "parent");
            json rows = value_at(state.data, list);
            if (!rows.is_array()) throw InvalidError(list + " is no list to reorder");
            bool tree = !parent.empty();
            std::vector<ReorderRow> places;
            for (const auto& row : rows) {
                bool record = row.is_object();
                places.push_back({record ? row.value("id", json(nullptr)) : json(nullptr), tree && record ? row.value(parent, json(nullptr)) : json(nullptr)});
            }
            RowMove move = move_of(args.value("move", json(nullptr)));
            auto moved = reorder(places, move, tree);
            if (!moved) return answered(drawn(state));
            json after = json::array();
            for (size_t i = 0; i < moved->order.size(); ++i) {
                json row = rows[size_t(moved->order[i])];
                if (row.is_object() && !by.empty()) row[by] = moved->numbers[i];
                if (row.is_object() && tree) row[parent] = moved->parents[i];
                after.push_back(std::move(row));
            }
            WindowState changed = state;
            place_at(changed.data, list) = std::move(after);
            auto at = std::find(moved->order.begin(), moved->order.end(), move.row);
            changed.selected[part] = int(at - moved->order.begin());
            return answered(edited(changed, part, scope, handler));
        }
        if (command == "item_event") {
            // A typed item of a list field (U1c): added, or refused with the list as it was.
            std::string path = text_of(args, "path");
            json before = value_at(state.data, path);
            if (!before.is_array()) before = json::array();
            auto added = add_typed(text_of(args, "item"), before, text_of(args, "typed"), args.value("values", json::array()));
            if (added.refused || added.items == before) {
                Reply reply = drawn(state);
                reply.refused = added.refused;
                return answered(reply);
            }
            WindowState changed = state;
            place_at(changed.data, path) = added.items;
            return answered(edited(changed, part, scope, handler));
        }
        if (command == "range_event") {
            // A slider's or spin box's value after a key, a drag to a place or typed text.
            std::string path = text_of(args, "path");
            NumberRange range{number_of(args, "min"), number_of(args, "max"), number_of(args, "step").value_or("1")};
            json before = value_at(state.data, path);
            json now;
            if (args.contains("key")) {
                now = range_value(keyed(number_text(before), range, text_of(args, "key")), args.value("whole", false));
            } else if (args.contains("at")) {
                now = range_value(dragged(number_of(args, "at").value_or("0"), range), args.value("whole", false));
            } else if (args.contains("typed")) {
                // Nothing typed is no value.
                auto typed = number_of(args, "typed");
                now = typed ? range_value(typed_in(*typed, range), args.value("whole", false)) : json(nullptr);
            } else {
                throw InvalidError("range_event takes a key, a place (at) or typed text");
            }
            if (now == before) return answered(drawn(state));
            WindowState changed = state;
            place_at(changed.data, path) = std::move(now);
            return answered(edited(changed, part, scope, handler));
        }
        if (command == "step_event") {
            // Next, Back or Finish on a wizard (DESIGN 5.15): Next runs on next on
            // the step shown and moves on, unless it refused or set the step itself.
            std::string button = text_of(args, "button");
            int64_t count = args.value("steps", int64_t(1));
            WindowState at = state;
            auto& step = at.parts[part].step;
            int64_t shown = shown_step(step, count);
            if (button == "back") {
                step = step_after_back(step, count);
                return answered(drawn(at));
            }
            if (button == "finish") {
                std::string finish = text_of(args, "finish");
                return answered(finish.empty() ? drawn(at) : stepped(at, finish));
            }
            if (button != "next") throw InvalidError("a wizard has no button " + button);
            step = shown;
            std::string next = text_of(args, "next");
            Reply reply = next.empty() ? drawn(at) : stepped(at, next);
            auto& left = reply.state.parts[part].step;
            left = step_after_next(shown, left, count);
            return answered(reply);
        }
        if (command == "view_event") {
            // The person sorted, turned a page or opened a tree row (DESIGN 1.7):
            // the part's view changes and the window is answered. Never an edit.
            WindowState changed = state;
            view_action(changed, part, args.value("action", json::object()));
            return answered(drawn(changed), &state.selected);
        }
        if (command == "undo_edit") {
            // Ctrl+Z (back) or Ctrl+Y on a scope: its last edit taken back or done
            // again, with its database steps, and the window as it was.
            auto data = edits_of(state.instance)->undo(text_of(args, "scope"), edit_data(state), args.value("back", false), [&](const std::vector<int64_t>& steps, bool back) {
                for (int64_t step : steps) {
                    if (back) db.undo(step);
                    else db.redo(step);
                }
            });
            return answered(drawn(data ? with_edit_data(state, *data) : state));
        }
        if (command == "answer_event") {
            // The answer to a question a handler asked runs the handler it goes to.
            json values = args.contains("values") && args["values"].is_array() ? args["values"] : json::array();
            return answered(options_.windows->redraw(db, with_views(options_.windows->answer(db, state, text_of(args, "handler"), values), state)));
        }
        if (command == "ref_options") return value(options_.windows->options(db, text_of(args, "entity")));
        if (command == "find_options") {
            // What a dropdown offers for typed text (U1c): a typed one the values
            // its field has, a search one the records with the fields it shows.
            std::string kind = text_of(args, "kind"), entity = text_of(args, "entity"), typed = text_of(args, "typed");
            if (kind == "typed") return value(suggestions(options_.windows->values(db, entity, text_of(args, "field")), typed));
            if (kind == "search") {
                std::vector<std::string> show;
                if (args.contains("show") && args["show"].is_array())
                    for (const auto& name : args["show"])
                        if (name.is_string()) show.push_back(name);
                return value(searched(options_.windows->options(db, entity), typed, show));
            }
            return {{"error", "a dropdown finds its options typed or by search, not " + kind}};
        }
        if (command == "field_values") return value(options_.windows->values(db, text_of(args, "entity"), text_of(args, "field")));
        if (command == "picture") {
            // A picture of a record, fetched on its own, as the page shows it.
            auto bytes = options_.windows->picture(db, text_of(args, "entity"), args.value("id", int64_t(0)), text_of(args, "field"));
            return value(bytes ? json(data_url(*bytes)) : json());
        }
        if (command == "file_data") {
            // The file of a file field of a record, fetched on its own, as base64.
            auto bytes = options_.windows->picture(db, text_of(args, "entity"), args.value("id", int64_t(0)), text_of(args, "field"));
            return value(bytes ? json(base64(*bytes)) : json());
        }
        if (command == "open_file") {
            // Opens a file in the program the system uses for its kind, from a temporary copy.
            std::string file = temporary_copy(text_of(args, "name"), from_base64(text_of(args, "data")));
            if (!open_link || !open_link(file)) return {{"error", "cannot open " + file}};
            return value(nullptr);
        }
        if (command == "save_file") {
            // Saves a copy of a file in the person's Downloads folder; where it went.
            return value(save_copy(text_of(args, "name"), from_base64(text_of(args, "data"))));
        }
        if (command == "store_file") {
            // A picture linked from a file of a store, before its record is saved.
            return value(data_url(db.store_file(text_of(args, "store"), text_of(args, "path"))));
        }
        if (command == "set_language") {
            // The language the screen picked, which text code makes is translated to.
            set_language(args.contains("language") && args["language"].is_string() ? std::optional(args["language"].get<std::string>()) : std::nullopt);
            return value(nullptr);
        }
        if (command == "choose_folder") {
            // A folder part's button: the system's folder picker over the window it is in.
            if (!caller) return json{{"error", "this window has no folder picker"}};
            auto folder = caller->choose_folder(text_of(args, "title"));
            return value(folder ? json(*folder) : json(nullptr));
        }
        if (command == "set_title") {
            if (caller) caller->set_title(text_of(args, "title"));
            return value(nullptr);
        }
        if (command == "open_link") {
            std::string address = text_of(args, "address");
            if (!link_opens(address)) return {{"error", "a link opens only an http, https or mailto address, not " + address}};
            if (!open_link || !open_link(address)) return {{"error", "cannot open " + address}};
            return value(nullptr);
        }
        if (command == "log_line") {
            if (log_line) log_line(text_of(args, "line"));
            return value(nullptr);
        }
        if (command == "quit_app") {
            to_screen_ = false;
            if (quit) quit();
            return value(nullptr);
        }
        if (command == "open_apart") {
            if (!caller || !open_desktop) return {{"error", "a window of its own needs the desktop"}};
            std::string name = "apart-" + std::to_string(++next_);
            apart_[name] = {text_of(args, "window"), with, caller->name()};
            open_desktop(*caller, name, window_size({args.value("width", 0), args.value("height", 0)}));
            return value(nullptr);
        }
        if (command == "apart_start") {
            auto found = caller ? apart_.find(caller->name()) : apart_.end();
            if (found == apart_.end()) return value(nullptr);
            return value({{"window", found->second.window}, {"with", found->second.with}});
        }
        // Custom parts (DESIGN 6.5): where each is and what it is given, for the part host.
        if (command == "place_part") {
            uint64_t instance = args.value("instance", uint64_t(0));
            if (caller) parts_windows_[instance] = caller->name();
            if (parts.place) parts.place(instance, text_of(args, "part"), text_of(args, "kind"), caller ? caller->part_parent() : 0, args.value("x", 0), args.value("y", 0),
                         args.value("width", 0), args.value("height", 0), args.value("shown", false));
            return value(nullptr);
        }
        if (command == "set_part") {
            if (parts.set) parts.set(args.value("instance", uint64_t(0)), text_of(args, "part"), text_of(args, "name"), args.contains("value") ? args["value"].dump() : "null");
            return value(nullptr);
        }
        if (command == "hear_part") {
            if (parts.hear) parts.hear(args.value("instance", uint64_t(0)), text_of(args, "part"), text_of(args, "hear"), args.contains("values") ? args["values"].dump() : "[]");
            return value(nullptr);
        }
        if (command == "close_parts") {
            uint64_t instance = args.value("instance", uint64_t(0));
            if (parts.close) parts.close(instance);
            parts_windows_.erase(instance);
            return value(nullptr);
        }
        if (command == "close_apart") {
            if (caller) caller->close();
            return value(nullptr);
        }
        return {{"error", "there is no command " + command}};
    } catch (const std::exception& e) {
        return failed(e);
    } catch (...) {
        // Fail safe: whatever else a handler throws is an error the screen shows.
        return {{"error", "the handler stopped"}};
    }
}

Size window_size(Size size) {
    if (size.width <= 0 || size.height <= 0) return {1100, 760};
    return size;
}

std::optional<std::string> Service::parts_window(uint64_t instance) {
    std::lock_guard lock(mutex_);
    auto found = parts_windows_.find(instance);
    if (found == parts_windows_.end()) return std::nullopt;
    return found->second;
}

std::optional<std::string> Service::closed(const std::string& name) {
    std::lock_guard lock(mutex_);
    auto found = apart_.find(name);
    if (found == apart_.end()) return std::nullopt;
    std::string opener = found->second.opener;
    apart_.erase(found);
    return opener;
}

}  // namespace clarity::shell
