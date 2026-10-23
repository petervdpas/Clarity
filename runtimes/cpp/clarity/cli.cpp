#include "clarity/cli.hpp"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <iostream>
#include <map>

#ifdef _WIN32
#include <io.h>
#define CLARITY_ISATTY _isatty
#else
#include <unistd.h>
#define CLARITY_ISATTY isatty
#endif

#include "clarity/decimal.hpp"
#include "clarity/values.hpp"

namespace clarity::cli {

const std::vector<std::pair<std::string, std::string>> words_in_english = {
    {"cliUsage", "Usage"},
    {"cliCommands", "Commands"},
    {"cliOptions", "Options"},
    {"cliCommandWord", "command"},
    {"cliOptionsWord", "options"},
    {"cliHelpOption", "Shows this help"},
    {"cliDefault", "default {value}"},
    {"cliRequiredMark", "required"},
    {"cliUnknownCommand", "There is no command {command}"},
    {"cliUnknownOption", "There is no option {option}"},
    {"cliMissingArgument", "{argument} is missing"},
    {"cliTooManyArguments", "There are more values than arguments"},
    {"cliNeedsValue", "{option} needs a value"},
    {"cliWrongValue", "{name} cannot be {value}"},
    {"cliTooSmall", "{name} is at least {min}"},
    {"cliTooLarge", "{name} is at most {max}"},
    {"cliRequired", "{option} is required"},
    {"cliWrongAnswer", "{value} does not fit; try again"},
    {"cliNoTerminal", "There is nobody to answer: {question}"},
    {"cliChooseNumber", "Type a number from 1 to {count}"},
};

std::string Words::say(const std::string& key, const Slots& slots) const {
    std::optional<std::string> text = translated_ ? translated_(key) : std::nullopt;
    if (!text || text->empty()) {
        text = key;
        for (const auto& [k, english] : words_in_english) {
            if (k == key) text = english;
        }
    }
    std::string out = *text;
    for (const auto& [name, value] : slots) {
        const std::string slot = "{" + name + "}";
        for (size_t at = out.find(slot); at != std::string::npos; at = out.find(slot, at + value.size())) out.replace(at, slot.size(), value);
    }
    return out;
}

namespace {

bool starts(const std::string& text, const std::string& start) {
    return text.rfind(start, 0) == 0;
}

bool plain_digits(const std::string& text) {
    return !text.empty() && std::all_of(text.begin(), text.end(), [](unsigned char c) { return std::isdigit(c); });
}

std::string lower(std::string text) {
    std::transform(text.begin(), text.end(), text.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return text;
}

std::string trimmed(const std::string& text) {
    const auto first = text.find_first_not_of(" \t");
    if (first == std::string::npos) return "";
    return text.substr(first, text.find_last_not_of(" \t") - first + 1);
}

/// The value without trailing zeros in its decimals: 7.50 is 7.5.
std::string normalized(std::string text) {
    if (text.find('.') == std::string::npos) return text;
    while (text.back() == '0') text.pop_back();
    if (text.back() == '.') text.pop_back();
    return text == "-0" ? "0" : text;
}

/// Whole seconds from 45, 3:45 or 1:02:03, as formats.json reads them.
std::optional<long long> duration_of(const std::string& text) {
    std::vector<std::string> parts;
    std::string t = trimmed(text);
    size_t start = 0;
    for (size_t i = 0; i <= t.size(); ++i) {
        if (i == t.size() || t[i] == ':') {
            parts.push_back(t.substr(start, i - start));
            start = i + 1;
        }
    }
    if (parts.size() > 3) return std::nullopt;
    for (const auto& p : parts) {
        if (!plain_digits(p) || p.size() > 15) return std::nullopt;
    }
    for (size_t i = 1; i < parts.size(); ++i) {
        if (parts[i].size() != 2 || std::stoll(parts[i]) > 59) return std::nullopt;
    }
    long long seconds = 0;
    for (const auto& p : parts) seconds = seconds * 60 + std::stoll(p);
    return seconds;
}

/// Whether the number `a` is less than `b`, exactly.
bool below(const std::string& a, const std::string& b) {
    try {
        return Decimal::parse(a) < Decimal::parse(b);
    } catch (...) {
        return false;
    }
}

size_t letters(const std::string& text) {
    return static_cast<size_t>(std::count_if(text.begin(), text.end(), [](unsigned char c) { return (c & 0xC0) != 0x80; }));
}

std::string without_trailing_spaces(std::string line) {
    while (!line.empty() && line.back() == ' ') line.pop_back();
    return line;
}

std::string joined(const std::vector<std::string>& words, const std::string& between) {
    std::string out;
    for (size_t i = 0; i < words.size(); ++i) out += (i ? between : "") + words[i];
    return out;
}

bool begins_with(const std::vector<std::string>& line, const std::vector<std::string>& path) {
    return line.size() >= path.size() && std::equal(path.begin(), path.end(), line.begin());
}

/// A heading and its lines, the left column padded to the longest.
std::string section(const std::string& heading, const std::vector<std::pair<std::string, std::string>>& rows) {
    size_t width = 0;
    for (const auto& [left, right] : rows) width = std::max(width, letters(left));
    std::string out = heading + ":\n";
    for (const auto& [left, right] : rows) {
        std::string line = "  " + left;
        if (!right.empty()) line += std::string(width - letters(left) + 2, ' ') + right;
        out += without_trailing_spaces(line) + "\n";
    }
    return out;
}

std::string group_usage(const Program& program, const std::vector<std::string>& path, const Words& words) {
    std::vector<std::string> line = {program.name};
    line.insert(line.end(), path.begin(), path.end());
    return words.say("cliUsage") + ": " + joined(line, " ") + " <" + words.say("cliCommandWord") + "> [" + words.say("cliOptionsWord") + "]";
}

std::string command_usage(const Program& program, const Command& c, const Words& words) {
    std::vector<std::string> line = {program.name};
    line.insert(line.end(), c.line.begin(), c.line.end());
    for (const auto& a : c.arguments) line.push_back("<" + a.name + ">");
    return words.say("cliUsage") + ": " + joined(line, " ") + " [" + words.say("cliOptionsWord") + "]";
}

Parsed wrong(const Program& program, const std::vector<std::string>& path, const Command* command, const Words& words, const std::string& key, const Slots& slots) {
    Parsed parsed;
    parsed.outcome = Parsed::Outcome::wrong;
    parsed.key = key;
    parsed.slots = slots;
    parsed.said = words.say(key, slots) + "\n" + (command ? command_usage(program, *command, words) : group_usage(program, path, words)) + "\n";
    return parsed;
}

std::string option_name(const std::string& word) {
    return word.substr(0, word.find('='));
}

}  // namespace

std::optional<std::string> read_value(const Param& param, const std::string& value) {
    switch (param.kind) {
    case Kind::text:
        return value;
    case Kind::boolean:
        if (value == "true" || value == "false") return value;
        return std::nullopt;
    case Kind::integer: {
        const std::string digits = starts(value, "-") ? value.substr(1) : value;
        if (!plain_digits(digits) || digits.size() > 18) return std::nullopt;
        return std::to_string(std::stoll(value));
    }
    case Kind::decimal:
    case Kind::money: {
        const std::string digits = starts(value, "-") ? value.substr(1) : value;
        const auto point = digits.find('.');
        if (!plain_digits(digits.substr(0, point)) || (point != std::string::npos && !plain_digits(digits.substr(point + 1)))) return std::nullopt;
        try {
            return normalized(Decimal::parse(value).str());
        } catch (...) {
            return std::nullopt;
        }
    }
    case Kind::date: {
        const auto d = Date::parse(value);
        if (!d || value.size() != 10) return std::nullopt;
        return d->str();
    }
    case Kind::datetime: {
        if (value.size() < 11 || value[10] != 'T' || (value.size() != 16 && value.size() != 19)) return std::nullopt;
        const auto t = DateTime::parse(value);
        if (!t) return std::nullopt;
        return t->str();
    }
    case Kind::duration: {
        const auto seconds = duration_of(value);
        if (!seconds) return std::nullopt;
        return std::to_string(*seconds);
    }
    case Kind::enumeration:
        for (const auto& v : param.values) {
            if (lower(v) == lower(value)) return v;
        }
        return std::nullopt;
    }
    return std::nullopt;
}

std::string group_help(const Program& program, const std::vector<std::string>& path, const Words& words) {
    std::string out;
    if (path.empty() && program.help) out += *program.help + "\n\n";
    out += group_usage(program, path, words) + "\n\n";
    std::vector<std::pair<std::string, std::string>> commands;
    for (const auto& c : program.commands) {
        if (c.line.size() > path.size() && begins_with(c.line, path)) commands.emplace_back(joined(c.line, " "), c.help.value_or(""));
    }
    out += section(words.say("cliCommands"), commands) + "\n";
    out += section(words.say("cliOptions"), {{"--help", words.say("cliHelpOption")}});
    return out;
}

std::string command_help(const Program& program, const Command& c, const Words& words) {
    std::string out;
    if (c.help) out += *c.help + "\n\n";
    out += command_usage(program, c, words) + "\n\n";
    std::vector<std::pair<std::string, std::string>> rows;
    for (const auto& o : c.options) {
        std::string left = "--" + o.name + (o.kind == Kind::boolean ? "" : " <" + o.name + ">");
        std::vector<std::string> marks;
        if (!o.values.empty()) marks.push_back(joined(o.values, ", "));
        if (o.fallback && o.kind != Kind::boolean) marks.push_back(words.say("cliDefault", {{"value", *o.fallback}}));
        if (o.required) marks.push_back(words.say("cliRequiredMark"));
        std::string right = o.help.value_or("");
        if (!marks.empty()) right += (right.empty() ? "" : " ") + std::string("(") + joined(marks, "; ") + ")";
        rows.emplace_back(left, right);
    }
    rows.emplace_back("--help", words.say("cliHelpOption"));
    out += section(words.say("cliOptions"), rows);
    return out;
}

Parsed parse(const Program& program, const std::vector<std::string>& args, const Words& words) {
    const size_t end = static_cast<size_t>(std::find(args.begin(), args.end(), "--") - args.begin());
    bool asked_help = false;
    std::vector<std::string> rest;
    for (size_t i = 0; i < args.size(); ++i) {
        if (i < end && args[i] == "--help") asked_help = true;
        else rest.push_back(args[i]);
    }
    size_t at = 0;
    if (!rest.empty() && rest[0] == "help") {
        asked_help = true;
        at = 1;
    }
    // The command, or the group read so far.
    std::vector<std::string> path;
    std::optional<size_t> found;
    while (!found) {
        if (at >= rest.size() || starts(rest[at], "--")) {
            if (at < rest.size() && !asked_help && path.empty()) return wrong(program, path, nullptr, words, "cliUnknownOption", {{"option", option_name(rest[at])}});
            Parsed help;
            help.outcome = Parsed::Outcome::help;
            help.help = group_help(program, path, words);
            return help;
        }
        auto line = path;
        line.push_back(rest[at]);
        bool group = false;
        for (size_t i = 0; i < program.commands.size(); ++i) {
            const auto& c = program.commands[i];
            if (c.line == line) found = i;
            else if (c.line.size() > line.size() && begins_with(c.line, line)) group = true;
        }
        if (!found && !group) return wrong(program, path, nullptr, words, "cliUnknownCommand", {{"command", joined(line, " ")}});
        path = line;
        ++at;
    }
    const Command& c = program.commands[*found];
    if (asked_help) {
        Parsed help;
        help.outcome = Parsed::Outcome::help;
        help.help = command_help(program, c, words);
        return help;
    }
    auto say_wrong = [&](const std::string& key, const Slots& slots) { return wrong(program, {}, &c, words, key, slots); };

    const std::vector<std::string> after(rest.begin() + static_cast<long>(at), rest.end());
    const size_t ends_at = static_cast<size_t>(std::find(after.begin(), after.end(), "--") - after.begin());
    auto option_of = [&](const std::string& word) -> std::optional<std::pair<size_t, std::optional<std::string>>> {
        if (!starts(word, "--")) return std::nullopt;
        const auto equals = word.find('=');
        const std::string name = word.substr(2, equals == std::string::npos ? std::string::npos : equals - 2);
        for (size_t i = 0; i < c.options.size(); ++i) {
            if (c.options[i].name == name) return std::pair{i, equals == std::string::npos ? std::nullopt : std::optional<std::string>(word.substr(equals + 1))};
        }
        return std::nullopt;
    };
    for (size_t i = 0; i < after.size() && i < ends_at; ++i) {
        if (starts(after[i], "--") && !option_of(after[i])) return say_wrong("cliUnknownOption", {{"option", option_name(after[i])}});
    }
    std::vector<std::pair<size_t, std::string>> given;
    std::vector<std::string> positional;
    for (size_t i = 0; i < after.size(); ++i) {
        if (i == ends_at) {
            positional.insert(positional.end(), after.begin() + static_cast<long>(i) + 1, after.end());
            break;
        }
        const auto option = option_of(after[i]);
        if (!option) {
            positional.push_back(after[i]);
            continue;
        }
        const auto& [index, written] = *option;
        const Param& o = c.options[index];
        std::string value;
        if (written) value = *written;
        else if (o.kind == Kind::boolean) value = "true";
        else if (i + 1 < after.size()) value = after[++i];
        else return say_wrong("cliNeedsValue", {{"option", "--" + o.name}});
        std::erase_if(given, [&](const auto& g) { return g.first == index; });
        given.emplace_back(index, value);
    }

    // Each value in the order it was typed: of its kind, then within its range.
    struct Read {
        std::string name;
        const Param* param;
        std::string typed;
    };
    std::vector<Read> reads;
    for (size_t i = 0; i < c.arguments.size() && i < positional.size(); ++i) reads.push_back({c.arguments[i].name, &c.arguments[i], positional[i]});
    for (const auto& [index, value] : given) reads.push_back({"--" + c.options[index].name, &c.options[index], value});
    std::map<std::string, std::string> values;
    for (const auto& r : reads) {
        const auto v = read_value(*r.param, r.typed);
        if (!v) return say_wrong("cliWrongValue", {{"name", r.name}, {"value", r.typed}});
        values[r.name] = *v;
    }
    for (const auto& r : reads) {
        const auto& v = values[r.name];
        if (r.param->min && below(v, *r.param->min)) return say_wrong("cliTooSmall", {{"name", r.name}, {"min", *r.param->min}});
        if (r.param->max && below(*r.param->max, v)) return say_wrong("cliTooLarge", {{"name", r.name}, {"max", *r.param->max}});
    }
    if (positional.size() < c.arguments.size()) return say_wrong("cliMissingArgument", {{"argument", c.arguments[positional.size()].name}});
    if (positional.size() > c.arguments.size()) return say_wrong("cliTooManyArguments", {});
    for (size_t i = 0; i < c.options.size(); ++i) {
        const bool typed = std::any_of(given.begin(), given.end(), [&](const auto& g) { return g.first == i; });
        if (c.options[i].required && !typed) return say_wrong("cliRequired", {{"option", "--" + c.options[i].name}});
    }
    Parsed parsed;
    parsed.command = *found;
    for (const auto& a : c.arguments) parsed.values.emplace_back(values[a.name]);
    for (const auto& o : c.options) {
        const auto typed = values.find("--" + o.name);
        if (typed != values.end()) parsed.values.emplace_back(typed->second);
        else if (o.fallback) parsed.values.emplace_back(*o.fallback);
        else if (o.kind == Kind::boolean) parsed.values.emplace_back("false");
        else parsed.values.emplace_back(std::nullopt);
    }
    return parsed;
}

std::string table(const std::vector<std::string>& headings, const std::vector<bool>& right, const std::vector<std::vector<std::string>>& rows) {
    std::vector<std::vector<std::string>> lines = {headings};
    lines.insert(lines.end(), rows.begin(), rows.end());
    std::vector<size_t> widths(headings.size(), 0);
    for (const auto& row : lines) {
        for (size_t i = 0; i < widths.size() && i < row.size(); ++i) widths[i] = std::max(widths[i], letters(row[i]));
    }
    std::string out;
    for (const auto& row : lines) {
        std::string line;
        for (size_t i = 0; i < widths.size(); ++i) {
            const std::string cell = i < row.size() ? row[i] : "";
            const std::string pad(widths[i] - letters(cell), ' ');
            line += (i ? "  " : "") + (i < right.size() && right[i] ? pad + cell : cell + pad);
        }
        out += without_trailing_spaces(line) + "\n";
    }
    return out;
}

/// A terminal, or Clarity's Output panel, which says so with CLARITY_TERMINAL=1.
bool Console::is_terminal() {
    const char* told = std::getenv("CLARITY_TERMINAL");
    return CLARITY_ISATTY(0) || (told && std::string(told) == "1");
}

std::optional<std::string> Console::read_line() {
    std::string line;
    if (!std::getline(std::cin, line)) return std::nullopt;
    if (!line.empty() && line.back() == '\r') line.pop_back();
    return line;
}

void Console::write(const std::string& text) {
    std::cerr << text << std::flush;
}

std::pair<std::optional<std::string>, int> ask(const std::string& question, const Asked& asked, Terminal& terminal, const Words& words) {
    if (!terminal.is_terminal()) {
        if (asked.kind == Asked::Kind::value && asked.param.fallback) return {asked.param.fallback, 0};
        terminal.write(words.say("cliNoTerminal", {{"question", question}}) + "\n");
        return {std::nullopt, 2};
    }
    if (asked.kind == Asked::Kind::confirm) {
        for (;;) {
            terminal.write(question + " [" + asked.yes + "/" + asked.no + "] ");
            const auto line = terminal.read_line();
            const std::string typed = line ? lower(trimmed(*line)) : "";
            if (typed.empty() || typed == lower(asked.no)) return {"false", 0};
            if (typed == lower(asked.yes)) return {"true", 0};
        }
    }
    if (asked.kind == Asked::Kind::choice) {
        std::string listed;
        for (size_t i = 0; i < asked.choices.size(); ++i) listed += std::to_string(i + 1) + ". " + asked.choices[i] + "\n";
        terminal.write(listed);
        for (;;) {
            terminal.write(question + " ");
            const auto line = terminal.read_line();
            if (!line) {
                terminal.write("\n");
                return {std::nullopt, 1};
            }
            const std::string typed = trimmed(*line);
            if (plain_digits(typed) && typed.size() < 10) {
                const auto n = std::stoul(typed);
                if (n >= 1 && n <= asked.choices.size()) return {std::to_string(n), 0};
            }
            terminal.write(words.say("cliChooseNumber", {{"count", std::to_string(asked.choices.size())}}) + "\n");
        }
    }
    const Param& param = asked.param;
    const std::optional<std::string> shown = param.kind == Kind::enumeration ? std::optional<std::string>(joined(param.values, "/")) : param.fallback;
    for (;;) {
        terminal.write(shown ? question + " [" + *shown + "] " : question + " ");
        const auto line = terminal.read_line();
        if (!line) {
            terminal.write("\n");
            return {std::nullopt, 1};
        }
        const std::string typed = trimmed(*line);
        if (typed.empty() && param.fallback) return {param.fallback, 0};
        if (auto value = read_value(param, typed)) return {value, 0};
        terminal.write(words.say("cliWrongAnswer", {{"value", typed}}) + "\n");
    }
}

}  // namespace clarity::cli

#include "clarity/errors.hpp"

namespace clarity::cli {

Words translated_words() {
    return Words([](const std::string& key) { return key_value(key); });
}

namespace {

std::string word(const std::string& key, const std::string& english) {
    const auto value = key_value(key);
    return value && !value->empty() ? *value : english;
}

}  // namespace

void Session::print(const Text& text) {
    std::cout << text.show() << "\n";
}

void Session::print_table(const std::vector<Text>& headings, const std::vector<bool>& right, const std::vector<std::vector<std::string>>& rows) {
    std::vector<std::string> shown;
    for (const auto& h : headings) shown.push_back(h.show());
    std::cout << table(shown, right, rows);
}

void Session::message(const Text& text) {
    std::cerr << text.show() << "\n";
}

std::string Session::yes_no(bool value) {
    return value ? word("yes", "Yes") : word("no", "No");
}

std::string Session::answer(const Text& question, const Asked& asked) {
    auto [given, code] = cli::ask(question.show(), asked, terminal_, translated_words());
    if (!given) std::exit(code);
    return *given;
}

bool Session::confirm(const Text& question, const std::optional<Text>& yes, const std::optional<Text>& no) {
    Asked asked;
    asked.kind = Asked::Kind::confirm;
    asked.yes = yes ? yes->show() : word("ok", "OK");
    asked.no = no ? no->show() : word("cancel", "Cancel");
    return answer(question, asked) == "true";
}

std::string Session::ask(const Text& question, Kind kind, const std::vector<std::string>& values, const std::optional<std::string>& fallback) {
    Asked asked;
    asked.kind = Asked::Kind::value;
    asked.param.kind = kind;
    asked.param.values = values;
    asked.param.fallback = fallback;
    return answer(question, asked);
}

size_t Session::choose(const Text& question, const std::vector<std::string>& choices) {
    Asked asked;
    asked.kind = Asked::Kind::choice;
    asked.choices = choices;
    return std::stoul(answer(question, asked)) - 1;
}

std::optional<std::string> pick_language(const std::string& locale, const std::vector<std::string>& available, const std::string& base) {
    std::string tag = locale.substr(0, locale.find_first_of(".@"));
    std::replace(tag.begin(), tag.end(), '_', '-');
    auto primary = [](const std::string& t) { return lower(t.substr(0, t.find('-'))); };
    if (tag.empty() || tag == "C" || tag == "POSIX" || primary(tag) == primary(base)) return std::nullopt;
    for (const auto& a : available) {
        if (a == tag) return a;
    }
    for (const auto& a : available) {
        if (primary(a) == primary(tag)) return a;
    }
    return std::nullopt;
}

int run(const Program& program, int argc, char** argv, const std::function<int(Session&, size_t, const std::vector<std::optional<std::string>>&)>& command) {
    const std::vector<std::string> args(argv + 1, argv + argc);
    const auto parsed = parse(program, args, translated_words());
    if (parsed.outcome == Parsed::Outcome::help) {
        std::cout << parsed.help;
        return 0;
    }
    if (parsed.outcome == Parsed::Outcome::wrong) {
        std::cerr << parsed.said;
        return 2;
    }
    Console console;
    Session session(console);
    try {
        return std::clamp(command(session, parsed.command, parsed.values), 0, 255);
    } catch (const RefusedError& refused) {
        std::cerr << refused.text.show() << "\n";
        return 1;
    } catch (const Error& error) {
        std::cerr << error.what() << "\n";
        return 1;
    }
}

std::string text(const std::optional<std::string>& value) {
    return value.value_or("");
}

int64_t integer(const std::optional<std::string>& value) {
    return value ? std::stoll(*value) : 0;
}

bool boolean(const std::optional<std::string>& value) {
    return value && *value == "true";
}

Decimal decimal(const std::optional<std::string>& value) {
    return value ? Decimal::parse(*value) : Decimal();
}

std::optional<Date> date(const std::optional<std::string>& value) {
    return value ? Date::parse(*value) : std::nullopt;
}

std::optional<DateTime> date_time(const std::optional<std::string>& value) {
    return value ? DateTime::parse(*value) : std::nullopt;
}

std::string text_of(const Decimal& value) {
    return normalized(value.str());
}

}  // namespace clarity::cli
