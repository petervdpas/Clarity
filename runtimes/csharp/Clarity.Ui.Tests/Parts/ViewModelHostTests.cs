// The host of a generated window's view model: parts read and change the
// view model's properties by their paths, run its commands by their
// handlers, read its part states, texts and choices, and hear when it
// changed. The view model here is written by hand in the shape the C#
// generator writes one (crates/clarity-gen/src/csharp/windows.rs).

using Clarity.Runtime;
using Clarity.Ui.Tests.Generated;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Tests.Generated
{
    public enum Status
    {
        Open,
        Done,
    }

    /// <summary>The choices of an enum, as the generator writes them beside it.</summary>
    public static partial class Choices
    {
        public static Choice[] Status => [new(Generated.Status.Open, "Open"), new(Generated.Status.Done, "Done")];
    }

    /// <summary>The facts of the entities' fields, as the generator writes them beside the entities.</summary>
    public static class Fields
    {
        public static IReadOnlyDictionary<string, FieldFacts> All { get; } = new Dictionary<string, FieldFacts>
        {
            ["Order.total"] = new("@orderTotalLabel", new FieldFormat(FormatKind.Decimal), Required: true, Min: "0", Max: "500"),
            ["Order.finished_by"] = new("Finished by", Refers: "Helper"),
            ["Order.note"] = new("Note"),
            ["Order.count"] = new("Count"),
        };
    }

    public sealed partial class Person : ObservableObject
    {
        public long Id { get; set; }
        public string Name { get; set; } = "";
    }

    public sealed partial class Order : ObservableObject
    {
        public long Id { get; set; }
        public string Note { get => note; set => SetProperty(ref note, value); }
        string note = "";
        public Status Status { get => status; set => SetProperty(ref status, value); }
        Status status;
        public long? FinishedBy { get => finishedBy; set => SetProperty(ref finishedBy, value); }
        long? finishedBy;
        public decimal Total { get; set; }
        public long Count { get; set; }
    }

    /// <summary>
    /// A window's view model as the generator writes one: its data as
    /// observable properties, a state per named part, and each handler as a
    /// command (the generator's [RelayCommand] makes the same properties).
    /// </summary>
    public sealed class OrdersViewModel : WindowViewModel
    {
        public OrdersViewModel() : base(null) { }

        public override string Title => "Orders";

        public Order Order { get => order; set => SetProperty(ref order, value); }
        Order order = new();

        public List<Order> Orders { get => orders; set => SetProperty(ref orders, value); }
        List<Order> orders = [new() { Id = 1, Note = "first" }, new() { Id = 2, Note = "second" }];

        public long Shown { get => shown; set => SetProperty(ref shown, value); }
        long shown;

        public Choice[] PersonChoices { get; } = [new(null, ""), new(1L, "Ann"), new(2L, "Bob")];

        public Choice[] HelperChoices { get; } = [new(null, ""), new(7L, "Cy")];

        // A typed dropdown's values of its field, and a search dropdown's records with the fields it shows.
        public List<string> OrderNoteValues { get; } = ["first", "second", "first"];

        public Choice[] HelperNameRoleChoices { get; } = [new(7L, "Cy", ["Cy", "lead"])];

        // A canvas's commands as the generator writes them: ClickAt_Pad, CanvasAt_Pad.
        public readonly List<string> Canvased = [];

        RelayCommand<Pointer>? clickAtPad;
        public IRelayCommand<Pointer> ClickAt_PadCommand => clickAtPad ??= new RelayCommand<Pointer>(at => Canvased.Add($"click {at!.X},{at.Y}"));

        RelayCommand<System.Text.Json.Nodes.JsonObject>? canvasAtPad;
        public IRelayCommand<System.Text.Json.Nodes.JsonObject> CanvasAt_PadCommand => canvasAtPad ??= new RelayCommand<System.Text.Json.Nodes.JsonObject>(a => Canvased.Add($"act {a!.ToJsonString()}"));

        // The fetchers of a stored file the generator writes for a file field.
        public Func<Task<byte[]?>> OrderNotePicture => () => Task.FromResult<byte[]?>([1]);

        public Func<string, Task<byte[]?>> OrderNoteFile => path => Task.FromResult<byte[]?>(path == "a/b.pdf" ? [2] : null);

        public PartState Note { get; } = new("note");
        public PartState ApproveButton { get; } = new("approveButton", caption: "approveCaption");
        public PartState Grid { get; } = new("grid");

        public int Approved;
        public int NoteChanges;
        public readonly List<long> Digits = [];
        public readonly List<string> Scaled = [];

        RelayCommand? approveClicked;
        public IRelayCommand ApproveClickedCommand => approveClicked ??= new RelayCommand(ApproveClicked);
        void ApproveClicked() => Handle(OnApproveClicked);

        void OnApproveClicked()
        {
            Approved++;
            ApproveButton.IsEnabled = false;
        }

        RelayCommand? noteChanged;
        public IRelayCommand NoteChangedCommand => noteChanged ??= new RelayCommand(NoteChanged);
        void NoteChanged() => Handle(OnNoteChanged);

        void OnNoteChanged()
        {
            NoteChanges++;
            Order.Status = Status.Done;
        }

        // A handler with values runs through a command per call: digit(7) is Digit7.
        void OnDigit(long digit) => Digits.Add(digit);

        RelayCommand? digit7;
        public IRelayCommand Digit7Command => digit7 ??= new RelayCommand(Digit7);
        void Digit7() => Run_Digit7();
        public bool Run_Digit7() => Handle(() => OnDigit(7), command: nameof(Digit7));

        void OnRefuseWith(long _) => throw new RefusedException(Text.Plain("not with that"));

        RelayCommand? refuseWith1;
        public IRelayCommand RefuseWith1Command => refuseWith1 ??= new RelayCommand(RefuseWith1);
        void RefuseWith1() => Run_RefuseWith1();
        public bool Run_RefuseWith1() => Handle(() => OnRefuseWith(1), command: nameof(RefuseWith1));

        void OnScale(double by, string name) => Scaled.Add($"{by} {name}");

        RelayCommand? scaleHalf;
        public IRelayCommand Scale0_5HalfCommand => scaleHalf ??= new RelayCommand(Scale0_5Half);
        void Scale0_5Half() => Handle(() => OnScale(0.5, "half"));

        RelayCommand? refuse;
        public IRelayCommand RefuseCommand => refuse ??= new RelayCommand(Refuse);
        void Refuse() => Handle(OnRefuse);

        void OnRefuse() => throw new RefusedException(Text.Plain("not now"));

        RelayCommand? warn;
        public IRelayCommand WarnCommand => warn ??= new RelayCommand(Warn);
        void Warn() => Handle(() => Say(Level.Warning, Text.Plain("careful")));

        RelayCommand? failing;
        public IRelayCommand FailingCommand => failing ??= new RelayCommand(Failing);
        void Failing() => Handle(() => throw new InvalidOperationException("broken"));

        RelayCommand? openOther;
        public IRelayCommand Open_OtherCommand => openOther ??= new RelayCommand(Open_Other);
        void Open_Other() => Handle(() => Open(new OtherViewModel()));

        RelayCommand? closeWindow;
        public IRelayCommand CloseWindow_Command => closeWindow ??= new RelayCommand(CloseWindow_);
        void CloseWindow_() => Handle(Close);

        // A part and a handler of the same name: the generator names the handler's command RedInk_.
        public PartState RedInk { get; } = new("redInk");
        public int Inked;

        RelayCommand? redInk;
        public IRelayCommand RedInk_Command => redInk ??= new RelayCommand(RedInk_);
        void RedInk_() => Handle(OnRedInk);

        void OnRedInk() => Inked++;

        public int OpenedTimes;
        public int ReturnedTimes;

        protected override void OnOpen() => OpenedTimes++;

        protected override void OnReturn() => ReturnedTimes++;
    }

    public sealed class OtherViewModel : WindowViewModel
    {
        public OtherViewModel() : base(null) { }

        public override string Title => "Other";

        public int OpenedTimes;

        protected override void OnOpen() => OpenedTimes++;

        RelayCommand? closeWindow;
        public IRelayCommand CloseWindow_Command => closeWindow ??= new RelayCommand(CloseWindow_);
        void CloseWindow_() => Handle(Close);
    }
}

