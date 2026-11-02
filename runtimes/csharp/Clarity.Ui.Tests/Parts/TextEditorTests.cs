// The text editor every text input holds: typing, the keys that move and
// delete, the selection with Shift, the pointer and the clipboard, a long
// line that scrolls, lines that wrap, the placeholder and a password's dots.

namespace Clarity.Ui.Tests.Parts;

public class TextEditorTests
{
    static (Screen Screen, TextEditor Editor) Shown(bool multiline = false, string text = "", double width = 300, int rows = 1)
    {
        var editor = new TextEditor(multiline) { Text = text };
        var grid = new GridPanel(columns: 1);
        grid.Add(editor, new GridItem(1, 1, 1, rows, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(width, 300, 1);
        screen.Focus(editor);
        return (screen, editor);
    }

    static KeyInput K(Key key, Modifiers modifiers = Modifiers.None) => new(key, modifiers);

    [Fact]
    public void Typed_text_goes_in_at_the_caret_and_says_it_was_edited()
    {
        var (screen, editor) = Shown(text: "Milk");
        var edits = 0;
        editor.Edited += () => edits++;
        screen.TextTyped("!");
        screen.KeyPressed(K(Key.Home));
        screen.TextTyped("Oat ");
        Assert.Equal("Oat Milk!", editor.Text);
        Assert.Equal(4, editor.Caret);
        Assert.Equal(2, edits);
    }

    [Fact]
    public void Text_set_by_code_is_no_edit()
    {
        var (_, editor) = Shown();
        var edits = 0;
        editor.Edited += () => edits++;
        editor.Text = "set";
        Assert.Equal(0, edits);
        Assert.Equal(3, editor.Caret);
    }

    [Fact]
    public void Backspace_and_Delete_take_a_character_or_the_selection()
    {
        var (screen, editor) = Shown(text: "abcdef");
        screen.KeyPressed(K(Key.Left));
        screen.KeyPressed(K(Key.Backspace));
        Assert.Equal("abcdf", editor.Text);
        screen.KeyPressed(K(Key.Delete));
        Assert.Equal("abcd", editor.Text);
        screen.KeyPressed(K(Key.Left, Modifiers.Shift));
        screen.KeyPressed(K(Key.Left, Modifiers.Shift));
        Assert.Equal("cd", editor.SelectedText);
        screen.KeyPressed(K(Key.Delete));
        Assert.Equal("ab", editor.Text);
        screen.KeyPressed(K(Key.Home));
        screen.KeyPressed(K(Key.Backspace));
        Assert.Equal("ab", editor.Text);
    }

    [Fact]
    public void Shift_selects_and_typing_replaces_the_selection()
    {
        var (screen, editor) = Shown(text: "one two");
        screen.KeyPressed(K(Key.Home, Modifiers.Shift));
        Assert.Equal("one two", editor.SelectedText);
        screen.KeyPressed(K(Key.Right));
        Assert.Equal((7, 7), editor.Selection);
        screen.KeyPressed(K(Key.Left, Modifiers.Control | Modifiers.Shift));
        Assert.Equal("two", editor.SelectedText);
        screen.TextTyped("three");
        Assert.Equal("one three", editor.Text);
    }

    [Fact]
    public void Copy_cut_and_paste_go_through_the_screens_clipboard()
    {
        var (screen, editor) = Shown(text: "hello world");
        screen.KeyPressed(K(Key.A, Modifiers.Control));
        Assert.Equal("hello world", editor.SelectedText);
        screen.KeyPressed(K(Key.C, Modifiers.Control));
        Assert.Equal("hello world", screen.Clipboard.GetText());
        editor.Select(5, 11);
        screen.KeyPressed(K(Key.X, Modifiers.Control));
        Assert.Equal((" world", "hello"), (screen.Clipboard.GetText(), editor.Text));
        screen.KeyPressed(K(Key.Home));
        screen.KeyPressed(K(Key.V, Modifiers.Control));
        Assert.Equal(" worldhello", editor.Text);
    }

    [Fact]
    public void One_line_takes_no_line_break_and_Enter_goes_to_its_holder()
    {
        var (screen, editor) = Shown();
        var entered = 0;
        editor.Entered = () => ++entered > 0;
        screen.Clipboard.SetText("a\r\nb");
        screen.KeyPressed(K(Key.V, Modifiers.Control));
        screen.KeyPressed(K(Key.Enter));
        Assert.Equal(("a b", 1), (editor.Text, entered));
    }

    [Fact]
    public void Several_lines_take_Enter_and_Up_and_Down_move_between_lines()
    {
        var (screen, editor) = Shown(multiline: true, text: "first", rows: 3);
        screen.KeyPressed(K(Key.Enter));
        screen.TextTyped("second");
        Assert.Equal("first\nsecond", editor.Text);
        screen.KeyPressed(K(Key.Up));
        Assert.True(editor.Caret <= 5);
        screen.KeyPressed(K(Key.Home));
        Assert.Equal(0, editor.Caret);
        screen.KeyPressed(K(Key.Down));
        screen.KeyPressed(K(Key.End));
        Assert.Equal(editor.Text.Length, editor.Caret);
    }

    [Fact]
    public void Read_only_moves_and_copies_but_takes_no_typing()
    {
        var (screen, editor) = Shown(text: "fixed");
        editor.ReadOnly = () => true;
        screen.TextTyped("x");
        screen.KeyPressed(K(Key.Backspace));
        screen.KeyPressed(K(Key.Home, Modifiers.Shift));
        screen.KeyPressed(K(Key.C, Modifiers.Control));
        Assert.Equal(("fixed", "fixed"), (editor.Text, screen.Clipboard.GetText()));
    }

    [Fact]
    public void A_press_places_the_caret_a_drag_selects_and_a_double_click_takes_a_word()
    {
        var (screen, editor) = Shown(text: "alpha beta gamma");
        var b = editor.Bounds;
        screen.PointerPressed(b.X + 2, b.Y + 10, PointerButton.Left);
        Assert.Equal(0, editor.Caret);
        screen.PointerMoved(b.Right - 2, b.Y + 10);
        screen.PointerReleased(b.Right - 2, b.Y + 10, PointerButton.Left);
        Assert.Equal("alpha beta gamma", editor.SelectedText);
        var middle = editor.Bounds.X + 8 + 12;
        screen.PointerPressed(middle, b.Y + 10, PointerButton.Left, clicks: 2);
        Assert.Equal("alpha", editor.SelectedText);
    }

    [Fact]
    public void A_long_line_scrolls_to_keep_the_caret_in_view()
    {
        var (screen, editor) = Shown(text: "", width: 120);
        screen.TextTyped(new string('w', 60));
        Assert.True(editor.ScrollX > 0);
        screen.KeyPressed(K(Key.Home));
        Assert.Equal(0, editor.ScrollX);
    }

    [Fact]
    public void Lines_wrap_and_scroll_down_with_the_caret_and_the_wheel()
    {
        var (screen, editor) = Shown(multiline: true, text: "", width: 120, rows: 2);
        screen.TextTyped(string.Join(" ", Enumerable.Repeat("word", 40)));
        Assert.True(editor.ScrollY > 0);
        screen.KeyPressed(K(Key.Home, Modifiers.Control));
        Assert.Equal(0, editor.ScrollY);
        var b = editor.Bounds;
        screen.Wheel(b.X + 5, b.Y + 5, 0, 30);
        Assert.Equal(30, editor.ScrollY);
    }

    [Fact]
    public void Empty_it_shows_its_placeholder_and_a_password_shows_dots()
    {
        var (screen, editor) = Shown();
        editor.Placeholder = "Type here";
        screen.Focus(null);
        using (var pixels = Shot.Of(screen, 300, 300))
            Assert.True(Inked(pixels, editor.Bounds, screen.Look.Color("muted")));
        editor.Text = "secret";
        editor.Password = true;
        screen.KeyPressed(K(Key.A, Modifiers.Control));
        screen.Focus(editor);
        screen.KeyPressed(K(Key.A, Modifiers.Control));
        screen.KeyPressed(K(Key.C, Modifiers.Control));
        Assert.Null(screen.Clipboard.GetText());
        using var shot = Shot.Of(screen, 300, 300);
        Assert.True(Inked(shot, editor.Bounds, screen.Look.Color("selection")));
    }

    [Fact]
    public void The_caret_and_the_selection_draw_in_the_themes_colors()
    {
        var (screen, editor) = Shown(text: "selected");
        editor.SelectAll();
        using var pixels = Shot.Of(screen, 300, 300);
        Assert.True(Inked(pixels, editor.Bounds, screen.Look.Color("selection")));
        Assert.True(Inked(pixels, editor.Bounds, screen.Look.Color("focus")));
    }

    /// <summary>Whether any pixel in a place has a color.</summary>
    internal static bool Inked(SKBitmap pixels, Box box, SKColor color)
    {
        for (var y = (int)box.Y; y < (int)box.Bottom; y++)
            for (var x = (int)box.X; x < (int)box.Right; x++)
            {
                var p = pixels.GetPixel(x, y);
                if (Math.Abs(p.Red - color.Red) < 6 && Math.Abs(p.Green - color.Green) < 6 && Math.Abs(p.Blue - color.Blue) < 6)
                    return true;
            }
        return false;
    }
}
