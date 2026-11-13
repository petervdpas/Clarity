#include "agents/agents.h"

#include <stdint.h>
#include <string.h>

#include "clarity/disk.h"
#include "clarity/settings.h"
#include "clarity/text.h"
#include "clarity_app_mcp.h"

static const ClarityAppMcpApi* api(void) {
    return clarity_app_mcp_api();
}

/// A word of Clarity's in the language shown, with the app's title.
static const char* say(const char* key, const char* app) {
    const char* text = clarity_key_value(key);
    if (!text || !*text) {
        for (size_t i = 0; i < CLARITY_AGENT_WORD_COUNT; ++i)
            if (strcmp(CLARITY_AGENT_WORDS[i].key, key) == 0) text = CLARITY_AGENT_WORDS[i].english;
    }
    ClarityBuffer out = {0};
    for (const char* at = text; *at;) {
        if (strncmp(at, "{app}", 5) == 0) {
            clarity_buffer_add(&out, app);
            at += 5;
        } else {
            clarity_buffer_add_char(&out, *at++);
        }
    }
    return clarity_buffer_text(&out);
}

const char* clarity_agents_options(const ClarityAgents* agents, const cJSON* kept) {
    cJSON* options = cJSON_CreateObject();
    cJSON_AddStringToObject(options, "version", agents->version ? agents->version : "");
    cJSON_AddBoolToObject(options, "open", false);
    // A switch the person never set is left out: the service uses what the project allows.
    const cJSON* reads = cJSON_GetObjectItemCaseSensitive(kept, CLARITY_AGENT_READS_SETTING);
    const cJSON* changes = cJSON_GetObjectItemCaseSensitive(kept, CLARITY_AGENT_CHANGES_SETTING);
    if (cJSON_IsBool(reads)) cJSON_AddBoolToObject(options, "closedReads", cJSON_IsTrue(reads));
    if (cJSON_IsBool(changes)) cJSON_AddBoolToObject(options, "closedChanges", cJSON_IsTrue(changes));
    cJSON* texts = cJSON_AddObjectToObject(options, "texts");
    cJSON_AddStringToObject(texts, "refused", say("agentRefused", agents->app));
    cJSON_AddStringToObject(texts, "closed", say("agentClosed", agents->app));
    char* printed = cJSON_PrintUnformatted(options);
    const char* out = clarity_format("%s", printed);
    cJSON_free(printed);
    cJSON_Delete(options);
    return out;
}

/// What a serve runs with.
typedef struct {
    const ClarityCliProgram* program;
    const ClarityAgentsLines* lines;
    ClarityAgentsRun run;
    void* context;
} Serving;

/// What the library gave in `out`, copied to the scratch arena and given back.
static const char* taken(uint8_t* out, size_t length) {
    const char* text = clarity_copy_n((const char*)out, length);
    api()->free(out, length);
    return text;
}

static const char* ask(void* context, const char* question, const char* form) {
    uint8_t* out = NULL;
    size_t length = 0;
    if (api()->ask_cli((ClarityAppMcpCall*)context, (const uint8_t*)question, strlen(question), (const uint8_t*)form, strlen(form), &out, &length) != CLARITY_APP_MCP_OK) return NULL;
    return taken(out, length);
}

static void answer(ClarityAppMcpCall* call, const char* text, bool refused) {
    size_t length = strlen(text);
    while (length > 0 && (text[length - 1] == '\n' || text[length - 1] == ' ')) --length;
    if (refused) {
        api()->refused(call, (const uint8_t*)text, length);
    } else {
        api()->printed(call, (const uint8_t*)text, length);
    }
}

static void run(void* user, ClarityAppMcpCall* call, const uint8_t* tool, size_t tool_len, const uint8_t* arguments, size_t arguments_len) {
    (void)tool, (void)tool_len, (void)arguments, (void)arguments_len;
    const Serving* serving = user;
    uint8_t* out = NULL;
    size_t length = 0;
    if (api()->command_line(call, &out, &length) != CLARITY_APP_MCP_OK) return answer(call, "", true);
    cJSON* line = cJSON_Parse(taken(out, length));
    size_t count = (size_t)cJSON_GetArraySize(line);
    const char** words = CLARITY_NEW(const char*, count + 1);
    for (size_t i = 0; i < count; ++i) words[i] = clarity_format("%s", cJSON_GetArrayItem(line, (int)i)->valuestring);
    cJSON_Delete(line);
    const char* text = "";
    bool refused = clarity_cli_run_line(serving->program, count, words, ask, call, serving->run, serving->context, &text);
    answer(call, text, refused);
}

/// While the app is closed nobody is there to ask.
static int32_t grant(void* user, const uint8_t* text, size_t length) {
    (void)user, (void)text, (void)length;
    return CLARITY_APP_MCP_REFUSE;
}

static int32_t read_line(void* user, const uint8_t** line, size_t* length) {
    const Serving* serving = user;
    const char* next = serving->lines->read(serving->lines->context);
    if (!next) return 1;
    *line = (const uint8_t*)next;
    *length = strlen(next);
    return 0;
}

static void write_line(void* user, const uint8_t* line, size_t length) {
    const Serving* serving = user;
    serving->lines->write(serving->lines->context, clarity_copy_n((const char*)line, length));
}

bool clarity_agents_forward(const char* program) {
    const char* settings = clarity_settings_file(program);
    if (!settings) return false;
    const char* file = clarity_format("%s/running.json", clarity_parent(settings));
    return api()->forward((const uint8_t*)file, strlen(file)) == 1;
}

bool clarity_agents_serve(const ClarityCliProgram* program, const ClarityAgents* agents, const cJSON* kept, const ClarityAgentsLines* lines, ClarityAgentsRun run_command, void* context) {
    Serving serving = {program, lines, run_command, context};
    ClarityAppMcpHost host = {&serving, lines ? read_line : NULL, lines ? write_line : NULL, run, grant};
    const char* options = clarity_agents_options(agents, kept);
    if (api()->serve((const uint8_t*)agents->tools, strlen(agents->tools), (const uint8_t*)options, strlen(options), &host) != CLARITY_APP_MCP_OK) {
        char why[1024];
        api()->problem(why, sizeof why);
        return clarity_fail(CLARITY_INVALID, "%s", why);
    }
    return true;
}
