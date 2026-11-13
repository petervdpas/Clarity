// A command-line program's commands for agents (docs/ledger/app-mcp.md),
// served through Clarity.AppMcp: a call runs the command as if typed; what
// it prints is the answer; its questions go to the agent; refusals, exit
// codes and unanswered questions are tool errors; while the app is closed
// the person's settings decide.

#include <string.h>

#include "agents/agents.h"
#include "check.h"
#include "clarity/text.h"

static const char* TOOLS =
    "{\"name\":\"shop\",\"instructions\":\"\",\"tools\":["
    "{\"name\":\"customer-add\",\"description\":\"Adds\",\"inputSchema\":{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"limit\":{\"type\":\"number\"}},\"required\":[\"name\"],\"additionalProperties\":false},"
    "\"changes\":{\"data\":true,\"files\":false,\"settings\":false,\"net\":false},\"allow\":false,\"line\":[\"customer\",\"add\"],\"arguments\":[\"name\"]},"
    "{\"name\":\"customer-remove\",\"description\":\"Removes\",\"inputSchema\":{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"],\"additionalProperties\":false},"
    "\"changes\":{\"data\":true,\"files\":false,\"settings\":false,\"net\":false},\"allow\":false,\"line\":[\"customer\",\"remove\"],\"arguments\":[\"name\"]},"
    "{\"name\":\"report-send\",\"description\":\"Sends\",\"inputSchema\":{\"type\":\"object\",\"properties\":{},\"required\":[],\"additionalProperties\":false},"
    "\"changes\":{\"data\":false,\"files\":false,\"settings\":false,\"net\":true},\"allow\":false,\"line\":[\"report\",\"send\"],\"arguments\":[]}]}";

static ClarityCliProgram agent_program(void) {
    static const char* const add_line[] = {"customer", "add"};
    static const char* const remove_line[] = {"customer", "remove"};
    static const char* const send_line[] = {"report", "send"};
    static const ClarityCliParam name[] = {{.name = "name", .kind = CLARITY_CLI_TEXT}};
    static const ClarityCliParam limit[] = {{.name = "limit", .kind = CLARITY_CLI_MONEY, .fallback = "1000"}};
    static const ClarityCliCommand commands[] = {
        {.line = add_line, .words = 2, .arguments = name, .argument_count = 1, .options = limit, .option_count = 1},
        {.line = remove_line, .words = 2, .arguments = name, .argument_count = 1},
        {.line = send_line, .words = 2},
    };
    return (ClarityCliProgram){.name = "shop", .commands = commands, .command_count = 3};
}

static bool agent_run(void* context, size_t command, const char** values, int* code) {
    (void)context;
    if (command == 0) {
        clarity_cli_print(clarity_plain_text(clarity_format("Added %s, limit %s", values[0], values[1])));
    } else if (command == 1) {
        const char* copies = clarity_cli_ask_value(clarity_plain_text("How many copies?"), CLARITY_CLI_INTEGER, NULL, 0, NULL);
        if (!clarity_cli_confirm(clarity_plain_text(clarity_format("Remove %s?", values[0])), NULL, NULL)) return clarity_refuse(clarity_plain_text("Kept"));
        clarity_cli_print(clarity_plain_text(clarity_format("Removed %s after %s copies", values[0], copies)));
    } else {
        *code = 3;
    }
    return true;
}

typedef struct {
    const char* const* sends;
    size_t next;
    cJSON* written;
} Script;

static const char* script_read(void* context) {
    Script* script = context;
    return script->sends[script->next] ? script->sends[script->next++] : NULL;
}

static void script_write(void* context, const char* line) {
    cJSON_AddItemToArray(((Script*)context)->written, cJSON_Parse(line));
}

/// What `<program> mcp` wrote for `sends`, with the person's settings `kept`.
static cJSON* served(const char* kept, const char* const* sends) {
    Script script = {sends, 0, cJSON_CreateArray()};
    ClarityAgentsLines lines = {script_read, script_write, &script};
    ClarityCliProgram program = agent_program();
    ClarityAgents agents = {.tools = TOOLS, .app = "Shop", .program = "shop", .version = "1.0"};
    cJSON* settings = cJSON_Parse(kept);
    CHECK(clarity_agents_serve(&program, &agents, settings, &lines, agent_run, NULL));
    cJSON_Delete(settings);
    return script.written;
}

