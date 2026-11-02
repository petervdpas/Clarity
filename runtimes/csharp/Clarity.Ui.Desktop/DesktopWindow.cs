// Clarity.UI on the desktop (DESIGN 7.9): each screen in a window of the
// system's own, drawn by Skia through OpenGL when it changed, at the window's
// own scale; what the person does with the mouse, the wheel and keys goes to
// its screen. A generated application runs its windows here, from the markup
// and themes its assembly carries: windows open in place in their desktop
// window, while a dialog, the keys window and a target marked `in new window`
// get a desktop window of their own, each its own screen and GL surface. A
// dialog's window is sized to the dialog, and the window under it takes no
// input while it is open. The theme follows the system's light or dark, at
// start and when it changes. One loop runs every window: what the person did
// (keys going down and up, the wheel with the keys held) is queued as it
// comes and done between frames, so a handler never runs
// inside the system's callbacks, and a long engine call keeps every window
// drawing (LongCall.Waiter) with its bar and Stop. The loop is the parts'
// clock too: timers and playing sounds tick from it (FrameTicks), sounds play
// through SoundFlow, and the file dialogs are the system's own.

using System.Collections.Concurrent;
using System.Diagnostics;
using System.Reflection;
using Clarity.Runtime;
using Clarity.Ui;
using Silk.NET.Input;
using Silk.NET.Maths;
using Silk.NET.Windowing;
using SkiaSharp;
using SilkKey = Silk.NET.Input.Key;

namespace Clarity.Ui.Desktop;

public static class DesktopWindow
{
    // OpenGL's RGBA8, the window's own framebuffer's format.
    const uint Rgba8 = 0x8058;

    // What a grown window leaves of the screen's height, for the system's own bars.
    const double ScreenMargin = 64;

    /// <summary>The size a window that names none opens at, on every stack (clarity_spec::layout).</summary>
    public static readonly Extent StandardSize = new(1100, 760);

    /// <summary>
    /// The size of a desktop window at <paramref name="now"/> when a window
    /// that asks for <paramref name="wanted"/> opens in it: as large as it
    /// asks, never smaller than it is, and no larger than <paramref name="room"/>
    /// (the screen's) where that is known.
    /// </summary>
    public static Extent Grown(Extent now, Extent? wanted, Extent? room)
    {
        if (wanted is not { } want)
            return now;
        var width = Math.Max(now.Width, want.Width);
        var height = Math.Max(now.Height, want.Height);
        if (room is { } r)
            (width, height) = (Math.Max(now.Width, Math.Min(width, r.Width)), Math.Max(now.Height, Math.Min(height, r.Height)));
        return new Extent(width, height);
    }

    /// <summary>
    /// The size of a desktop window <paramref name="now"/> when its sidebars
    /// went from <paramref name="was"/> wide to <paramref name="width"/>: a
    /// sidebar that opens grows it by the room it takes, so the window's own
    /// parts keep theirs, as far as the screen has <paramref name="room"/>;
    /// one that folds leaves it as it is.
    /// </summary>
    public static Extent SidebarGrown(Extent now, double was, double width, Extent? room) =>
        width <= was ? now : Grown(now, now with { Width = now.Width + width - was }, room);

    /// <summary>
    /// Shows a generated application from its main window until the person
    /// closes it: each window drawn from its markup (<c>Windows/Main.cxaml</c>,
    /// carried in the application's assembly) over its view model, in the
    /// theme shown laid over Clarity's own (<c>Themes/*.json</c>).
    /// </summary>
    public static void Run(WindowViewModel main)
    {
        var desk = new Desk(main.GetType().Assembly);
        desk.OpenApp(main, [], null);
        desk.Loop();
    }

    /// <summary>
    /// The databases did not open (DESIGN 5.27): says why, then shows what
    /// keeps the connection, or what the app's <c>on unconnected</c> opens;
    /// once the databases open the main window shows on them, until the person
    /// closes it.
    /// </summary>
    public static void Run(Unconnected unconnected, Text problem)
    {
        Console.Error.WriteLine(Locale.Show(problem));
        var app = Assembly.GetEntryAssembly() ?? typeof(DesktopWindow).Assembly;
        var desk = new Desk(app);
        unconnected.Start(problem, desk);
        try
        {
            desk.Loop();
        }
        finally
        {
            desk.Connected?.Dispose();
        }
    }

