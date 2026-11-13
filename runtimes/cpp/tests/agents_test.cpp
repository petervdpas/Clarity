// A command-line program's commands for agents (docs/ledger/app-mcp.md),
// served through Clarity.AppMcp: a call runs the command as if typed; what
// it prints is the answer; its questions go to the agent; refusals, exit
// codes and unanswered questions are tool errors; while the app is closed
// the person's settings decide.

#include <deque>

#include "agents/agents.hpp"
#include "check.hpp"
#include "clarity/errors.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

const char* agent_tools = R"({"name":"shop","instructions":"","tools":[
{"name":"customer-add","description":"Adds","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"limit":{"type":"number"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","add"],"arguments":["name"]},
{"name":"customer-remove","description":"Removes","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","remove"],"arguments":["name"]},
{"name":"report-send","description":"Sends","inputSchema":{"type":"object","properties":{},"required":[],"additionalProperties":false},"changes":{"data":false,"files":false,"settings":false,"net":true},"allow":false,"line":["report","send"],"arguments":[]}]})";

cli::Program agent_program() {
    cli::Program program;
    program.name = "shop";
    cli::Param name;
    name.name = "name";
    cli::Param limit;
    limit.name = "limit";
    limit.kind = cli::Kind::money;
    limit.fallback = "1000";
    cli::Command add;
    add.line = {"customer", "add"};
    add.arguments = {name};
    add.options = {limit};
    cli::Command remove;
    remove.line = {"customer", "remove"};
    remove.arguments = {name};
    cli::Command send;
    send.line = {"report", "send"};
    program.commands = {add, remove, send};
    return program;
}

int agent_run(cli::Session& cli, size_t command, const std::vector<std::optional<std::string>>& values) {
    if (command == 0) {
        cli.print(Text::plain("Added " + *values[0] + ", limit " + *values[1]));
    } else if (command == 1) {
        const auto copies = cli.ask(Text::plain("How many copies?"), cli::Kind::integer, {}, std::nullopt);
        if (!cli.confirm(Text::plain("Remove " + *values[0] + "?"), std::nullopt, std::nullopt)) throw RefusedError(Text::plain("Kept"));
        cli.print(Text::plain("Removed " + *values[0] + " after " + copies + " copies"));
    } else {
        return 3;
    }
    return 0;
}

std::vector<json> served(const std::string& kept, std::deque<std::string> sends) {
    std::vector<json> written;
    agents::Lines lines{
        [&]() -> std::optional<std::string> {
            if (sends.empty()) return std::nullopt;
            auto next = sends.front();
            sends.pop_front();
            return next;
        },
        [&](const std::string& line) { written.push_back(json::parse(line)); },
    };
    agents::serve(agent_program(), agents::Agents{agent_tools, "Shop", "shop", "1.0"}, json::parse(kept), lines, agent_run);
    return written;
}

std::pair<std::string, bool> said(const json& reply) {
    return {reply["result"]["content"][0]["text"].get<std::string>(), reply["result"]["isError"].get<bool>()};
}

const std::string allowed = R"({"agentReadsWhileClosed": true, "agentChangesWhileClosed": true})";

}  // namespace

TEST(an_agent_call_runs_the_command_as_typed) {
    auto written = served(allowed, {
                                       R"({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Ada"}}})",
                                       R"({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Bob","limit":200.5}}})",
                                       R"({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"report-send","arguments":{}}})",
                                   });
    CHECK(said(written[0]) == std::make_pair(std::string("Added Ada, limit 1000"), false));
    CHECK(said(written[1]) == std::make_pair(std::string("Added Bob, limit 200.5"), false));
    CHECK(said(written[2]) == std::make_pair(std::string("exit 3"), true));
}

TEST(an_agent_is_asked_and_a_command_stops_without_an_answer) {
    auto written = served(allowed, {
                                       R"({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{"elicitation":{}}}})",
                                       R"({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}})",
                                       R"({"jsonrpc":"2.0","id":"ask-1","result":{"action":"accept","content":{"answer":2}}})",
                                       R"({"jsonrpc":"2.0","id":"ask-2","result":{"action":"accept","content":{"answer":true}}})",
                                       R"({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}})",
                                       R"({"jsonrpc":"2.0","id":"ask-3","result":{"action":"accept","content":{"answer":1}}})",
                                       R"({"jsonrpc":"2.0","id":"ask-4","result":{"action":"accept","content":{"answer":false}}})",
                                   });
    CHECK(written.size() == 7u);
    CHECK(written[1]["params"]["requestedSchema"]["properties"]["answer"] == json::parse(R"({"type":"integer"})"));
    CHECK(said(written[3]) == std::make_pair(std::string("Removed Ada after 2 copies"), false));
    CHECK(said(written[6]) == std::make_pair(std::string("Kept"), true));
    written = served(allowed, {R"({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}})"});
    CHECK(said(written[0]) == std::make_pair(std::string("There is nobody to answer: How many copies?"), true));
}

TEST(while_the_app_is_closed_the_person_s_settings_decide) {
    auto written = served("{}", {R"({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Ada"}}})"});
    CHECK(said(written[0]) == std::make_pair(std::string("Shop is not open; open it to allow this."), true));
}

TEST(the_words_of_refusals_are_the_shared_ones) {
    json cases = conformance("app-mcp.json");
    CHECK(cases["words"].size() == agents::words.size());
    for (const auto& [key, english] : agents::words) CHECK(cases["words"][key] == english);
}
