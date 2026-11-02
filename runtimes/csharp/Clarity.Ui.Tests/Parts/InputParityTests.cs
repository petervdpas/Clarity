// The inputs as the Avalonia version had them, where the audit found Clarity.UI
// apart (audit-inputs): a text input's right-click menu, its length limit and
// its pointer; the password's stars; the checkbox and switch by Enter and by
// dragging; the radio group's Tab stops; the slider that snaps while dragged;
// numbers read from the right and a spin box that repeats while held; the
// typed dropdown's list that goes round and an Enter that stays there; the
// color part's palette by keyboard, its tooltips and its Enter that submits.

using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public class InputParityTests
{
    static void Press(Screen screen, Key key, Modifiers modifiers = Modifiers.None) => screen.KeyPressed(new KeyInput(key, modifiers));

    static void Click(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    static (double X, double Y) Middle(Element e) => (e.Bounds.X + e.Bounds.Width / 2, e.Bounds.Y + e.Bounds.Height / 2);

    // -- Text inputs ----------------------------------------------------------

    [Fact]
    public void A_right_press_on_a_text_box_offers_cut_copy_paste_and_select_all()
    {
        var host = new TestHost();
        host.Values["task.title"] = "Write";
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="6" name="title" bind="task.title" />""", host);
        Shot.Of(screen, 800, 400);
        var editor = ((TextBoxPart)parts[0]).Editor;
        var (x, y) = Middle(editor);
        screen.PointerPressed(x, y, PointerButton.Right);
        Assert.True(screen.ContextMenus.IsOpen);
        var entries = editor.ContextMenuAt(x, y)!;
        Assert.Equal([Locale.Translate("cutText", "Cut"), Locale.Translate("copyText", "Copy"), Locale.Translate("pasteText", "Paste"), Locale.Translate("selectAllText", "Select all")], entries.Select(e => e.Caption));
        // Nothing selected: Cut and Copy wait.
        Assert.Equal([false, false, true, true], entries.Select(e => e.Enabled));
        screen.ContextMenus.Picked(entries[3]);
        Assert.Equal("Write", editor.SelectedText);
        entries = editor.ContextMenuAt(x, y)!;
        screen.ContextMenus.Picked(entries[0]);
        Assert.Equal("", editor.Text);
        Assert.Equal("Write", screen.Clipboard.GetText());
        screen.ContextMenus.Picked(editor.ContextMenuAt(x, y)![2]);
        screen.ContextMenus.Picked(editor.ContextMenuAt(x, y)![2]);
        Assert.Equal("WriteWrite", editor.Text);
        // A read-only box copies but neither cuts nor pastes.
        editor.ReadOnly = () => true;
        editor.SelectAll();
        Assert.Equal([false, true, false, true], editor.ContextMenuAt(x, y)!.Select(e => e.Enabled));
    }

    [Fact]
    public void A_text_input_shows_the_text_cursor_and_a_password_shows_stars()
    {
        var host = new TestHost();
        host.Values["user.secret"] = "abc";
        var (screen, _, parts) = Made.Window("""<Password at="1,1" span="6" name="secret" bind="user.secret" />""", host);
        Shot.Of(screen, 800, 400);
        var box = (TextBoxPart)parts[0];
        Assert.Equal("***", box.Editor.Shown);
        var (x, y) = Middle(box.Editor);
        Assert.Equal(Cursor.Text, box.Editor.CursorAt(x, y));
        // Its buttons take no focus.
        Assert.False(box.Eye!.Focusable);
        Assert.False(box.Copy!.Focusable);
        Assert.DoesNotContain(box.Eye, screen.TabOrder());
    }

    [Fact]
    public void A_text_editor_takes_at_most_its_length_of_typing()
    {
        var editor = new TextEditor { MaxLength = 7 };
        var screen = new Screen(editor, ThemeLook.Clarity(dark: false));
        screen.Resize(200, 40, 1);
        screen.Focus(editor);
        screen.TextTyped("#ff00aa11");
        Assert.Equal("#ff00aa", editor.Text);
        screen.TextTyped("x");
        Assert.Equal("#ff00aa", editor.Text);
        // Text set from elsewhere is not cut.
        editor.Text = "#1234567";
        Assert.Equal("#1234567", editor.Text);
    }

    // -- Checkbox and switch ------------------------------------------------------

    [Fact]
    public void Enter_ticks_the_focused_checkbox_and_a_read_only_one_is_no_tab_stop()
    {
        var host = new TestHost();
        host.Values["task.done"] = false;
        var (screen, _, parts) = Made.Window("""
            <Checkbox at="1,1" span="6" name="done" bind="task.done" caption="Done" />
            <Checkbox at="1,2" span="6" name="kept" bind="task.done" caption="Kept" readonly="true" />
            """, host);
        Shot.Of(screen, 800, 400);
        Press(screen, Key.Tab);
        Assert.Same(parts[0], screen.Focused);
        Press(screen, Key.Enter);
        Assert.Equal(true, host.Values["task.done"]);
        Assert.False(parts[1].Focusable);
        Assert.DoesNotContain(parts[1], screen.TabOrder());
    }

    [Fact]
    public void A_switchs_knob_dragged_to_the_other_side_flips_it_once()
    {
        var host = new TestHost();
        host.Values["task.done"] = false;
        var (screen, _, parts) = Made.Window("""<Switch at="1,1" span="6" name="done" bind="task.done" caption="Done" onchange="flipped" />""", host);
        Shot.Of(screen, 800, 400);
        var part = parts[0];
        var y = part.Bounds.Y + part.Bounds.Height / 2;
        var x = part.Bounds.X + 8;
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerMoved(x + 10, y);
        screen.PointerMoved(x + 22, y);
        screen.PointerReleased(x + 22, y, PointerButton.Left);
        Assert.Equal(true, host.Values["task.done"]);
        Assert.Single(host.Log, l => l.StartsWith("run done change"));
        // Dragged only a little, back on its own side: no flip.
        x = part.Bounds.X + CheckPart.TrackWidth - 8;
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerMoved(x - 6, y);
        screen.PointerReleased(x - 6, y, PointerButton.Left);
        Assert.Equal(true, host.Values["task.done"]);
        // A click still flips it.
        Click(screen, x, y);
        Assert.Equal(false, host.Values["task.done"]);
    }

    // -- Radio group ---------------------------------------------------------------

    [Fact]
    public void Every_choice_is_a_tab_stop_and_Enter_or_Space_picks_it()
    {
        var host = new TestHost();
        host.Values["task.size"] = "Small";
        host.Offered["task.size"] = [new("Small", "Small"), new("Medium", "Medium"), new("Large", "Large")];
        var (screen, _, parts) = Made.Window("""
            <Radio at="1,1" span="12" name="size" bind="task.size" />
            <Button at="1,2" span="2" name="after" caption="After" />
            """, host);
        Shot.Of(screen, 800, 400);
        var radio = (RadioPart)parts[0];
        Press(screen, Key.Tab);
        Assert.Same(radio.Buttons[0], screen.Focused);
        Press(screen, Key.Tab);
        Assert.Same(radio.Buttons[1], screen.Focused);
        // Tab only moves the focus.
        Assert.Equal("Small", host.Values["task.size"]);
        Press(screen, Key.Enter);
        Assert.Equal("Medium", host.Values["task.size"]);
        Press(screen, Key.Tab);
        Press(screen, Key.Space);
        Assert.Equal("Large", host.Values["task.size"]);
        Press(screen, Key.Tab);
        Assert.Same(parts[1], screen.Focused);
        Press(screen, Key.Tab, Modifiers.Shift);
        Assert.Same(radio.Buttons[2], screen.Focused);
        // Side by side the choices stand 12 apart, as Avalonia's row did.
        var places = radio.Places(screen.Fonts);
        Assert.Equal(12, places[1].X - places[0].Right, 3);
    }

    // -- Slider -------------------------------------------------------------------

    [Fact]
    public void A_dragged_thumb_snaps_to_the_steps_and_no_value_shows_at_the_lowest()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Slider at="1,1" span="12" name="rating" bind="task.rating" min="2" max="10" step="2" />""", host);
        Shot.Of(screen, 800, 400);
        var slider = (SliderPart)parts[0];
        Assert.Equal(2m, slider.Shown);
        var track = slider.Bounds.Width - SliderPart.Thumb;
        var y = slider.Bounds.Y + slider.Bounds.Height / 2;
        screen.PointerPressed(slider.Bounds.X + SliderPart.Thumb / 2 + track * 0.3, y, PointerButton.Left);
        Assert.Equal(4m, slider.Shown);
        screen.PointerMoved(slider.Bounds.X + SliderPart.Thumb / 2 + track * 0.6, y);
        Assert.Equal(6m, slider.Shown);
        Assert.Null(host.Values.GetValueOrDefault("task.rating"));
        screen.PointerReleased(slider.Bounds.X + SliderPart.Thumb / 2 + track * 0.6, y, PointerButton.Left);
        Assert.Equal(6L, host.Values["task.rating"]);
    }

    // -- Number and spin box --------------------------------------------------------

    [Fact]
    public void Numbers_read_from_the_right_and_a_spin_boxs_arrows_take_no_focus()
    {
        var host = new TestHost();
        host.Values["line.quantity"] = 3L;
        var (screen, _, parts) = Made.Window("""
            <Number at="1,1" span="6" name="amount" bind="line.amount" />
            <Spinbox at="1,2" span="6" name="quantity" bind="line.quantity" min="0" max="10" />
            """, host);
        Shot.Of(screen, 800, 400);
        Assert.Equal(TextAlign.Right, ((NumberPart)parts[0]).Editor.Align);
        var spin = (NumberPart)parts[1];
        Assert.Equal(TextAlign.Right, spin.Editor.Align);
        Assert.False(spin.Up!.Focusable);
        Assert.DoesNotContain(spin.Up, screen.TabOrder());
    }

    [Fact]
    public void A_spin_boxs_arrow_held_keeps_spinning_and_a_spin_clears_the_mark()
    {
        var host = new TestHost();
        host.Values["line.quantity"] = "3";
        var (screen, _, parts) = Made.Window("""<Spinbox at="1,1" span="6" name="quantity" min="0" max="100" />""", host);
        Shot.Of(screen, 800, 400);
        var spin = (NumberPart)parts[0];
        TextWindow.Type(screen, spin.Editor, "x");
        screen.Focus(null);
        Assert.NotNull(spin.Bad);
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        var (x, y) = Middle(spin.Up!);
        screen.PointerPressed(x, y, PointerButton.Left);
        Assert.Null(spin.Bad);
        var first = decimal.Parse(spin.Editor.Text, System.Globalization.CultureInfo.InvariantCulture);
        decimal Now() => decimal.Parse(spin.Editor.Text, System.Globalization.CultureInfo.InvariantCulture);
        // Held: after a pause it repeats, and goes on.
        now += InputSideButton.RepeatDelay;
        screen.WakeDue(now);
        Assert.Equal(first + 1, Now());
        now += InputSideButton.RepeatInterval;
        screen.WakeDue(now);
        now += InputSideButton.RepeatInterval;
        screen.WakeDue(now);
        Assert.Equal(first + 3, Now());
        screen.PointerReleased(x, y, PointerButton.Left);
        now += InputSideButton.RepeatInterval;
        screen.WakeDue(now);
        Assert.Equal(first + 3, Now());
    }

    // -- Typed and search dropdowns -----------------------------------------------------

    static (Screen Screen, TypedDropdownPart Part, TestHost Host, TextTestForm Form) Typed(string? value = "Gent", string bind = "bind=\"person.city\"")
    {
        var host = new TestHost();
        host.Values["person.city"] = value;
        host.Offered["person.city"] = [new("Utrecht", "Utrecht"), new("Gent", "Gent"), new("Graz", "Graz")];
        var spec = WindowMarkup.Read($"""<Window name="Test"><Dropdown at="1,1" span="4" name="city" {bind} typed="true" /></Window>""");
        var form = new TextTestForm();
        var part = PartVariants.Make(spec.Parts[0], host);
        form.Grid.Add(part, spec.Parts[0].Place(part.Grows));
        var screen = new Screen(form, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 400, 1);
        Shot.Of(screen, 800, 400);
        return (screen, Assert.IsType<TypedDropdownPart>(part), host, form);
    }

    [Fact]
    public void Up_and_Down_go_round_the_typed_list_and_Enter_with_no_row_does_not_submit()
    {
        var (screen, part, host, form) = Typed();
        var submitted = 0;
        form.Submitted = () => submitted++;
        // Emptied, the box offers every value.
        TextWindow.Type(screen, part.Editor, "");
        var list = (DropdownList)screen.Overlays[^1].Content;
        Assert.Equal(3, list.Count);
        Press(screen, Key.Up);
        Assert.Equal(2, list.Highlighted);
        Press(screen, Key.Down);
        Assert.Equal(0, list.Highlighted);
        Press(screen, Key.Up);
        Assert.Equal(2, list.Highlighted);
        Press(screen, Key.Escape);
        part.Editor.SelectAll();
        screen.TextTyped("Rome");
        Assert.Empty(screen.Overlays);
        Press(screen, Key.Enter);
        Assert.Equal("Rome", host.Values["person.city"]);
        Assert.Equal(0, submitted);
        // The list is at most 240 high, and scrolls.
        Assert.True(DropdownList.Below(screen, part.Bounds, 100, 50).Height <= 240);
    }

    [Fact]
    public void The_typed_list_closes_when_nothing_fits_and_leaving_over_it_takes_nothing()
    {
        var (screen, part, host, _) = Typed();
        screen.Focus(part.Editor);
        part.Editor.SelectAll();
        screen.TextTyped("G");
        var list = screen.Overlays[^1];
        // The pointer over the list: the focus going for a press there is no leaving.
        screen.PointerMoved(list.Place.X + 5, list.Place.Y + 5);
        screen.Focus(null);
        Assert.Equal("Gent", host.Values["person.city"]);
        screen.Focus(part.Editor);
        part.Editor.SelectAll();
        screen.TextTyped("zz");
        Assert.Null(part.List);
    }

    [Fact]
    public void An_unbound_typed_dropdown_keeps_what_was_typed()
    {
        var (screen, part, _, _) = Typed(bind: "");
        screen.Focus(part.Editor);
        screen.TextTyped("Lima");
        screen.Focus(null);
        Assert.Equal("Lima", part.Value);
        Assert.Equal("Lima", part.Editor.Text);
    }

    // -- Color ---------------------------------------------------------------------------

    static (Screen Screen, ColorPart Part, TestHost Host, TextTestForm Form) Colored(string markup = """<Color at="1,1" span="6" name="tint" bind="task.tint" />""")
    {
        var host = new TestHost();
        host.Values["task.tint"] = "#3b82f6";
        var (screen, form, parts) = TextWindow.InForm(markup, host);
        Shot.Of(screen, 800, 400);
        return (screen, (ColorPart)parts[0], host, form);
    }

    [Fact]
    public void The_palette_works_by_keyboard()
    {
        var (screen, part, host, _) = Colored();
        screen.Focus(part.Swatch);
        Press(screen, Key.Enter);
        var grid = part.Grid!;
        var chosen = grid.Colors.ToList().IndexOf("#3b82f6");
        Assert.Equal(Math.Max(0, chosen), grid.Marked);
        Press(screen, Key.Home);
        Assert.Equal(0, grid.Marked);
        Press(screen, Key.Right);
        Press(screen, Key.Down);
        Assert.Equal(11, grid.Marked);
        Press(screen, Key.Tab);
        Assert.Equal(12, grid.Marked);
        Assert.Same(part.Swatch, screen.Focused);
        Press(screen, Key.Tab, Modifiers.Shift);
        Press(screen, Key.Space);
        Assert.Equal(grid.Colors[11], host.Values["task.tint"]);
        Assert.Null(part.Palette);
        // Opened again, Escape closes it and picks nothing.
        Press(screen, Key.Down);
        Press(screen, Key.Escape);
        Assert.Null(part.Palette);
        Assert.Equal(grid.Colors[11], host.Values["task.tint"]);
    }

    [Fact]
    public void Each_palette_color_says_its_code_and_the_box_takes_seven_characters()
    {
        var (screen, part, _, _) = Colored();
        part.Toggle();
        Shot.Of(screen, 800, 400);
        var grid = part.Grid!;
        var place = grid.Place(grid.Colors[5]);
        Assert.Equal(grid.Colors[5], grid.TooltipAt(place.X + 3, place.Y + 3));
        Assert.Null(grid.TooltipAt(grid.Bounds.X + 1, grid.Bounds.Y + 1));
        part.Close();
        Assert.Equal(7, part.Editor.MaxLength);
        Assert.Equal(ColorPart.SwatchWidth, part.Swatch.Bounds.Width);
    }

    [Fact]
    public void Enter_in_the_color_box_takes_the_color_and_submits_the_form()
    {
        var (screen, part, host, form) = Colored();
        var submitted = 0;
        form.Submitted = () => submitted++;
        TextWindow.Type(screen, part.Editor, "#ff0000");
        Press(screen, Key.Enter);
        Assert.Equal("#ff0000", host.Values["task.tint"]);
        Assert.Equal(1, submitted);
    }
}