    /// <summary>Shows <paramref name="screen"/> in a window until the person closes it.</summary>
    public static void Run(Screen screen, string title, int width, int height)
    {
        var desk = new Desk(typeof(DesktopWindow).Assembly);
        desk.Open(screen, title, new Extent(width, height), null, modal: false, resizable: true);
        desk.Loop();
    }

    /// <summary>
    /// What the desktop gives the parts before its first window: sound through
    /// SoundFlow (a device opened when a sound first plays), the clock of its
    /// frame loop. The clock is the loop's to advance; the system's file
    /// dialogs (<see cref="Files"/>) go to every window it opens.
    /// </summary>
    internal static FrameTicks Prepare()
    {
        SoundPart.Output = new SoundFlowOutput();
        var ticks = new FrameTicks();
        Ticks.Source = ticks;
        return ticks;
    }

    /// <summary>The system's file dialogs, which the parts of every desktop window use.</summary>
    internal static readonly IFilePicker Files = new SystemFilePicker();

    /// <summary>A desktop window: its system window, its screen and GL surface, and what the person did on it, waiting to be done.</summary>
    sealed class Shown(IWindow window, Screen screen)
    {
        public IWindow Window { get; } = window;
        public Screen Screen { get; } = screen;
        public AppScreen? App { get; set; }
        public Shown? Owner { get; set; }
        public bool Modal { get; set; }
        public List<Shown> Children { get; } = [];
        public Queue<Action> Input { get; } = new();
        public GRContext? Skia { get; set; }
        public SKSurface? Surface { get; set; }
        public IInputContext? Inputs { get; set; }
        public bool Changed { get; set; } = true;

        /// <summary>It closes by what it shows (its last window closed): it hears no on close again.</summary>
        public bool Ending { get; set; }

        /// <summary>The person or the application closes it: what it shows hears its on close.</summary>
        public Action? Closing { get; set; }

        /// <summary>It is gone.</summary>
        public Action? Gone { get; set; }

        /// <summary>The window view whose sidebars were last seen, and how wide they were: one that opens grows the desktop window.</summary>
        public WindowView? SidebarsOf { get; set; }
        public double Sidebars { get; set; }

        /// <summary>A dialog of its own is open over it: it takes no input.</summary>
        public bool Blocked => Children.Any(c => c.Modal);
    }

    /// <summary>The application's desktop windows, and the loop that runs them.</summary>
    sealed class Desk : IUnconnectedScreen, IDesktop
    {
        readonly Func<string, string?> carried;
        readonly IReadOnlyList<System.Xml.Linq.XElement>? appFrame;
        readonly List<Shown> windows = [];
        readonly ConcurrentQueue<Action> later = new();
        readonly FrameTicks ticks;
        readonly long started = Stopwatch.GetTimestamp();
        bool systemDark;
        Shown? active;

        public Desk(Assembly app)
        {
            carried = AppScreen.CarriedBy(app);
            appFrame = AppScreen.AppFrameOf(app);
            systemDark = SystemTheme.Here.Dark();
            ticks = Prepare();
            // A tick that fails says why in the window the person works in.
            ticks.Failed = e =>
            {
                Console.Error.WriteLine($"Unhandled: {e}");
                active?.App?.Stack.Messages.Add(new Message(Level.Error, Text.Plain(e.Message)));
            };
            // What runs once the work at hand is done (a change handler joining its edit) runs between frames.
            Later.Poster = later.Enqueue;
            DesktopProgram.Current = this;
            // A long engine call: every window goes on drawing until it ended.
            LongCall.Waiter = call =>
            {
                while (!call.IsCompleted && windows.Count > 0)
                    Pump();
                // Only waits: the engine reads the call's refusal or failure itself.
                ((IAsyncResult)call).AsyncWaitHandle.WaitOne();
            };
            Themes.Changed += Relook;
        }