namespace Clarity.Ui.Tests
{
    [Collection("PersonKeys")]
    public class ViewModelHostTests
    {
        static (OrdersViewModel Model, ViewModelHost Host) Made(string markup = "")
        {
            var model = new OrdersViewModel();
            var spec = WindowMarkup.Read($"<Window name=\"Orders\">{markup}</Window>");
            return (model, new ViewModelHost(model, spec));
        }

        [Fact]
        public void A_path_reads_the_view_models_properties_in_pascal_case()
        {
            var (model, host) = Made();
            model.Order.Note = "hello";
            model.Order.FinishedBy = 2;
            Assert.Equal("hello", host.Read("order.note"));
            Assert.Equal(2L, host.Read("order.finished_by"));
            Assert.Equal(0L, host.Read("shown"));
            Assert.Null(host.Read("order.nothing"));
            Assert.Null(host.Read("nothing.at.all"));
            Assert.Equal(typeof(Status), host.TypeOf("order.status"));
            Assert.Equal(typeof(string), host.TypeOf("orders.note"));
        }

        [Fact]
        public void A_table_cell_edit_writes_the_field_of_the_record_at_its_place()
        {
            var (model, host) = Made("""<Table name="grid" bind="orders" columns="note, status" edit="note, status" undoScope="grid" />""");
            Assert.Equal("second", host.Read("orders[1].note"));
            Assert.Null(host.Read("orders[5].note"));
            host.Write("grid", "orders[1].note", "changed");
            host.Write("grid", "orders[0].status", "Done");
            Assert.Equal("changed", model.Orders[1].Note);
            Assert.Equal(Status.Done, model.Orders[0].Status);
            // It is an edit of the table, which Ctrl+Z takes back.
            model.UndoEdit("grid", back: true);
            Assert.Equal(Status.Open, model.Orders[0].Status);
        }

