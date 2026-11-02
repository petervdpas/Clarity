// Clarity's keys window on Clarity.UI (DESIGN 5.28): every command per place
// with its keys; Change, then press the new key; a key another command has
// in the same place offers a swap; Reset and Reset all; refused keys say
// why. As the root of a window of its own, and as a modal overlay over the
// window the person works in. One class: the person's keys are shared.

using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

[Collection("PersonKeys")]
public sealed class KeysWindowTests
{
    static readonly KeyLayerInfo[] Catalog =
    [
        new("", null, [new("keysItem", Text.Plain("Keys"), []), new("quitItem", Text.Plain("Quit"), ["Ctrl+Q"])]),
        new("Card", Text.Plain("Card"), [new("Card.saveButton", Text.Plain("Save"), ["Ctrl+S"])]),
    ];

    static readonly KeyInput CtrlK = new(Key.K, Modifiers.Control);

    /// <summary>The window as the root of a screen of its own, which hands it its keys first.</summary>
    static (Screen Screen, KeysWindow Window) Shown(IReadOnlyList<KeyLayerInfo>? layers = null, double height = 640)
    {
        PersonKeys.Stop();
        var window = new KeysWindow(layers ?? Catalog);
        var screen = new Screen(window, ThemeLook.Clarity(dark: false));
        screen.Keys = window.KeyPressed;
        screen.Resize(500, height, 1);
        return (screen, window);
    }

