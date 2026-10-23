// Command-line programs (DESIGN 5.20): reading a command line against the
// program's commands, writing its help, printing a table and asking on a
// terminal, as conformance/command-line.json says every runtime does. A
// generated program describes itself as a ClarityCliProgram and runs the
// command clarity_cli_parse finds with the values it read. Everything it
// gives is in the scratch arena.

#pragma once

#include "clarity/core.h"
#include "clarity/decimal.h"
#include "clarity/values.h"

/// What an argument or option holds; bool only for an option.
typedef enum {
    CLARITY_CLI_TEXT,
    CLARITY_CLI_INTEGER,
    CLARITY_CLI_DECIMAL,
    CLARITY_CLI_MONEY,
    CLARITY_CLI_DATE,
    CLARITY_CLI_DATETIME,
    CLARITY_CLI_DURATION,
    CLARITY_CLI_ENUM,
    CLARITY_CLI_BOOL,
} ClarityCliKind;

/// An argument or an option; its name as typed (`call-after`). The texts
/// that may be absent are NULL.
typedef struct {
    const char* name;
    ClarityCliKind kind;
    /// The values of an enum, by name.
    const char* const* values;
    size_t value_count;
    const char* fallback;
    const char* min;
    const char* max;
    bool required;
    const char* help;
} ClarityCliParam;

typedef struct {
    /// The words typed: {"customer", "add"}.
    const char* const* line;
    size_t words;
    const char* help;
    const ClarityCliParam* arguments;
    size_t argument_count;
    const ClarityCliParam* options;
    size_t option_count;
} ClarityCliCommand;

typedef struct {
    const char* name;
    const char* help;
    /// In the order they are declared.
    const ClarityCliCommand* commands;
    size_t command_count;
} ClarityCliProgram;

/// Clarity's own words of the command line, in English: what a program
/// shows for a key its translations leave out.
typedef struct {
    const char* key;
    const char* english;
} ClarityCliWord;
extern const ClarityCliWord CLARITY_CLI_WORDS[];
extern const size_t CLARITY_CLI_WORD_COUNT;

/// The translation of a key in the language shown; NULL or "" for none.
typedef struct {
    const char* (*translated)(const char* key, void* context);
    void* context;
} ClarityCliWords;

/// The word of `key` with `count` slots filled by name.
const char* clarity_cli_say(ClarityCliWords words, const char* key, size_t count, const char* const* names, const char* const* values);

typedef enum {
    /// Run `command` with `values`.
    CLARITY_CLI_RUN,
    /// Write `help` to standard output; the program ends with 0.
    CLARITY_CLI_HELP,
    /// A wrong command line: `key` and its slots, and `said` for standard
    /// error; the program ends with 2.
    CLARITY_CLI_WRONG,
} ClarityCliOutcome;

typedef struct {
    ClarityCliOutcome outcome;
    size_t command;
    /// Every argument's value and then every option's, in their order;
    /// NULL for the empty value.
    const char** values;
    const char* help;
    const char* key;
    const char* slot_names[2];
    const char* slot_values[2];
    size_t slot_count;
    const char* said;
} ClarityCliParsed;

/// Reads the `count` words after the program's name.
ClarityCliParsed clarity_cli_parse(const ClarityCliProgram* program, size_t count, const char* const* args, ClarityCliWords words);
/// `value` as text of `param`'s kind; NULL when it is not one.
const char* clarity_cli_read_value(const ClarityCliParam* param, const char* value);
/// The help of the program (no words) or of a group.
const char* clarity_cli_group_help(const ClarityCliProgram* program, size_t words, const char* const* path, ClarityCliWords say);
/// The help of one command.
const char* clarity_cli_command_help(const ClarityCliProgram* program, const ClarityCliCommand* command, ClarityCliWords say);

/// What `print list columns a, b` writes: `cells` holds `rows` rows of
/// `columns` texts each.
const char* clarity_cli_table(size_t columns, const char* const* headings, const bool* right, size_t rows, const char* const* cells);

typedef enum {
    CLARITY_CLI_CONFIRM,
    CLARITY_CLI_VALUE,
    CLARITY_CLI_CHOICE,
} ClarityCliAskKind;

/// What a command asks: yes or no by its two words, a value of a kind with
/// its default, or one of the choices by number from 1.
typedef struct {
    ClarityCliAskKind kind;
    const char* yes;
    const char* no;
    ClarityCliParam param;
    const char* const* choices;
    size_t choice_count;
} ClarityCliAsked;

/// Where a command asks: a terminal, or a pipe or script with nobody at it.
typedef struct {
    bool (*is_terminal)(void* context);
    /// The next line typed, without its end; NULL at the end of input.
    const char* (*read_line)(void* context);
    /// Writes to standard error, where questions go.
    void (*write)(void* context, const char* text);
    void* context;
} ClarityCliTerminal;

/// The terminal the program runs in.
ClarityCliTerminal clarity_cli_console(void);

/// Asks `question`: 0 with `answer` set (`true` or `false` for yes or no,
/// the number of a choice), or the exit code the program ends with.
int clarity_cli_ask(const char* question, const ClarityCliAsked* asked, ClarityCliTerminal terminal, ClarityCliWords words, const char** answer);

// What generated commands print, say and ask through: standard output,
// standard error and the terminal the program runs in, in the language
// shown (clarity_set_language). A question nobody can answer ends the
// program.

/// Clarity's words of the command line as the translations give them, or their English.
ClarityCliWords clarity_cli_words(void);
/// `print value`: one line on standard output.
void clarity_cli_print(const ClarityText* text);
/// `print list columns a, b`: `rows` rows of `columns` cells.
void clarity_cli_print_table(size_t columns, const ClarityText* const* headings, const bool* right, size_t rows, const char* const* cells);
/// `message`: one line on standard error.
void clarity_cli_message(const ClarityText* text);
/// `ask ... yes @a no @b`: whether the person said yes; NULL words take Clarity's OK and Cancel.
bool clarity_cli_confirm(const ClarityText* question, const ClarityText* yes, const ClarityText* no);
/// `ask ... for type [default value]`: the value typed, as text of its kind.
const char* clarity_cli_ask_value(const ClarityText* question, ClarityCliKind kind, const char* const* values, size_t value_count, const char* fallback);
/// `ask ... for Entity from list`: the position, from 0, of the choice made.
size_t clarity_cli_choose(const ClarityText* question, const char* const* choices, size_t count);
/// Clarity's word for yes or no, as a table shows a bool.
const char* clarity_cli_yes_no(bool value);
/// What went wrong in a command, on standard error: a refusal in its words.
void clarity_cli_failed(void);
/// The language to show for the system's locale, among the project's
/// translations; NULL for the language the project is written in (`base`).
const char* clarity_cli_pick_language(const char* locale, const char* const* available, size_t count, const char* base);

// A value read from the command line, or answered, as the type code holds
// it: NULL is the empty value of that type.
const char* clarity_cli_text(const char* text);
int64_t clarity_cli_integer(const char* text);
bool clarity_cli_bool(const char* text);
ClarityDecimal clarity_cli_decimal(const char* text);
ClarityDate clarity_cli_date(const char* text);
ClarityDateTime clarity_cli_date_time(const char* text);
/// A decimal as the command line writes it, without trailing zeros.
const char* clarity_cli_decimal_text(ClarityDecimal value);