        /// <summary>The databases opened after the application started unconnected: closed as it ends.</summary>
        public Db? Connected { get; private set; }

        // -- Opening -----------------------------------------------------------------

        /// <summary>Opens <paramref name="first"/> in a desktop window of the application's own, saying <paramref name="said"/>.</summary>
        public Shown OpenApp(WindowViewModel first, IReadOnlyList<Message> said, Action? gone)
        {
            var shown = OpenStack(first, null, null);
            foreach (var message in said)
                shown.App!.Stack.Messages.Add(message);
            shown.Gone = () =>
            {
                gone?.Invoke();
                // The application's last own window closed: the application ends.
                if (!windows.Any(w => w.Owner is null))
                    Quit();
            };
            return shown;
        }

        /// <summary>A dialog or a target marked <c>in new window</c>: a desktop window of its own over the one it was opened from.</summary>
        void Apart(WindowViewModel other, WindowViewModel opener)
        {
            var owner = windows.LastOrDefault(w => w.App?.Stack.Windows.Contains(opener) == true) ?? windows.FirstOrDefault();
            OpenStack(other, owner, opener);
        }

        Shown OpenStack(WindowViewModel first, Shown? owner, WindowViewModel? opener)
        {
            var screen = new AppScreen(carried, systemDark, SystemAddress.Open, Apart, opener, appFrame, Files);
            screen.Stack.Push(first);
            var dialog = owner is not null && first.IsDialog;
            var size = dialog && screen.Stack.View is { } view ? WindowStack.DialogSize(view, first) : first.WindowSize ?? StandardSize;
            var shown = Open(screen.Screen, first.Title, size, owner, modal: dialog, resizable: !dialog);
            shown.App = screen;
            screen.Stack.TitleChanged += () =>
            {
                if (screen.Stack.Current is not { } top)
                    return;
                shown.Window.Title = top.Title;
                // A window that names its size grows the desktop window to it, as far as the screen has room.
                if (!dialog && top.WindowSize is { } wanted)
                {
                    var now = new Extent(shown.Window.Size.X, shown.Window.Size.Y);
                    var room = shown.Window.Monitor?.VideoMode.Resolution is { } r ? new Extent(r.X, r.Y - ScreenMargin) : (Extent?)null;
                    var grown = Grown(now, wanted, room);
                    if (grown != now)
                        shown.Window.Size = new Vector2D<int>((int)grown.Width, (int)grown.Height);
                }
            };
            screen.Stack.Ended += () =>
            {
                shown.Ending = true;
                Close(shown);
            };
            shown.Closing = screen.Stack.Closing;
            return shown;
        }

        /// <summary>Shows <paramref name="screen"/> in a desktop window of its own; over <paramref name="owner"/>, in its middle.</summary>
        public Shown Open(Screen screen, string title, Extent size, Shown? owner, bool modal, bool resizable)
        {
            var width = (int)Math.Max(120, size.Width);
            var height = (int)Math.Max(80, size.Height);
            var options = WindowOptions.Default with
            {
                Title = title,
                Size = new Vector2D<int>(width, height),
                API = new GraphicsAPI(ContextAPI.OpenGL, ContextProfile.Core, ContextFlags.ForwardCompatible, new APIVersion(3, 3)),
                PreferredStencilBufferBits = 8,
                // Every window swaps in the one loop, which keeps its own pace.
                VSync = false,
                ShouldSwapAutomatically = false,
                WindowBorder = resizable ? WindowBorder.Resizable : WindowBorder.Fixed,
            };
            if (owner is not null)
                options = options with { Position = owner.Window.Position + (owner.Window.Size - new Vector2D<int>(width, height)) / 2 };
            var window = Window.Create(options);
            var shown = new Shown(window, screen) { Owner = owner, Modal = modal };
            screen.Invalidated += () => shown.Changed = true;
            window.Load += () => Loaded(shown);
            window.FramebufferResize += _ => Fit(shown);
            window.FocusChanged += focused =>
            {
                if (focused)
                    active = shown;
            };
            window.Initialize();
            windows.Add(shown);
            owner?.Children.Add(shown);
            active = shown;
            return shown;
        }

