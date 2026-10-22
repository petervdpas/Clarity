#include "shell/shell.hpp"

#include <cstdlib>

#if defined(_WIN32)
#include <windows.h>
#include <shellapi.h>
#else
#include <spawn.h>
extern char** environ;
#endif

#include "webview/webview.h"

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
    return options_.windows->redraw(*options_.db, Ui(state).reply(state.window, state.data));
}

Reply Service::dispatch(const WindowState& state, const std::string& handler) {
    return options_.windows->redraw(*options_.db, options_.windows->dispatch(*options_.db, state, handler));
}

json Service::invoke(const std::string& command, const json& args, Desktop* caller) {
    std::lock_guard lock(mutex_);
    Db& db = *options_.db;
    try {
        WindowState state = args.contains("state") ? args["state"].get<WindowState>() : WindowState{};
        json with = args.value("with", json::object());
        if (command == "start") return value({{"model", json::parse(options_.model)}, {"main", options_.main}});
        if (command == "open_window") return value(on_event(options_.windows->open(db, text_of(args, "window"), with), "open"));
        if (command == "close_window") {
            auto reply = on_event(state, "close");
            // The window has closed: what it had of an engine goes.
            window_closed(state.instance);
            return value(reply);
        }
        if (command == "return_to") return value(on_event(state, "return"));
        if (command == "start_app") return value(on_event(WindowState{}, "start"));
        if (command == "dispatch_event") return value(dispatch(state, text_of(args, "handler")));
        if (command == "answer_event") {
            // The answer to a question a handler asked runs the handler it goes to.
            json values = args.contains("values") && args["values"].is_array() ? args["values"] : json::array();
            return value(options_.windows->redraw(db, options_.windows->answer(db, state, text_of(args, "handler"), values)));
        }
        if (command == "ref_options") return value(options_.windows->options(db, text_of(args, "entity")));
        if (command == "picture") {
            // A picture of a record, fetched on its own, as the page shows it.
            auto bytes = options_.windows->picture(db, text_of(args, "entity"), args.value("id", int64_t(0)), text_of(args, "field"));
            return value(bytes ? json(data_url(*bytes)) : json());
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
        if (command == "quit_app") {
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
        if (command == "close_apart") {
            if (caller) caller->close();
            return value(nullptr);
        }
        return {{"error", "there is no command " + command}};
    } catch (const std::exception& e) {
        return failed(e);
    }
}

Size window_size(Size size) {
    if (size.width <= 0 || size.height <= 0) return {1100, 760};
    return size;
}

std::optional<std::string> Service::closed(const std::string& name) {
    std::lock_guard lock(mutex_);
    auto found = apart_.find(name);
    if (found == apart_.end()) return std::nullopt;
    std::string opener = found->second.opener;
    apart_.erase(found);
    return opener;
}

namespace {

/// A desktop window with a webview showing the renderer.
class Window final : public Desktop {
public:
    Window(std::string name, Service& service, const Options& options, Size size) : name_(std::move(name)), view_(false, nullptr) {
        view_.set_title(options.title);
        view_.set_size(size.width, size.height, WEBVIEW_HINT_NONE);
        // The renderer calls clarityInvoke(command, args); it answers like the Go shell.
        view_.bind("clarityInvoke", [this, &service](const std::string& request) {
            json args = json::parse(request);
            return service.invoke(args.at(0).get<std::string>(), args.size() > 1 ? args.at(1) : json::object(), this).dump();
        });
        view_.set_html(options.html);
    }

    std::string name() const override { return name_; }
    void set_title(const std::string& title) override { view_.set_title(title); }

    void close() override {
#if defined(__linux__)
        if (auto native = view_.window(); native.ok()) gtk_window_close(GTK_WINDOW(native.value()));
#endif
    }

    void eval(const std::string& js) { view_.eval(js); }
    webview::webview& view() { return view_; }

private:
    std::string name_;
    webview::webview view_;
};

}  // namespace

int run(const Options& options) {
#if defined(__linux__)
    // WebKitGTK 2.52 paints parts of a new window black through its DMABUF
    // renderer; it must be off before the web process starts.
    setenv("WEBKIT_DISABLE_DMABUF_RENDERER", "1", 0);
#endif
    Service service(options);
    std::map<std::string, std::unique_ptr<Window>> windows;
    windows["main"] = std::make_unique<Window>("main", service, options, window_size(options.size));
    service.quit = [&] { windows["main"]->view().terminate(); };
    service.open_desktop = [&](Desktop&, const std::string& name, Size size) {
        auto& window = windows[name] = std::make_unique<Window>(name, service, options, size);
#if defined(__linux__)
        // When it closes, its opener is told, so it can reload its data.
        struct Closing {
            Service* service;
            std::map<std::string, std::unique_ptr<Window>>* windows;
            std::string name;
        };
        if (auto native = window->view().window(); native.ok()) {
            g_signal_connect_data(
                G_OBJECT(native.value()), "destroy",
                G_CALLBACK(+[](GtkWidget*, gpointer data) {
                    auto* c = static_cast<Closing*>(data);
                    if (auto opener = c->service->closed(c->name); opener && c->windows->contains(*opener))
                        c->windows->at(*opener)->eval("window.dispatchEvent(new Event('clarity-return'))");
                }),
                new Closing{&service, &windows, name}, +[](gpointer data, GClosure*) { delete static_cast<Closing*>(data); }, GConnectFlags(0));
        }
#endif
    };
    windows["main"]->view().run();
    return 0;
}

}  // namespace clarity::shell
