// Command-line programs against Clarity's shared cases
// (conformance/command-line.json): reading a command line, the help, a
// printed table and asking on a terminal.

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class CliCaseTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "command-line.json"))).RootElement;

    static string? Text(JsonElement e, string name) =>
        e.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

    static List<string> Texts(JsonElement array) => array.EnumerateArray().Select(t => t.GetString()!).ToList();

    static CliKind KindOf(string word) => word switch
    {
        "integer" => CliKind.Integer,
        "decimal" => CliKind.Decimal,
        "money" => CliKind.Money,
        "date" => CliKind.Date,
        "datetime" => CliKind.DateTime,
        "duration" => CliKind.Duration,
        "enum" => CliKind.Enum,
        "bool" => CliKind.Bool,
        _ => CliKind.Text,
    };

    // An asked value names its kind as `type`; `kind` is the form's there.
    static CliParam ParamOf(JsonElement p) => new(Text(p, "name") ?? "", KindOf(Text(p, "type") ?? Text(p, "kind") ?? "text"))
    {
        Values = p.TryGetProperty("values", out var values) ? Texts(values) : [],
        Default = Text(p, "default"),
        Min = Text(p, "min"),
        Max = Text(p, "max"),
        Required = p.TryGetProperty("required", out var required) && required.GetBoolean(),
        Help = Text(p, "help"),
    };

    static CliProgram ProgramOf(JsonElement p) => new(
        Text(p, "name")!,
        Text(p, "help"),
        p.GetProperty("commands").EnumerateArray().Select(c => new CliCommand(
            Texts(c.GetProperty("line")),
            Text(c, "help"),
            c.GetProperty("arguments").EnumerateArray().Select(ParamOf).ToList(),
            c.GetProperty("options").EnumerateArray().Select(ParamOf).ToList())).ToList());

    static readonly CliWords English = new();

    [Fact]
    public void TheWordsAreTheSharedOnes()
    {
        var words = Cases().GetProperty("words");
        Assert.Equal(words.EnumerateObject().Count(), CliWords.English.Count);
        foreach (var (key, english) in CliWords.English)
            Assert.Equal(words.GetProperty(key).GetString(), english);
    }

    [Fact]
    public void CommandLinesAreReadAsEveryRuntimeReadsThem()
    {
        var cases = Cases();
        var program = ProgramOf(cases.GetProperty("program"));
        foreach (var c in cases.GetProperty("parse").EnumerateArray())
        {
            var what = Text(c, "what");
            var parsed = Cli.Parse(program, Texts(c.GetProperty("args")), English);
            if (c.TryGetProperty("run", out var run))
            {
                var ran = Assert.IsType<CliParsed.Run>(parsed);
                var command = program.Commands[ran.Command];
                Assert.Equal(Text(run, "command"), string.Join(' ', command.Line));
                var names = command.Arguments.Concat(command.Options).Select(p => p.Name).ToList();
                var wanted = run.GetProperty("values").EnumerateObject().ToDictionary(v => v.Name, v => v.Value.ValueKind == JsonValueKind.Null ? null : v.Value.GetString());
                var got = names.Select((n, i) => (n, ran.Values[i])).ToDictionary(p => p.n, p => p.Item2);
                Assert.True(wanted.Count == got.Count && wanted.All(w => got[w.Key] == w.Value), $"{what}: {string.Join(", ", got)}");
            }
            else if (c.TryGetProperty("help", out var help))
            {
                Assert.Equal(help.GetString(), Assert.IsType<CliParsed.Help>(parsed).Text);
            }
            else
            {
                var wanted = c.GetProperty("wrong");
                var wrong = Assert.IsType<CliParsed.Wrong>(parsed);
                Assert.Equal(Text(wanted, "key"), wrong.Key);
                var slots = wanted.GetProperty("slots").EnumerateObject().Select(s => (s.Name, s.Value.GetString()!)).ToList();
                Assert.Equal(slots, wrong.Slots);
                Assert.Equal(Text(wanted, "said"), wrong.Said);
            }
        }
    }

    [Fact]
    public void APrintedTableLinesUpAsEveryRuntimeDoes()
    {
        foreach (var c in Cases().GetProperty("table").EnumerateArray())
        {
            var right = c.GetProperty("right").EnumerateArray().Select(b => b.GetBoolean()).ToList();
            var rows = c.GetProperty("rows").EnumerateArray().Select(r => (IReadOnlyList<string>)Texts(r)).ToList();
            Assert.Equal(Text(c, "said"), Cli.Table(Texts(c.GetProperty("headings")), right, rows));
        }
    }

    sealed class Typed(bool terminal, List<string?> lines) : ICliTerminal
    {
        public readonly System.Text.StringBuilder Said = new();
        public bool IsTerminal => terminal;
        public string? ReadLine()
        {
            if (lines.Count == 0)
                return null;
            var line = lines[0];
            lines.RemoveAt(0);
            return line;
        }
        public void Write(string text) => Said.Append(text);
    }

    [Fact]
    public void ACommandAsksOnATerminalAsEveryRuntimeDoes()
    {
        foreach (var c in Cases().GetProperty("ask").EnumerateArray())
        {
            var form = c.GetProperty("form");
            CliAsked asked = Text(form, "kind") switch
            {
                "confirm" => new CliAsked.Confirm(Text(form, "yes")!, Text(form, "no")!),
                "choice" => new CliAsked.Choice(Texts(form.GetProperty("choices"))),
                _ => new CliAsked.Value(ParamOf(form)),
            };
            var typed = new Typed(c.GetProperty("terminal").GetBoolean(), c.GetProperty("typed").EnumerateArray().Select(l => l.ValueKind == JsonValueKind.Null ? null : l.GetString()).ToList());
            var (answer, exit) = Cli.Ask(Text(c, "question")!, asked, typed, English);
            if (c.TryGetProperty("exit", out var code))
                Assert.Equal(code.GetInt32(), exit);
            else
                Assert.Equal((Text(c, "answer"), 0), (answer, exit));
            Assert.Equal(Text(c, "said"), typed.Said.ToString());
        }
    }

    [Fact]
    public void ValuesReadFromTheCommandLineBecomeTyped()
    {
        Assert.Equal(42L, Cli.Long("42"));
        Assert.Equal(0L, Cli.Long(null));
        Assert.Equal("Ada", Cli.Text("Ada"));
        Assert.Equal("", Cli.Text(null));
        Assert.True(Cli.Bool("true"));
        Assert.Equal(12.5m, Cli.Decimal("12.5"));
        Assert.Equal(new DateOnly(2026, 10, 23), Cli.Date("2026-10-23"));
        Assert.Null(Cli.Date(null));
        Assert.Equal(new DateTime(2026, 10, 23, 14, 30, 0), Cli.Moment("2026-10-23T14:30:00"));
        Assert.Equal(DayOfWeek.Friday, Cli.Enum<DayOfWeek>("Friday"));
        Assert.Equal("42", Cli.TextOf(42L));
        Assert.Equal("12.5", Cli.TextOf(12.50m));
        Assert.Equal("2026-10-23", Cli.TextOf((DateOnly?)new DateOnly(2026, 10, 23)));
        Assert.Equal("", Cli.TextOf((DateOnly?)null));
        Assert.Equal("Friday", Cli.TextOf(DayOfWeek.Friday));
    }

    [Fact]
    public void TheLanguageIsTheSystemsWhenTheProjectHasIt()
    {
        string[] available = ["en", "nl"];
        Assert.Equal("nl", Cli.PickLanguage("nl_NL.UTF-8", available, "en"));
        Assert.Null(Cli.PickLanguage("en_US.UTF-8", available, "en"));
        Assert.Null(Cli.PickLanguage("de_DE.UTF-8", available, "en"));
        Assert.Null(Cli.PickLanguage("C", available, "en"));
    }
}
