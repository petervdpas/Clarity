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

    /// A record as generated: it tells when a field changes.
    sealed class TaskRow(string title) : System.ComponentModel.INotifyPropertyChanged
    {
        public event System.ComponentModel.PropertyChangedEventHandler? PropertyChanged;
        string title = title;
        bool done;
        string notes = "";
        DateTime? due;
        long? owner;
        public DateTime? Due { get => due; set => Set(ref due, value); }
        public long? Owner { get => owner; set => Set(ref owner, value); }
        public string Title { get => title; set => Set(ref title, value); }
        public bool Done { get => done; set => Set(ref done, value); }
        public string Notes { get => notes; set => Set(ref notes, value); }

        void Set<T>(ref T field, T value, [System.Runtime.CompilerServices.CallerMemberName] string name = "")
        {
            if (EqualityComparer<T>.Default.Equals(field, value))
                return;
            field = value;
            PropertyChanged?.Invoke(this, new(name));
        }
    }

    static DataGrid Table(IEnumerable<TaskRow> rows, Control? titleCell = null) => new()
    {
        ItemsSource = rows,
        AutoGenerateColumns = false,
        IsReadOnly = true,
        Columns =
        {
            new DataGridTemplateColumn
            {
                Header = "Task",
                CellTemplate = new global::Avalonia.Controls.Templates.FuncDataTemplate<TaskRow>((_, _) =>
                    new TextBox { [!TextBox.TextProperty] = new global::Avalonia.Data.Binding("Title") { UpdateSourceTrigger = global::Avalonia.Data.UpdateSourceTrigger.LostFocus } }),
            },
            new DataGridTemplateColumn
            {
                Header = "Done",
                CellTemplate = new global::Avalonia.Controls.Templates.FuncDataTemplate<TaskRow>((_, _) =>
                    new CheckBox { [!global::Avalonia.Controls.Primitives.ToggleButton.IsCheckedProperty] = new global::Avalonia.Data.Binding("Done") }),
            },
        },
    };

    [AvaloniaFact]
    public void AChangedFieldOfAnEditedColumnIsAnEditOfItsRow()
    {
        var milk = new TaskRow("Milk");
        var bread = new TaskRow("Bread");
        var grid = Table([milk, bread]);
        var window = new Window { Content = grid };
        window.Show();
        var handling = false;
        var edited = new List<object?>();
        Events.OnCellEdit(grid, () => handling, ["Title", "Done"], row => edited.Add(row));

        milk.Done = true;
        milk.Done = true; // the same value again is no change
        milk.Notes = "not a column that is edited";
        handling = true;
        bread.Done = true; // what a handler changes is no edit
        handling = false;
        Assert.Equal(new object?[] { milk }, edited);

        // The data read again: the new rows count, the old ones no longer.
        var eggs = new TaskRow("Eggs");
        grid.ItemsSource = new[] { eggs };
        milk.Done = false;
        eggs.Title = "Six eggs";
        Assert.Equal(new object?[] { milk, eggs }, edited);
    }

    [AvaloniaFact]
    public void ClickingACheckBoxInATableEditsItsRowAndSelectsIt()
    {
        var milk = new TaskRow("Milk");
        var bread = new TaskRow("Bread");
        var grid = Table([milk, bread]);
        var window = new Window { Width = 400, Height = 300, Content = grid };
        window.Show();
        var edited = new List<object?>();
        Events.OnCellEdit(grid, () => false, ["Title", "Done"], row =>
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

    [AvaloniaFact]
    public void ATextCellCommitsOnEnterOnceAsOnTheWeb()
    {
        var milk = new TaskRow("Milk");
        var grid = Table([milk]);
        var other = new Button();
        var window = new Window { Width = 400, Height = 300, Content = new StackPanel { Children = { grid, other } } };
        window.Show();
        var edited = new List<object?>();
        Events.OnCellEdit(grid, () => false, ["Title", "Done"], row => edited.Add(row));
        Dispatcher.UIThread.RunJobs();

        var box = global::Avalonia.VisualTree.VisualExtensions.GetVisualDescendants(grid).OfType<TextBox>().First();
        box.Focus();
        box.Text = "Oat milk";
        Assert.Empty(edited); // typing is no edit yet
        window.KeyPressQwerty(global::Avalonia.Input.PhysicalKey.Enter, global::Avalonia.Input.RawInputModifiers.None);
        Assert.Equal("Oat milk", milk.Title);
        other.Focus();
        Assert.Equal(new object?[] { milk }, edited);
    }

    [AvaloniaFact]
    public void ADateTimeBoxPicksTheDayAndTypesTheTime()
    {
        var box = new DateTimeBox();
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
    public void AChoiceBoxWritesOnlyWhatThePersonPicks()
    {
        var people = Choice.Of(new[] { (1L, "Ada"), (2L, "Grace") }, p => p.Item2, p => p.Item1);
        // The value comes before the choices, as a binding to the window may give them.
        var box = new ChoiceBox { Value = 2L };
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

    [AvaloniaFact]
    public void DrawingRowsWithPickersAndDropdownsIsNoEdit()
    {
        var milk = new TaskRow("Milk") { Due = new DateTime(2026, 10, 20, 9, 30, 0), Owner = 1 };
        var none = new TaskRow("Report");
        var people = Choice.Of(new[] { (1L, "Ada"), (2L, "Grace") }, p => p.Item2, p => p.Item1);
        var grid = new DataGrid
        {
            ItemsSource = new[] { milk, none },
            AutoGenerateColumns = false,
            IsReadOnly = true,
            Columns =
            {
                new DataGridTemplateColumn
                {
                    Header = "Owner",
                    // The choices come from the window, after the row's value: as generated.
                    CellTemplate = new global::Avalonia.Controls.Templates.FuncDataTemplate<TaskRow>((_, _) => new ChoiceBox
                    {
                        [!ChoiceBox.ValueProperty] = new global::Avalonia.Data.Binding("Owner"),
                        [!ChoiceBox.ChoicesProperty] = new global::Avalonia.Data.Binding("Tag") { RelativeSource = new global::Avalonia.Data.RelativeSource(global::Avalonia.Data.RelativeSourceMode.FindAncestor) { AncestorType = typeof(DataGrid) } },
                    }),
                },
                new DataGridTemplateColumn
                {
                    Header = "Due",
                    CellTemplate = new global::Avalonia.Controls.Templates.FuncDataTemplate<TaskRow>((_, _) =>
                        new DateTimeBox { [!DateTimeBox.ValueProperty] = new global::Avalonia.Data.Binding("Due") }),
                },
            },
        };
        grid.Tag = people;
        var window = new Window { Width = 600, Height = 300, Content = grid };
        var edited = new List<object?>();
        Events.OnCellEdit(grid, () => false, ["Owner", "Due"], row => edited.Add(row));
        window.Show();
        Dispatcher.UIThread.RunJobs();
        Assert.Empty(edited);
        Assert.Equal(new DateTime(2026, 10, 20, 9, 30, 0), milk.Due);
        Assert.Equal(1L, milk.Owner);
        Assert.Null(none.Due);
    }
}
