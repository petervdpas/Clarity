// The Avalonia half of the runtime: what the generated ViewModels and
// windows stand on, tested headless.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Threading;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

[assembly: AvaloniaTestApplication(typeof(Clarity.Avalonia.Tests.TestApp))]

namespace Clarity.Avalonia.Tests;

public class TestApp : Application
{
    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<TestApp>().UseHeadless(new AvaloniaHeadlessPlatformOptions());

    // The themes a generated application has, so controls have their parts and can be clicked.
    public override void Initialize()
    {
        Styles.Add(new global::Avalonia.Themes.Fluent.FluentTheme());
        Styles.Add(new global::Avalonia.Markup.Xaml.Styling.StyleInclude(new Uri("avares://Clarity.Avalonia.Tests"))
        {
            Source = new Uri("avares://Avalonia.Controls.DataGrid/Themes/Fluent.xaml"),
        });
    }
}

/// A window as Clarity generates one: data, parts and handlers.
sealed class CardViewModel(Db db, string title = "Card", bool dialog = false) : WindowViewModel(db)
{
    public override string Title => title;
    public override bool IsDialog => dialog;
    public List<string> Rows { get; set; } = ["a", "b"];
    public PartState Grid { get; } = new("grid");
    public PartState Save { get; } = new("save", caption: "Save");
    public List<string> Said { get; } = [];
    public CardViewModel? Child { get; set; }

    protected override void OnOpen() => Said.Add("open");
    protected override void OnClose() => Said.Add("close");
    protected override void OnReturn() => Said.Add("return");

    public void Refuse() => Handle(() => throw new RefusedException(new Text("Order {n} is over").With("n", Slot.OfText("ORD-1"))));
    public void Fail() => Handle(() => throw new NotFoundException("customer", 7));
    public void Fine() => Handle(() => Said.Add("fine"));
    public string Picked() => Selected<string>(Grid);
    public void OpenChild() => Handle(() => Open(Child = new CardViewModel(Db, "Child")));
    public void OpenDialog() => Handle(() => Open(Child = new CardViewModel(Db, "Ask", dialog: true)));
    public void SaveAndClose() => Handle(() =>
    {
        Say(Level.Info, Text.Plain("Saved"));
        Close();
    });
    public void CloseMe() => Handle(Close);
}

public class WindowTests
{
    [AvaloniaFact]
    public void HandlersShowWhatWentWrongAsAnErrorMessage()
    {
        using var db = Db.Memory();
        var vm = new CardViewModel(db);
        vm.Fine();
        Assert.Empty(vm.Messages);
        vm.Refuse();
        vm.Fail();
        Assert.Equal([Level.Error, Level.Error], vm.Messages.Select(m => m.Level));
        Assert.Equal(["Order ORD-1 is over", "there is no customer with id 7"], vm.Messages.Select(m => m.Shown));
        vm.Say(Level.Info, Text.Plain("Saved"));
        Assert.Equal(Level.Info, vm.Messages[^1].Level);
    }

    [AvaloniaFact]
    public void ASelectedRowIsReadFromItsPart()
    {
        using var db = Db.Memory();
        var vm = new CardViewModel(db);
        Assert.Equal("no row of grid is selected", Assert.Throws<InvalidValueException>(vm.Picked).Message.Replace("invalid value: ", ""));
        vm.Grid.SelectedItem = "b";
        Assert.Equal("b", vm.Picked());
    }

    [AvaloniaFact]
    public void APartShowsItsErrorBeforeItsHelp()
    {
        var part = new PartState("name", help: "Your full name");
        Assert.Equal("Your full name", part.Shown);
        var changed = new List<string?>();
        part.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        part.ErrorText = Text.Plain("Name is required");
        Assert.Equal("Name is required", part.Shown);
        Assert.True(part.HasError);
        Assert.Contains(nameof(PartState.Shown), changed);
        Assert.True(part.IsEnabled && part.IsVisible && !part.IsReadOnly);
    }

