// Undo of typed inputs in a real window (DESIGN 5.29), as a person types,
// ported from the Avalonia InputUndoTests: typing taken when a text box is
// left is the window's edit, so Ctrl+Z there walks back the form, while
// typing not yet taken is the box's own to take back first; an input that
// wraps its text box (a color, a typed dropdown) records what was typed when
// it is left; a slider moved with a key and a setting changed by a part are
// taken back too. Each part carries the undoScope the generator writes.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;

namespace Clarity.Ui.Tests.Parts;

/// <summary>Settings as a generated app's are: shared, read and given back whole.</summary>
public sealed class KeptForUndo : ObservableObject, ISettings
{
    public bool ShowToolbar { get => showToolbar; set => SetProperty(ref showToolbar, value); }
    bool showToolbar = true;

    public JsonObject ToJson() => new() { ["showToolbar"] = ShowToolbar };

    public void Apply(JsonObject sent) => ShowToolbar = sent["showToolbar"]?.GetValue<bool>() ?? ShowToolbar;
}

public sealed class Client : ObservableObject
{
    public string Name { get => name; set => SetProperty(ref name, value); }
    string name = "Ada";

    public string City { get => city; set => SetProperty(ref city, value); }
    string city = "Rome";

    public string Tint { get => tint; set => SetProperty(ref tint, value); }
    string tint = "#3b82f6";

    public decimal Rating { get => rating; set => SetProperty(ref rating, value); }
    decimal rating;
}

/// <summary>A window's view model as the generator writes one: every change told, so each part that shows it follows.</summary>
public sealed class TypedViewModel : WindowViewModel
{
    public TypedViewModel() : base(null) { }

    public override string Title => "Typed";

    public Client Client { get => client; set => SetProperty(ref client, value); }
    Client client = new();

    // A typed dropdown's values of its field, as the generator writes them.
    public List<string> ClientCityValues { get; } = ["Lima", "Oslo", "Rome"];

    public KeptForUndo Settings { get; } = new();

    protected override ISettings? AppSettings => Settings;
}

public class InputUndoTests
{
    static (Screen Screen, WindowStack Stack, TypedViewModel Model) Shown(string parts)
    {
        var markup = $"""<Window name="Typed"><Button name="elsewhere" at="1,1" span="2" caption="Elsewhere" />{parts}</Window>""";
        var stack = new WindowStack(_ => markup);
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Keys = stack.KeyPressed;
        var model = new TypedViewModel();
        stack.Push(model);
        Shot.Of(screen, 800, 400);
        return (screen, stack, model);
    }

    static T Part<T>(WindowStack stack, string name) where T : Part => (T)stack.View!.Parts.First(p => p.Spec.Name == name);

    static void Press(Screen screen, Key key, Modifiers modifiers = Modifiers.None) => screen.KeyPressed(new KeyInput(key, modifiers));

    static void TypeAndLeave(Screen screen, WindowStack stack, TextEditor box, string text)
    {
        screen.Focus(box);
        box.SelectAll();
        screen.TextTyped(text);
        screen.Focus(Part<ButtonPart>(stack, "elsewhere"));
        stack.Tick();
    }

    static void Undo(Screen screen, WindowStack stack, Element focused)
    {
        screen.Focus(focused);
        stack.Tick();
        Press(screen, Key.Z, Modifiers.Control);
    }

    [Fact]
    public void Typing_taken_when_a_box_is_left_is_the_forms_to_take_back()
    {
        var (screen, stack, model) = Shown("""
            <Textbox name="nameBox" at="1,2" span="6" bind="client.name" undoScope="customerForm" />
            <Textbox name="cityBox" at="1,3" span="6" bind="client.city" undoScope="customerForm" />
            """);
        var name = Part<TextBoxPart>(stack, "nameBox").Editor;
        TypeAndLeave(screen, stack, name, "Ann");
        TypeAndLeave(screen, stack, Part<TextBoxPart>(stack, "cityBox").Editor, "Oslo");
        Undo(screen, stack, name);
        Assert.Equal(("Ann", "Rome"), (model.Client.Name, model.Client.City));
        Press(screen, Key.Z, Modifiers.Control);
        Assert.Equal("Ada", model.Client.Name);
    }

    /// Ctrl+Z while typing takes back the box's own typing, a word at a time, before any edit of the form; Ctrl+Y does it again.
    [Fact]
    public void Typing_not_yet_taken_is_the_boxs_own_to_take_back_first()
    {
        var (screen, stack, model) = Shown("""
            <Textbox name="nameBox" at="1,2" span="6" bind="client.name" undoScope="customerForm" />
            <Textbox name="cityBox" at="1,3" span="6" bind="client.city" undoScope="customerForm" />
            """);
        TypeAndLeave(screen, stack, Part<TextBoxPart>(stack, "cityBox").Editor, "Oslo");
        var name = Part<TextBoxPart>(stack, "nameBox").Editor;
        screen.Focus(name);
        stack.Tick();
        screen.TextTyped(" Lovelace");
        Assert.Equal("Ada Lovelace", name.Text);
        Press(screen, Key.Z, Modifiers.Control);
        Assert.Equal("Ada", name.Text);
        Assert.Equal("Oslo", model.Client.City);
        Press(screen, Key.Y, Modifiers.Control);
        Assert.Equal("Ada Lovelace", name.Text);
        Press(screen, Key.Z, Modifiers.Control);
        // No typing of its own left: the next one is the form's.
        Press(screen, Key.Z, Modifiers.Control);
        Assert.Equal("Rome", model.Client.City);
        Assert.Equal("Ada", model.Client.Name);
    }

