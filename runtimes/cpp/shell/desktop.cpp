#include "shell/desktop.hpp"

#include <cstdio>
#include <set>

#include "clarity/splash.hpp"
#include "clarity/text.hpp"
#include "clarity/watch.hpp"

namespace clarity::shell {

using nlohmann::json;

namespace {

json value(json v) { return {{"value", std::move(v)}}; }

}  // namespace

DesktopProgram::DesktopProgram(Options options) : service_(std::move(options)) {}

std::optional<std::string> DesktopProgram::call(std::string_view command_view, std::string_view args_text) {
    std::string command(command_view);
    json args;
    try {
        args = args_text.empty() ? json::object() : json::parse(args_text);
    } catch (const std::exception& e) {
        return json{{"error", "the library sent " + command + " with JSON that does not read: " + e.what()}}.dump();
    }
    if (!args.is_object()) args = json::object();
    if (command == "watches") return value(service_.options().watches).dump();
    // The commands AI agents may call while the app runs (docs/ledger/app-mcp.md).
    if (command == "agents" || command == "agent_call" || command == "agent_undo") {
        const auto& options = service_.options();
        if (options.agents.is_null() || !options.agent_call) return std::nullopt;
        if (command == "agents") return value(options.agents).dump();
        if (command == "agent_undo") {
            try {
                options.db->replay_steps({args.value("step", int64_t{0})}, true);
                return value(nullptr).dump();
            } catch (const std::exception& e) {
                return json{{"error", e.what()}}.dump();
            }
        }
        // One database step: all of the call's changes, or none when it is refused; Undo takes the step back.
        const auto line = args.value("line", std::vector<std::string>{});
        struct AgentRefused {};
        std::pair<std::string, bool> ran{"", true};
        int64_t step = 0;
        const int64_t before = options.db->last_step();
        try {
            options.db->step([&] {
                ran = options.agent_call(line, ui::agent_ask);
                if (ran.second) throw AgentRefused{};
            });
            if (options.db->last_step() != before) step = options.db->last_step();
        } catch (const AgentRefused&) {
        } catch (const std::exception& e) {
            ran = {e.what(), true};
        }
        return value(json{{"text", ran.first}, {"refused", ran.second}, {"step", step}}).dump();
    }
    // Asked from the screen's thread while the app's on start runs.
    if (command == "splash_status") {
        auto said = clarity::splash::status();
        return value(said ? json(*said) : json(nullptr)).dump();
    }
    if (Watches::is_command(command)) return watches_.answer(command, args.value("id", uint64_t(0))).dump();
    // Engines beside the screen run in the program, each canvas's by its number.
    if (command.rfind("engine_", 0) == 0) {
        if (!service_.options().screen_engines) return std::nullopt;
        auto answered = service_.options().screen_engines(command, args);
        if (!answered) return std::nullopt;
        return answered->dump();
    }
    return answered(command, args);
}

json DesktopProgram::kept_settings() {
    json kept = service_.invoke("read_settings", json::object());
    return kept.contains("value") && kept["value"].is_object() ? kept["value"] : json::object();
}

std::optional<std::string> DesktopProgram::answered(const std::string& command, const json& args) {
    if (command != "open_window") return wire_answer(service_, command, args);
    // A window that opens starts from the settings the person kept, as a page's starts from the page's.
    Page kept{kept_settings(), clarity::language()};
    AsPage as(kept);
    return wire_answer(service_, command, args);
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
