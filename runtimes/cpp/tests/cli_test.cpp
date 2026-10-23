// Command-line programs against Clarity's shared cases
// (conformance/command-line.json): reading a command line, the help, a
// printed table and asking on a terminal.

#include "check.hpp"
#include "clarity/cli.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::optional<std::string> text_of(const json& object, const char* name) {
    return object.contains(name) && object[name].is_string() ? std::optional<std::string>(object[name].get<std::string>()) : std::nullopt;
}

cli::Kind kind_of(const std::string& word) {
    static const std::vector<std::pair<std::string, cli::Kind>> kinds = {
        {"text", cli::Kind::text},         {"integer", cli::Kind::integer},   {"decimal", cli::Kind::decimal},
        {"money", cli::Kind::money},       {"date", cli::Kind::date},         {"datetime", cli::Kind::datetime},
        {"duration", cli::Kind::duration}, {"enum", cli::Kind::enumeration}, {"bool", cli::Kind::boolean},
    };
    for (const auto& [w, k] : kinds) {
        if (w == word) return k;
    }
    return cli::Kind::text;
}

cli::Param param_of(const json& p) {
    cli::Param param;
    param.name = text_of(p, "name").value_or("");
    // An asked value names its kind as `type`; `kind` is the form's there.
    param.kind = kind_of(text_of(p, "type").value_or(text_of(p, "kind").value_or("text")));
    if (p.contains("values")) param.values = p["values"].get<std::vector<std::string>>();
    param.fallback = text_of(p, "default");
    param.min = text_of(p, "min");
    param.max = text_of(p, "max");
    param.required = p.value("required", false);
    param.help = text_of(p, "help");
    return param;
}

cli::Program program_of(const json& p) {
    cli::Program program{p["name"].get<std::string>(), text_of(p, "help"), {}};
    for (const auto& c : p["commands"]) {
        cli::Command command{c["line"].get<std::vector<std::string>>(), text_of(c, "help"), {}, {}};
        for (const auto& a : c["arguments"]) command.arguments.push_back(param_of(a));
        for (const auto& o : c["options"]) command.options.push_back(param_of(o));
        program.commands.push_back(command);
    }
    return program;
}

const cli::Words english;

}  // namespace

TEST(the_command_line_words_are_the_shared_ones) {
    json cases = conformance("command-line.json");
    CHECK_EQ(cases["words"].size(), cli::words_in_english.size());
    for (const auto& [key, value] : cli::words_in_english) CHECK_EQ(cases["words"][key].get<std::string>(), value);
}

TEST(command_lines_are_read_as_every_runtime_reads_them) {
    json cases = conformance("command-line.json");
    const auto program = program_of(cases["program"]);
    for (const auto& c : cases["parse"]) {
        const auto what = c["what"].get<std::string>();
        const auto parsed = cli::parse(program, c["args"].get<std::vector<std::string>>(), english);
        if (c.contains("run")) {
            if (parsed.outcome != cli::Parsed::Outcome::run) {
                check::fail(__FILE__, __LINE__, what);
                continue;
            }
            const auto& command = program.commands[parsed.command];
            json values = json::object();
            std::vector<cli::Param> params = command.arguments;
            params.insert(params.end(), command.options.begin(), command.options.end());
            for (size_t i = 0; i < params.size(); ++i) values[params[i].name] = parsed.values[i] ? json(*parsed.values[i]) : json(nullptr);
            if (values != c["run"]["values"]) check::fail(__FILE__, __LINE__, what + ": " + values.dump());
        } else if (c.contains("help")) {
            CHECK_EQ(parsed.help, c["help"].get<std::string>());
        } else {
            const auto& wanted = c["wrong"];
            CHECK_EQ(parsed.key, wanted["key"].get<std::string>());
            json slots = json::object();
            for (const auto& [k, v] : parsed.slots) slots[k] = v;
            if (slots != wanted["slots"]) check::fail(__FILE__, __LINE__, what + ": " + slots.dump());
            CHECK_EQ(parsed.said, wanted["said"].get<std::string>());
        }
    }
}

TEST(a_printed_table_lines_up_as_every_runtime_does) {
    json cases = conformance("command-line.json");
    for (const auto& c : cases["table"]) {
        CHECK_EQ(cli::table(c["headings"].get<std::vector<std::string>>(), c["right"].get<std::vector<bool>>(), c["rows"].get<std::vector<std::vector<std::string>>>()),
                 c["said"].get<std::string>());
    }
}

namespace {

struct Typed : cli::Terminal {
    bool terminal = true;
    std::vector<std::optional<std::string>> lines;
    std::string said;
    bool is_terminal() override { return terminal; }
    std::optional<std::string> read_line() override {
        if (lines.empty()) return std::nullopt;
        auto line = lines.front();
        lines.erase(lines.begin());
        return line;
    }
    void write(const std::string& text) override { said += text; }
};

}  // namespace

TEST(a_command_asks_on_a_terminal_as_every_runtime_does) {
    json cases = conformance("command-line.json");
    for (const auto& c : cases["ask"]) {
        const auto what = c["what"].get<std::string>();
        const json& form = c["form"];
        cli::Asked asked;
        const auto kind = form["kind"].get<std::string>();
        if (kind == "confirm") {
            asked.kind = cli::Asked::Kind::confirm;
            asked.yes = form["yes"].get<std::string>();
            asked.no = form["no"].get<std::string>();
        } else if (kind == "choice") {
            asked.kind = cli::Asked::Kind::choice;
            asked.choices = form["choices"].get<std::vector<std::string>>();
        } else {
            asked.kind = cli::Asked::Kind::value;
            asked.param = param_of(form);
        }
        Typed typed;
        typed.terminal = c["terminal"].get<bool>();
        for (const auto& line : c["typed"]) typed.lines.push_back(line.is_string() ? std::optional<std::string>(line.get<std::string>()) : std::nullopt);
        const auto [answer, exit] = cli::ask(c["question"].get<std::string>(), asked, typed, english);
        if (c.contains("exit")) {
            CHECK_EQ(exit, c["exit"].get<int>());
        } else {
            CHECK_EQ(exit, 0);
            CHECK_EQ(answer.value_or("<none>"), c["answer"].get<std::string>());
        }
        if (typed.said != c["said"].get<std::string>()) check::fail(__FILE__, __LINE__, what + ": said " + typed.said);
    }
}