        void Loaded(Shown shown)
        {
            var (window, screen) = (shown.Window, shown.Screen);
            // Skia finds OpenGL's functions itself (GLX or EGL, WGL, CGL) in the window's current context.
            window.MakeCurrent();
            shown.Skia = GRContext.CreateGl(GRGlInterface.Create());
            Fit(shown);
            var input = window.CreateInput();
            shown.Inputs = input;
            var queue = shown.Input;
            var keyboards = input.Keyboards;
            // The keys held as the wheel turns: Ctrl with the wheel zooms where a part does.
            Modifiers Held() => keyboards.FirstOrDefault() is { } k ? ModifiersOf(k) : Modifiers.None;
            foreach (var mouse in input.Mice)
            {
                mouse.MouseMove += (_, at) => queue.Enqueue(() => screen.PointerMoved(at.X, at.Y));
                // The window gives only presses: a double or triple click is counted here.
                var presses = new PressCounter();
                mouse.MouseDown += (m, button) =>
                {
                    var at = m.Position;
                    var clicks = presses.Press(at.X, at.Y, Button(button), Environment.TickCount64);
                    queue.Enqueue(() => screen.PointerPressed(at.X, at.Y, Button(button), clicks));
                };
                mouse.MouseUp += (m, button) =>
                {
                    var at = m.Position;
                    queue.Enqueue(() => screen.PointerReleased(at.X, at.Y, Button(button)));
                };
                // A turn of the wheel scrolls three lines of 40; up is toward the top.
                mouse.Scroll += (m, wheel) =>
                {
                    var at = m.Position;
                    var held = Held();
                    queue.Enqueue(() => screen.Wheel(at.X, at.Y, -wheel.X * 40, -wheel.Y * 40, held));
                };
            }
            foreach (var keyboard in input.Keyboards)
            {
                keyboard.KeyDown += (k, key, _) =>
                {
                    var held = ModifiersOf(k);
                    var pressed = new KeyInput(KeyOf(key, held.HasFlag(Modifiers.Shift)), held);
                    queue.Enqueue(() => screen.KeyPressed(pressed));
                };
                keyboard.KeyUp += (k, key, _) =>
                {
                    var held = ModifiersOf(k);
                    var released = new KeyInput(KeyOf(key, held.HasFlag(Modifiers.Shift)), held);
                    queue.Enqueue(() => screen.KeyReleased(released));
                };
                keyboard.KeyChar += (_, typed) =>
                {
                    if (!char.IsControl(typed))
                        queue.Enqueue(() => screen.TextTyped(typed.ToString()));
                };
            }
            // Copying and pasting go through the system's clipboard.
            if (input.Keyboards.FirstOrDefault() is { } first)
                screen.Clipboard = new SystemClipboard(first);
        }

        void Fit(Shown shown)
        {
            if (shown.Skia is null)
                return;
            shown.Window.MakeCurrent();
            shown.Surface?.Dispose();
            var pixels = shown.Window.FramebufferSize;
            var target = new GRBackendRenderTarget(Math.Max(1, pixels.X), Math.Max(1, pixels.Y), 0, 8, new GRGlFramebufferInfo(0, Rgba8));
            shown.Surface = SKSurface.Create(shown.Skia, target, GRSurfaceOrigin.BottomLeft, SKColorType.Rgba8888);
            var size = shown.Window.Size;
            var scale = size.X > 0 ? (float)pixels.X / size.X : 1;
            shown.Screen.Resize(size.X, size.Y, scale);
            shown.Changed = true;
        }

        // -- Running -------------------------------------------------------------------