        [Fact]
        public void A_list_gives_its_rows()
        {
            var (model, host) = Made();
            Assert.Equal(model.Orders, host.Rows("orders"));
            Assert.Empty(host.Rows("order.note"));
            Assert.Empty(host.Rows("nothing"));
        }

        [Fact]
        public void A_write_is_the_persons_edit_which_undo_takes_back()
        {
            var (model, host) = Made("<Textbox name=\"note\" bind=\"order.note\" undoScope=\"form\" />");
            host.Write("note", "order.note", "typed");
            Assert.Equal("typed", model.Order.Note);
            Assert.True(model.Edits.CanUndo("form"));
            model.UndoEdit("form", back: true);
            Assert.Equal("", model.Order.Note);
        }

        [Fact]
        public void A_part_the_markup_gives_no_undo_scope_makes_no_edit()
        {
            // The generator writes undoScope on every part clarity_spec gives one (Avalonia's Edits.Scope); without it, no edit.
            var (model, host) = Made("<Number name=\"shownBox\" bind=\"shown\" />");
            host.Write("shownBox", "shown", 12.0);
            Assert.Equal(12L, model.Shown);
            Assert.False(model.Edits.CanUndo("shownBox"));
            var (scoped, scopedHost) = Made("<Number name=\"shownBox\" bind=\"shown\" undoScope=\"shownBox\" />");
            scopedHost.Write("shownBox", "shown", 12.0);
            Assert.True(scoped.Edits.CanUndo("shownBox"));
        }

        [Fact]
        public void A_part_with_no_undo_changes_the_value_without_an_edit()
        {
            var (model, host) = Made("<Textbox name=\"note\" bind=\"order.note\" noUndo=\"true\" />");
            host.Write("note", "order.note", "typed");
            Assert.Equal("typed", model.Order.Note);
            Assert.False(model.Edits.CanUndo("note"));
        }

        [Fact]
        public void A_value_is_written_in_the_type_of_its_property()
        {
            var (model, host) = Made();
            host.Write("status", "order.status", "Done");
            Assert.Equal(Status.Done, model.Order.Status);
            host.Write("finished", "order.finished_by", 2.0);
            Assert.Equal(2L, model.Order.FinishedBy);
            host.Write("finished", "order.finished_by", null);
            Assert.Null(model.Order.FinishedBy);
            host.Write("total", "order.total", "12.5");
            Assert.Equal(12.5m, model.Order.Total);
        }

