// The windows the wire and the library tests of the tests agent drive:
// view models in the shape the C# generator writes them (commands per
// handler, On<Handler> methods, part states, choices, fetchers, the facts of the
// fields beside them), and a custom part written as a
// program's library writes one.

using System.Text.Json;
using System.Windows.Input;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Native.Tests;

public enum TicketStatus
{
    Open,
    Done,
}

/// <summary>The facts of the entities' fields, as the generator writes them beside the entities (Fields.cs).</summary>
public static class Fields
{
    public static readonly IReadOnlyDictionary<string, FieldFacts> All = new Dictionary<string, FieldFacts>
    {
        ["Ticket.title"] = new("@ticketTitleLabel", null, Required: true),
        ["Ticket.total"] = new("Total", new FieldFormat(FormatKind.Money), Min: "0", Max: "500"),
        ["Ticket.finished_by"] = new("Finished by", Refers: "Person"),
    };
}

public sealed class Ticket
{
    public long Id { get; set; }
    public string Title { get; set; } = "";
    public string Note { get; set; } = "";
    public decimal Total { get; set; }
    public TicketStatus Status { get; set; }
    public long? FinishedBy { get; set; }
    public DateOnly? Due { get; set; }
}

/// <summary>Settings as a generated app writes them: each setting a property, read and given back whole.</summary>
public sealed class TicketSettings : ISettings
{
    public bool ShowDone { get; set; } = true;
    public string Greeting { get; set; } = "hello";

    public System.Text.Json.Nodes.JsonObject ToJson() => new() { ["show_done"] = ShowDone, ["greeting"] = Greeting };

    public void Apply(System.Text.Json.Nodes.JsonObject sent)
    {
        ShowDone = sent["show_done"]?.GetValue<bool>() ?? ShowDone;
        Greeting = sent["greeting"]?.GetValue<string>() ?? Greeting;
    }
}

public sealed class TicketsViewModel() : WindowViewModel(Db.Empty())
{
    public const string Markup = """
        <Window name="Tickets">
          <Textbox name="titleBox" at="1,1" span="6" bind="ticket.title" undoScope="ticket" />
          <Button name="copyButton" at="7,1" span="2" caption="Copy" onclick="copyIt" />
          <Button name="sureButton" at="9,1" span="2" caption="Sure" onclick="makeSure" />
          <Button name="warnButton" at="11,1" span="2" caption="Warn" onclick="warnTwice" />
          <Button name="countButton" at="1,2" span="2" caption="Count" onclick="count" />
        </Window>
        """;

    public override string Title => "Tickets";

    public Ticket Ticket { get; set; } = new() { Id = 5, Title = "Leak", Note = "first", Total = 12.5m };

    public TicketSettings Settings { get; } = new();

    public Choice[] PersonChoices { get; } = [new(null, ""), new(1L, "Ann"), new(2L, "Bob")];

    // A typed dropdown's values of its field, and a search dropdown's records with the fields it shows.
    public List<string> TicketNoteValues { get; } = ["first", "second", "first"];

    public Choice[] PersonNameRoleChoices { get; } = [new(7L, "Cy", ["Cy", "lead"]), new(8L, "Di", ["Di", "help"])];

    // The fetchers of a stored picture and a file, as the generator writes them for a file field.
    public Func<Task<byte[]?>> TicketPhotoPicture => () => Task.FromResult<byte[]?>([1, 2, 3]);

    public Func<string, Task<byte[]?>> TicketScanFile => path => Task.FromResult<byte[]?>(path == "a/b.pdf" ? [2] : null);

    public PartState TitleBox { get; } = new("titleBox");

    // Fields, not properties: what the handlers count is no value of the window the library sends back.
    public int Confirmed;
    public int Counted;

    public ICommand CopyItCommand => new RelayCommand(() => Handle(() => Copy(Text.Plain("Name: Ada"))));
    public ICommand MakeSureCommand => new RelayCommand(() => Handle(() => Ask(Text.Plain("Sure?"), new ConfirmForm(null, null), _ => Confirmed++)));
    public ICommand WarnTwiceCommand => new RelayCommand(() => Handle(OnWarnTwice));
    public ICommand CountCommand => new RelayCommand(() => Handle(() => Counted++));

    void OnWarnTwice()
    {
        Say(Level.Error, Text.Plain("one"));
        Say(Level.Error, Text.Plain("two"));
    }
}

/// <summary>A window with a custom part that binds a value, hears code and tells its window.</summary>
public sealed class GaugesViewModel() : WindowViewModel(Db.Empty())
{
    public static string MarkupOf(string kind) => $$"""
        <Window name="Gauges">
          <Custom name="gauge" at="1,1" span="4" rows="3" part="{{kind}}" bind="level" undoScope="gauge" onchange="gaugeChanged" tells='[{"tell":"reached","handler":"gaugeReached"}]' />
          <Button name="resetButton" at="5,1" span="2" caption="Reset" onclick="reset" />
          <Button name="countButton" at="7,1" span="2" caption="Count" onclick="count" />
        </Window>
        """;

    public override string Title => "Gauges";

    public long Level { get; set; } = 3;

    public PartState Gauge { get; } = new PartState("gauge").With("title", "Speed");

    public int Changes;
    public string? Reached;
    public int Counted;

    public ICommand GaugeChangedCommand => new RelayCommand(() => Handle(() => Changes++));
    public ICommand ResetCommand => new RelayCommand(() => Handle(OnReset));
    public ICommand CountCommand => new RelayCommand(() => Handle(() => Counted++));

    void OnReset()
    {
        Gauge.Hear("reset", 2L);
        Gauge.SetValue("speed", 7L);
    }

    void OnGaugeReached(long count, string how) => Reached = $"{count} {how}";
}

/// <summary>A custom part as a program's library writes one: it says what it got, paints itself red and changes and tells on a click.</summary>
public sealed class WireGauge : CustomPart
{
    /// <summary>Every gauge the library made, by the test that registered its kind.</summary>
    public static readonly Dictionary<string, List<WireGauge>> Made = [];

    public static readonly Rgba Red = Rgba.Of(200, 30, 40);

    public readonly List<string> Got = [];
    public readonly List<string> Heard = [];
    public bool Breaks { get; init; }

    public override bool Focusable => true;

    public WireGauge() => Told += (tell, values) => Heard.Add($"{tell} {string.Join(",", values.Select(v => v.GetRawText()))}");

    public static Func<CustomPart> Maker(string kind, bool breaks = false) => () =>
    {
        var made = new WireGauge { Breaks = breaks };
        lock (Made)
        {
            if (!Made.TryGetValue(kind, out var list))
                Made[kind] = list = [];
            list.Add(made);
        }
        return made;
    };

    static string Written(object? value) => value switch
    {
        null => "null",
        bool b => b ? "true" : "false",
        JsonElement e => e.GetRawText(),
        _ => Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture) ?? "",
    };

    protected override void Set(string name, object? value) => Got.Add($"set {name} {Written(value)}");

    protected override void Hear(string hear, object?[] values) => Got.Add($"hear {hear} {string.Join(",", values.Select(Written))}");

    public override void Draw(Painter painter)
    {
        if (Breaks)
            throw new InvalidOperationException("the gauge cannot draw");
        painter.Fill(Bounds, Red);
    }

    public override void OnClick()
    {
        if (Breaks)
            throw new InvalidOperationException("the gauge cannot click");
        ChangeValue(9L);
        Tell("reached", 3L, "high");
        Tell("dropped", 1L);
    }
}