        /// <summary>Runs every window until the last one closed.</summary>
        public void Loop()
        {
            using var watch = SystemTheme.Here.Watch(systemDark, TimeSpan.FromSeconds(3), dark => later.Enqueue(() => SetDark(dark)));
            try
            {
                while (windows.Count > 0)
                    Pump();
            }
            finally
            {
                Themes.Changed -= Relook;
                LongCall.Waiter = null;
                if (DesktopProgram.Current == this)
                    DesktopProgram.Current = null;
                // The sound device goes with the last window.
                if (SoundPart.Output is SoundFlowOutput sound)
                    sound.Dispose();
            }
        }

        /// <summary>One frame of every window: what the system says, what the person did, time, closing, drawing.</summary>
        void Pump()
        {
            var started = Stopwatch.GetTimestamp();
            foreach (var shown in windows.ToList())
                if (!shown.Window.IsClosing)
                    shown.Window.DoEvents();
            foreach (var shown in windows.ToList())
                while (shown.Input.TryDequeue(out var action))
                {
                    // The window under a dialog of its own takes nothing while it is open.
                    if (!shown.Blocked && windows.Contains(shown))
                        Safely(shown, action);
                }
            while (later.TryDequeue(out var action))
                Safely(active, action);
            foreach (var shown in windows.ToList())
                Widen(shown);
            // The parts' clock: timers and playing sounds whose time has come.
            ticks.Advance(Stopwatch.GetElapsedTime(this.started));
            foreach (var shown in windows.ToList())
            {
                // An element whose time came wakes (a trend moves on, a tooltip shows).
                if (shown.Screen.NextWake is { } wake && wake <= shown.Screen.Now())
                    Safely(shown, () => shown.Screen.WakeDue(shown.Screen.Now()));
                if (!shown.Window.IsClosing)
                    shown.App?.Stack.Tick();
                else if (shown.Blocked)
                    // A window with a dialog of its own open over it stays, as the system's own dialogs keep theirs.
                    shown.Window.IsClosing = false;
                else
                    Close(shown);
            }
            var drew = false;
            foreach (var shown in windows)
                drew |= Draw(shown);
            // About sixty frames a second, and resting while nothing changes.
            var spent = Stopwatch.GetElapsedTime(started);
            var frame = TimeSpan.FromMilliseconds(drew ? 16 : 8);
            if (spent < frame)
                Thread.Sleep(frame - spent);
        }

        /// <summary>A sidebar of the window shown opened: the desktop window grows by the room it takes (a dialog keeps its size).</summary>
        static void Widen(Shown shown)
        {
            if (shown.Modal || shown.Window.IsClosing || shown.App?.Stack.View is not { } view)
                return;
            var width = view.SidebarsWidth;
            if (shown.SidebarsOf == view && width > shown.Sidebars)
            {
                var now = new Extent(shown.Window.Size.X, shown.Window.Size.Y);
                var room = shown.Window.Monitor?.VideoMode.Resolution is { } r ? new Extent(r.X, r.Y - ScreenMargin) : (Extent?)null;
                var grown = SidebarGrown(now, shown.Sidebars, width, room);
                if (grown != now)
                    shown.Window.Size = new Vector2D<int>((int)grown.Width, (int)grown.Height);
            }
            shown.SidebarsOf = view;
            shown.Sidebars = width;
        }

        static bool Draw(Shown shown)
        {
            // While something animates, every frame is drawn.
            if ((!shown.Changed && shown.Screen.Animating == 0) || shown.Surface is null || shown.Skia is null)
                return false;
            shown.Changed = false;
            shown.Window.MakeCurrent();
            shown.Screen.Draw(shown.Surface.Canvas);
            shown.Skia.Flush();
            shown.Window.SwapBuffers();
            return true;
        }

        /// <summary>Fail safe: what goes wrong in what the person did says why in the window, and the application keeps working.</summary>
        static void Safely(Shown? shown, Action action)
        {
            try
            {
                action();
            }
            catch (Exception e) when (e is not OutOfMemoryException)
            {
                Console.Error.WriteLine($"Unhandled: {e}");
                shown?.App?.Stack.Messages.Add(new Message(Level.Error, Text.Plain(e.Message)));
            }
        }

