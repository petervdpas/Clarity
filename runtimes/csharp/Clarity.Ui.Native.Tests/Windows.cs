// The windows the tests drive: view models written as the C# generator
// writes them (commands named after their handlers, On<Handler> methods,
// part states, choices), with the markup the library draws them from.

using System.Windows.Input;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Native.Tests;

public sealed class Book
{
    public long Id { get; set; }
    public string Title { get; set; } = "";
    public long Pages { get; set; }
}

public sealed class ShelfViewModel() : WindowViewModel(Db.Empty())
{
    public const string Markup = """
        <Window name="Shelf">
          <Textbox name="note" at="1,1" span="6" bind="note" undoScope="note" />
          <Button name="saveButton" at="7,1" span="2" caption="Save" onclick="save" />
          <Button name="failButton" at="9,1" span="2" caption="Fail" onclick="fail" />
          <Button name="askButton" at="11,1" span="2" caption="Ask" onclick="ask" />
          <Table name="shelf" at="1,2" span="12" rows="6" bind="books" columns="title, pages" edit="title, pages" undoScope="shelf" />
          <Button name="digitButton" at="1,9" span="2" caption="Seven" onclick="digit(7)" />
          <Button name="otherButton" at="3,9" span="2" caption="Other" onclick="openOther" />
          <Canvas name="pad" at="1,10" span="12" rows="2" onclick="padClicked" ondrag="padDragged" />
        </Window>
        """;

    public override string Title => "Shelf";

    public List<Book> Books { get; set; } = [new() { Id = 1, Title = "Dune", Pages = 412 }, new() { Id = 2, Title = "Emma", Pages = 474 }];

    public string Note { get; set; } = "";

    public string FinishedBy { get; set; } = "nobody";

    public PartState NoteBox { get; } = new("note");
    public PartState SaveButton { get; } = new("saveButton", caption: "saveCaption");
    public PartState Shelf { get; } = new("shelf");
    public PartState Pad { get; } = new("pad");

    /// <summary>Where the pad's command was last given the pointer, as the generator's ClickAt_ and DragAt_ take it.</summary>
    public Pointer? PadAt { get; private set; }
    public string PadEvent { get; private set; } = "";

    public ICommand ClickAt_PadCommand => new RelayCommand<Pointer>(at => (PadAt, PadEvent) = (at, "click"));
    public ICommand DragAt_PadCommand => new RelayCommand<Pointer>(at => (PadAt, PadEvent) = (at, "drag"));

    public Choice[] PersonChoices { get; } = [new(null, ""), new(1L, "Ann"), new(2L, "Bob")];

    public int Saved { get; private set; }
    public long Digit { get; private set; }
    public object? Answered { get; private set; } = "none";

    public ICommand SaveCommand => new RelayCommand(() => Handle(OnSave));
    public ICommand FailCommand => new RelayCommand(() => Handle(OnFail));
    public ICommand AskCommand => new RelayCommand(() => Handle(OnAsk));
    public ICommand OpenOtherCommand => new RelayCommand(() => Handle(OnOpenOther));
    public ICommand Digit7Command => new RelayCommand(() => Run_Digit7());
    public ICommand QuitNowCommand => new RelayCommand(() => Handle(OnQuitNow));

    void OnQuitNow() => QuitApplication();

    public bool Run_Digit7() => Handle(() => OnDigit(7), command: "Digit7");

    void OnSave()
    {
        Saved++;
        Say(Level.Info, Text.Plain("saved"));
    }

    void OnFail() => throw new InvalidOperationException("the shelf broke");

    void OnAsk() => Ask(Text.Plain("Who?"), new ValueForm(null, PersonChoices, null), answer => Answered = answer);

    void OnOpenOther() => Open(new OtherViewModel());

    void OnDigit(long digit) => Digit = digit;

    public void Told(long count) => Digit = count;

    void OnCounted(long count) => Digit = count * 10;
}

public sealed class OtherViewModel() : WindowViewModel(Db.Empty())
{
    public const string Markup = """
        <Window name="Other">
          <Label name="hello" at="1,1" span="4" caption="Other" />
        </Window>
        """;

    public override string Title => "Other";
}