    static Control View(WindowViewModel vm) => new ContentControl { DataContext = vm };

    [AvaloniaFact]
    public void WindowsOpenInPlaceAndCloseBackToTheOneBelow()
    {
        using var db = Db.Memory();
        var card = new CardViewModel(db);
        var shell = Shell.Start(card, View);
        Assert.True(shell.IsVisible);
        Assert.Equal("Card", shell.Title);
        Assert.Equal(["open"], card.Said);

        card.OpenChild();
        var child = card.Child!;
        // In the same desktop window, on top of the card.
        Assert.Same(child, shell.Current);
        Assert.Equal("Child", shell.Title);
        Assert.Equal(["open"], child.Said);

        child.SaveAndClose();
        Assert.Equal(["open", "close"], child.Said);
        // The window below is back, told so it can reload its data.
        Assert.Same(card, shell.Current);
        Assert.Equal("Card", shell.Title);
        Assert.Equal(["open", "return"], card.Said);
        // A message said just before closing stays on screen.
        Assert.Equal(["Saved"], shell.Messages.Select(m => m.Shown));
        Assert.Same(shell.Messages, card.Messages);

        // Closing the last window closes the desktop window.
        card.CloseMe();
        Assert.False(shell.IsVisible);
        Assert.Equal(["open", "return", "close"], card.Said);
    }

    [AvaloniaFact]
    public void OnlyADialogOpensADesktopWindowOfItsOwn()
    {
        using var db = Db.Memory();
        var card = new CardViewModel(db);
        var shell = Shell.Start(card, View);
        card.OpenDialog();
        var dialog = card.Child!;
        Assert.Same(card, shell.Current);
        var window = Assert.Single(shell.OwnedWindows);
        Assert.Equal("Ask", window.Title);
        Assert.Equal(["open"], dialog.Said);

        dialog.CloseMe();
        Assert.Empty(shell.OwnedWindows);
        Assert.Equal(["open", "return"], card.Said);
        Assert.True(shell.IsVisible);
    }

    [AvaloniaFact]
    public async Task InfoMessagesGoAwayByThemselves()
    {
        using var db = Db.Memory();
        var vm = new CardViewModel(db);
        var list = new MessageList { InfoDuration = TimeSpan.FromMilliseconds(50), Messages = vm.Messages };
        var window = new Window { Content = list };
        window.Show();
        vm.Say(Level.Info, Text.Plain("Saved"));
        vm.Say(Level.Warning, Text.Plain("Careful"));
        Assert.Equal(2, vm.Messages.Count);
        for (var i = 0; i < 20 && vm.Messages.Count > 1; i++)
        {
            await Task.Delay(20);
            Dispatcher.UIThread.RunJobs();
        }
        // Warnings and errors stay until they are dismissed.
        Assert.Equal(["Careful"], vm.Messages.Select(m => m.Shown));
        list.Dismiss(vm.Messages[0]);
        Assert.Empty(vm.Messages);
    }
}

public class BindingTests
{
    enum Kind
    {
        Private,
        Business,
    }

    sealed record Person(long Id, string Name);

    [AvaloniaFact]
    public void ChoicesOfRecordsStartEmptyAndHoldTheirIds()
    {
        var choices = Choice.Of([new Person(1, "Ada"), new Person(2, "Grace")], p => p.Name, p => p.Id);
        Assert.Equal([null, 1L, 2L], choices.Select(c => c.Value));
        Assert.Equal(["", "Ada", "Grace"], choices.Select(c => c.Label));
    }