static const char* said(const cJSON* reply, bool* refused) {
    const cJSON* result = cJSON_GetObjectItem(reply, "result");
    *refused = cJSON_IsTrue(cJSON_GetObjectItem(result, "isError"));
    return cJSON_GetObjectItem(cJSON_GetArrayItem(cJSON_GetObjectItem(result, "content"), 0), "text")->valuestring;
}

#define ALLOWED "{\"agentReadsWhileClosed\": true, \"agentChangesWhileClosed\": true}"

void an_agent_call_runs_the_command_as_typed(void) {
    const char* sends[] = {
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"customer-add\",\"arguments\":{\"name\":\"Ada\"}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"customer-add\",\"arguments\":{\"name\":\"Bob\",\"limit\":200.5}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"report-send\",\"arguments\":{}}}",
        NULL,
    };
    cJSON* written = served(ALLOWED, sends);
    bool refused = true;
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 0), &refused), "Added Ada, limit 1000");
    CHECK(!refused);
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 1), &refused), "Added Bob, limit 200.5");
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 2), &refused), "exit 3");
    CHECK(refused);
    cJSON_Delete(written);
}

void an_agent_is_asked_and_a_command_stops_without_an_answer(void) {
    const char* sends[] = {
        "{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"initialize\",\"params\":{\"capabilities\":{\"elicitation\":{}}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"customer-remove\",\"arguments\":{\"name\":\"Ada\"}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":\"ask-1\",\"result\":{\"action\":\"accept\",\"content\":{\"answer\":2}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":\"ask-2\",\"result\":{\"action\":\"accept\",\"content\":{\"answer\":true}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"customer-remove\",\"arguments\":{\"name\":\"Ada\"}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":\"ask-3\",\"result\":{\"action\":\"accept\",\"content\":{\"answer\":1}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":\"ask-4\",\"result\":{\"action\":\"accept\",\"content\":{\"answer\":false}}}",
        NULL,
    };
    cJSON* written = served(ALLOWED, sends);
    bool refused = true;
    CHECK_INT(cJSON_GetArraySize(written), 7);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetArrayItem(written, 1), "params"), "requestedSchema"), "properties"), "answer"), "{\"type\":\"integer\"}");
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 3), &refused), "Removed Ada after 2 copies");
    CHECK(!refused);
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 6), &refused), "Kept");
    CHECK(refused);
    cJSON_Delete(written);
    // A client that cannot ask: the value without a default has no answer, and the command stops.
    const char* unasked[] = {"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"customer-remove\",\"arguments\":{\"name\":\"Ada\"}}}", NULL};
    written = served(ALLOWED, unasked);
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 0), &refused), "There is nobody to answer: How many copies?");
    CHECK(refused);
    cJSON_Delete(written);
}

void while_the_app_is_closed_the_person_s_settings_decide(void) {
    const char* sends[] = {"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"customer-add\",\"arguments\":{\"name\":\"Ada\"}}}", NULL};
    cJSON* written = served("{}", sends);
    bool refused = false;
    CHECK_TEXT(said(cJSON_GetArrayItem(written, 0), &refused), "Shop is not open; open it to allow this.");
    CHECK(refused);
    cJSON_Delete(written);
}

void the_words_of_refusals_are_the_shared_ones(void) {
    cJSON* cases = conformance("app-mcp.json");
    const cJSON* words = cJSON_GetObjectItem(cases, "words");
    CHECK_INT(cJSON_GetArraySize(words), (int)CLARITY_AGENT_WORD_COUNT);
    for (size_t i = 0; i < CLARITY_AGENT_WORD_COUNT; ++i)
        CHECK_TEXT(cJSON_GetObjectItem(words, CLARITY_AGENT_WORDS[i].key)->valuestring, CLARITY_AGENT_WORDS[i].english);
    cJSON_Delete(cases);
}
