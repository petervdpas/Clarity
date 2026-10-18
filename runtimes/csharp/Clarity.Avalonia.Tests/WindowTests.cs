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
    public void DatesConvertBetweenTheRecordAndThePicker()
    {
        var date = Converters.Date;
        var culture = System.Globalization.CultureInfo.InvariantCulture;
        Assert.Equal(new DateTime(2026, 10, 17), date.Convert(new DateOnly(2026, 10, 17), typeof(DateTime?), null, culture));
        Assert.Null(date.Convert(null, typeof(DateTime?), null, culture));
        Assert.Equal(new DateOnly(2026, 10, 17), date.ConvertBack(new DateTime(2026, 10, 17, 13, 5, 0), typeof(DateOnly?), null, culture));
        Assert.Null(date.ConvertBack(null, typeof(DateOnly?), null, culture));
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
    public void ARadioButtonIsCheckedForItsValue()
    {
        var equal = Converters.Equal;
        var culture = System.Globalization.CultureInfo.InvariantCulture;
        Assert.Equal(true, equal.Convert(Kind.Business, typeof(bool?), Kind.Business, culture));
        Assert.Equal(false, equal.Convert(Kind.Private, typeof(bool?), Kind.Business, culture));
        Assert.Equal(Kind.Business, equal.ConvertBack(true, typeof(Kind), Kind.Business, culture));
        Assert.Equal(global::Avalonia.Data.BindingOperations.DoNothing, equal.ConvertBack(false, typeof(Kind), Kind.Business, culture));
    }

    [AvaloniaFact]
    public void ARadioGroupChangesWhenAButtonIsChecked()
    {
        var first = new RadioButton { IsChecked = true };
        var second = new RadioButton();
        var group = new StackPanel { Children = { first, second } };
        var window = new Window { Content = group };
        window.Show();
        var changes = 0;
        Events.OnChoice(group, () => changes++);
        second.IsChecked = true;
        // The button that was unchecked tells nothing.
        Assert.Equal(1, changes);
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
    public void ATextBoxCommitsWhenFocusLeavesIt()
    {
        var box = new TextBox();
        var other = new Button();
        var window = new Window { Content = new StackPanel { Children = { box, other } } };
        window.Show();
        var commits = 0;
        Events.OnCommit(box, () => commits++);
        box.Focus();
        box.Text = "Ada";
        Assert.Equal(0, commits);
        other.Focus();
        Assert.Equal(1, commits);
    }

    sealed record Row(string Title);

    [AvaloniaFact]
    public void AnEditInATableCellTellsItsRow()
    {
        var first = new Row("Milk");
        var second = new Row("Bread");
        var check = new CheckBox { DataContext = second };
        var choice = new ComboBox { DataContext = first, ItemsSource = new[] { "Low", "High" } };
        var box = new TextBox { DataContext = first };
        var other = new Button();
        var table = new StackPanel { Children = { check, choice, box } };
        var window = new Window { Content = new StackPanel { Children = { table, other } } };
        window.Show();
        var edited = new List<object?>();
        Events.OnCellEdit(table, row => edited.Add(row));

        // Values the rows show when they are drawn are no edits.
        check.IsChecked = true;
        Assert.Empty(edited);

        check.Focus();
        check.IsChecked = false;
        choice.Focus();
        choice.SelectedIndex = 1;
        box.Focus();
        box.Text = "Oat milk";
        other.Focus();
        Assert.Equal(new object?[] { second, first, first }, edited);
    }

    sealed class TaskRow
    {
        public string Title { get; set; } = "";
        public bool Done { get; set; }
    }

    [AvaloniaFact]
    public void ClickingACheckBoxInATableEditsItsRowAndSelectsIt()
    {
        var milk = new TaskRow { Title = "Milk" };
        var bread = new TaskRow { Title = "Bread" };
        var grid = new DataGrid
        {
            ItemsSource = new[] { milk, bread },
            AutoGenerateColumns = false,
            IsReadOnly = true,
            Columns =
            {
                new DataGridTextColumn { Header = "Task", Binding = new global::Avalonia.Data.Binding("Title"), Width = new DataGridLength(1, DataGridLengthUnitType.Star) },
                new DataGridTemplateColumn
                {
                    Header = "Done",
                    CellTemplate = new global::Avalonia.Controls.Templates.FuncDataTemplate<TaskRow>((_, _) =>
                        new CheckBox { [!global::Avalonia.Controls.Primitives.ToggleButton.IsCheckedProperty] = new global::Avalonia.Data.Binding("Done") }),
                },
            },
        };
        var window = new Window { Width = 400, Height = 300, Content = grid };
        window.Show();
        var edited = new List<object?>();
        Events.OnCellEdit(grid, row =>
        {
            grid.SelectedItem = row;
            edited.Add(row);
        });
        Dispatcher.UIThread.RunJobs();

        var box = global::Avalonia.VisualTree.VisualExtensions.GetVisualDescendants(grid).OfType<CheckBox>().First(c => c.DataContext == bread);
        var middle = box.TranslatePoint(new Point(box.Bounds.Width / 2, box.Bounds.Height / 2), window)!.Value;
        window.MouseDown(middle, global::Avalonia.Input.MouseButton.Left);
        window.MouseUp(middle, global::Avalonia.Input.MouseButton.Left);

        Assert.True(bread.Done);
        Assert.Equal(new object?[] { bread }, edited);
        Assert.Same(bread, grid.SelectedItem);
    }
}
