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
using Clarity.Ui.Desktop;

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

    [Fact]
    public async Task While_a_file_dialog_is_open_what_the_person_does_on_a_window_is_dropped()
    {
        var probe = new Probe();
        var screen = new Screen(probe, ThemeLook.Clarity(dark: false));
        screen.Resize(200, 100, 1);
        using var open = new SemaphoreSlim(0);
        var picker = new SystemFilePicker(_ =>
        {
            open.Wait();
            return null;
        }, _ => { }, action => action());
        var input = new Queue<Action>();
        void Press() => input.Enqueue(() => screen.PointerPressed(50, 50, PointerButton.Left));
        void Release() => input.Enqueue(() => screen.PointerReleased(50, 50, PointerButton.Left));
        void Pump() => DesktopWindow.Drain(input, () => DesktopWindow.BlocksInput(false, picker), action => action());
        var choosing = picker.ChooseFolder("Choose a folder");
        Press();
        Release();
        Pump();
        Assert.Empty(probe.Seen);
        // Dropped, not kept for when the dialog closed.
        Assert.Empty(input);
        open.Release();
        Assert.Null(await choosing.WaitAsync(TestContext.Current.CancellationToken));
        Press();
        Release();
        Pump();
        Assert.Equal(["press"], probe.Seen);
        // A dialog of the window's own blocks it too.
        Assert.True(DesktopWindow.BlocksInput(true, picker));
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
    public void A_desktop_window_opens_at_the_standard_size_the_size_its_window_names_or_a_dialog_s()
    {
        Assert.Equal(new Extent(1100, 760), DesktopWindow.StandardSize);
        using (var plain = App())
        {
            var first = new PlainViewModel();
            plain.Stack.Push(first);
            Assert.Equal(DesktopWindow.StandardSize, DesktopWindow.OpenSize(plain, first, dialog: false));
        }
        using (var sized = App())
        {
            var first = new SizedViewModel();
            sized.Stack.Push(first);
            Assert.Equal(new Extent(640, 480), DesktopWindow.OpenSize(sized, first, dialog: false));
        }
        using var dialog = App();
        var pick = new PickFriendViewModel();
        dialog.Stack.Push(pick);
        Assert.Equal(WindowStack.DialogSize(dialog.Stack.View!, pick), DesktopWindow.OpenSize(dialog, pick, dialog: true));
    }

    [Fact]
    public void A_dialog_in_a_window_of_its_own_starts_on_its_first_part_and_the_title_is_the_top_window_s()
    {
        using var dialog = App();
        dialog.Stack.Push(new PickFriendViewModel());
        Shot.Of(dialog.Screen, 560, 200).Dispose();
        var first = Assert.IsType<ButtonPart>(DesktopWindow.FirstFocus(dialog));
        Assert.Equal("cancelButton", first.Spec.Name);
        Assert.Equal("Pick a friend", DesktopWindow.TitleOf(dialog));

        using var app = App();
        var lender = new LenderViewModel();
        app.Stack.Push(lender);
        string? title = null;
        app.Stack.TitleChanged += () => title = DesktopWindow.TitleOf(app);
        lender.OpenHere(new PlainViewModel());
        Assert.Equal("Plain", title);
    }

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

    [Fact]
    public void A_window_shows_the_application_s_icon_or_Clarity_s()
    {
        // This assembly carries no icon.png: Clarity's own, at every size the system picks from.
        var icon = WindowSystem.Icon(typeof(CheckFixHostsTests).Assembly);
        Assert.Equal([16, 32, 48, 64, 128], icon.Select(i => i.Width));
        Assert.All(icon, i => Assert.Equal(i.Width, i.Height));
        Assert.All(icon, i => Assert.Equal(i.Width * i.Height * 4, i.Pixels.Length));
    }

    // -- Sound -----------------------------------------------------------------------------------------

    sealed class Held : IDisposable
    {
        public int Released;
        public void Dispose() => Released++;
    }

    [Fact]
    public void A_sound_that_ended_lets_go_on_the_screen_s_thread_and_one_that_repeats_goes_on()
    {
        var posted = new List<Action>();
        var once = new Held();
        SoundFlowOutput.Ended(repeats: false, once, posted.Add);
        // Not on the audio thread that said it ended: posted to the screen's.
        Assert.Equal(0, once.Released);
        Assert.Single(posted)();
        Assert.Equal(1, once.Released);
        var looping = new Held();
        posted.Clear();
        SoundFlowOutput.Ended(repeats: true, looping, posted.Add);
        Assert.Empty(posted);
        Assert.Equal(0, looping.Released);
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

    [Fact]
    public void The_databases_connected_while_unconnected_close_as_the_application_ends()
    {
        foreach (var fails in new[] { false, true })
        {
            var setting = new Setting();
            var memory = new List<Db>();
            var flow = new Unconnected
            {
                Connect = Db.Empty,
                Empty = () =>
                {
                    var db = Db.Empty();
                    memory.Add(db);
                    return db;
                },
                Main = db => new Card(db, "Main"),
                Window = db => new Card(db, "Preferences"),
                Settings = setting,
            };
            var desk = new Desk();
            void Loop()
            {
                // The person keeps the connection and closes the window; the main window works on it until the app ends.
                setting.Change();
                desk.Close!();
                Assert.False(desk.Connected!.Disposed);
                if (fails)
                    throw new InvalidOperationException("the loop failed");
            }
            try
            {
                DesktopWindow.RunUnconnected(flow, Text.Plain("not kept"), desk, Loop, () => desk.Connected);
            }
            catch (InvalidOperationException) when (fails)
            {
            }
            Assert.True(desk.Connected!.Disposed);
            // The memory database the windows worked on meanwhile went with its window.
            Assert.True(Assert.Single(memory).Disposed);
        }
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