        [Fact]
        public void A_handler_runs_its_command()
        {
            var (model, host) = Made();
            var changed = 0;
            host.Changed += () => changed++;
            host.Run("approveButton", new HandlerCall("click", "approveClicked", []));
            Assert.Equal(1, model.Approved);
            Assert.False(model.ApproveButton.IsEnabled);
            Assert.True(changed > 0);
        }

        [Fact]
        public void A_call_with_values_runs_the_command_the_generator_names_for_it()
        {
            var (model, host) = Made();
            host.Run("seven", WindowMarkup.HandlerOf("click", "digit(7)"));
            host.Run("half", WindowMarkup.HandlerOf("click", "scale(0.5, \"half\")"));
            Assert.Equal([7L], model.Digits);
            Assert.Equal(["0.5 half"], model.Scaled);
            Assert.Equal("Digit7", ViewModelHost.CallCommand(WindowMarkup.HandlerOf("click", "digit(7)")));
            Assert.Equal("ScaleMinus1_5_", ViewModelHost.CallCommand(WindowMarkup.HandlerOf("click", "scale(-1.5, \"\")")));
            Assert.Equal("PickTrueRed", ViewModelHost.CallCommand(WindowMarkup.HandlerOf("click", "pick(true, red)")));
            Assert.Equal("SayHelloWorld", ViewModelHost.CallCommand(WindowMarkup.HandlerOf("click", "say(\"hello world\")")));
        }

        [Fact]
        public void A_change_handler_runs_inside_the_edit()
        {
            var (model, host) = Made("<Textbox name=\"note\" bind=\"order.note\" onchange=\"noteChanged\" undoScope=\"note\" />");
            host.Write("note", "order.note", "typed");
            host.Run("note", new HandlerCall("change", "noteChanged", []));
            Assert.Equal(1, model.NoteChanges);
            Assert.Equal(Status.Done, model.Order.Status);
            // What the handler changed is taken back with the edit.
            while (model.Edits.CanUndo("note"))
                model.UndoEdit("note", back: true);
            Assert.Equal(Status.Open, model.Order.Status);
            Assert.Equal("", model.Order.Note);
        }

        [Fact]
        public void A_click_that_opens_a_window_runs_its_open_command()
        {
            var (model, host) = Made();
            WindowViewModel? opened = null;
            model.OpenRequested += (other, _) => opened = other;
            host.Run("usersButton", new HandlerCall("click", "Other", []));
            Assert.IsType<OtherViewModel>(opened);
        }

        [Fact]
        public void A_built_in_target_runs_its_command()
        {
            var (model, host) = Made();
            var closed = false;
            model.CloseRequested += () => closed = true;
            host.Run("closeButton", new HandlerCall("click", "close", []));
            Assert.True(closed);
        }

        [Fact]
        public void A_handler_the_view_model_has_not_is_said_and_the_window_goes_on()
        {
            var (model, host) = Made();
            host.Run("x", new HandlerCall("click", "nowhere", []));
            var said = Assert.Single(model.Messages);
            Assert.Equal(Level.Error, said.Level);
            host.Run("y", new HandlerCall("click", "failing", []));
            Assert.Equal(2, model.Messages.Count);
        }

        [Fact]
        public void A_part_state_is_the_view_models_by_its_name()
        {
            var (model, host) = Made();
            Assert.Same(model.ApproveButton, host.State("approveButton"));
            Assert.Same(model.Grid, host.State("grid"));
            Assert.Null(host.State("nothing"));
            Assert.Null(host.State(null));
        }

        [Fact]
        public void The_host_says_when_the_view_model_or_a_part_state_changed()
        {
            var (model, host) = Made();
            var changed = 0;
            host.Changed += () => changed++;
            model.Shown = 3;
            Assert.Equal(1, changed);
            model.Note.Text = "x";
            Assert.Equal(2, changed);
            host.Dispose();
            model.Shown = 4;
            Assert.Equal(2, changed);
        }

        [Fact]
        public void A_text_is_translated_by_its_key_and_plain_text_stays()
        {
            var (_, host) = Made();
            Assert.Equal("", host.Text(null));
            Assert.Equal("plain words", host.Text("plain words"));
            Assert.Equal("someMissingKey", host.Text("@someMissingKey"));
        }