        /// <summary>Closes a desktop window, the windows of its own it opened first; what it shows hears its on close unless it ended itself.</summary>
        void Close(Shown shown)
        {
            if (!windows.Remove(shown))
                return;
            foreach (var child in shown.Children.ToList())
                Close(child);
            if (!shown.Ending)
            {
                shown.Ending = true;
                Safely(shown, () => shown.Closing?.Invoke());
            }
            shown.Owner?.Children.Remove(shown);
            if (active == shown)
                active = shown.Owner ?? windows.LastOrDefault();
            try
            {
                shown.Window.MakeCurrent();
                shown.Surface?.Dispose();
                shown.Skia?.Dispose();
                shown.Inputs?.Dispose();
                shown.Window.Reset();
                shown.Window.Dispose();
            }
            catch (Exception e) when (e is not OutOfMemoryException)
            {
                Console.Error.WriteLine($"a window did not close cleanly: {e.Message}");
            }
            shown.App?.Dispose();
            // What its elements started (timers, sounds, engines) stops as they leave the screen.
            Safely(null, shown.Screen.Dispose);
            shown.Gone?.Invoke();
        }

        // -- The theme -----------------------------------------------------------------

        void SetDark(bool dark)
        {
            systemDark = dark;
            foreach (var shown in windows)
            {
                if (shown.App is { } screen)
                    screen.SystemDark = dark;
                else
                    shown.Screen.Show(Look);
            }
        }

        ThemeLook Look => AppLook.Of(carried, Themes.Current, systemDark);

        /// <summary>The person chose another theme: the windows that are no application's (the keys window) show it too.</summary>
        void Relook()
        {
            foreach (var shown in windows.Where(w => w.App is null))
                shown.Screen.Show(Look);
        }

        // -- What a generated application asks of the desktop ------------------------

        /// <summary>Ends the application: every window closes, hearing its on close.</summary>
        public void Quit()
        {
            foreach (var shown in windows.Where(w => w.Owner is null).ToList())
                Close(shown);
            foreach (var shown in windows.ToList())
                Close(shown);
        }

        /// <summary>Clarity's keys window, in a desktop window of its own over the window the person works in.</summary>
        public void ShowKeys()
        {
            if (windows.Any(w => w.Screen.Root is KeysWindow))
                return;
            var keys = new KeysWindow(KeysWindow.Catalog);
            var screen = new Screen(keys, Look) { Keys = keys.KeyPressed };
            Shown? shown = null;
            keys.Done = () =>
            {
                if (shown is not null)
                    Close(shown);
            };
            shown = Open(screen, KeysWindow.Title, keys.Wanted, active ?? windows.FirstOrDefault(), modal: true, resizable: true);
            screen.Focus(keys);
        }

        // -- The unconnected flow ------------------------------------------------------

        void IUnconnectedScreen.Show(WindowViewModel first, IReadOnlyList<Message> said, Action whenClosed) => OpenApp(first, said, whenClosed);

        void IUnconnectedScreen.Connected(Db db, WindowViewModel main)
        {
            Connected = db;
            OpenApp(main, [], null);
        }

        void IUnconnectedScreen.Ended() => Quit();
    }

    /// <summary>The system's clipboard, through the keyboard's window.</summary>
    sealed class SystemClipboard(IKeyboard keyboard) : IClipboard
    {
        public string? GetText() => keyboard.ClipboardText;

        public void SetText(string text) => keyboard.ClipboardText = text;
    }

    static PointerButton Button(MouseButton button) => button switch
    {
        MouseButton.Right => PointerButton.Right,
        MouseButton.Middle => PointerButton.Middle,
        _ => PointerButton.Left,
    };

