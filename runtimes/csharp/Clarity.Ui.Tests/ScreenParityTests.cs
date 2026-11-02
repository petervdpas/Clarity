// What the screen does for every part, as Avalonia did for every control:
// a disabled container disables what is in it, a right press opens a part's
// menu, a part's tooltip and style show, taborder sorts Tab, the part with
// the focus hears a key before the window's shortcuts, a symbol shortcut
// answers the character typed, a part that fails stays in its place, and the
// pointer takes the shape of what is under it.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class ScreenParityTests
{
    static (Screen Screen, WindowView View, TestHost Host) Shown(string parts, double width = 800, double height = 400)
    {
        var host = new TestHost();
        var spec = WindowMarkup.Read($"<Window name=\"Parity\">{parts}</Window>");
        var view = new WindowView(spec, host);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false)) { Keys = view.KeyPressed, KeysUp = view.KeyReleased };
        screen.Resize(width, height, 1);
        return (screen, view, host);
    }

    static void Click(Screen screen, Element at, PointerButton button = PointerButton.Left)
    {
        var (x, y) = (at.Bounds.X + at.Bounds.Width / 2, at.Bounds.Y + at.Bounds.Height / 2);
        screen.PointerPressed(x, y, button);
        screen.PointerReleased(x, y, button);
    }

    static T Part<T>(WindowView view, string name) where T : Part => view.Find<T>(p => p.Spec.Name == name)!;

    [Fact]
    public void A_disabled_panel_disables_its_parts_for_the_pointer_tab_and_keys()
    {
        var (screen, view, host) = Shown("""
            <Panel name="box" at="1,1" span="6" rows="3" caption="Box">
              <Button name="inner" at="1,1" span="3" caption="Inner" key="F5" onclick="inner" />
            </Panel>
            <Button name="outer" at="7,1" span="3" caption="Outer" onclick="outer" />
            """);
        host.State("box")!.IsEnabled = false;
        var inner = Part<ButtonPart>(view, "inner");
        Assert.False(inner.Enabled);
        Click(screen, inner);
        screen.KeyPressed(new KeyInput(Key.F5, Modifiers.None));
        Assert.Empty(host.Log);
        Assert.Null(screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(Part<ButtonPart>(view, "outer"), screen.Focused);
        host.State("box")!.IsEnabled = true;
        Click(screen, inner);
        Assert.Equal(["run inner click inner"], host.Log);
    }

    [Fact]
    public void A_right_click_opens_a_parts_menu_and_its_item_runs()
    {
        var (screen, view, host) = Shown("""
            <Label name="note" at="1,1" span="6" caption="Right click me">
              <Menu name="noteMenu">
                <Item name="copyItem" caption="Copy" onclick="copied" />
                <Item name="goneItem" caption="Gone" onclick="gone" />
              </Menu>
            </Label>
            """);
        host.State("goneItem")!.IsVisible = false;
        var label = view.Parts[0];
        Click(screen, label, PointerButton.Right);
        var list = Assert.Single(screen.ContextMenus.Lists);
        Assert.Equal(["Copy"], list.Entries.Select(e => e.Caption));
        list.Activate(0);
        Assert.False(screen.ContextMenus.IsOpen);
        Assert.Equal(["run note click copied"], host.Log);
        // A left click opens no menu.
        Click(screen, label);
        Assert.False(screen.ContextMenus.IsOpen);
    }

    [Fact]
    public void Every_part_shows_its_tooltip_when_the_pointer_rests_on_it()
    {
        var (screen, view, host) = Shown("""
            <Label name="note" at="1,1" span="6" caption="Note" tooltip="@noteTip" />
            <Textbox name="box" at="1,2" span="6" tooltip="Type here" />
            """);
        host.Texts["noteTip"] = "What the note says";
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        foreach (var (part, tip) in new[] { (view.Parts[0], "What the note says"), (view.Parts[1], "Type here") })
        {
            screen.PointerMoved(part.Bounds.X + 5, part.Bounds.Y + 5);
            now += Screen.TipPause;
            screen.WakeDue(now);
            Assert.Equal(tip, screen.Tip?.Text);
        }
    }

    [Fact]
    public void A_parts_style_fills_it_and_colors_its_text()
    {
        var (screen, view, _) = Shown("""<Label name="loud" at="1,1" span="6" caption="Loud" style="loud" />""");
        screen.Show(ThemeLook.Over("""{"styles":{"loud":{"background":"#ff0000","text":"#00ff00"}}}""", ThemeLook.Clarity(dark: false)));
        using var shot = Shot.Of(screen, 800, 400);
        var label = view.Parts[0].Bounds;
        Assert.Equal(new SKColor(255, 0, 0), shot.PixelAt((int)label.Right - 4, (int)(label.Y + label.Height / 2)));
        var green = false;
        for (var x = (int)label.X; x < label.Right && !green; x++)
            for (var y = (int)label.Y; y < label.Bottom && !green; y++)
                green = shot.PixelAt(x, y) is { Green: > 200, Red: < 60 };
        Assert.True(green, "the style's text color draws the caption");
    }

    /// <summary>A style's weight and align reach a part that does not read the style itself (a label), as an Avalonia class did.</summary>
    [Fact]
    public void A_parts_style_weight_and_align_reach_its_text()
    {
        (int Left, int Count) Ink(string style)
        {
            var (screen, view, _) = Shown("""<Label name="loud" at="1,1" span="6" caption="Loud words" style="loud" />""");
            screen.Show(ThemeLook.Over("{\"styles\":{\"loud\":{\"text\":\"#00ff00\"" + style + "}}}", ThemeLook.Clarity(dark: false)));
            using var shot = Shot.Of(screen, 800, 400);
            var label = view.Parts[0].Bounds;
            var (left, count) = (int.MaxValue, 0);
            for (var x = (int)label.X; x < label.Right; x++)
                for (var y = (int)label.Y; y < label.Bottom; y++)
                    if (shot.PixelAt(x, y) is { Green: > 150, Red: < 100 })
                    {
                        left = Math.Min(left, x - (int)label.X);
                        count++;
                    }
            return (left, count);
        }
        var plain = Ink("");
        var bold = Ink(",\"weight\":\"bold\"");
        var right = Ink(",\"align\":\"right\"");
        Assert.True(bold.Count > plain.Count * 11 / 10, $"bold {bold.Count} over plain {plain.Count}");
        Assert.True(right.Left > plain.Left + 100, $"right {right.Left} over left {plain.Left}");
    }

    [Fact]
    public void Taborder_sorts_tab_before_the_tree_order()
    {
        var (screen, view, _) = Shown("""
            <Button name="a" at="1,1" span="2" caption="A" />
            <Button name="b" at="3,1" span="2" caption="B" taborder="2" />
            <Button name="c" at="5,1" span="2" caption="C" taborder="1" />
            """);
        var order = new List<string?>();
        for (var i = 0; i < 3; i++)
        {
            screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
            order.Add((screen.Focused as Part)?.Spec.Name);
        }
        Assert.Equal(["c", "b", "a"], order);
    }

    [Fact]
    public void The_focused_text_box_hears_its_keys_before_the_windows_shortcuts()
    {
        var (screen, view, host) = Shown("""
            <Textbox name="note" at="1,1" span="6" />
            <Button name="all" at="7,1" span="3" caption="All" key="Ctrl+A" onclick="all" />
            <Button name="save" at="10,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />
            """);
        var editor = ((TextBoxPart)view.Parts[0]).Editor;
        screen.Focus(editor);
        screen.TextTyped("hello");
        screen.KeyPressed(new KeyInput(Key.A, Modifiers.Control));
        Assert.Equal("hello", editor.SelectedText);
        screen.KeyPressed(new KeyInput(Key.S, Modifiers.Control));
        Assert.DoesNotContain("run all click all", host.Log);
        Assert.Contains("run save click save", host.Log);
    }

    [Fact]
    public void A_typed_symbol_shortcut_answers_the_character_on_any_keyboard()
    {
        var (screen, _, host) = Shown("""
            <Button name="times" at="1,1" span="3" caption="Times" key="*" onclick="times" />
            <Button name="plus" at="4,1" span="3" caption="Plus" key="+" onclick="plus" />
            """);
        // Shift+8 on a US keyboard types the star; the key is a digit, the character is the shortcut.
        screen.KeyPressed(new KeyInput(Key.D8, Modifiers.Shift));
        screen.TextTyped("*");
        // The number pad's plus is the key itself: its character does not press it twice.
        screen.KeyPressed(new KeyInput(Key.Plus, Modifiers.None));
        screen.TextTyped("+");
        Assert.Equal(["run times click times", "run plus click plus"], host.Log);
        // A typed character is the person's while the focus is in a text box.
        Assert.False(Shortcuts.Matches("*", new KeyInput(Key.Other, Modifiers.None) { Typed = "*" }, typing: true));
    }

    sealed class Failing : Element
    {
        public bool Throws = true;

        public override bool Focusable => true;

        public override void Draw(Painter painter)
        {
            if (Throws)
                throw new InvalidOperationException("no paint today");
        }

        public override void OnPointerPressed(PointerInput at) => throw new InvalidOperationException("no press today");
    }

    sealed class Row : Element
    {
        public void Add(Element child) => AddChild(child);

        public override void Arrange(Box box)
        {
            Bounds = box;
            Children[0].Arrange(box with { Width = box.Width / 2 });
            Children[1].Arrange(box with { X = box.X + box.Width / 2, Width = box.Width / 2 });
        }
    }

    sealed class Filled : Element
    {
        public override void Draw(Painter painter) => painter.Fill(Bounds, painter.Look.Color("accent"));
    }

    [Fact]
    public void A_control_whose_drawing_fails_shows_why_and_the_window_stays()
    {
        var row = new Row();
        var failing = new Failing();
        row.Add(failing);
        row.Add(new Filled());
        var screen = new Screen(row, ThemeLook.Clarity(dark: false));
        using var shot = Shot.Of(screen, 400, 100);
        Assert.Equal("no paint today", failing.Failed);
        Assert.Equal(screen.Look.Color("accent"), shot.PixelAt(300, 50));
        Assert.Equal(screen.Look.Color("error"), shot.PixelAt(0, 50));
        // A press that fails says why too, and the screen goes on.
        screen.PointerPressed(50, 50, PointerButton.Left);
        screen.PointerReleased(50, 50, PointerButton.Left);
        Assert.Equal("no press today", failing.Failed);
    }

    sealed class Handy : Element
    {
        public override Cursor? CursorAt(double x, double y) => Cursor.Hand;
    }

    [Fact]
    public void The_pointer_takes_the_shape_of_what_is_under_it()
    {
        var row = new Row();
        row.Add(new Handy());
        row.Add(new Filled());
        var screen = new Screen(row, ThemeLook.Clarity(dark: false));
        screen.Resize(400, 100, 1);
        var changes = 0;
        screen.CursorChanged += () => changes++;
        screen.PointerMoved(50, 50);
        Assert.Equal(Cursor.Hand, screen.Cursor);
        screen.PointerMoved(300, 50);
        Assert.Equal(Cursor.Default, screen.Cursor);
        Assert.Equal(2, changes);
    }

    [Fact]
    public void Shift_turns_the_wheel_sideways()
    {
        var probe = new WheelProbe();
        var screen = new Screen(probe, ThemeLook.Clarity(dark: false));
        screen.Resize(100, 100, 1);
        screen.Wheel(10, 10, 0, 50, Modifiers.Shift);
        screen.Wheel(10, 10, 0, 50);
        Assert.Equal([(50.0, 0.0), (0.0, 50.0)], probe.Turns);
    }

    sealed class WheelProbe : Element
    {
        public readonly List<(double, double)> Turns = [];

        public override bool OnWheel(double dx, double dy)
        {
            Turns.Add((dx, dy));
            return true;
        }
    }

    [Fact]
    public void Enter_presses_a_wizards_next_and_on_its_last_step_finish()
    {
        var (screen, view, host) = Shown("""
            <Steps name="wizard" at="1,1" span="12" rows="6" onfinish="done">
              <Step title="One"><Textbox name="first" at="1,1" span="4" /></Step>
              <Step title="Two"><Label at="1,1" span="4" caption="Two" /></Step>
            </Steps>
            """);
        var wizard = (StepsPart)view.Parts[0];
        screen.Focus(view.Find<TextBoxPart>()!.Editor);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal(2, wizard.Step);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal(["run wizard finish done"], host.Log);
    }

    static string Tall => string.Concat(Enumerable.Range(1, 20).Select(i => $"""<Textbox name="box{i}" at="1,{i}" span="6" />"""));

    [Fact]
    public void Tab_to_a_part_below_the_fold_scrolls_it_into_view()
    {
        var (screen, view, _) = Shown(Tall, height: 300);
        Assert.True(view.MaxScrollY > 0);
        var last = view.Find<TextBoxPart>(p => p.Spec.Name == "box20")!;
        screen.Focus(last.Editor);
        Assert.Equal(view.MaxScrollY, view.ScrollY);
        Assert.True(last.Bounds.Bottom <= 300);
        var first = view.Find<TextBoxPart>(p => p.Spec.Name == "box1")!;
        screen.Focus(first.Editor);
        Assert.Equal(0, view.ScrollY);
    }

    [Fact]
    public void PageDown_and_PageUp_scroll_the_window_and_its_bar_drags()
    {
        var (screen, view, _) = Shown(Tall, height: 300);
        screen.KeyPressed(new KeyInput(Key.PageDown, Modifiers.None));
        var paged = view.ScrollY;
        Assert.True(paged > 100);
        screen.KeyPressed(new KeyInput(Key.PageUp, Modifiers.None));
        Assert.Equal(0, view.ScrollY);
        // Dragging the bar's thumb to the bottom shows the end.
        var thumb = view.Thumb;
        screen.PointerPressed(thumb.X + 1, thumb.Y + 2, PointerButton.Left);
        screen.PointerMoved(thumb.X + 1, 299);
        screen.PointerReleased(thumb.X + 1, 299, PointerButton.Left);
        Assert.Equal(view.MaxScrollY, view.ScrollY);
    }

    [Fact]
    public void Enter_on_a_slider_in_a_form_submits_it()
    {
        // A checkbox or a radio takes Enter itself, as Avalonia's toggle buttons did; an input that leaves it submits its form.
        var (screen, view, host) = Shown("""
            <Form name="order" at="1,1" span="12" rows="3" onsubmit="send">
              <Slider name="amount" at="1,1" span="4" min="0" max="10" />
            </Form>
            """);
        var box = view.Find<Part>(p => p.Spec.Name == "amount")!;
        screen.Focus(box.InOrder().First(e => e.Focusable));
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal(["run order submit send"], host.Log);
    }

    [Fact]
    public void A_key_nothing_takes_is_left_to_the_host()
    {
        var (screen, view, _) = Shown("""<Button name="save" at="1,1" span="3" caption="Save" key="Ctrl+S" onclick="save" />""");
        Assert.True(screen.KeyPressed(new KeyInput(Key.S, Modifiers.Control)));
        Assert.False(screen.KeyPressed(new KeyInput(Key.F5, Modifiers.None)));
        Assert.True(screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None)));
    }

    [Fact]
    public void A_notes_x_says_dismiss_and_shows_a_hand()
    {
        var model = new NotingViewModel();
        var stack = new WindowStack(_ => "<Window name=\"Noting\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 400, 1);
        stack.Push(model);
        model.Note();
        Shot.Of(screen, 800, 400);
        // The x sits at the top right of the first note.
        var (x, y) = (800 - WindowView.ContentPadding - 18, WindowView.ContentPadding + 18);
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        screen.PointerMoved(x, y);
        now += Screen.TipPause;
        screen.WakeDue(now);
        Assert.Equal("Dismiss", screen.Tip?.Text);
        Assert.Equal(Cursor.Hand, screen.Cursor);
    }

    sealed class NotingViewModel() : WindowViewModel(null)
    {
        public override string Title => "Noting";

        public void Note() => Handle(() => Say(Level.Info, Text.Plain("Saved")));
    }

    [Fact]
    public void A_splitters_divider_shows_the_way_it_drags()
    {
        var (screen, view, _) = Shown("""
            <Splitter name="split" at="1,1" span="12" rows="4">
              <Pane><Label at="1,1" span="3" caption="Left" /></Pane>
              <Pane><Label at="1,1" span="3" caption="Right" /></Pane>
            </Splitter>
            """);
        var divider = view.Parts[0].Children.First(c => c is not GridPanel && c.Bounds.Width < 20);
        screen.PointerMoved(divider.Bounds.X + divider.Bounds.Width / 2, divider.Bounds.Y + 10);
        Assert.Equal(Cursor.ResizeLeftRight, screen.Cursor);
    }

    [Fact]
    public void Frame_keys_wait_while_a_frame_menu_is_open()
    {
        var (screen, view, host) = Shown("""
            <Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" key="Ctrl+P" onclick="print" /></Item></Menu>
            """);
        var bar = view.Frame.MenuBar!;
        screen.KeyPressed(new KeyInput(Key.F, Modifiers.Alt));
        Assert.True(view.Frame.Menus.IsOpen);
        screen.KeyPressed(new KeyInput(Key.P, Modifiers.Control));
        Assert.DoesNotContain(host.Log, l => l.Contains("print", StringComparison.Ordinal));
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.P, Modifiers.Control));
        Assert.Contains(host.Log, l => l.Contains("print", StringComparison.Ordinal));
    }

    [Fact]
    public void A_list_field_shows_each_item_joined()
    {
        var host = new TestHost();
        host.Values["tags"] = new List<string> { "red", "blue" };
        var (_, _, parts) = Made.Window("""<Text name="shown" at="1,1" span="6" bind="tags" />""", host);
        Assert.Equal("red, blue", parts[0].Shown("tags", host.Values["tags"]));
    }

    [Fact]
    public void A_dialog_is_as_wide_as_its_captions_need_or_the_standard()
    {
        var host = new TestHost();
        var narrow = new WindowView(WindowMarkup.Read("""<Window name="D"><Label at="1,1" span="6" caption="Hi" /></Window>"""), host);
        Assert.Equal(WindowStack.DialogWidth, WindowStack.DialogSize(narrow, new NotingViewModel()).Width);
        var caption = string.Join(" ", Enumerable.Repeat("a long caption", 8));
        var wide = new WindowView(WindowMarkup.Read($"""<Window name="D"><Label at="1,1" span="3" caption="{caption}" /></Window>"""), host);
        Assert.True(WindowStack.DialogSize(wide, new NotingViewModel()).Width > WindowStack.DialogWidth);
    }

    [Fact]
    public void F10_and_Alt_alone_put_the_focus_on_the_menu_bar()
    {
        var (screen, view, _) = Shown("""
            <Menu name="appMenu"><Item caption="File"><Item name="print" caption="Print" onclick="print" /></Item><Item caption="Edit"><Item name="copy" caption="Copy" onclick="copy" /></Item></Menu>
            <Textbox name="note" at="1,1" span="6" />
            """);
        screen.KeyReleased(new KeyInput(Key.Alt, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.F10, Modifiers.None));
        Assert.Same(view.Frame.MenuBar, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.NotSame(view.Frame.MenuBar, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Alt, Modifiers.Alt));
        screen.KeyReleased(new KeyInput(Key.Alt, Modifiers.None));
        Assert.Same(view.Frame.MenuBar, screen.Focused);
    }

    [Fact]
    public void A_wizard_shows_the_titles_of_all_steps_the_current_one_marked()
    {
        var (screen, view, _) = Shown("""
            <Steps name="wizard" at="1,1" span="12" rows="6">
              <Step title="One"><Label at="1,1" span="4" caption="1" /></Step>
              <Step title="Two"><Label at="1,1" span="4" caption="2" /></Step>
            </Steps>
            """);
        var wizard = (StepsPart)view.Parts[0];
        bool AccentIn(SKBitmap shot, double from, double to)
        {
            var accent = screen.Look.Color("accent");
            for (var x = (int)from; x < to; x++)
                for (var y = (int)wizard.Bounds.Y; y < wizard.Bounds.Y + StepsPart.TitlesHeight; y++)
                    if (shot.PixelAt(x, y) == accent)
                        return true;
            return false;
        }
        var first = wizard.Bounds.X;
        var second = first + screen.Fonts.Measure("One", "text", screen.Look.Size("text"), true) + 6 * screen.Look.Padding;
        using (var shot = Shot.Of(screen, 800, 400))
        {
            Assert.True(AccentIn(shot, first, second - 4));
            Assert.False(AccentIn(shot, second, second + 40));
        }
        wizard.Step = 2;
        using (var shot = Shot.Of(screen, 800, 400))
        {
            Assert.False(AccentIn(shot, first, second - 8));
            Assert.True(AccentIn(shot, second, second + 40));
        }
    }

    [Fact]
    public void A_press_on_a_part_half_out_of_view_clicks_it_without_scrolling_away()
    {
        var parts = string.Concat(Enumerable.Range(1, 8).Select(i => $"""<Label at="1,{i}" span="6" caption="Row {i}" />"""))
            + """<Button name="low" at="1,9" span="3" rows="2" caption="Low" onclick="low" />""";
        var (screen, view, host) = Shown(parts, height: 900);
        var button = Part<ButtonPart>(view, "low");
        // The window ends just below the button's top: it is half out of view.
        var height = button.Bounds.Y + 16 + WindowView.ContentPadding;
        screen.Resize(800, height, 1);
        Assert.True(view.MaxScrollY > 0);
        var y = button.Bounds.Y + 4;
        screen.PointerPressed(button.Bounds.X + 4, y, PointerButton.Left);
        screen.PointerReleased(button.Bounds.X + 4, y, PointerButton.Left);
        Assert.Equal(["run low click low"], host.Log);
    }

    [Fact]
    public void A_right_click_on_a_panels_header_opens_its_menu()
    {
        var (screen, view, host) = Shown("""
            <Panel name="box" at="1,1" span="6" rows="4" caption="Customer">
              <Menu name="boxMenu"><Item name="clearItem" caption="Clear" onclick="clear" /></Menu>
              <Label at="1,1" span="3" caption="Inside" />
            </Panel>
            """);
        var panel = view.Parts[0];
        screen.PointerPressed(panel.Bounds.X + 20, panel.Bounds.Y + 6, PointerButton.Right);
        Assert.True(screen.ContextMenus.IsOpen);
    }
}
