// Command-line programs against Clarity's shared cases
// (conformance/command-line.json): reading a command line, the help, a
// printed table and asking on a terminal.

#include <stdlib.h>

#include "check.h"
#include "clarity/cli.h"
#include "tests.h"

static const char* text_of(const cJSON* object, const char* name) {
    const cJSON* item = cJSON_GetObjectItemCaseSensitive(object, name);
    return cJSON_IsString(item) ? item->valuestring : NULL;
}

static const char* const* texts(const cJSON* array, size_t* count) {
    *count = (size_t)cJSON_GetArraySize(array);
    const char** out = CLARITY_NEW(const char*, *count + 1);
    for (size_t i = 0; i < *count; ++i) out[i] = cJSON_GetArrayItem(array, (int)i)->valuestring;
    return out;
}

static ClarityCliKind kind_of(const char* word) {
    static const char* const kinds[] = {"text", "integer", "decimal", "money", "date", "datetime", "duration", "enum", "bool"};
    for (size_t i = 0; i < sizeof kinds / sizeof kinds[0]; ++i) {
        if (strcmp(kinds[i], word) == 0) return (ClarityCliKind)i;
    }
    return CLARITY_CLI_TEXT;
}

static ClarityCliParam param_of(const cJSON* p) {
    // An asked value names its kind as `type`; `kind` is the form's there.
    ClarityCliParam param = {.name = text_of(p, "name"), .kind = kind_of(text_of(p, "type") ? text_of(p, "type") : text_of(p, "kind"))};
    const cJSON* values = cJSON_GetObjectItem(p, "values");
    if (values) param.values = texts(values, &param.value_count);
    param.fallback = text_of(p, "default");
    param.min = text_of(p, "min");
    param.max = text_of(p, "max");
    param.required = cJSON_IsTrue(cJSON_GetObjectItem(p, "required"));
    param.help = text_of(p, "help");
    return param;
}

static const ClarityCliParam* params_of(const cJSON* array, size_t* count) {
    *count = (size_t)cJSON_GetArraySize(array);
    ClarityCliParam* out = CLARITY_NEW(ClarityCliParam, *count + 1);
    for (size_t i = 0; i < *count; ++i) out[i] = param_of(cJSON_GetArrayItem(array, (int)i));
    return out;
}

static ClarityCliProgram program_of(const cJSON* p) {
    ClarityCliProgram program = {.name = text_of(p, "name"), .help = text_of(p, "help")};
    const cJSON* commands = cJSON_GetObjectItem(p, "commands");
    program.command_count = (size_t)cJSON_GetArraySize(commands);
    ClarityCliCommand* out = CLARITY_NEW(ClarityCliCommand, program.command_count + 1);
    for (size_t i = 0; i < program.command_count; ++i) {
        const cJSON* c = cJSON_GetArrayItem(commands, (int)i);
        out[i].line = texts(cJSON_GetObjectItem(c, "line"), &out[i].words);
        out[i].help = text_of(c, "help");
        out[i].arguments = params_of(cJSON_GetObjectItem(c, "arguments"), &out[i].argument_count);
        out[i].options = params_of(cJSON_GetObjectItem(c, "options"), &out[i].option_count);
    }
    program.commands = out;
    return program;
}

static const ClarityCliWords english = {NULL, NULL};

void the_command_line_words_are_the_shared_ones(void) {
    cJSON* cases = conformance("command-line.json");
    REQUIRE(cases);
    const cJSON* words = cJSON_GetObjectItem(cases, "words");
    CHECK((size_t)cJSON_GetArraySize(words) == CLARITY_CLI_WORD_COUNT);
    for (size_t i = 0; i < CLARITY_CLI_WORD_COUNT; ++i) CHECK_TEXT(CLARITY_CLI_WORDS[i].english, text_of(words, CLARITY_CLI_WORDS[i].key));
    cJSON_Delete(cases);
}

