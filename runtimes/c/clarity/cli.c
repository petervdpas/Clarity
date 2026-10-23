#include "clarity/cli.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <io.h>
#define clarity_isatty _isatty
#else
#include <unistd.h>
#define clarity_isatty isatty
#endif

#include "clarity/decimal.h"
#include "clarity/values.h"

const ClarityCliWord CLARITY_CLI_WORDS[] = {
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
const size_t CLARITY_CLI_WORD_COUNT = sizeof CLARITY_CLI_WORDS / sizeof CLARITY_CLI_WORDS[0];

/// `text` with every `{name}` replaced by `value`.
static const char* replaced(const char* text, const char* name, const char* value) {
    const char* slot = clarity_format("{%s}", name);
    size_t length = strlen(slot);
    ClarityBuffer out = {0};
    for (const char* at = text; *at;) {
        if (strncmp(at, slot, length) == 0) {
            clarity_buffer_add(&out, value);
            at += length;
        } else {
            clarity_buffer_add_char(&out, *at++);
        }
    }
    return clarity_buffer_text(&out);
}

const char* clarity_cli_say(ClarityCliWords words, const char* key, size_t count, const char* const* names, const char* const* values) {
    const char* text = words.translated ? words.translated(key, words.context) : NULL;
    if (!text || !*text) {
        text = key;
        for (size_t i = 0; i < CLARITY_CLI_WORD_COUNT; ++i) {
            if (strcmp(CLARITY_CLI_WORDS[i].key, key) == 0) text = CLARITY_CLI_WORDS[i].english;
        }
    }
    for (size_t i = 0; i < count; ++i) text = replaced(text, names[i], values[i]);
    return text;
}

static const char* say0(ClarityCliWords words, const char* key) {
    return clarity_cli_say(words, key, 0, NULL, NULL);
}

static const char* say1(ClarityCliWords words, const char* key, const char* name, const char* value) {
    return clarity_cli_say(words, key, 1, &name, &value);
}

static bool starts(const char* text, const char* start) {
    return strncmp(text, start, strlen(start)) == 0;
}

static bool plain_digits(const char* text, size_t length) {
    if (length == 0) return false;
    for (size_t i = 0; i < length; ++i) {
        if (!isdigit((unsigned char)text[i])) return false;
    }
    return true;
}

/// The value without trailing zeros in its decimals: 7.50 is 7.5.
static const char* normalized(ClarityDecimal d) {
    while (d.scale > 0 && d.units % 10 == 0) {
        d.units /= 10;
        --d.scale;
    }
    return clarity_decimal_text(d);
}

/// Whole seconds from 45, 3:45 or 1:02:03, as formats.json reads them; -1 for none.
static long long duration_of(const char* text) {
    while (*text == ' ') ++text;
    size_t length = strlen(text);
    while (length > 0 && text[length - 1] == ' ') --length;
    long long parts[3];
    size_t sizes[3];
    size_t count = 0;
    size_t start = 0;
    for (size_t i = 0; i <= length; ++i) {
        if (i == length || text[i] == ':') {
            if (count == 3 || !plain_digits(text + start, i - start)) return -1;
            parts[count] = strtoll(clarity_copy_n(text + start, i - start), NULL, 10);
            sizes[count++] = i - start;
            start = i + 1;
        }
    }
    for (size_t i = 1; i < count; ++i) {
        if (sizes[i] != 2 || parts[i] > 59) return -1;
    }
    if (count == 1) return parts[0];
    if (count == 2) return parts[0] * 60 + parts[1];
    return parts[0] * 3600 + parts[1] * 60 + parts[2];
}

const char* clarity_cli_read_value(const ClarityCliParam* param, const char* value) {
    switch (param->kind) {
    case CLARITY_CLI_TEXT:
        return clarity_copy(value);
    case CLARITY_CLI_BOOL:
        return strcmp(value, "true") == 0 || strcmp(value, "false") == 0 ? clarity_copy(value) : NULL;
    case CLARITY_CLI_INTEGER: {
        const char* digits = value[0] == '-' ? value + 1 : value;
        if (!plain_digits(digits, strlen(digits)) || strlen(digits) > 18) return NULL;
        return clarity_format("%lld", strtoll(value, NULL, 10));
    }
    case CLARITY_CLI_DECIMAL:
    case CLARITY_CLI_MONEY: {
        const char* digits = value[0] == '-' ? value + 1 : value;
        const char* point = strchr(digits, '.');
        size_t whole = point ? (size_t)(point - digits) : strlen(digits);
        if (!plain_digits(digits, whole) || (point && !plain_digits(point + 1, strlen(point + 1)))) return NULL;
        ClarityDecimal d;
        return clarity_decimal_parse(value, &d) ? normalized(d) : NULL;
    }
    case CLARITY_CLI_DATE: {
        ClarityDate d;
        return clarity_date_parse(value, &d) ? clarity_date_text(d) : NULL;
    }
    case CLARITY_CLI_DATETIME: {
        ClarityDateTime t;
        if (strlen(value) < 11 || value[10] != 'T' || !clarity_date_time_parse(value, &t)) return NULL;
        return clarity_format("%04d-%02d-%02dT%02d:%02d:%02d", t.year, t.month, t.day, t.hour, t.minute, t.second);
    }
    case CLARITY_CLI_DURATION: {
        long long seconds = duration_of(value);
        return seconds < 0 ? NULL : clarity_format("%lld", seconds);
    }
    case CLARITY_CLI_ENUM:
        for (size_t i = 0; i < param->value_count; ++i) {
            const char* v = param->values[i];
            size_t n = strlen(v);
            if (strlen(value) != n) continue;
            bool same = true;
            for (size_t j = 0; j < n && same; ++j) same = tolower((unsigned char)v[j]) == tolower((unsigned char)value[j]);
            if (same) return v;
        }
        return NULL;
    }
    return NULL;
}

/// Whether the number `a` is less than `b`, exactly.
static bool below(const char* a, const char* b) {
    ClarityDecimal x, y;
    return clarity_decimal_parse(a, &x) && clarity_decimal_parse(b, &y) && clarity_decimal_compare(x, y) < 0;
}

static size_t letters(const char* text) {
    size_t count = 0;
    for (const unsigned char* at = (const unsigned char*)text; *at; ++at) count += (*at & 0xC0) != 0x80;
    return count;
}

static void add_spaces(ClarityBuffer* out, size_t count) {
    for (size_t i = 0; i < count; ++i) clarity_buffer_add_char(out, ' ');
}

/// Ends the line, without the spaces it ended with.
static void end_line(ClarityBuffer* out, size_t line_start) {
    while (out->length > line_start && out->data[out->length - 1] == ' ') out->data[--out->length] = '\0';
    clarity_buffer_add_char(out, '\n');
}

/// A heading and its lines, the left column padded to the longest.
static void section(ClarityBuffer* out, const char* heading, size_t count, const char* const* left, const char* const* right) {
    size_t width = 0;
    for (size_t i = 0; i < count; ++i) width = letters(left[i]) > width ? letters(left[i]) : width;
    clarity_buffer_addf(out, "%s:\n", heading);
    for (size_t i = 0; i < count; ++i) {
        size_t start = out->length;
        clarity_buffer_addf(out, "  %s", left[i]);
        if (*right[i]) {
            add_spaces(out, width - letters(left[i]) + 2);
            clarity_buffer_add(out, right[i]);
        }
        end_line(out, start);
    }
}

static const char* group_usage(const ClarityCliProgram* program, size_t words, const char* const* path, ClarityCliWords say) {
    ClarityBuffer out = {0};
    clarity_buffer_addf(&out, "%s: %s", say0(say, "cliUsage"), program->name);
    for (size_t i = 0; i < words; ++i) clarity_buffer_addf(&out, " %s", path[i]);
    clarity_buffer_addf(&out, " <%s> [%s]", say0(say, "cliCommandWord"), say0(say, "cliOptionsWord"));
    return clarity_buffer_text(&out);
}

static const char* command_usage(const ClarityCliProgram* program, const ClarityCliCommand* c, ClarityCliWords say) {
    ClarityBuffer out = {0};
    clarity_buffer_addf(&out, "%s: %s", say0(say, "cliUsage"), program->name);
    for (size_t i = 0; i < c->words; ++i) clarity_buffer_addf(&out, " %s", c->line[i]);
    for (size_t i = 0; i < c->argument_count; ++i) clarity_buffer_addf(&out, " <%s>", c->arguments[i].name);
    clarity_buffer_addf(&out, " [%s]", say0(say, "cliOptionsWord"));
    return clarity_buffer_text(&out);
}

static bool begins_with(const ClarityCliCommand* c, size_t words, const char* const* path) {
    if (c->words < words) return false;
    for (size_t i = 0; i < words; ++i) {
        if (strcmp(c->line[i], path[i]) != 0) return false;
    }
    return true;
}

static const char* joined(size_t count, const char* const* words, const char* between) {
    ClarityBuffer out = {0};
    for (size_t i = 0; i < count; ++i) {
        if (i) clarity_buffer_add(&out, between);
        clarity_buffer_add(&out, words[i]);
    }
    return clarity_buffer_text(&out);
}

const char* clarity_cli_group_help(const ClarityCliProgram* program, size_t words, const char* const* path, ClarityCliWords say) {
    ClarityBuffer out = {0};
    if (words == 0 && program->help) clarity_buffer_addf(&out, "%s\n\n", program->help);
    clarity_buffer_addf(&out, "%s\n\n", group_usage(program, words, path, say));
    const char** left = CLARITY_NEW(const char*, program->command_count);
    const char** right = CLARITY_NEW(const char*, program->command_count);
    size_t count = 0;
    for (size_t i = 0; i < program->command_count; ++i) {
        const ClarityCliCommand* c = &program->commands[i];
        if (c->words > words && begins_with(c, words, path)) {
            left[count] = joined(c->words, c->line, " ");
            right[count++] = c->help ? c->help : "";
        }
    }
    section(&out, say0(say, "cliCommands"), count, left, right);
    clarity_buffer_add_char(&out, '\n');
    const char* help = "--help";
    const char* shows = say0(say, "cliHelpOption");
    section(&out, say0(say, "cliOptions"), 1, &help, &shows);
    return clarity_buffer_text(&out);
}

const char* clarity_cli_command_help(const ClarityCliProgram* program, const ClarityCliCommand* c, ClarityCliWords say) {
    ClarityBuffer out = {0};
    if (c->help) clarity_buffer_addf(&out, "%s\n\n", c->help);
    clarity_buffer_addf(&out, "%s\n\n", command_usage(program, c, say));
    const char** left = CLARITY_NEW(const char*, c->option_count + 1);
    const char** right = CLARITY_NEW(const char*, c->option_count + 1);
    for (size_t i = 0; i < c->option_count; ++i) {
        const ClarityCliParam* o = &c->options[i];
        left[i] = o->kind == CLARITY_CLI_BOOL ? clarity_format("--%s", o->name) : clarity_format("--%s <%s>", o->name, o->name);
        const char* marks[3];
        size_t count = 0;
        if (o->value_count) marks[count++] = joined(o->value_count, o->values, ", ");
        if (o->fallback && o->kind != CLARITY_CLI_BOOL) marks[count++] = say1(say, "cliDefault", "value", o->fallback);
        if (o->required) marks[count++] = say0(say, "cliRequiredMark");
        ClarityBuffer text = {0};
        if (o->help) clarity_buffer_add(&text, o->help);
        if (count) clarity_buffer_addf(&text, "%s(%s)", o->help && *o->help ? " " : "", joined(count, marks, "; "));
        right[i] = clarity_buffer_text(&text);
    }
    left[c->option_count] = "--help";
    right[c->option_count] = say0(say, "cliHelpOption");
    section(&out, say0(say, "cliOptions"), c->option_count + 1, left, right);
    return clarity_buffer_text(&out);
}

static ClarityCliParsed wrong(const ClarityCliProgram* program, size_t words, const char* const* path, const ClarityCliCommand* command, ClarityCliWords say,
                              const char* key, size_t count, const char* const* names, const char* const* values) {
    ClarityCliParsed parsed = {.outcome = CLARITY_CLI_WRONG, .key = key, .slot_count = count};
    for (size_t i = 0; i < count; ++i) {
        parsed.slot_names[i] = names[i];
        parsed.slot_values[i] = values[i];
    }
    const char* usage = command ? command_usage(program, command, say) : group_usage(program, words, path, say);
    parsed.said = clarity_format("%s\n%s\n", clarity_cli_say(say, key, count, names, values), usage);
    return parsed;
}

/// The option a word names, and the value written after `=`.
static long option_of(const ClarityCliCommand* c, const char* word, const char** value) {
    *value = NULL;
    if (!starts(word, "--")) return -1;
    const char* equals = strchr(word, '=');
    size_t length = equals ? (size_t)(equals - word) - 2 : strlen(word) - 2;
    for (size_t i = 0; i < c->option_count; ++i) {
        if (strlen(c->options[i].name) == length && strncmp(c->options[i].name, word + 2, length) == 0) {
            if (equals) *value = equals + 1;
            return (long)i;
        }
    }
    return -1;
}

static const char* option_name(const char* word) {
    const char* equals = strchr(word, '=');
    return equals ? clarity_copy_n(word, (size_t)(equals - word)) : word;
}

ClarityCliParsed clarity_cli_parse(const ClarityCliProgram* program, size_t count, const char* const* args, ClarityCliWords say) {
    size_t end = count;
    for (size_t i = 0; i < count; ++i) {
        if (strcmp(args[i], "--") == 0) {
            end = i;
            break;
        }
    }
    bool asked_help = false;
    const char** rest = CLARITY_NEW(const char*, count + 1);
    size_t rest_count = 0;
    for (size_t i = 0; i < count; ++i) {
        if (i < end && strcmp(args[i], "--help") == 0) {
            asked_help = true;
        } else {
            rest[rest_count++] = args[i];
        }
    }
    size_t at = 0;
    if (rest_count > 0 && strcmp(rest[0], "help") == 0) {
        asked_help = true;
        at = 1;
    }
    // The command, or the group read so far.
    const char** path = CLARITY_NEW(const char*, rest_count + 1);
    size_t words = 0;
    const ClarityCliCommand* c = NULL;
    size_t command = 0;
    while (!c) {
        if (at >= rest_count || starts(rest[at], "--")) {
            if (at < rest_count && !asked_help && words == 0) {
                const char* names[] = {"option"};
                const char* values[] = {option_name(rest[at])};
                return wrong(program, 0, path, NULL, say, "cliUnknownOption", 1, names, values);
            }
            return (ClarityCliParsed){.outcome = CLARITY_CLI_HELP, .help = clarity_cli_group_help(program, words, path, say)};
        }
        path[words] = rest[at];
        bool group = false;
        for (size_t i = 0; i < program->command_count; ++i) {
            const ClarityCliCommand* candidate = &program->commands[i];
            if (candidate->words == words + 1 && begins_with(candidate, words + 1, path)) {
                c = candidate;
                command = i;
            } else if (candidate->words > words + 1 && begins_with(candidate, words + 1, path)) {
                group = true;
            }
        }
        if (!c && !group) {
            const char* names[] = {"command"};
            const char* values[] = {joined(words + 1, path, " ")};
            return wrong(program, words, path, NULL, say, "cliUnknownCommand", 1, names, values);
        }
        ++words;
        ++at;
    }
    if (asked_help) return (ClarityCliParsed){.outcome = CLARITY_CLI_HELP, .help = clarity_cli_command_help(program, c, say)};
#define WRONG(key, n, names, values) wrong(program, 0, NULL, c, say, key, n, names, values)

    const char* const* after = rest + at;
    size_t after_count = rest_count - at;
    size_t ends_at = after_count;
    for (size_t i = 0; i < after_count; ++i) {
        if (strcmp(after[i], "--") == 0) {
            ends_at = i;
            break;
        }
    }
    for (size_t i = 0; i < ends_at; ++i) {
        const char* value;
        if (starts(after[i], "--") && option_of(c, after[i], &value) < 0) {
            const char* names[] = {"option"};
            const char* values[] = {option_name(after[i])};
            return WRONG("cliUnknownOption", 1, names, values);
        }
    }
    // The options given, the last of each kept, in the order typed.
    long* given = CLARITY_NEW(long, after_count + 1);
    const char** given_values = CLARITY_NEW(const char*, after_count + 1);
    size_t given_count = 0;
    const char** positional = CLARITY_NEW(const char*, after_count + 1);
    size_t positional_count = 0;
    for (size_t i = 0; i < after_count; ++i) {
        if (i == ends_at) {
            for (size_t j = i + 1; j < after_count; ++j) positional[positional_count++] = after[j];
            break;
        }
        const char* value;
        long index = option_of(c, after[i], &value);
        if (index < 0) {
            positional[positional_count++] = after[i];
            continue;
        }
        const ClarityCliParam* option = &c->options[index];
        if (!value && option->kind == CLARITY_CLI_BOOL) {
            value = "true";
        } else if (!value) {
            if (i + 1 >= after_count) {
                const char* names[] = {"option"};
                const char* values[] = {clarity_format("--%s", option->name)};
                return WRONG("cliNeedsValue", 1, names, values);
            }
            value = after[++i];
        }
        size_t kept = 0;
        for (size_t g = 0; g < given_count; ++g) {
            if (given[g] != index) {
                given[kept] = given[g];
                given_values[kept++] = given_values[g];
            }
        }
        given[kept] = index;
        given_values[kept] = value;
        given_count = kept + 1;
    }

    // Each value in the order it was typed: of its kind, then within its range.
    size_t read_count = (positional_count < c->argument_count ? positional_count : c->argument_count) + given_count;
    const char** names = CLARITY_NEW(const char*, read_count + 1);
    const ClarityCliParam** params = CLARITY_NEW(const ClarityCliParam*, read_count + 1);
    const char** typed = CLARITY_NEW(const char*, read_count + 1);
    const char** read = CLARITY_NEW(const char*, read_count + 1);
    size_t n = 0;
    for (size_t i = 0; i < c->argument_count && i < positional_count; ++i, ++n) {
        names[n] = c->arguments[i].name;
        params[n] = &c->arguments[i];
        typed[n] = positional[i];
    }
    for (size_t g = 0; g < given_count; ++g, ++n) {
        params[n] = &c->options[given[g]];
        names[n] = clarity_format("--%s", params[n]->name);
        typed[n] = given_values[g];
    }
    for (size_t i = 0; i < n; ++i) {
        read[i] = clarity_cli_read_value(params[i], typed[i]);
        if (!read[i]) {
            const char* slot_names[] = {"name", "value"};
            const char* slot_values[] = {names[i], typed[i]};
            return WRONG("cliWrongValue", 2, slot_names, slot_values);
        }
    }
    for (size_t i = 0; i < n; ++i) {
        const char* slot_names[2] = {"name", "min"};
        if (params[i]->min && below(read[i], params[i]->min)) {
            const char* slot_values[] = {names[i], params[i]->min};
            return WRONG("cliTooSmall", 2, slot_names, slot_values);
        }
        slot_names[1] = "max";
        if (params[i]->max && below(params[i]->max, read[i])) {
            const char* slot_values[] = {names[i], params[i]->max};
            return WRONG("cliTooLarge", 2, slot_names, slot_values);
        }
    }
    if (positional_count < c->argument_count) {
        const char* slot_names[] = {"argument"};
        const char* slot_values[] = {c->arguments[positional_count].name};
        return WRONG("cliMissingArgument", 1, slot_names, slot_values);
    }
    if (positional_count > c->argument_count) return WRONG("cliTooManyArguments", 0, NULL, NULL);
    for (size_t i = 0; i < c->option_count; ++i) {
        bool found = false;
        for (size_t g = 0; g < given_count; ++g) found = found || given[g] == (long)i;
        if (c->options[i].required && !found) {
            const char* slot_names[] = {"option"};
            const char* slot_values[] = {clarity_format("--%s", c->options[i].name)};
            return WRONG("cliRequired", 1, slot_names, slot_values);
        }
    }
#undef WRONG
    ClarityCliParsed parsed = {.outcome = CLARITY_CLI_RUN, .command = command};
    parsed.values = CLARITY_NEW(const char*, c->argument_count + c->option_count + 1);
    for (size_t i = 0; i < c->argument_count; ++i) parsed.values[i] = read[i];
    for (size_t i = 0; i < c->option_count; ++i) {
        const ClarityCliParam* o = &c->options[i];
        const char* value = o->fallback ? o->fallback : (o->kind == CLARITY_CLI_BOOL ? "false" : NULL);
        for (size_t g = 0; g < given_count; ++g) {
            if (given[g] == (long)i) value = read[c->argument_count + g];
        }
        parsed.values[c->argument_count + i] = value;
    }
    return parsed;
}

const char* clarity_cli_table(size_t columns, const char* const* headings, const bool* right, size_t rows, const char* const* cells) {
    size_t* widths = CLARITY_NEW(size_t, columns + 1);
    for (size_t col = 0; col < columns; ++col) {
        widths[col] = letters(headings[col]);
        for (size_t row = 0; row < rows; ++row) {
            size_t w = letters(cells[row * columns + col]);
            if (w > widths[col]) widths[col] = w;
        }
    }
    ClarityBuffer out = {0};
    for (size_t row = 0; row <= rows; ++row) {
        size_t start = out.length;
        for (size_t col = 0; col < columns; ++col) {
            const char* cell = row == 0 ? headings[col] : cells[(row - 1) * columns + col];
            size_t pad = widths[col] - letters(cell);
            if (col) clarity_buffer_add(&out, "  ");
            if (right[col]) add_spaces(&out, pad);
            clarity_buffer_add(&out, cell);
            if (!right[col]) add_spaces(&out, pad);
        }
        end_line(&out, start);
    }
    return clarity_buffer_text(&out);
}

/// A terminal, or Clarity's Output panel, which says so with CLARITY_TERMINAL=1.
static bool console_is_terminal(void* context) {
    (void)context;
    const char* told = getenv("CLARITY_TERMINAL");
    return clarity_isatty(0) || (told && strcmp(told, "1") == 0);
}

static const char* console_read_line(void* context) {
    (void)context;
    ClarityBuffer line = {0};
    int c;
    bool any = false;
    while ((c = getchar()) != EOF) {
        any = true;
        if (c == '\n') break;
        if (c != '\r') clarity_buffer_add_char(&line, (char)c);
    }
    return any ? clarity_buffer_text(&line) : NULL;
}

static void console_write(void* context, const char* text) {
    (void)context;
    fputs(text, stderr);
    fflush(stderr);
}

ClarityCliTerminal clarity_cli_console(void) {
    return (ClarityCliTerminal){console_is_terminal, console_read_line, console_write, NULL};
}

static bool same_words(const char* a, const char* b) {
    while (*a && *b && tolower((unsigned char)*a) == tolower((unsigned char)*b)) ++a, ++b;
    return !*a && !*b;
}

static const char* trimmed(const char* text) {
    while (*text == ' ' || *text == '\t') ++text;
    size_t length = strlen(text);
    while (length > 0 && (text[length - 1] == ' ' || text[length - 1] == '\t')) --length;
    return clarity_copy_n(text, length);
}

int clarity_cli_ask(const char* question, const ClarityCliAsked* asked, ClarityCliTerminal terminal, ClarityCliWords words, const char** answer) {
    *answer = NULL;
    if (!terminal.is_terminal(terminal.context)) {
        if (asked->kind == CLARITY_CLI_VALUE && asked->param.fallback) {
            *answer = asked->param.fallback;
            return 0;
        }
        terminal.write(terminal.context, clarity_format("%s\n", say1(words, "cliNoTerminal", "question", question)));
        return 2;
    }
    if (asked->kind == CLARITY_CLI_CONFIRM) {
        for (;;) {
            terminal.write(terminal.context, clarity_format("%s [%s/%s] ", question, asked->yes, asked->no));
            const char* line = terminal.read_line(terminal.context);
            line = line ? trimmed(line) : NULL;
            if (!line || !*line || same_words(line, asked->no)) {
                *answer = "false";
                return 0;
            }
            if (same_words(line, asked->yes)) {
                *answer = "true";
                return 0;
            }
        }
    }
    if (asked->kind == CLARITY_CLI_CHOICE) {
        ClarityBuffer listed = {0};
        for (size_t i = 0; i < asked->choice_count; ++i) clarity_buffer_addf(&listed, "%zu. %s\n", i + 1, asked->choices[i]);
        terminal.write(terminal.context, clarity_buffer_text(&listed));
        for (;;) {
            terminal.write(terminal.context, clarity_format("%s ", question));
            const char* line = terminal.read_line(terminal.context);
            if (!line) {
                terminal.write(terminal.context, "\n");
                return 1;
            }
            line = trimmed(line);
            if (plain_digits(line, strlen(line)) && strlen(line) < 10) {
                long n = strtol(line, NULL, 10);
                if (n >= 1 && (size_t)n <= asked->choice_count) {
                    *answer = clarity_format("%ld", n);
                    return 0;
                }
            }
            terminal.write(terminal.context, clarity_format("%s\n", say1(words, "cliChooseNumber", "count", clarity_format("%zu", asked->choice_count))));
        }
    }
    const ClarityCliParam* param = &asked->param;
    const char* shown = param->kind == CLARITY_CLI_ENUM ? joined(param->value_count, param->values, "/") : param->fallback;
    for (;;) {
        terminal.write(terminal.context, shown ? clarity_format("%s [%s] ", question, shown) : clarity_format("%s ", question));
        const char* line = terminal.read_line(terminal.context);
        if (!line) {
            terminal.write(terminal.context, "\n");
            return 1;
        }
        line = trimmed(line);
        if (!*line && param->fallback) {
            *answer = param->fallback;
            return 0;
        }
        const char* value = clarity_cli_read_value(param, line);
        if (value) {
            *answer = value;
            return 0;
        }
        terminal.write(terminal.context, clarity_format("%s\n", say1(words, "cliWrongAnswer", "value", line)));
    }
}

#include "clarity/text.h"

static const char* translated(const char* key, void* context) {
    (void)context;
    return clarity_key_value(key);
}

ClarityCliWords clarity_cli_words(void) {
    return (ClarityCliWords){translated, NULL};
}

void clarity_cli_print(const ClarityText* text) {
    puts(clarity_text_show(text));
}

void clarity_cli_print_table(size_t columns, const ClarityText* const* headings, const bool* right, size_t rows, const char* const* cells) {
    const char** shown = CLARITY_NEW(const char*, columns + 1);
    for (size_t i = 0; i < columns; ++i) shown[i] = clarity_text_show(headings[i]);
    fputs(clarity_cli_table(columns, shown, right, rows, cells), stdout);
}

void clarity_cli_message(const ClarityText* text) {
    fprintf(stderr, "%s\n", clarity_text_show(text));
}

const char* clarity_cli_yes_no(bool value) {
    const char* word = clarity_key_value(value ? "yes" : "no");
    return word && *word ? word : (value ? "Yes" : "No");
}

static const char* answer(const ClarityText* question, const ClarityCliAsked* asked) {
    const char* given = NULL;
    int code = clarity_cli_ask(clarity_text_show(question), asked, clarity_cli_console(), clarity_cli_words(), &given);
    // Nobody to answer, or no answer: the program ends here.
    if (code != 0) exit(code);
    return given;
}

bool clarity_cli_confirm(const ClarityText* question, const ClarityText* yes, const ClarityText* no) {
    const char* ok = clarity_key_value("ok");
    const char* cancel = clarity_key_value("cancel");
    ClarityCliAsked asked = {
        .kind = CLARITY_CLI_CONFIRM,
        .yes = yes ? clarity_text_show(yes) : (ok && *ok ? ok : "OK"),
        .no = no ? clarity_text_show(no) : (cancel && *cancel ? cancel : "Cancel"),
    };
    return strcmp(answer(question, &asked), "true") == 0;
}

const char* clarity_cli_ask_value(const ClarityText* question, ClarityCliKind kind, const char* const* values, size_t value_count, const char* fallback) {
    ClarityCliAsked asked = {.kind = CLARITY_CLI_VALUE, .param = {.kind = kind, .values = values, .value_count = value_count, .fallback = fallback}};
    return answer(question, &asked);
}

size_t clarity_cli_choose(const ClarityText* question, const char* const* choices, size_t count) {
    ClarityCliAsked asked = {.kind = CLARITY_CLI_CHOICE, .choices = choices, .choice_count = count};
    return (size_t)strtoul(answer(question, &asked), NULL, 10) - 1;
}

void clarity_cli_failed(void) {
    const ClarityError* error = clarity_error();
    fprintf(stderr, "%s\n", error->kind == CLARITY_REFUSED && error->text ? clarity_text_show(error->text) : error->message);
}

const char* clarity_cli_pick_language(const char* locale, const char* const* available, size_t count, const char* base) {
    if (!locale) return NULL;
    size_t length = strcspn(locale, ".@");
    char* tag = clarity_copy_n(locale, length);
    for (char* at = tag; *at; ++at) if (*at == '_') *at = '-';
    size_t primary = strcspn(tag, "-");
    size_t base_primary = strcspn(base, "-");
    if (!*tag || strcmp(tag, "C") == 0 || strcmp(tag, "POSIX") == 0) return NULL;
    if (primary == base_primary && strncmp(tag, base, primary) == 0) return NULL;
    for (size_t i = 0; i < count; ++i) if (strcmp(available[i], tag) == 0) return available[i];
    for (size_t i = 0; i < count; ++i) if (strcspn(available[i], "-") == primary && strncmp(available[i], tag, primary) == 0) return available[i];
    return NULL;
}

const char* clarity_cli_text(const char* text) {
    return text ? text : "";
}

int64_t clarity_cli_integer(const char* text) {
    return text ? strtoll(text, NULL, 10) : 0;
}

bool clarity_cli_bool(const char* text) {
    return text && strcmp(text, "true") == 0;
}

ClarityDecimal clarity_cli_decimal(const char* text) {
    ClarityDecimal d = {0, 0};
    if (text) clarity_decimal_parse(text, &d);
    return d;
}

ClarityDate clarity_cli_date(const char* text) {
    ClarityDate d = {0};
    if (text) clarity_date_parse(text, &d);
    return d;
}

ClarityDateTime clarity_cli_date_time(const char* text) {
    ClarityDateTime t = {0};
    if (text) clarity_date_time_parse(text, &t);
    return t;
}

const char* clarity_cli_decimal_text(ClarityDecimal value) {
    return normalized(value);
}