        /// <summary>Braces a translation writes doubled show single in a part's text, a placeholder's too (hosts-7).</summary>
        [Fact]
        public void A_translated_text_shows_its_braces_single()
        {
            var (_, host) = Made();
            Assert.Equal("Type {name}", host.Text("@Type {{name}}"));
            Assert.Equal("Type {{name}}", host.Text("Type {{name}}"));
        }

        /// <summary>A command that cannot run now says so, so a timer leaves its tick out (hosts-4).</summary>
        [Fact]
        public void A_handler_whose_command_cannot_run_is_known()
        {
            var (_, host) = Made();
            Assert.True(host.CanRun("approveButton", new HandlerCall("click", "approveClicked", [])));
            Assert.True(host.CanRun("x", new HandlerCall("click", "nowhere", [])));
        }

        [Fact]
        public void An_enum_offers_its_generated_choices_and_a_reference_its_records()
        {
            var (model, host) = Made();
            Assert.Equal(["Open", "Done"], host.Choices("order.status").Select(c => c.Label));
            Assert.Equal(Status.Done, host.Choices("order.status")[1].Value);
            // The field's facts say what it refers to, over its name.
            Assert.Same(model.HelperChoices, host.Choices("order.finished_by"));
            Assert.Same(model.HelperChoices, host.Choices("orders.finished_by"));
            Assert.Empty(host.Choices("order.note"));
        }

        /// <summary>A plain whole number whose facts refer to nothing is no reference: it offers no records (data-3).</summary>
        [Fact]
        public void A_plain_integer_field_offers_no_records()
        {
            var (model, host) = Made();
            Assert.Empty(host.Choices("order.count"));
            Assert.Empty(host.Choices("orders.count"));
            Assert.Same(model.HelperChoices, host.Choices("order.finished_by"));
        }

        /// <summary>A handler named as a part runs by the command the generator named around the part (RedInk_).</summary>
        [Fact]
        public void A_handler_named_as_a_part_runs_by_its_stepped_around_command()
        {
            var (model, host) = Made();
            Assert.True(host.Run("redInk", new HandlerCall("click", "redInk", [])));
            Assert.Equal(1, model.Inked);
        }

        [Fact]
        public void A_typed_dropdown_offers_its_fields_values_and_a_search_one_its_records_with_their_cells()
        {
            var (model, host) = Made();
            var typed = WindowMarkup.Read("""<Window name="Orders"><Dropdown at="1,1" bind="order.note" typed="true" /></Window>""").Parts[0];
            Assert.Equal(["first", "second", "first"], host.Offered(typed, "order.note").Select(c => c.Label));
            var search = WindowMarkup.Read("""<Window name="Orders"><Dropdown at="1,1" bind="order.finished_by" search="true" show="name, role" /></Window>""").Parts[0];
            Assert.Same(model.HelperNameRoleChoices, host.Offered(search, "order.finished_by"));
            // A search without fields to show picks from the reference's own choices; a plain dropdown too.
            var plain = WindowMarkup.Read("""<Window name="Orders"><Dropdown at="1,1" bind="order.finished_by" search="true" /></Window>""").Parts[0];
            Assert.Same(model.HelperChoices, host.Offered(plain, "order.finished_by"));
        }

        [Fact]
        public void A_window_of_a_view_model_makes_its_search_dropdown()
        {
            var model = new OrdersViewModel();
            var spec = WindowMarkup.Read("""<Window name="Orders"><Dropdown at="1,1" span="4" bind="order.finished_by" search="true" show="name, role" /></Window>""");
            var host = new ViewModelHost(model, spec);
            var part = Assert.IsType<SearchDropdownPart>(PartKinds.Make(spec.Parts[0], host));
            Assert.Equal(["Cy, lead"], part.Offered.Select(SearchDropdownPart.Line));
        }

        [Fact]
        public async Task A_stored_file_is_fetched_through_the_generated_fetchers()
        {
            var (_, host) = Made();
            Assert.Equal([1], await host.Fetch("order.note", null));
            Assert.Equal([2], await host.Fetch("order.note", "a/b.pdf"));
            Assert.Null(await host.Fetch("order.total", null));
            Assert.Null(await host.Fetch("order", null));
        }