void command_lines_are_read_as_every_runtime_reads_them(void) {
    cJSON* cases = conformance("command-line.json");
    REQUIRE(cases);
    ClarityCliProgram program = program_of(cJSON_GetObjectItem(cases, "program"));
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "parse")) {
        const char* what = text_of(c, "what");
        size_t count;
        const char* const* args = texts(cJSON_GetObjectItem(c, "args"), &count);
        ClarityCliParsed parsed = clarity_cli_parse(&program, count, args, english);
        const cJSON* run = cJSON_GetObjectItem(c, "run");
        const cJSON* help = cJSON_GetObjectItem(c, "help");
        if (run) {
            if (parsed.outcome != CLARITY_CLI_RUN) {
                check_fail(__FILE__, __LINE__, what);
                continue;
            }
            const ClarityCliCommand* command = &program.commands[parsed.command];
            cJSON* values = cJSON_CreateObject();
            for (size_t i = 0; i < command->argument_count + command->option_count; ++i) {
                const ClarityCliParam* p = i < command->argument_count ? &command->arguments[i] : &command->options[i - command->argument_count];
                cJSON_AddItemToObject(values, p->name, parsed.values[i] ? cJSON_CreateString(parsed.values[i]) : cJSON_CreateNull());
            }
            if (!cJSON_Compare(values, cJSON_GetObjectItem(run, "values"), 1)) check_fail(__FILE__, __LINE__, clarity_format("%s: %s", what, cJSON_PrintUnformatted(values)));
            CHECK_TEXT(clarity_format("%s%s%s", command->line[0], command->words > 1 ? " " : "", command->words > 1 ? command->line[1] : ""), text_of(run, "command"));
            cJSON_Delete(values);
        } else if (help) {
            if (parsed.outcome != CLARITY_CLI_HELP) check_fail(__FILE__, __LINE__, what);
            else CHECK_TEXT(parsed.help, help->valuestring);
        } else {
            const cJSON* wanted = cJSON_GetObjectItem(c, "wrong");
            if (parsed.outcome != CLARITY_CLI_WRONG) {
                check_fail(__FILE__, __LINE__, what);
                continue;
            }
            CHECK_TEXT(parsed.key, text_of(wanted, "key"));
            cJSON* slots = cJSON_CreateObject();
            for (size_t i = 0; i < parsed.slot_count; ++i) cJSON_AddStringToObject(slots, parsed.slot_names[i], parsed.slot_values[i]);
            if (!cJSON_Compare(slots, cJSON_GetObjectItem(wanted, "slots"), 1)) check_fail(__FILE__, __LINE__, what);
            cJSON_Delete(slots);
            CHECK_TEXT(parsed.said, text_of(wanted, "said"));
        }
    }
    cJSON_Delete(cases);
}

void a_printed_table_lines_up_as_every_runtime_does(void) {
    cJSON* cases = conformance("command-line.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "table")) {
        size_t columns, rows;
        const char* const* headings = texts(cJSON_GetObjectItem(c, "headings"), &columns);
        bool* right = CLARITY_NEW(bool, columns + 1);
        for (size_t i = 0; i < columns; ++i) right[i] = cJSON_IsTrue(cJSON_GetArrayItem(cJSON_GetObjectItem(c, "right"), (int)i));
        const cJSON* row_items = cJSON_GetObjectItem(c, "rows");
        rows = (size_t)cJSON_GetArraySize(row_items);
        const char** cells = CLARITY_NEW(const char*, rows * columns + 1);
        for (size_t r = 0; r < rows; ++r) {
            for (size_t col = 0; col < columns; ++col) cells[r * columns + col] = cJSON_GetArrayItem(cJSON_GetArrayItem(row_items, (int)r), (int)col)->valuestring;
        }
        CHECK_TEXT(clarity_cli_table(columns, headings, right, rows, cells), text_of(c, "said"));
    }
    cJSON_Delete(cases);
}

typedef struct {
    bool terminal;
    const cJSON* lines;
    int next;
    ClarityBuffer said;
} Typed;

static bool typed_is_terminal(void* context) {
    return ((Typed*)context)->terminal;
}

static const char* typed_read_line(void* context) {
    Typed* t = context;
    const cJSON* line = cJSON_GetArrayItem(t->lines, t->next++);
    return cJSON_IsString(line) ? line->valuestring : NULL;
}

static void typed_write(void* context, const char* text) {
    clarity_buffer_add(&((Typed*)context)->said, text);
}

void a_command_asks_on_a_terminal_as_every_runtime_does(void) {
    cJSON* cases = conformance("command-line.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "ask")) {
        const char* what = text_of(c, "what");
        const cJSON* form = cJSON_GetObjectItem(c, "form");
        const char* kind = text_of(form, "kind");
        ClarityCliAsked asked = {.yes = text_of(form, "yes"), .no = text_of(form, "no")};
        if (strcmp(kind, "confirm") == 0) {
            asked.kind = CLARITY_CLI_CONFIRM;
        } else if (strcmp(kind, "choice") == 0) {
            asked.kind = CLARITY_CLI_CHOICE;
            asked.choices = texts(cJSON_GetObjectItem(form, "choices"), &asked.choice_count);
        } else {
            asked.kind = CLARITY_CLI_VALUE;
            asked.param = param_of(form);
        }
        Typed typed = {.terminal = cJSON_IsTrue(cJSON_GetObjectItem(c, "terminal")), .lines = cJSON_GetObjectItem(c, "typed")};
        ClarityCliTerminal terminal = {typed_is_terminal, typed_read_line, typed_write, &typed};
        const char* answer;
        int exit = clarity_cli_ask(text_of(c, "question"), &asked, terminal, english, &answer);
        const cJSON* code = cJSON_GetObjectItem(c, "exit");
        if (code) {
            if (exit != code->valueint) check_fail(__FILE__, __LINE__, what);
        } else if (exit != 0) {
            check_fail(__FILE__, __LINE__, what);
        } else {
            CHECK_TEXT(answer, text_of(c, "answer"));
        }
        CHECK_TEXT(clarity_buffer_text(&typed.said), text_of(c, "said"));
    }
    cJSON_Delete(cases);
}
