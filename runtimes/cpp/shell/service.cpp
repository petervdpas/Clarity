// The service of the shell: it answers the renderer's commands, for the
// desktop window (shell.cpp) and for a browser (serve.cpp). It needs nothing
// of the desktop toolkit: what belongs to a desktop window it asks of the
// hooks `run` sets, and a web server sets none.

#include "shell/shell.hpp"

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

#include "clarity/settings.hpp"

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
    return options_.windows->redraw(*options_.db, options_.windows->dispatch(*options_.db, state, handler));
}

bool Service::watches(const std::string& command) {
    static const std::set<std::string> watched{"start_app", "open_window", "close_window", "return_to", "dispatch_event", "answer_event"};
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
        if (command == "open_window") return value(on_event(options_.windows->open(db, text_of(args, "window"), with), "open"));
        if (command == "close_window") {
            auto reply = on_event(state, "close");
            // The window has closed: what it had of an engine goes.
            window_closed(state.instance);
            return value(reply);
        }
        if (command == "return_to") return value(on_event(state, "return"));
        if (command == "start_app") return value(on_event(WindowState{}, "start"));
        if (command == "dispatch_event") {
            // One undo step (DESIGN 5.29): the reply carries its id when the handler changed the database.
            int64_t before = db.last_step();
            Reply reply;
            db.step([&] { reply = dispatch(state, text_of(args, "handler")); });
            if (db.last_step() != before) reply.step = db.last_step();
            return value(reply);
        }
        if (command == "undo_step") {
            // A step taken back or done again, on the window's data as the screen put it back.
            int64_t step = args.contains("step") && args["step"].is_number_integer() ? args["step"].get<int64_t>() : 0;
            if (step > 0) {
                if (args.value("back", false)) db.undo(step);
                else db.redo(step);
            }
            return value(drawn(state));
        }
        if (command == "answer_event") {
            // The answer to a question a handler asked runs the handler it goes to.
            json values = args.contains("values") && args["values"].is_array() ? args["values"] : json::array();
            return value(options_.windows->redraw(db, options_.windows->answer(db, state, text_of(args, "handler"), values)));
        }
        if (command == "ref_options") return value(options_.windows->options(db, text_of(args, "entity")));
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