        [Fact]
        public void A_canvas_runs_its_generated_commands_and_a_command_it_has_not_is_none()
        {
            var (model, host) = Made();
            Assert.True(host.Pointed("pad", "click", new Pointer(3, 4)));
            Assert.Null(host.Pointed("pad", "drag", new Pointer(3, 4)));
            Assert.True(host.Acted("pad", new System.Text.Json.Nodes.JsonObject { ["kind"] = "move" }));
            Assert.Null(host.Acted("other", []));
            Assert.Equal(["click 3,4", "act {\"kind\":\"move\"}"], model.Canvased);
            Assert.Null(host.Engine("nothing"));
        }

        [Fact]
        public void A_commands_keys_are_the_persons_else_its_designed_one()
        {
            var (_, host) = Made();
            try
            {
                PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>> { ["Orders.saveButton"] = ["Ctrl+K"] });
                Assert.Equal(["Ctrl+K"], host.KeysOf("Orders.saveButton", "Ctrl+S"));
                Assert.Equal(["Ctrl+P"], host.KeysOf("print", "Ctrl+P"));
                Assert.Empty(host.KeysOf("print", null));
            }
            finally
            {
                PersonKeys.Stop();
            }
        }

        [Fact]
        public void What_a_part_says_is_among_the_windows_messages()
        {
            var (model, host) = Made();
            host.Say(Level.Info, Text.Plain("saved"));
            Assert.Equal("saved", Locale.Show(Assert.Single(model.Messages).Text));
        }

        [Fact]
        public void A_fields_facts_are_the_generated_fields_of_its_records_type()
        {
            var (_, host) = Made();
            Assert.Equal("@orderTotalLabel", host.Field("order.total")?.Label);
            Assert.True(host.Field("order.total")!.Required);
            Assert.Equal(("0", "500"), (host.Field("orders[1].total")!.Min, host.Field("orders.total")!.Max));
            Assert.Equal("Helper", host.Field("orders.finished_by")?.Refers);
            Assert.Null(host.Field("order.status"));
            Assert.Null(host.Field("shown"));
            Assert.Null(host.Field("nowhere.total"));
        }

        [Fact]
        public void A_handler_says_whether_it_ran_or_refused()
        {
            var (model, host) = Made();
            Assert.True(host.Run("approveButton", new HandlerCall("click", "approveClicked", [])));
            Assert.False(host.Run("next", new HandlerCall("next", "refuse", [])));
            Assert.Equal(Level.Error, Assert.Single(model.Messages).Level);
            Assert.False(host.Run("x", new HandlerCall("click", "nowhere", [])));
            Assert.True(host.Run("seven", WindowMarkup.HandlerOf("click", "digit(7)")));
            // A call with values says it too.
            Assert.False(host.Run("one", WindowMarkup.HandlerOf("click", "refuseWith(1)")));
            Assert.Equal(3, model.Messages.Count);
        }

        [Fact]
        public void An_unbound_parts_own_text_is_an_edit_undo_takes_back()
        {
            var (model, host) = Made("<Textbox name=\"note\" undoScope=\"note\" />");
            model.Opened();
            model.Note.Text = "typed";
            host.Edited("note");
            Assert.True(model.Edits.CanUndo("note"));
            model.UndoEdit("note", back: true);
            Assert.Equal("", model.Note.Text);
        }

        [Fact]
        public void A_button_that_opens_a_window_opens_it_when_clicked()
        {
            var model = new OrdersViewModel();
            var spec = WindowMarkup.Read("<Window name=\"Orders\"><Button name=\"users\" at=\"1,1\" span=\"3\" caption=\"Users\" opens=\"Other\" /></Window>");
            var host = new ViewModelHost(model, spec);
            WindowViewModel? opened = null;
            model.OpenRequested += (other, _) => opened = other;
            var button = (ButtonPart)PartKinds.Make(spec.Parts[0], host);
            button.Click();
            Assert.IsType<OtherViewModel>(opened);
        }

        [Fact]
        public void An_address_goes_to_the_hosts_opener()
        {
            var opened = new List<string>();
            var host = new ViewModelHost(new OrdersViewModel()) { Opener = opened.Add };
            host.OpenAddress("https://example.org");
            Assert.Equal(["https://example.org"], opened);
            Assert.Null(host.Resource("not-there.png"));
            Assert.Null(host.IconPath("not-there"));
        }
    }
}
