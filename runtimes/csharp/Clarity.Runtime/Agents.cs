// A command-line program's commands for AI agents (docs/ledger/app-mcp.md):
// `<program> mcp` serves them over MCP through Clarity.AppMcp while the app
// is closed. The package Clarity.AppMcp calls the native library; this
// runtime stays managed. A call becomes the command line the program reads
// with its own parser, runs as if typed, and what it prints is the call's
// answer; its questions go to the agent. Nobody is there to ask for a
// grant: the person's settings agentReadsWhileClosed and
// agentChangesWhileClosed allow, else the call is refused.

using System.Text;
using System.Text.Json.Nodes;

namespace Clarity.Runtime;

/// <summary>What a program serves: its tool list (as clarity-gen writes it), the app's title, the name
/// its settings are kept under, and its version.</summary>
public sealed record Agents(string Tools, string App, string Program, string Version)
{
    /// <summary>The setting, kept in the person's settings.json, that allows reads while the app is closed.</summary>
    public const string ReadsSetting = "agentReadsWhileClosed";

    /// <summary>The setting that allows changes while the app is closed.</summary>
    public const string ChangesSetting = "agentChangesWhileClosed";

    /// <summary>Clarity's words of a program whose commands agents may call, in English
    /// (conformance/app-mcp.json): what its MCP service says of a refused call, and its help's line for <c>mcp</c>.</summary>
    public static readonly IReadOnlyList<(string Key, string English)> Words =
    [
        ("agentRefused", "The person using {app} refused this."),
        ("agentClosed", "{app} is not open; open it to allow this."),
        ("agentMcp", "Serves the commands for AI agents over MCP"),
    ];

    static string Say(string key, string app) =>
        CliLanguage.Translate(key, Words.First(w => w.Key == key).English).Replace("{app}", app);

    /// <summary>How the program serves while the app is closed, with what the person's settings
    /// <paramref name="kept"/> allow, as JSON.</summary>
    public string Options(JsonObject kept)
    {
        var given = new JsonObject
        {
            ["version"] = Version,
            ["open"] = false,
            ["texts"] = new JsonObject { ["refused"] = Say("agentRefused", App), ["closed"] = Say("agentClosed", App) },
        };
        // A switch the person never set is left out: the service uses what the project allows.
        foreach (var (key, setting) in new[] { ("closedReads", ReadsSetting), ("closedChanges", ChangesSetting) })
            if (kept[setting] is JsonValue v && v.TryGetValue<bool>(out var on))
                given[key] = on;
        return given.ToJsonString();
    }

    /// <summary>The commands agents may call while the desktop app runs (Clarity.UI asks <c>agents</c>
    /// and <c>agent_call</c>): what it offers, how a command line runs, and how a command's question is asked
    /// of the agent (Clarity.UI's <c>clarity_agent_ask</c>, set by Clarity.Ui). Empty for an app without them.</summary>
    public static class Desktop
    {
        public static Agents? Offered { get; set; }
        public static Func<IReadOnlyList<string>, Func<string, string, string?>, (string Text, bool Refused)>? Call { get; set; }
        public static Func<string, string, string?> Ask { get; set; } = (_, _) => null;
    }

    /// <summary>Runs a call as the command line it becomes: what the command printed, and whether the
    /// call is refused (a wrong line, a refusal, an exit code, a question nobody answered).</summary>
    public static (string Text, bool Refused) RunCall(CliProgram program, IReadOnlyList<string> line, Func<string, string, string?> ask, Func<CliSession, int, IReadOnlyList<string?>, int> run)
    {
        switch (Cli.Parse(program, line, CliLanguage.Words))
        {
            case CliParsed.Wrong wrong:
                return (wrong.Said.TrimEnd('\n'), true);
            case CliParsed.Help help:
                return (help.Text.TrimEnd('\n'), false);
            case CliParsed.Run ran:
                var printed = new StringBuilder();
                try
                {
                    var code = run(new CliSession(new CliConsole(), ask, printed), ran.Command, ran.Values);
                    var said = printed.ToString().TrimEnd('\n', ' ');
                    return code == 0 ? (said, false) : (said.Length == 0 ? $"exit {code}" : said, true);
                }
                catch (CliUnanswered unanswered)
                {
                    return (unanswered.Message, true);
                }
                catch (RefusedException refused)
                {
                    return (CliLanguage.Show(refused.Text), true);
                }
                catch (Exception err) when (err is ClarityException or IOException)
                {
                    return (err.Message, true);
                }
        }
        return ("", true);
    }
}
