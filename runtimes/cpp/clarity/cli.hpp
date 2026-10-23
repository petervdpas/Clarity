// Command-line programs (DESIGN 5.20): reading a command line against the
// program's commands, writing its help, printing a table and asking on a
// terminal, as conformance/command-line.json says every runtime does. A
// generated program describes itself as a cli::Program and runs the
// command cli::parse finds with the values it read.

#pragma once

#include <functional>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "clarity/decimal.hpp"
#include "clarity/text.hpp"
#include "clarity/values.hpp"

namespace clarity::cli {

/// Clarity's own words of the command line, in English: what a program
/// shows for a key its translations leave out.
extern const std::vector<std::pair<std::string, std::string>> words_in_english;

/// What an argument or option holds; bool only for an option.
enum class Kind { text, integer, decimal, money, date, datetime, duration, enumeration, boolean };

/// An argument or an option; its name as typed (`call-after`).
struct Param {
    std::string name;
    Kind kind = Kind::text;
    /// The values of an enum, by name.
    std::vector<std::string> values;
    std::optional<std::string> fallback;
    std::optional<std::string> min;
    std::optional<std::string> max;
    bool required = false;
    std::optional<std::string> help;
};

struct Command {
    /// The words typed: {"customer", "add"}.
    std::vector<std::string> line;
    std::optional<std::string> help;
    std::vector<Param> arguments;
    std::vector<Param> options;
};

struct Program {
    std::string name;
    std::optional<std::string> help;
    /// In the order they are declared.
    std::vector<Command> commands;
};

using Slots = std::vector<std::pair<std::string, std::string>>;

/// Clarity's words in the language shown: the translation of a key, or its English.
class Words {
public:
    explicit Words(std::function<std::optional<std::string>(const std::string&)> translated = {}) : translated_(std::move(translated)) {}
    /// The word of `key` with its slots filled.
    std::string say(const std::string& key, const Slots& slots = {}) const;

private:
    std::function<std::optional<std::string>(const std::string&)> translated_;
};

/// What a command line becomes.
struct Parsed {
    enum class Outcome { run, help, wrong } outcome = Outcome::run;
    /// The command to run, by its index in the program.
    size_t command = 0;
    /// Every argument's value and then every option's, in their order;
    /// nothing for the empty value.
    std::vector<std::optional<std::string>> values;
    /// Help for standard output; the program ends with 0.
    std::string help;
    /// A wrong command line: its message's key and slots, and what is
    /// written to standard error; the program ends with 2.
    std::string key;
    Slots slots;
    std::string said;
};

/// Reads `args`, the words after the program's name.
Parsed parse(const Program& program, const std::vector<std::string>& args, const Words& words);
/// `value` as text of `param`'s kind; nothing when it is not one.
std::optional<std::string> read_value(const Param& param, const std::string& value);
/// The help of the program (`path` empty) or of a group.
std::string group_help(const Program& program, const std::vector<std::string>& path, const Words& words);
/// The help of one command.
std::string command_help(const Program& program, const Command& command, const Words& words);
/// What `print list columns a, b` writes.
std::string table(const std::vector<std::string>& headings, const std::vector<bool>& right, const std::vector<std::vector<std::string>>& rows);

/// What a command asks: yes or no by its two words, a value of a kind with
/// its default, or one of the choices by number from 1.
struct Asked {
    enum class Kind { confirm, value, choice } kind = Kind::confirm;
    std::string yes;
    std::string no;
    Param param;
    std::vector<std::string> choices;
};

/// Where a command asks: a terminal, or a pipe or script with nobody at it.
class Terminal {
public:
    virtual ~Terminal() = default;
    virtual bool is_terminal() = 0;
    /// The next line typed, without its end; nothing at the end of input.
    virtual std::optional<std::string> read_line() = 0;
    /// Writes to standard error, where questions go.
    virtual void write(const std::string& text) = 0;
};

/// The terminal the program runs in.
class Console : public Terminal {
public:
    bool is_terminal() override;
    std::optional<std::string> read_line() override;
    void write(const std::string& text) override;
};

/// The answer to `question` as text (`true` or `false` for yes or no, the
/// number of a choice), and 0; or nothing and the exit code the program ends with.
std::pair<std::optional<std::string>, int> ask(const std::string& question, const Asked& asked, Terminal& terminal, const Words& words);

}  // namespace clarity::cli

namespace clarity::cli {

/// What a running command prints, says and asks through: standard output,
/// standard error and the terminal, in the language shown. A question
/// nobody can answer ends the program.
class Session {
public:
    explicit Session(Terminal& terminal) : terminal_(terminal) {}
    /// `print value`: one line on standard output.
    void print(const Text& text);
    /// `print list columns a, b`: the records as a table on standard output.
    void print_table(const std::vector<Text>& headings, const std::vector<bool>& right, const std::vector<std::vector<std::string>>& rows);
    /// `message`: one line on standard error.
    void message(const Text& text);
    /// `ask ... yes @a no @b`: whether the person said yes; without the words, Clarity's OK and Cancel.
    bool confirm(const Text& question, const std::optional<Text>& yes, const std::optional<Text>& no);
    /// `ask ... for type [default value]`: the value typed, as text of its kind.
    std::string ask(const Text& question, Kind kind, const std::vector<std::string>& values, const std::optional<std::string>& fallback);
    /// `ask ... for Entity from list`: the position, from 0, of the choice made.
    size_t choose(const Text& question, const std::vector<std::string>& choices);
    /// Clarity's word for yes or no, as a table shows a bool.
    static std::string yes_no(bool value);

private:
    std::string answer(const Text& question, const Asked& asked);
    Terminal& terminal_;
};

/// Clarity's words of the command line as the translations give them, or their English.
Words translated_words();

/// The language to show for the system's locale, among the project's
/// translations; nothing for the language the project is written in.
std::optional<std::string> pick_language(const std::string& locale, const std::vector<std::string>& available, const std::string& base);

/// Runs a command-line program: reads the command line, writes help or what
/// is wrong with it, or runs the command it names with its values, and gives
/// the exit code its run ends with (DESIGN 5.20).
int run(const Program& program, int argc, char** argv, const std::function<int(Session&, size_t, const std::vector<std::optional<std::string>>&)>& command);

// A value read from the command line, or answered, as the type code holds
// it: nothing is the empty value of that type.
std::string text(const std::optional<std::string>& value);
int64_t integer(const std::optional<std::string>& value);
bool boolean(const std::optional<std::string>& value);
Decimal decimal(const std::optional<std::string>& value);
std::optional<Date> date(const std::optional<std::string>& value);
std::optional<DateTime> date_time(const std::optional<std::string>& value);

// A value of code as the command line writes it.
inline std::string text_of(const std::string& value) { return value; }
inline std::string text_of(int64_t value) { return std::to_string(value); }
std::string text_of(const Decimal& value);
inline std::string text_of(const std::optional<Date>& value) { return value ? value->str() : ""; }
inline std::string text_of(const std::optional<DateTime>& value) { return value ? value->str() : ""; }

}  // namespace clarity::cli