    static Modifiers ModifiersOf(IKeyboard k)
    {
        var held = Modifiers.None;
        if (k.IsKeyPressed(SilkKey.ShiftLeft) || k.IsKeyPressed(SilkKey.ShiftRight))
            held |= Modifiers.Shift;
        if (k.IsKeyPressed(SilkKey.ControlLeft) || k.IsKeyPressed(SilkKey.ControlRight))
            held |= Modifiers.Control;
        if (k.IsKeyPressed(SilkKey.AltLeft) || k.IsKeyPressed(SilkKey.AltRight))
            held |= Modifiers.Alt;
        if (k.IsKeyPressed(SilkKey.SuperLeft) || k.IsKeyPressed(SilkKey.SuperRight))
            held |= Modifiers.Meta;
        return held;
    }

    /// <summary>
    /// Clarity.UI's key of a key of the keyboard: the number pad's symbols
    /// are the same keys as the keyboard's, and Shift with = is Plus (as on
    /// most keyboards), so <c>Ctrl++</c> works from either.
    /// </summary>
    internal static Clarity.Ui.Key KeyOf(SilkKey key, bool shift = false) => key switch
    {
        SilkKey.Equal when shift => Clarity.Ui.Key.Plus,
        SilkKey.Equal => Clarity.Ui.Key.Equal,
        SilkKey.KeypadAdd => Clarity.Ui.Key.Plus,
        SilkKey.Minus or SilkKey.KeypadSubtract => Clarity.Ui.Key.Minus,
        SilkKey.KeypadMultiply => Clarity.Ui.Key.Multiply,
        SilkKey.Slash or SilkKey.KeypadDivide => Clarity.Ui.Key.Slash,
        SilkKey.Period or SilkKey.KeypadDecimal => Clarity.Ui.Key.Period,
        SilkKey.Comma => Clarity.Ui.Key.Comma,
        SilkKey.Semicolon => Clarity.Ui.Key.Semicolon,
        SilkKey.Apostrophe => Clarity.Ui.Key.Quote,
        SilkKey.BackSlash => Clarity.Ui.Key.Backslash,
        SilkKey.LeftBracket => Clarity.Ui.Key.BracketLeft,
        SilkKey.RightBracket => Clarity.Ui.Key.BracketRight,
        SilkKey.GraveAccent => Clarity.Ui.Key.Backquote,
        >= SilkKey.Keypad0 and <= SilkKey.Keypad9 => Clarity.Ui.Key.D0 + (key - SilkKey.Keypad0),
        SilkKey.Tab => Clarity.Ui.Key.Tab,
        SilkKey.Enter or SilkKey.KeypadEnter => Clarity.Ui.Key.Enter,
        SilkKey.Escape => Clarity.Ui.Key.Escape,
        SilkKey.Space => Clarity.Ui.Key.Space,
        SilkKey.Backspace => Clarity.Ui.Key.Backspace,
        SilkKey.Delete => Clarity.Ui.Key.Delete,
        SilkKey.Insert => Clarity.Ui.Key.Insert,
        SilkKey.Left => Clarity.Ui.Key.Left,
        SilkKey.Right => Clarity.Ui.Key.Right,
        SilkKey.Up => Clarity.Ui.Key.Up,
        SilkKey.Down => Clarity.Ui.Key.Down,
        SilkKey.Home => Clarity.Ui.Key.Home,
        SilkKey.End => Clarity.Ui.Key.End,
        SilkKey.PageUp => Clarity.Ui.Key.PageUp,
        SilkKey.PageDown => Clarity.Ui.Key.PageDown,
        SilkKey.ShiftLeft or SilkKey.ShiftRight => Clarity.Ui.Key.Shift,
        SilkKey.ControlLeft or SilkKey.ControlRight => Clarity.Ui.Key.Control,
        SilkKey.AltLeft or SilkKey.AltRight => Clarity.Ui.Key.Alt,
        >= SilkKey.F1 and <= SilkKey.F12 => Clarity.Ui.Key.F1 + (key - SilkKey.F1),
        >= SilkKey.A and <= SilkKey.Z => Clarity.Ui.Key.A + (key - SilkKey.A),
        >= SilkKey.Number0 and <= SilkKey.Number9 => Clarity.Ui.Key.D0 + (key - SilkKey.Number0),
        _ => Clarity.Ui.Key.Other,
    };
}
