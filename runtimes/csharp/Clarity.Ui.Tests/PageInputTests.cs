// What the person does on a browser page reaches the screen as on the
// desktop: presses counted into double and triple clicks, characters made
// with AltGr, Option, dead keys and input methods as text, copying and
// pasting through the browser's clipboard, and the browser's own keys left
// to it. The page's storage keeps the person's settings across reloads.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using Clarity.Ui.Browser;

namespace Clarity.Ui.Tests;

public class PageInputTests
{
    static (Screen Screen, TextEditor Editor, PageInput Input, List<string> Copied) Page(string text = "")
    {
        var editor = new TextEditor { Text = text };
        var grid = new GridPanel(columns: 1);
        grid.Add(editor, new GridItem(1, 1, 1, 1, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(300, 200, 1);
        screen.Focus(editor);
        var copied = new List<string>();
        return (screen, editor, new PageInput(screen, copied.Add), copied);
    }

    [Fact]
    public void Presses_close_together_are_a_double_and_a_triple_click()
    {
        var (_, editor, input, _) = Page("one two three");
        var at = editor.Bounds;
        var (x, y) = (at.X + 40, at.Y + at.Height / 2);
        input.Pressed(x, y, PointerButton.Left, 1000);
        input.Pressed(x, y, PointerButton.Left, 1200);
        // A double click selects the word.
        Assert.Equal("two", editor.SelectedText);
        input.Pressed(x, y, PointerButton.Left, 1350);
        // A triple click selects the line.
        Assert.Equal("one two three", editor.SelectedText);
        // Long after, a press is a first click again.
        input.Pressed(x, y, PointerButton.Left, 5000);
        Assert.Equal("", editor.SelectedText);
    }

    [Fact]
    public void A_symbol_made_with_AltGr_is_typed_and_is_no_shortcut()
    {
        var (_, editor, input, _) = Page();
        var heard = new List<KeyInput>();
        editor.KeyFirst = k =>
        {
            heard.Add(k);
            return false;
        };
        // Windows says AltGr as Ctrl and Alt: the page lets the field take the character.
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("@", Modifiers.Control | Modifiers.Alt, altGraph: true, mac: false));
        Assert.Equal(Modifiers.None, Assert.Single(heard).Modifiers);
        input.Typed("@");
        input.KeyDown("€", Modifiers.None, altGraph: true, mac: false);
        input.Typed("€");
        Assert.Equal("@€", editor.Text);
    }

    [Fact]
    public void Option_makes_a_character_on_a_Mac_and_Alt_is_a_shortcut_elsewhere()
    {
        Assert.True(PageInput.Types("ø", Modifiers.Alt, mac: true));
        Assert.False(PageInput.Types("f", Modifiers.Alt, mac: false));
        Assert.False(PageInput.Types("s", Modifiers.Control, mac: false));
        Assert.False(PageInput.Types("Enter", Modifiers.None, mac: false));
        Assert.True(PageInput.Types("é", Modifiers.None, mac: false));
        Assert.True(PageInput.Types("😀", Modifiers.None, mac: false));
    }

    [Fact]
    public void A_dead_key_and_an_input_method_give_their_text_when_done()
    {
        var (_, editor, input, _) = Page();
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("Dead", Modifiers.None, false, false));
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("Process", Modifiers.None, false, false));
        Assert.Equal("", editor.Text);
        // What the field took: the accented letter, then an input method's word.
        input.Typed("é");
        input.Typed("日本");
        Assert.Equal("é日本", editor.Text);
        // Control characters a field may give are no text.
        input.Typed("\n");
        Assert.Equal("é日本", editor.Text);
    }

    [Fact]
    public void Keys_that_move_or_run_are_the_page_s_and_the_browser_keeps_its_own()
    {
        var (_, editor, input, _) = Page("abc");
        Assert.Equal(PageKeyWay.Prevent, input.KeyDown("ArrowLeft", Modifiers.None, false, false));
        Assert.Equal(2, editor.Caret);
        Assert.Equal(PageKeyWay.Prevent, input.KeyDown("Tab", Modifiers.None, false, false));
        Assert.Equal(PageKeyWay.Prevent, input.KeyDown("a", Modifiers.Control, false, false));
        Assert.Equal("abc", editor.SelectedText);
        // Reloading and the browser's tools stay the browser's.
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("F5", Modifiers.None, false, false));
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("r", Modifiers.Control, false, false));
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("r", Modifiers.Meta, false, true));
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("I", Modifiers.Control | Modifiers.Shift, false, false));
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("F12", Modifiers.None, false, false));
    }

    [Fact]
    public void A_copy_goes_to_the_browser_s_clipboard_and_a_paste_comes_from_it()
    {
        var (_, editor, input, copied) = Page("hello world");
        editor.Select(0, 5);
        // The browser's copy event follows the key: the page lets it happen.
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("c", Modifiers.Control, false, false));
        Assert.Equal(["hello"], copied);
        editor.Select(11, 11);
        // Ctrl+V waits for the browser's paste, which brings what another program copied.
        Assert.Equal(PageKeyWay.Browser, input.KeyDown("v", Modifiers.Control, false, false));
        Assert.Equal("hello world", editor.Text);
        input.Pasted("!");
        Assert.Equal("hello world!", editor.Text);
        // Cmd+V on a Mac too.
        input.KeyDown("v", Modifiers.Meta, false, true);
        input.Pasted("?");
        Assert.Equal("hello world!?", editor.Text);
    }

    [Fact]
    public void What_a_handler_copies_goes_to_the_browser_s_clipboard()
    {
        var (screen, _, _, copied) = Page();
        screen.Clipboard.SetText("from the server");
        Assert.Equal(["from the server"], copied);
        Assert.Equal("from the server", screen.Clipboard.GetText());
    }

    [Fact]
    public void A_phone_shows_its_keyboard_only_for_a_part_that_takes_text()
    {
        var (screen, editor, input, _) = Page();
        Assert.True(input.TakesText);
        Assert.Equal(editor.Bounds, input.FocusedPlace);
        editor.ReadOnly = () => true;
        Assert.False(input.TakesText);
        screen.Focus(null);
        Assert.False(input.TakesText);
        Assert.Null(input.FocusedPlace);
    }

    [Fact]
    public void A_tap_clicks_a_swipe_pans_and_a_rest_then_a_move_drags()
    {
        var grid = new GridPanel(columns: 1);
        var probe = new Swiped();
        grid.Add(probe, new GridItem(1, 1, 1, 1, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(300, 200, 1);
        var input = new PageInput(screen, _ => { });
        var (x, y) = (probe.Bounds.X + 20, probe.Bounds.Y + 10);
        // A tap: a click where the finger went down.
        input.TouchDown(x, y, 0);
        input.TouchMoved(x + 3, y + 2, 50);
        input.TouchUp(x + 3, y + 2, 80);
        Assert.Equal(["down", "up", "click"], probe.Seen);
        probe.Seen.Clear();
        // A swipe up: the content follows the finger, shown further down.
        input.TouchDown(x, y, 1000);
        input.TouchMoved(x, y - 20, 1050);
        input.TouchMoved(x, y - 30, 1080);
        input.TouchUp(x, y - 30, 1100);
        Assert.Equal(["wheel 0,20", "wheel 0,10"], probe.Seen);
        probe.Seen.Clear();
        // A rest, then a move: a drag of what the finger is on (let go on it, a click too, as with a mouse).
        input.TouchDown(x, y, 3000);
        input.TouchMoved(x + 40, y, 3000 + PageInput.Hold);
        input.TouchUp(x + 40, y, 3600);
        Assert.Equal(["down", "drag", "up", "click"], probe.Seen);
        // A gesture the browser takes over does nothing.
        probe.Seen.Clear();
        input.TouchDown(x, y, 5000);
        input.TouchCancelled();
        input.TouchUp(x, y, 5050);
        Assert.Empty(probe.Seen);
    }

    sealed class Swiped : Element
    {
        public List<string> Seen = [];
        public override bool Focusable => true;
        public override void OnPointerPressed(PointerInput at) => Seen.Add("down");
        public override void OnPointerMoved(PointerInput at)
        {
            if (Pressed)
                Seen.Add("drag");
        }
        public override void OnPointerReleased(PointerInput at) => Seen.Add("up");
        public override void OnClick() => Seen.Add("click");
        public override bool OnWheel(double dx, double dy)
        {
            Seen.Add($"wheel {dx:0},{dy:0}");
            return true;
        }
    }
}

public class PageCursorTests
{
    [Theory]
    [InlineData(Cursor.Default, "default")]
    [InlineData(Cursor.Hand, "pointer")]
    [InlineData(Cursor.Text, "text")]
    [InlineData(Cursor.ResizeLeftRight, "ew-resize")]
    [InlineData(Cursor.ResizeUpDown, "ns-resize")]
    [InlineData(Cursor.Move, "move")]
    [InlineData(Cursor.Crosshair, "crosshair")]
    [InlineData(Cursor.NotAllowed, "not-allowed")]
    public void The_page_s_pointer_is_css_s_cursor(Cursor cursor, string css) => Assert.Equal(css, PageCursor.Css(cursor));
}

// The page and the settings' store are the whole program's: alone, with the tests of what is remembered.
[Collection("Remembered")]
public class PageServicesTests
{
    sealed class StoragePage : IBrowserPage
    {
        public readonly Dictionary<string, string> Storage = [];
        public void SetTitle(string title) { }
        public void Download(string name, byte[] data) { }
        public void Show(string name, byte[] data) { }
        public string? Read(string key) => Storage.GetValueOrDefault(key);
        public void Write(string key, string value) => Storage[key] = value;
        public int Play(byte[] file, double volume, bool repeat) => 0;
        public void Stop(int sound) { }
    }

    [Fact]
    public void The_person_s_settings_are_kept_in_the_browser_s_storage_across_reloads()
    {
        var (store, page) = (KeptSettings.Store, BrowserPage.Current);
        try
        {
            var storage = new StoragePage();
            PageServices.Use(storage);
            KeptSettings.Keep("Hosting", new JsonObject { ["nickname"] = "Ada" });
            Assert.Contains("\"Ada\"", storage.Storage["settings:Hosting"]);
            // A reload: a new program reads them from the same storage.
            KeptSettings.Store = null;
            PageServices.Use(new StoragePage { Storage = { ["settings:Hosting"] = storage.Storage["settings:Hosting"] } });
            Assert.Equal("Ada", KeptSettings.Read("Hosting")["nickname"]!.GetValue<string>());
        }
        finally
        {
            (KeptSettings.Store, BrowserPage.Current) = (store, page);
        }
    }
}