    [AvaloniaFact]
    public void ValuesShowInThePersonsFormatWithEnumLabels()
    {
        Locale.Start("en", [], "EUR", (_, _) => null, (_, v) => v, new System.Globalization.CultureInfo("en-US"));
        try
        {
            var shown = new ShownConverter(v => v is Kind k ? (k == Kind.Private ? "Private person" : "Business") : null);
            var culture = System.Globalization.CultureInfo.InvariantCulture;
            Assert.Equal("Private person", shown.Convert(Kind.Private, typeof(string), null, culture));
            // Without a field's scale a decimal keeps its own digits.
            Assert.Equal("1,250.5", shown.Convert(1250.50m, typeof(string), null, culture));
            Assert.Equal("10/17/2026", shown.Convert(new DateOnly(2026, 10, 17), typeof(string), null, culture));
            Assert.Equal("Yes", shown.Convert(true, typeof(string), null, culture));
            Assert.Equal("", shown.Convert(null, typeof(string), null, culture));
            var money = new ShownConverter(_ => null, new FieldFormat(FormatKind.Money));
            Assert.Equal("€1,250.50", money.Convert(1250.5m, typeof(string), null, culture));
        }
        finally
        {
            Locale.Reset();
        }
    }

    [AvaloniaFact]
    public void EnterInAFormSubmitsIt()
    {
        var box = new TextBox();
        var form = new Border { Child = box };
        var window = new Window { Content = form };
        window.Show();
        var submits = 0;
        Events.OnEnter(form, () => submits++);
        box.Focus();
        window.KeyPressQwerty(global::Avalonia.Input.PhysicalKey.Enter, global::Avalonia.Input.RawInputModifiers.None);
        Assert.Equal(1, submits);
    }

    [AvaloniaFact]
    public void ADateTimeInputPicksTheDayAndTypesTheTime()
    {
        var box = new DateTimeInput();
        var other = new Button();
        var window = new Window { Content = new StackPanel { Children = { box, other } } };
        window.Show();
        var changes = 0;
        box.ValueChanged += (_, _) => changes++;

        box.Day.SelectedDate = new DateTime(2026, 10, 19);
        Assert.Equal(new DateTime(2026, 10, 19, 0, 0, 0), box.Value);
        Assert.Equal("00:00", box.Time.Text);

        // The time is typed, and taken when the field is left, as a text input is.
        box.Time.Focus();
        box.Time.Text = "14:30";
        Assert.Equal(new DateTime(2026, 10, 19, 0, 0, 0), box.Value);
        other.Focus();
        Assert.Equal(new DateTime(2026, 10, 19, 14, 30, 0), box.Value);
        Assert.Equal(2, changes);

        // A time that is no time is marked, and the moment stays.
        box.Time.Focus();
        box.Time.Text = "25:99";
        other.Focus();
        Assert.Contains("error", box.Time.Classes);
        Assert.Equal(new DateTime(2026, 10, 19, 14, 30, 0), box.Value);

        // A value from the data shows in both, and is no change by the person.
        box.Value = new DateTime(2027, 1, 2, 8, 5, 0);
        Assert.Equal(new DateTime(2027, 1, 2), box.Day.SelectedDate);
        Assert.Equal("08:05", box.Time.Text);
        Assert.DoesNotContain("error", box.Time.Classes);
        Assert.Equal(2, changes);

        box.Day.SelectedDate = null;
        Assert.Null(box.Value);
        Assert.Equal("", box.Time.Text);
        Assert.Equal(3, changes);
    }

    [AvaloniaFact]
    public void AChoiceInputWritesOnlyWhatThePersonPicks()
    {
        var people = Choice.Of(new[] { (1L, "Ada"), (2L, "Grace") }, p => p.Item2, p => p.Item1);
        // The value comes before the choices, as a binding to the window may give them.
        var box = new ChoiceInput { Value = 2L };
        var window = new Window { Content = box };
        window.Show();
        var changes = 0;
        box.ValueChanged += (_, _) => changes++;
        box.Choices = people;
        Assert.Equal(2L, box.Value);
        Assert.Equal("Grace", (box.Combo.SelectedItem as Choice)?.Label);

        box.Combo.SelectedIndex = 1;
        Assert.Equal(1L, box.Value);
        box.Combo.SelectedIndex = 0;
        Assert.Null(box.Value);
        Assert.Equal(2, changes);
    }

}