    static void Click(Screen screen, KeysWindow.Push push)
    {
        var (x, y) = (push.Bounds.X + push.Bounds.Width / 2, push.Bounds.Y + push.Bounds.Height / 2);
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    [Fact]
    public void Lists_every_command_by_place_with_its_keys()
    {
        var (_, window) = Shown();
        foreach (var expected in new[] { "Keys", "Everywhere", "Quit", "Ctrl+Q", "Card", "Save", "Ctrl+S", "None" })
            Assert.Contains(expected, window.Says);
        // Everywhere first, then the window's own.
        Assert.True(window.Says.ToList().IndexOf("Everywhere") < window.Says.ToList().IndexOf("Card"));
        Assert.Equal("Keys", KeysWindow.Title);
    }

    [Fact]
    public void A_new_key_is_taken_and_kept_and_reset_puts_it_back()
    {
        var (screen, window) = Shown();
        Click(screen, window.Button("change", "Card.saveButton"));
        Assert.Contains("Press the new key; Escape stops", window.Says);
        screen.KeyPressed(CtrlK);
        Assert.Equal(["Ctrl+K"], PersonKeys.Choices["Card.saveButton"]);
        Assert.Contains("Ctrl+K", window.Says);
        Assert.False(window.Capturing);
        Click(screen, window.Button("reset", "Card.saveButton"));
        Assert.Empty(PersonKeys.Choices);
        Assert.Contains("Ctrl+S", window.Says);
    }

    [Fact]
    public void Escape_stops_waiting_for_a_key_and_then_closes()
    {
        var (screen, window) = Shown();
        var closed = 0;
        window.Done = () => closed++;
        Click(screen, window.Button("change", "Card.saveButton"));
        Assert.True(window.Capturing);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.False(window.Capturing);
        Assert.Equal(0, closed);
        Assert.Empty(PersonKeys.Choices);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Equal(1, closed);
        Click(screen, window.Button("close"));
        Assert.Equal(2, closed);
    }

    [Fact]
    public void A_key_another_command_has_in_the_same_place_offers_a_swap()
    {
        var (screen, window) = Shown();
        Click(screen, window.Button("change", "keysItem"));
        screen.KeyPressed(new KeyInput(Key.Q, Modifiers.Control));
        Assert.Empty(PersonKeys.Choices);
        Assert.Contains("Ctrl+Q is the key of Quit. Swap them?", window.Says);
        Click(screen, window.Button("swap"));
        Assert.Equal(["Ctrl+Q"], PersonKeys.Choices["keysItem"]);
        Assert.Empty(PersonKeys.Choices["quitItem"]);
        Assert.DoesNotContain("Ctrl+Q is the key of Quit. Swap them?", window.Says);
        Click(screen, window.Button("resetAll"));
        Assert.Empty(PersonKeys.Choices);
    }

    [Fact]
    public void A_swap_can_be_cancelled()
    {
        var (screen, window) = Shown();
        Click(screen, window.Button("change", "keysItem"));
        screen.KeyPressed(new KeyInput(Key.Q, Modifiers.Control));
        Click(screen, window.Button("cancel"));
        Assert.Empty(PersonKeys.Choices);
        Assert.DoesNotContain("Ctrl+Q is the key of Quit. Swap them?", window.Says);
    }

    [Fact]
    public void A_key_of_another_place_is_allowed_and_said()
    {
        var (screen, window) = Shown();
        Click(screen, window.Button("change", "Card.saveButton"));
        screen.KeyPressed(new KeyInput(Key.Q, Modifiers.Control));
        Assert.Equal(["Ctrl+Q"], PersonKeys.Choices["Card.saveButton"]);
        Assert.Contains("Ctrl+Q is also the key of Quit, elsewhere; here Save goes first", window.Says);
    }

    [Fact]
    public void A_key_the_inputs_need_is_refused_saying_why()
    {
        var (screen, window) = Shown();
        Click(screen, window.Button("change", "Card.saveButton"));
        screen.KeyPressed(new KeyInput(Key.A, Modifiers.None));
        Assert.Empty(PersonKeys.Choices);
        Assert.Contains("A cannot be used: a key without Ctrl or Alt is typing", window.Says);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Contains("Tab cannot be used: Tab moves between fields", window.Says);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.Control));
        Assert.Contains("Ctrl+Enter cannot be used: Enter confirms", window.Says);
        // It still waits for a key it can take.
        screen.KeyPressed(CtrlK);
        Assert.Equal(["Ctrl+K"], PersonKeys.Choices["Card.saveButton"]);
    }

    [Fact]
    public void Reset_is_only_offered_for_a_key_the_person_changed()
    {
        var (_, window) = Shown();
        Assert.False(window.Button("reset", "Card.saveButton").Enabled);
        Assert.False(window.Button("resetAll").Enabled);
        PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>> { ["Card.saveButton"] = ["Ctrl+K"] });
        Assert.True(window.Button("reset", "Card.saveButton").Enabled);
        Assert.True(window.Button("resetAll").Enabled);
        // It shows what the person's keys are now, set from anywhere.
        Assert.Contains("Ctrl+K", window.Says);
    }

    [Fact]
    public void A_pressed_key_reads_as_clarity_writes_keys()
    {
        Assert.Equal("Ctrl+Shift+N", KeysWindow.Written(new KeyInput(Key.N, Modifiers.Control | Modifiers.Shift)));
        Assert.Equal("Ctrl+1", KeysWindow.Written(new KeyInput(Key.D1, Modifiers.Control)));
        Assert.Equal("Alt+Left", KeysWindow.Written(new KeyInput(Key.Left, Modifiers.Alt)));
        Assert.Equal("Backspace", KeysWindow.Written(new KeyInput(Key.Backspace, Modifiers.None)));
        Assert.Equal("Ctrl++", KeysWindow.Written(new KeyInput(Key.Plus, Modifiers.Control)));
        Assert.Equal("Shift+F5", KeysWindow.Written(new KeyInput(Key.F5, Modifiers.Shift)));
        Assert.Equal("Ctrl+Alt+Shift+Meta+X", KeysWindow.Written(new KeyInput(Key.X, Modifiers.Control | Modifiers.Alt | Modifiers.Shift | Modifiers.Meta)));
        Assert.Null(KeysWindow.Written(new KeyInput(Key.Other, Modifiers.Control)));
        // Each key written so reads back as the same key.
        foreach (var key in Enum.GetValues<Key>().Where(k => k != Key.Other))
            Assert.Equal(key, Shortcuts.KeyOf(Shortcuts.Parts(KeysWindow.Written(new KeyInput(key, Modifiers.Control))!).Key));
    }

    /// <summary>A message or a swap offer takes a line kept for it, so nothing moves under the mouse.</summary>
    [Fact]
    public void Its_buttons_stay_put_when_it_says_something()
    {
        var (screen, window) = Shown();
        var change = window.Button("change", "Card.saveButton").Bounds;
        var close = window.Button("close").Bounds;
        Click(screen, window.Button("change", "Card.saveButton"));
        screen.KeyPressed(new KeyInput(Key.A, Modifiers.None));
        Assert.Equal(change, window.Button("change", "Card.saveButton").Bounds);
        Assert.Equal(close, window.Button("close").Bounds);
        Click(screen, window.Button("change", "keysItem"));
        screen.KeyPressed(new KeyInput(Key.Q, Modifiers.Control));
        Assert.Contains("Ctrl+Q is the key of Quit. Swap them?", window.Says);
        Assert.Equal(change, window.Button("change", "Card.saveButton").Bounds);
        Assert.Equal(close, window.Button("close").Bounds);
    }

    [Fact]
    public void It_wants_a_size_of_its_own_and_at_most_640_high()
    {
        var (_, window) = Shown();
        Assert.InRange(window.Wanted.Width, 400, 640);
        Assert.InRange(window.Wanted.Height, 200, 640);
        var many = new KeyLayerInfo[] { new("", null, [.. Enumerable.Range(0, 40).Select(i => new KeyCommandInfo($"c{i}", Text.Plain($"Command {i}"), []))]) };
        Assert.Equal(640, new KeysWindow(many).Wanted.Height);
    }

    [Fact]
    public void It_scrolls_with_the_wheel_when_the_commands_are_taller()
    {
        var many = new KeyLayerInfo[] { new("", null, [.. Enumerable.Range(0, 40).Select(i => new KeyCommandInfo($"c{i}", Text.Plain($"Command {i}"), []))]) };
        var (screen, window) = Shown(many, height: 400);
        var close = window.Button("close");
        Assert.True(close.Bounds.Y > 400);
        screen.Wheel(250, 200, 0, 10000);
        Assert.True(close.Bounds.Bottom <= 400);
        Assert.True(close.ShowsInTree);
        // A button scrolled away is out of reach.
        Assert.False(window.Button("change", "c0").ShowsInTree);
        Click(screen, close);
        screen.Wheel(250, 200, 0, -10000);
        Assert.True(window.Button("change", "c0").ShowsInTree);
    }

    [Fact]
    public void As_a_modal_overlay_it_takes_keys_through_the_focus_and_closes()
    {
        PersonKeys.Stop();
        var screen = new Screen(new Plate("background"), ThemeLook.Clarity(dark: false));
        screen.Resize(900, 700, 1);
        var window = new KeysWindow(Catalog);
        var closed = false;
        Overlay? overlay = null;
        overlay = new Overlay(window, new Box(0, 0, 900, 700)) { Modal = true, Closed = () => closed = true };
        window.Done = () => screen.Close(overlay);
        screen.Open(overlay);
        Assert.Same(window, screen.Focused);
        // A centered panel over the dimmed window.
        Assert.True(window.Panel.Width < 900 && window.Panel.X > 0);
        Assert.Equal(450, window.Panel.X + window.Panel.Width / 2, 1);
        Click(screen, window.Button("change", "Card.saveButton"));
        screen.KeyPressed(CtrlK);
        Assert.Equal(["Ctrl+K"], PersonKeys.Choices["Card.saveButton"]);
        // Tab while waiting is a key it refuses, not a move of the focus.
        Click(screen, window.Button("change", "quitItem"));
        var focused = screen.Focused;
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(focused, screen.Focused);
        Assert.Contains("Tab cannot be used: Tab moves between fields", window.Says);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.False(closed);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.True(closed);
        Assert.Empty(screen.Overlays);
        PersonKeys.Stop();
    }

    [Fact]
    public void Enter_on_a_focused_button_presses_it()
    {
        var (screen, window) = Shown();
        screen.Focus(window.Button("change", "Card.saveButton"));
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.True(window.Capturing);
    }

    [Fact]
    public void It_draws_in_the_theme_as_a_root_and_as_an_overlay()
    {
        var (screen, window) = Shown();
        using var root = Shot.Of(screen, 500, 640);
        // The heading is written at the top left.
        var background = ThemeLook.Clarity(dark: false).Color("background");
        var written = 0;
        for (var x = 16; x < 120; x++)
            for (var y = 16; y < 44; y++)
                if (root.GetPixel(x, y) != background)
                    written++;
        Assert.True(written > 0);
        var under = new Screen(new Plate("background"), ThemeLook.Clarity(dark: false));
        var over = new KeysWindow(Catalog);
        under.Resize(900, 700, 1);
        under.Open(new Overlay(over, new Box(0, 0, 900, 700)) { Modal = true });
        using var shot = Shot.Of(under, 900, 700);
        // The window under it is dimmed; the panel is in the panel color.
        Assert.NotEqual(ThemeLook.Clarity(dark: false).Color("background"), shot.GetPixel(5, 5));
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("panel"), shot.GetPixel((int)over.Panel.X + 6, (int)over.Panel.Bottom - 6));
    }
}
