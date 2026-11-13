#include "agents/agents.hpp"

#include <algorithm>
#include <cstring>
#include <filesystem>
#include <iostream>

#include "clarity/errors.hpp"
#include "clarity/settings.hpp"
#include "clarity/text.hpp"
#include "clarity_app_mcp.h"

namespace clarity::agents {

const std::vector<std::pair<std::string, std::string>>& words = cli::agent_words_in_english;

namespace {

const ClarityAppMcpApi& api() {
    return *clarity_app_mcp_api();
}

/// A word of Clarity's in the language shown, with the app's title.
std::string say(const std::string& key, const std::string& app) {
    std::string text;
    if (auto value = key_value(key); value && !value->empty()) {
        text = *value;
    } else {
        for (const auto& [k, english] : words)
            if (k == key) text = english;
    }
    for (size_t at = text.find("{app}"); at != std::string::npos; at = text.find("{app}", at + app.size())) text.replace(at, 5, app);
    return text;
}

/// What the library gave in `out`, given back to it.
std::string taken(uint8_t* out, size_t length) {
    std::string text(reinterpret_cast<const char*>(out), length);
    api().free(out, length);
    return text;
}

/// What a serve runs with.
struct Serving {
    const cli::Program& program;
    const std::optional<Lines>& lines;
    const Run& run;
    std::string line;
};

void answer(ClarityAppMcpCall* call, std::string text, bool refused) {
    while (!text.empty() && (text.back() == '\n' || text.back() == ' ')) text.pop_back();
    const auto* bytes = reinterpret_cast<const uint8_t*>(text.data());
    if (refused) {
        api().refused(call, bytes, text.size());
    } else {
        api().printed(call, bytes, text.size());
    }
}

void run_call(void* user, ClarityAppMcpCall* call, const uint8_t*, size_t, const uint8_t*, size_t) {
    auto& serving = *static_cast<Serving*>(user);
    uint8_t* out = nullptr;
    size_t length = 0;
    if (api().command_line(call, &out, &length) != CLARITY_APP_MCP_OK) return answer(call, "", true);
    const auto line = nlohmann::json::parse(taken(out, length)).get<std::vector<std::string>>();
    const auto [text, refused] = cli::run_line(serving.program, line, [call](const std::string& question, const std::string& form) -> std::optional<std::string> {
        uint8_t* given = nullptr;
        size_t given_length = 0;
        if (api().ask_cli(call, reinterpret_cast<const uint8_t*>(question.data()), question.size(), reinterpret_cast<const uint8_t*>(form.data()), form.size(), &given, &given_length) != CLARITY_APP_MCP_OK) return std::nullopt;
        return taken(given, given_length);
    }, serving.run);
    answer(call, text, refused);
}

/// While the app is closed nobody is there to ask.
int32_t grant(void*, const uint8_t*, size_t) {
    return CLARITY_APP_MCP_REFUSE;
}

int32_t read_line(void* user, const uint8_t** line, size_t* length) {
    auto& serving = *static_cast<Serving*>(user);
    auto next = serving.lines->read();
    if (!next) return 1;
    serving.line = std::move(*next);
    *line = reinterpret_cast<const uint8_t*>(serving.line.data());
    *length = serving.line.size();
    return 0;
}

void write_line(void* user, const uint8_t* line, size_t length) {
    static_cast<Serving*>(user)->lines->write(std::string(reinterpret_cast<const char*>(line), length));
}

}  // namespace

std::string options(const Agents& agents, const nlohmann::json& kept) {
    nlohmann::json given{
        {"version", agents.version},
        {"open", false},
        {"texts", {{"refused", say("agentRefused", agents.app)}, {"closed", say("agentClosed", agents.app)}}},
    };
    // A switch the person never set is left out: the service uses what the project allows.
    for (const auto& [key, setting] : {std::pair{"closedReads", reads_setting}, std::pair{"closedChanges", changes_setting}}) {
        if (kept.is_object() && kept.contains(setting) && kept[setting].is_boolean()) given[key] = kept[setting].get<bool>();
    }
    return given.dump();
}

void serve(const cli::Program& program, const Agents& agents, const nlohmann::json& kept, const std::optional<Lines>& lines, const Run& run) {
    Serving serving{program, lines, run, {}};
    ClarityAppMcpHost host{&serving, lines ? read_line : nullptr, lines ? write_line : nullptr, run_call, grant};
    const std::string given = options(agents, kept);
    if (api().serve(reinterpret_cast<const uint8_t*>(agents.tools.data()), agents.tools.size(), reinterpret_cast<const uint8_t*>(given.data()), given.size(), &host) != CLARITY_APP_MCP_OK) {
        char why[1024];
        api().problem(why, sizeof why);
        throw InvalidError(why);
    }
}

int run_program(const cli::Program& program, const Agents& agents, int argc, char** argv, const Run& run) {
    if (argc != 2 || std::strcmp(argv[1], "mcp") != 0) return cli::run(program, argc, argv, run);
    // The running app answers agents itself, granting each change.
    if (auto settings = settings_file(agents.program)) {
        const std::string file = (std::filesystem::path(*settings).parent_path() / "running.json").string();
        if (api().forward(reinterpret_cast<const uint8_t*>(file.data()), file.size()) == 1) return 0;
    }
    try {
        serve(program, agents, read_settings(agents.program), std::nullopt, run);
        return 0;
    } catch (const Error& error) {
        std::cerr << error.what() << "\n";
        return 1;
    }
}

}  // namespace clarity::agents
