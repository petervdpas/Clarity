// What the program does with a handler or an edit the library sends, beside
// what ProgramWireTests checks: a handler named as a part, whether one ran
// or refused, a change handler inside its edit, the canvases drawn anew
// after a change, and the empty database of an unconnected run closing at
// the end. These were checked on C#'s own drawing (ViewModelHost) before the
// library drew the windows in the browser too.

using System.Text.Json.Nodes;
using System.Windows.Input;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Native.Tests;

public sealed class InkViewModel() : WindowViewModel(Db.Empty())
{
    public const string Markup = """
        <Window name="Ink">
          <Textbox name="nameBox" at="1,1" span="6" bind="name" undoScope="nameBox" onchange="nameChanged" />
          <Textbox name="plainBox" at="1,2" span="6" />
          <Canvas name="field" at="1,3" span="12" rows="4" />
          <Button name="redInk" at="1,7" span="2" caption="Ink" onclick="redInk" />
        </Window>
        """;

    public override string Title => "Ink";

    public string Name { get => name; set => SetProperty(ref name, value); }
    string name = "";

    public string Status { get; set; } = "open";

    public PartState NameBox { get; } = new("nameBox");
    public PartState PlainBox { get; } = new("plainBox");
    public PartState Field { get; } = new("field");

    // A part and a handler of the same name: the generator names the handler's command RedInk_.
    public PartState RedInk { get; } = new("redInk");

    public int Inked { get; private set; }
    public int Draws { get; private set; }
    public int NameChanges { get; private set; }

    public ICommand RedInk_Command => new RelayCommand(() => Handle(OnRedInk));
    public ICommand NameChangedCommand => new RelayCommand(() => Handle(OnNameChanged));
    public ICommand RefuseCommand => new RelayCommand(() => Handle(OnRefuse));
    public ICommand Digit7Command => new RelayCommand(() => Run_Digit7());
    public ICommand RefuseWith1Command => new RelayCommand(() => Run_RefuseWith1());

    public long Digit { get; private set; }

    public bool Run_Digit7() => Handle(() => Digit = 7, command: "Digit7");

    public bool Run_RefuseWith1() => Handle(() => throw new RefusedException(Text.Plain("not with that")), command: "RefuseWith1");

    void OnRedInk() => Inked++;

    void OnNameChanged()
    {
        NameChanges++;
        Status = "changed";
    }

    void OnRefuse() => throw new RefusedException(Text.Plain("not now"));

    protected override void Redraw()
    {
        Field.BeginDrawing();
        Draws++;
        Field.Draw(new Shape.Clear("panel"));
        Field.Draw(new Shape.Words(50, 50, Text.Plain(Name), "text", 10));
        Field.EndDrawing();
    }
}

public sealed class WireHandlerTests
{
    static (ProgramWire Wire, InkViewModel Ink, JsonObject State) Opened()
    {
        var ink = new InkViewModel();
        var wire = new ProgramWire(typeof(InkViewModel).Assembly, ink, n => n == "Ink" ? InkViewModel.Markup : null);
        var opened = wire.Answer("open_window", """{"window":"Ink","with":{}}""");
        return (wire, ink, opened!["value"]!["state"]!.AsObject());
    }

    static JsonObject Edit(ProgramWire wire, JsonObject state, string part, string? handler = null)
    {
        var args = new JsonObject { ["state"] = state.DeepClone(), ["part"] = part, ["scope"] = part, ["ends"] = true };
        if (handler is not null)
            args["handler"] = handler;
        var answer = wire.Answer("edit_event", args.ToJsonString());
        Assert.Null(answer!["error"]);
        return answer["value"]!.AsObject();
    }

    static string Said(InkViewModel ink) => Locale.Show(ink.Field.Drawing.OfType<Shape.Words>().Single().Text);

    [Fact]
    public void A_handler_named_as_a_part_runs_by_its_stepped_around_command()
    {
        var (_, ink, _) = Opened();
        Assert.True(Handlers.Run(ink, "redInk"));
        Assert.Equal(1, ink.Inked);
    }

    [Fact]
    public void A_handler_says_whether_it_ran_or_refused()
    {
        var (_, ink, _) = Opened();
        Assert.True(Handlers.Run(ink, "redInk"));
        Assert.False(Handlers.Run(ink, "refuse"));
        Assert.Equal(Level.Error, Assert.Single(ink.Messages).Level);
        // One the window has no command for says so.
        Assert.False(Handlers.Run(ink, "nowhere"));
        Assert.True(Handlers.Run(ink, "digit(7)"));
        Assert.Equal(7, ink.Digit);
        // A call with values says it too.
        Assert.False(Handlers.Run(ink, "refuseWith(1)"));
        Assert.Equal(3, ink.Messages.Count);
    }

    [Fact]
    public void A_change_handler_runs_inside_the_edit_and_goes_back_with_it()
    {
        var (wire, ink, state) = Opened();
        state["data"]!["name"] = "typed";
        Edit(wire, state, "nameBox", "nameChanged");
        Assert.Equal(("typed", "changed", 1), (ink.Name, ink.Status, ink.NameChanges));
        while (ink.Edits.CanUndo("nameBox"))
            ink.UndoEdit("nameBox", back: true);
        Assert.Equal(("", "open"), (ink.Name, ink.Status));
    }

    [Fact]
    public void A_canvas_draws_anew_after_a_bound_value_the_person_changed()
    {
        var (wire, ink, state) = Opened();
        var drawn = ink.Draws;
        state["data"]!["name"] = "Ada";
        Edit(wire, state, "nameBox");
        Assert.Equal("Ada", Said(ink));
        Assert.Equal(drawn + 1, ink.Draws);
    }

    [Fact]
    public void A_canvas_draws_anew_after_an_unbound_part_the_person_changed()
    {
        var (wire, ink, state) = Opened();
        var drawn = ink.Draws;
        ink.Name = "Bo";
        Edit(wire, state, "plainBox");
        Assert.Equal("Bo", Said(ink));
        Assert.Equal(drawn + 1, ink.Draws);
    }

    [Fact]
    public void A_handler_draws_anew_once_and_a_window_without_a_canvas_draws_nothing()
    {
        var (wire, ink, state) = Opened();
        var drawn = ink.Draws;
        var reply = wire.Answer("dispatch_event", new JsonObject { ["state"] = state.DeepClone(), ["handler"] = "redInk" }.ToJsonString());
        Assert.Null(reply!["error"]);
        Assert.Equal(drawn + 1, ink.Draws);
        // Nothing to draw: told of a change, a window without draw handlers runs nothing.
        var plain = new OtherViewModel();
        plain.Changed();
        Assert.Empty(plain.Messages);
    }

    [Fact]
    public void The_empty_database_of_an_unconnected_run_closes_as_the_program_ends()
    {
        var empties = new List<Db>();
        var unconnected = new Unconnected
        {
            Connect = () => throw new StorageException("no server"),
            Empty = () =>
            {
                var db = Db.Empty();
                empties.Add(db);
                return db;
            },
            Main = _ => new OtherViewModel(),
            Window = _ => new ShelfViewModel(),
        };
        var wire = new ProgramWire(typeof(ShelfViewModel).Assembly, null, _ => null, unconnected, Text.Plain("no server"));
        wire.Answer("started", "{}");
        Assert.False(Assert.Single(empties).Disposed);
        wire.Ended();
        Assert.True(Assert.Single(empties).Disposed);
    }
}