    /// `no undo`: nothing, not even the typing.
    [Fact]
    public void A_box_with_no_undo_takes_back_not_even_its_typing()
    {
        var (screen, stack, _) = Shown("""<Textbox name="nameBox" at="1,2" span="6" bind="client.name" noUndo="true" />""");
        var name = Part<TextBoxPart>(stack, "nameBox").Editor;
        screen.Focus(name);
        screen.TextTyped("x");
        Press(screen, Key.Z, Modifiers.Control);
        Assert.Equal("Adax", name.Text);
    }

    /// Two parts show the same field: what the person typed in one is that part's edit, not the other's, which only follows.
    [Fact]
    public void A_part_that_shows_the_same_data_does_not_take_the_edit()
    {
        var (screen, stack, model) = Shown("""
            <Textbox name="nameBox" at="1,2" span="6" bind="client.name" undoScope="customerForm" />
            <Textbox name="nameShown" at="7,2" span="6" bind="client.name" />
            <Textbox name="cityBox" at="1,3" span="6" bind="client.city" undoScope="customerForm" />
            """);
        var name = Part<TextBoxPart>(stack, "nameBox").Editor;
        TypeAndLeave(screen, stack, name, "Ann");
        TypeAndLeave(screen, stack, Part<TextBoxPart>(stack, "cityBox").Editor, "Oslo");
        Shot.Of(screen, 800, 400);
        Assert.Equal("Ann", Part<TextBoxPart>(stack, "nameShown").Editor.Text);
        Undo(screen, stack, name);
        Assert.Equal(("Ann", "Rome"), (model.Client.Name, model.Client.City));
        Press(screen, Key.Z, Modifiers.Control);
        Assert.Equal("Ada", model.Client.Name);
        Assert.False(model.Edits.CanUndo("nameShown"));
    }

    /// The Avalonia risk was a text box inside a user control; here every input's editor is inside its part, which the window finds by walking up.
    [Fact]
    public void An_input_that_wraps_its_text_box_records_what_was_typed_when_it_is_left()
    {
        var (screen, stack, model) = Shown("""<Number name="ratingBox" undoScope="ratingBox" at="1,2" span="6" bind="client.rating" />""");
        var box = Part<NumberPart>(stack, "ratingBox").Editor;
        TypeAndLeave(screen, stack, box, "4");
        Assert.Equal(4m, model.Client.Rating);
        Assert.True(model.Edits.CanUndo("ratingBox"));
        Undo(screen, stack, box);
        Assert.Equal(0m, model.Client.Rating);
    }

    [Fact]
    public void A_color_typed_and_left_is_taken_back()
    {
        var (screen, stack, model) = Shown("""<Color name="tintBox" undoScope="tintBox" at="1,2" span="6" bind="client.tint" />""");
        var box = Part<ColorPart>(stack, "tintBox").Editor;
        TypeAndLeave(screen, stack, box, "#ff0000");
        Assert.Equal("#ff0000", model.Client.Tint);
        Undo(screen, stack, box);
        Assert.Equal("#3b82f6", model.Client.Tint);
    }

    [Fact]
    public void A_typed_dropdown_typed_and_left_is_taken_back()
    {
        var (screen, stack, model) = Shown("""<Dropdown name="cityBox" undoScope="cityBox" at="1,2" span="6" bind="client.city" typed="true" />""");
        var typed = (TypedDropdownPart)stack.View!.Parts.First(p => p.Spec.Name == "cityBox");
        TypeAndLeave(screen, stack, typed.Editor, "Oslo");
        Assert.Equal("Oslo", model.Client.City);
        Undo(screen, stack, typed.Editor);
        Assert.Equal("Rome", model.Client.City);
    }

    [Fact]
    public void A_slider_moved_with_a_key_is_taken_back()
    {
        var (screen, stack, model) = Shown("""<Slider name="ratingSlider" undoScope="ratingSlider" at="1,2" span="6" bind="client.rating" min="0" max="5" step="0.5" />""");
        var slider = Part<SliderPart>(stack, "ratingSlider");
        screen.Focus(slider);
        stack.Tick();
        Press(screen, Key.Right);
        Assert.Equal(0.5m, model.Client.Rating);
        Press(screen, Key.Z, Modifiers.Control);
        Assert.Equal(0m, model.Client.Rating);
    }

    [Fact]
    public void A_setting_a_part_changed_is_taken_back()
    {
        var (screen, stack, model) = Shown("""<Checkbox name="toolbarSwitch" undoScope="toolbarSwitch" at="1,2" span="6" bind="settings.showToolbar" caption="Toolbar" />""");
        var check = Part<CheckPart>(stack, "toolbarSwitch");
        screen.Focus(check);
        stack.Tick();
        Press(screen, Key.Space);
        Assert.False(model.Settings.ShowToolbar);
        Press(screen, Key.Z, Modifiers.Control);
        Assert.True(model.Settings.ShowToolbar);
    }
}
