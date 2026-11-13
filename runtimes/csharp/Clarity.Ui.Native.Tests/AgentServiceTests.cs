// A command-line program's commands for agents (docs/ledger/app-mcp.md),
// served through Clarity.AppMcp: a call runs the command as if typed; what
// it prints is the answer; its questions go to the agent; refusals, exit
// codes and unanswered questions are tool errors; while the app is closed
// the person's settings decide.

using System.Text.Json.Nodes;
using Clarity.AppMcp;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

public class AgentServiceTests
{
    const string Tools = """
        {"name":"shop","instructions":"","tools":[
        {"name":"customer-add","description":"Adds","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"limit":{"type":"number"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","add"],"arguments":["name"]},
        {"name":"customer-remove","description":"Removes","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","remove"],"arguments":["name"]},
        {"name":"report-send","description":"Sends","inputSchema":{"type":"object","properties":{},"required":[],"additionalProperties":false},"changes":{"data":false,"files":false,"settings":false,"net":true},"allow":false,"line":["report","send"],"arguments":[]}]}
        """;

    static readonly CliParam Name = new("name", CliKind.Text);

    static readonly CliProgram Program = new("shop", null,
    [
        new(["customer", "add"], null, [Name], [new CliParam("limit", CliKind.Money) { Default = "1000" }]),
        new(["customer", "remove"], null, [Name], []),
        new(["report", "send"], null, [], []),
    ]);

    static int Run(CliSession cli, int command, IReadOnlyList<string?> values)
    {
        switch (command)
        {
            case 0:
                cli.Print(Text.Plain($"Added {values[0]}, limit {values[1]}"));
                return 0;
            case 1:
                var copies = cli.Ask(Text.Plain("How many copies?"), CliKind.Integer, [], null);
                if (!cli.Confirm(Text.Plain($"Remove {values[0]}?"), null, null))
                    throw new RefusedException(Text.Plain("Kept"));
                cli.Print(Text.Plain($"Removed {values[0]} after {copies} copies"));
                return 0;
            default:
                return 3;
        }
    }

    static List<JsonNode> Served(string kept, params string[] sends)
    {
        var queue = new Queue<string>(sends);
        var written = new List<JsonNode>();
        AgentService.Serve(Program, new Agents(Tools, "Shop", "shop", "1.0"), JsonNode.Parse(kept)!.AsObject(), new AgentLines(() => queue.Count > 0 ? queue.Dequeue() : null, line => written.Add(JsonNode.Parse(line)!)), Run);
        return written;
    }

    static (string, bool) Said(JsonNode reply) => (reply["result"]!["content"]![0]!["text"]!.GetValue<string>(), reply["result"]!["isError"]!.GetValue<bool>());

    const string Allowed = """{"agentReadsWhileClosed": true, "agentChangesWhileClosed": true}""";

    [Fact]
    public void An_agent_call_runs_the_command_as_typed()
    {
        var written = Served(Allowed,
            """{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Ada"}}}""",
            """{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Bob","limit":200.5}}}""",
            """{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"report-send","arguments":{}}}""");
        Assert.Equal(("Added Ada, limit 1000", false), Said(written[0]));
        Assert.Equal(("Added Bob, limit 200.5", false), Said(written[1]));
        Assert.Equal(("exit 3", true), Said(written[2]));
    }

    [Fact]
    public void An_agent_is_asked_and_a_command_stops_without_an_answer()
    {
        var written = Served(Allowed,
            """{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{"elicitation":{}}}}""",
            """{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}}""",
            """{"jsonrpc":"2.0","id":"ask-1","result":{"action":"accept","content":{"answer":2}}}""",
            """{"jsonrpc":"2.0","id":"ask-2","result":{"action":"accept","content":{"answer":true}}}""",
            """{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}}""",
            """{"jsonrpc":"2.0","id":"ask-3","result":{"action":"accept","content":{"answer":1}}}""",
            """{"jsonrpc":"2.0","id":"ask-4","result":{"action":"accept","content":{"answer":false}}}""");
        Assert.Equal(7, written.Count);
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"type":"integer"}"""), written[1]["params"]!["requestedSchema"]!["properties"]!["answer"]));
        Assert.Equal(("Removed Ada after 2 copies", false), Said(written[3]));
        Assert.Equal(("Kept", true), Said(written[6]));
        written = Served(Allowed, """{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-remove","arguments":{"name":"Ada"}}}""");
        Assert.Equal(("There is nobody to answer: How many copies?", true), Said(written[0]));
    }

    [Fact]
    public void While_the_app_is_closed_the_person_s_settings_decide()
    {
        var written = Served("{}", """{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"customer-add","arguments":{"name":"Ada"}}}""");
        Assert.Equal(("Shop is not open; open it to allow this.", true), Said(written[0]));
    }

    [Fact]
    public void The_words_of_refusals_are_the_shared_ones()
    {
        var words = JsonNode.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "app-mcp.json")))!["words"]!.AsObject();
        Assert.Equal(Agents.Words.Count, words.Count);
        foreach (var (key, english) in Agents.Words)
            Assert.Equal(english, words[key]!.GetValue<string>());
    }
}
