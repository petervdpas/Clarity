// Undo of typed inputs in a real window (DESIGN 5.29), as a person types:
// typing taken when a text box is left is the window's edit, so Ctrl+Z
// there walks back the form, not the box's own typing; an input that wraps
// its text box (a color, a typed dropdown, a date) records what was typed
// when it is left; and a setting changed by a part is taken back too.

using System.Text.Json.Nodes;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// <summary>Settings as a generated app's are: shared, read and given back whole.</summary>
sealed class KeptForTest : ISettings
{
    public bool ShowToolbar { get; set; } = true;
    public JsonObject ToJson() => new() { ["showToolbar"] = ShowToolbar };
    public void Apply(JsonObject sent) => ShowToolbar = sent["showToolbar"]?.GetValue<bool>() ?? ShowToolbar;
}

sealed class TypedViewModel(Db db) : WindowViewModel(db)
{
    // As generated: every change is told, so each part that shows it follows.
    public string Name { get => name; set => SetProperty(ref name, value); }
    string name = "Ada";
    public string City { get => city; set => SetProperty(ref city, value); }
    string city = "Rome";
    public string Tint { get => tint; set => SetProperty(ref tint, value); }
    string tint = "#3b82f6";
    public object? Rating { get => rating; set => SetProperty(ref rating, value); }
    object? rating = 0m;
    public List<string> Cities { get; } = ["Lima", "Oslo", "Rome"];
    public KeptForTest Kept { get; } = new();
    protected override ISettings? AppSettings => Kept;
}

public class InputUndoTests
{
    static (Window, TypedViewModel) Shown(params Control[] parts)
    {
        var db = Sqlite.Memory();
        var model = new TypedViewModel(db);
        var window = new Window { Width = 400, Height = 300, DataContext = model, Content = new StackPanel { Children = { new Button { Content = "Elsewhere" } } } };
        foreach (var part in parts)
            ((StackPanel)window.Content).Children.Add(part);
        Edits.Listen(window, () => model);
        window.Show();
        model.Opened();
        Dispatcher.UIThread.RunJobs();
        return (window, model);
    }

    static TextBox Box(string name, string path, string scope)
    {
        var box = new TextBox { Name = name, [!TextBox.TextProperty] = new Binding(path) { Mode = BindingMode.TwoWay } };
        Edits.SetScope(box, scope);
        return box;
    }

    static void Click(Window window, Control control)
    {
        var at = control.TranslatePoint(new Point(5, control.Bounds.Height / 2), window)!.Value;
        window.MouseDown(at, MouseButton.Left);
        window.MouseUp(at, MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
    }

    static void Key(Window window, PhysicalKey key, RawInputModifiers modifiers = RawInputModifiers.None)
    {
        window.KeyPressQwerty(key, modifiers);
        window.KeyReleaseQwerty(key, modifiers);
        Dispatcher.UIThread.RunJobs();
    }

    static void TypeAndLeave(Window window, Control box, string text)
    {
        Click(window, box);
        Key(window, PhysicalKey.A, RawInputModifiers.Control);
        window.KeyTextInput(text);
        Dispatcher.UIThread.RunJobs();
        Click(window, ((StackPanel)window.Content!).Children[0]);
    }

    [AvaloniaFact]
    public void Typing_taken_when_a_box_is_left_is_the_forms_to_take_back()
    {
        var name = Box("nameBox", "Name", "customerForm");
        var city = Box("cityBox", "City", "customerForm");
        var (window, model) = Shown(name, city);
        TypeAndLeave(window, name, "Ann");
        TypeAndLeave(window, city, "Oslo");
        Click(window, name);
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal(("Ann", "Rome"), (model.Name, model.City));
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal("Ada", model.Name);
        window.Close();
    }

    /// Two parts show the same field: what the person typed in one is that part's edit, not the other's, which only follows.
    [AvaloniaFact]
    public void A_part_that_shows_the_same_data_does_not_take_the_edit()
    {
        var name = Box("nameBox", "Name", "customerForm");
        var shown = Box("nameShown", "Name", "nameShown");
        var city = Box("cityBox", "City", "customerForm");
        var (window, model) = Shown(name, shown, city);
        TypeAndLeave(window, name, "Ann");
        TypeAndLeave(window, city, "Oslo");
        Assert.Equal("Ann", shown.Text);
        Click(window, name);
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal(("Ann", "Rome"), (model.Name, model.City));
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal("Ada", model.Name);
        Assert.False(model.Edits.CanUndo("nameShown"));
        window.Close();
    }

    [AvaloniaFact]
    public void An_input_that_wraps_its_text_box_records_what_was_typed_when_it_is_left()
    {
        var wrapped = new UserControl { Name = "cityBox", Content = new TextBox { [!TextBox.TextProperty] = new Binding("City") { Mode = BindingMode.TwoWay } } };
        Edits.SetScope(wrapped, "cityBox");
        var (window, model) = Shown(wrapped);
        TypeAndLeave(window, (Control)wrapped.Content!, "Oslo");
        Assert.Equal("Oslo", model.City);
        Click(window, (Control)wrapped.Content!);
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal("Rome", model.City);
        window.Close();
    }

    [AvaloniaFact]
    public void A_color_typed_and_left_is_taken_back()
    {
        var color = new ColorInput { Name = "tintBox", [!ColorInput.ValueProperty] = new Binding("Tint") { Mode = BindingMode.TwoWay } };
        Edits.SetScope(color, "tintBox");
        var (window, model) = Shown(color);
        TypeAndLeave(window, color.Box, "#ff0000");
        Assert.Equal("#ff0000", model.Tint);
        Click(window, color.Box);
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal("#3b82f6", model.Tint);
        window.Close();
    }

    [AvaloniaFact]
    public void A_typed_dropdown_typed_and_left_is_taken_back()
    {
        var typed = new TypedInput { Name = "cityBox", [!TypedInput.ValuesProperty] = new Binding("Cities"), [!TypedInput.ValueProperty] = new Binding("City") { Mode = BindingMode.TwoWay } };
        Edits.SetScope(typed, "cityBox");
        var (window, model) = Shown(typed);
        TypeAndLeave(window, typed.Box, "Oslo");
        Assert.Equal("Oslo", model.City);
        Click(window, typed.Box);
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal("Rome", model.City);
        window.Close();
    }

    [AvaloniaFact]
    public void A_slider_moved_with_a_key_is_taken_back()
    {
        var slider = new SliderInput { Name = "ratingSlider", Min = "0", Max = "5", Step = "0.5", Whole = false, [!SliderInput.ValueProperty] = new Binding("Rating") { Mode = BindingMode.TwoWay } };
        Edits.SetScope(slider, "ratingSlider");
        var (window, model) = Shown(slider);
        slider.Bar.Focus();
        Key(window, PhysicalKey.ArrowRight);
        Assert.Equal(0.5m, Convert.ToDecimal(model.Rating));
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.Equal(0m, Convert.ToDecimal(model.Rating));
        window.Close();
    }

    [AvaloniaFact]
    public void A_setting_a_part_changed_is_taken_back()
    {
        var check = new CheckBox { Name = "toolbarSwitch", [!CheckBox.IsCheckedProperty] = new Binding("Kept.ShowToolbar") { Mode = BindingMode.TwoWay } };
        Edits.SetScope(check, "toolbarSwitch");
        var (window, model) = Shown(check);
        Click(window, check);
        Assert.False(model.Kept.ShowToolbar);
        Key(window, PhysicalKey.Z, RawInputModifiers.Control);
        Assert.True(model.Kept.ShowToolbar);
        window.Close();
    }
}
