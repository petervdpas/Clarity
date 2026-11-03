#include "shell/desktop.hpp"

#include <cstdio>
#include <set>

#include "clarity/watch.hpp"

namespace clarity::shell {

using nlohmann::json;

struct DesktopProgram::Watch {
    /// How far the call is; done is -1 until an engine said.
    std::atomic<int64_t> done{-1};
    std::atomic<int64_t> total{0};
    std::shared_ptr<std::atomic<bool>> stop = std::make_shared<std::atomic<bool>>(false);
    EngineWatch engine;
};

namespace {

/// The watch the handler on this thread runs under, from start_watch to its end_watch.
struct OnThread {
    uint64_t id = 0;
    std::shared_ptr<void> kept;
    std::unique_ptr<Watching> watching;
};
thread_local OnThread on_thread;

json value(json v) { return {{"value", std::move(v)}}; }

}  // namespace

DesktopProgram::DesktopProgram(Options options) : service_(std::move(options)) {}

std::shared_ptr<DesktopProgram::Watch> DesktopProgram::watch(uint64_t id) {
    std::lock_guard lock(watches_mutex_);
    auto& found = watches_[id];
    if (!found) found = std::make_shared<Watch>();
    return found;
}

json DesktopProgram::watching(const std::string& command, uint64_t id) {
    if (command == "watches") return value(service_.options().watches);
    if (command == "start_watch") {
        // On the handler's thread, just before it runs: its engine calls report here.
        auto w = watch(id);
        Watch* raw = w.get();
        w->engine = EngineWatch{[raw](int64_t done, int64_t total) {
                                    raw->total = total;
                                    raw->done = done;
                                },
                                w->stop};
        on_thread.watching.reset();
        on_thread.id = id;
        on_thread.kept = w;
        on_thread.watching = std::make_unique<Watching>(w->engine);
        return value(nullptr);
    }
    if (command == "watch_progress") {
        std::shared_ptr<Watch> w;
        {
            std::lock_guard lock(watches_mutex_);
            auto found = watches_.find(id);
            if (found != watches_.end()) w = found->second;
        }
        if (!w || w->done < 0) return value(nullptr);
        return value(json::array({w->done.load(), w->total.load()}));
    }
    if (command == "stop_watch") {
        // It may come before its start_watch: it holds once the call starts.
        watch(id)->stop->store(true);
        return value(nullptr);
    }
    // end_watch: on the handler's thread, and once more from the screen's.
    if (on_thread.kept && on_thread.id == id) {
        on_thread.watching.reset();
        on_thread.kept.reset();
        on_thread.id = 0;
    }
    std::lock_guard lock(watches_mutex_);
    watches_.erase(id);
    return value(nullptr);
}

std::optional<std::string> DesktopProgram::call(std::string_view command_view, std::string_view args_text) {
    std::string command(command_view);
    json args;
    try {
        args = args_text.empty() ? json::object() : json::parse(args_text);
    } catch (const std::exception& e) {
        return json{{"error", "the library sent " + command + " with JSON that does not read: " + e.what()}}.dump();
    }
    if (!args.is_object()) args = json::object();
    static const std::set<std::string> watch_commands{"watches", "start_watch", "watch_progress", "stop_watch", "end_watch"};
    if (watch_commands.contains(command)) return watching(command, args.value("id", uint64_t(0))).dump();
    // Engines beside the screen run in the program, each canvas's by its number.
    if (command.rfind("engine_", 0) == 0) {
        if (!service_.options().screen_engines) return std::nullopt;
        auto answered = service_.options().screen_engines(command, args);
        if (!answered) return std::nullopt;
        return answered->dump();
    }
    if (command == "facts") {
        if (service_.options().facts.empty()) return std::nullopt;
        return value(json::parse(service_.options().facts)).dump();
    }
    if (command == "started") {
        json started = service_.invoke("start", json::object());
        if (!started.contains("value")) return started.dump();
        started["value"].erase("model");
        if (!started["value"].contains("unconnected")) started["value"]["unconnected"] = nullptr;
        return started.dump();
    }
    // A record's file as base64: the service's file_data.
    if (command == "field_bytes") return service_.invoke("file_data", args).dump();
    json answered = service_.invoke(command, args);
    // A store's file: base64, without the data URL the web screen shows.
    if (command == "store_file" && answered.contains("value") && answered["value"].is_string()) {
        std::string url = answered["value"];
        auto comma = url.find(',');
        if (url.rfind("data:", 0) == 0 && comma != std::string::npos) answered["value"] = url.substr(comma + 1);
    }
    // A command the service does not know takes the library's default.
    if (answered == json{{"error", "there is no command " + command}}) return std::nullopt;
    return answered.dump();
}

ui::Files desktop_files(const Options& options) {
    ui::Files files;
    for (const UiFile* file = options.ui_files; file && file->kind; ++file) {
        std::string_view kind = file->kind;
        std::string_view bytes(reinterpret_cast<const char*>(file->bytes), file->size);
        if (kind == "window") files.window(file->name, bytes);
        else if (kind == "app") files.app(file->name, bytes);
        else if (kind == "theme") files.theme(file->name, bytes);
        else if (kind == "resource") files.resource(file->name, bytes);
    }
    files.model(options.model.empty() ? "{}" : options.model);
    files.title(options.title);
    files.size(options.size.width, options.size.height);
    return files;
}

int run(const Options& options) {
    try {
        if (!ui::fits()) {
            std::fprintf(stderr, "%s: Clarity.UI %s does not fit this program; build it against the library's own clarity_ui.h\n", options.name.c_str(), ui::version().c_str());
            return 1;
        }
        if (options.register_parts) options.register_parts();
        DesktopProgram program(options);
        ui::Files files = desktop_files(options);
        return ui::run_desktop(program, files);
    } catch (const std::exception& e) {
        std::fprintf(stderr, "%s: %s\n", options.name.c_str(), e.what());
        return 1;
    }
}

}  // namespace clarity::shell
