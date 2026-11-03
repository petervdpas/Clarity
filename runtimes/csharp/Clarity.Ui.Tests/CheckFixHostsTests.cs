// Rows of the hosting area the check found checked live only, now tested
// where a test reaches them: a browser's wheel notch is 50 of the screen's
// units whatever the browser says it in, so Ctrl+wheel zooms 1.15 a notch;
// a window takes no input while a file dialog is open; a desktop window
// opens at the standard size, the size its window names, or a dialog's,
// with a dialog's first part focused and the top window's title; a sound
// that ended lets go on the screen's thread; the databases connected while
// unconnected close as the application ends; and a custom part's state
// reaches it and its tells come back, as Avalonia's DotNetPartTests asked.

using System.ComponentModel;
using System.Text.Json;
using Clarity.Runtime;
using Clarity.Ui.Browser;

namespace Clarity.Ui.Tests;

public class CheckFixHostsTests
{
    // -- The browser's wheel -------------------------------------------------------------------

    [Fact]
    public void A_browser_s_wheel_notch_is_fifty_in_pixels_lines_or_pages()
    {
        // Chrome, Edge and Safari: 100 pixels a notch; Firefox: 3 lines.
        Assert.Equal((0.0, 50.0), PageInput.WheelUnits(0, 100, 0, 700));
        Assert.Equal((50.0, 0.0), PageInput.WheelUnits(3, 0, 1, 700));
        Assert.Equal((0.0, -700.0), PageInput.WheelUnits(0, -1, 2, 700));
        // A touchpad's small pixel turns, in proportion.
        Assert.Equal((0.0, 2.0), PageInput.WheelUnits(0, 4, 0, 700));
    }

    [Fact]
    public void Ctrl_and_a_browser_s_wheel_notch_zoom_once_not_squared()
    {
        var (screen, part) = CanvasPartTests.Alone("""<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" zoom="true" />""", new CanvasPartHost());
        var canvas = (CanvasPart)part;
        var input = new PageInput(screen, _ => { });
        var whole = canvas.Viewport;
        screen.PointerMoved(400, 200);
        input.Wheel(400, 200, 0, -100, 0, 400, Modifiers.Control);
        Assert.Equal(whole.Scale * CanvasPart.NotchZoom, canvas.Viewport.Scale, 6);
        input.Wheel(400, 200, 0, 3, 1, 400, Modifiers.Control);
        Assert.Equal(whole.Scale, canvas.Viewport.Scale, 6);
    }

    // -- Desktop windows ------------------------------------------------------------------------------

    sealed class Probe : Element
    {
        public readonly List<string> Seen = [];
        public override void OnPointerPressed(PointerInput at) => Seen.Add("press");
    }

    sealed class SizedViewModel() : WindowViewModel(null)
    {
        public override string Title => "Sized";

        public override Extent? WindowSize => new(640, 480);
    }

    sealed class PlainViewModel() : WindowViewModel(null)
    {
        public override string Title => "Plain";
    }

    const string Dialog = """
        <Window name="PickFriend">
          <Label name="who" at="1,1" span="12" caption="Who?" />
          <Button name="cancelButton" at="1,2" span="6" caption="Cancel" cancel="true" onclick="close" />
        </Window>
        """;

    static AppScreen App() => new(name => name switch
    {
        "Windows/PickFriend.cxaml" => Dialog,
        "Windows/Sized.cxaml" => """<Window name="Sized"><Label at="1,1" span="4" caption="Sized" /></Window>""",
        "Windows/Plain.cxaml" => """<Window name="Plain"><Label at="1,1" span="4" caption="Plain" /></Window>""",
        "Windows/Lender.cxaml" => """<Window name="Lender"><Label at="1,1" span="4" caption="Lender" /></Window>""",
        _ => null,
    }, systemDark: false);

    [Fact]
    public void The_systems_close_makes_the_top_window_hear_its_on_close_and_tells_the_opener()
    {
        var lender = new LenderViewModel();
        using var apart = new AppScreen(name => name == "Windows/PickFriend.cxaml" ? Dialog : null, systemDark: false, opener: lender);
        var pick = new PickFriendViewModel();
        apart.Stack.Push(pick);
        // The desktop window's close button: DesktopWindow's Close runs the stack's Closing.
        apart.Stack.Closing();
        Assert.Equal(1, pick.ClosingTimes);
        Assert.Equal(1, lender.ReturnedTimes);
    }

    [Fact]
    public void A_window_of_its_own_ends_when_its_view_says_done()
    {
        var lender = new LenderViewModel();
        using var apart = new AppScreen(name => name == "Windows/PickFriend.cxaml" ? Dialog : null, systemDark: false, opener: lender);
        var pick = new PickFriendViewModel();
        apart.Stack.Push(pick);
        var ended = 0;
        // DesktopWindow closes the desktop window on Ended, without asking on close again.
        apart.Stack.Ended += () => ended++;
        pick.CloseWindow_Command.Execute(null);
        Assert.Equal(1, ended);
        Assert.Equal(1, pick.ClosingTimes);
        Assert.Equal(1, lender.ReturnedTimes);
    }

    sealed class Board : IClipboard
    {
        public string? Text;
        public string? GetText() => Text;
        public void SetText(string text) => Text = text;
    }

    [Fact]
    public void A_copy_goes_to_the_window_s_clipboard_as_shown()
    {
        using var app = App();
        var plain = new PlainViewModel();
        app.Stack.Push(plain);
        var board = new Board();
        // The desktop gives its screen the system's clipboard (SystemClipboard), the browser the page's.
        app.Screen.Clipboard = board;
        plain.Copy(Text.Plain("Name: Ada"));
        Assert.Equal("Name: Ada", board.Text);
    }

    // -- Sound -----------------------------------------------------------------------------------------

    sealed class Held : IDisposable
    {
        public int Released;
        public void Dispose() => Released++;
    }

    // -- Unconnected -----------------------------------------------------------------------------------

    sealed class Setting : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged;
        public void Change() => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs("shopDb"));
    }

    sealed class Card(Db db, string title) : WindowViewModel(db)
    {
        public override string Title => title;
    }

    /// <summary>A desktop as the unconnected flow sees it: what it shows closes when the loop says so; it keeps the databases connected.</summary>
    sealed class Desk : IUnconnectedScreen
    {
        public Action? Close;
        public Db? Connected;

        public void Show(WindowViewModel first, IReadOnlyList<Message> said, Action whenClosed) => Close = whenClosed;
        void IUnconnectedScreen.Connected(Db db, WindowViewModel main) => Connected = db;
        void IUnconnectedScreen.Ended() { }
    }

    // -- A custom part ----------------------------------------------------------------------------------

    sealed class TellingHost : TestHost
    {
        public override void Told(string part, string tell, JsonElement[] values) => Log.Add($"told {part} {tell}");
    }

    /// <summary>DotNetPartTests.AStateReachesThePartAndItsTellsComeBack, on Clarity.UI's CustomPart.</summary>
    [Fact]
    public void A_state_reaches_the_part_and_its_tells_come_back()
    {
        var host = new TellingHost();
        var state = host.State("gauge")!.With("speed", 5L);
        state.Hear("restart", 2L);
        var (_, _, parts) = Made.Window("""<Custom name="gauge" at="1,1" span="4" rows="3" part="Gauge" bind="level" onchange="gaugeChanged" tells='[{"tell":"reached","handler":"gaugeReached"}]' />""", host);
        var gauge = Assert.IsType<Gauge>(Assert.IsType<CustomPlace>(parts[0]).Inner);
        // What code set and told before the window showed, in that order.
        Assert.True(gauge.Got.IndexOf("set speed 5") < gauge.Got.IndexOf("hear restart 2"));
        gauge.Got.Clear();
        state.SetValue("speed", 8L);
        Assert.Equal(["set speed 8"], gauge.Got);
        var told = new List<string>();
        gauge.Told += (tell, values) => told.Add($"{tell} {values[0].GetInt64()}");
        // The person's change and a tell: what it binds takes the value, on change runs once, the tell comes back.
        gauge.OnClick();
        Assert.Equal(["reached 3"], told);
        Assert.Equal(9L, host.Values["level"]);
        Assert.Single(host.Log, l => l == "run gauge change gaugeChanged");
        Assert.Contains("told gauge reached", host.Log);
    }
}
