// A screen of Clarity.UI on a browser page's canvas (DESIGN 7.9): the same
// program as on the desktop, compiled to WebAssembly. Skia draws the screen
// into pixels when it changed, on the page's next frame, at the page's pixel
// scale, and on every frame while something animates; clarity-ui.js puts
// them on the canvas and sends what the person does: the pointer (clicks
// counted into double and triple ones, a finger's taps, swipes and drags),
// the wheel, keys going down and up, and text from a hidden text field (so
// AltGr, Option, dead keys, input methods and a phone's keyboard type what
// they make), to the same screen methods the desktop sends them to (PageInput).
// Copying and pasting go through the browser's clipboard, the pointer takes
// the shape of what is under it, and frames and the parts' clock go on in a
// hidden tab.
// A generated application runs its windows here as on the desktop, through
// the same stack (AppScreen). A program served to the browser runs its
// windows on the server instead and draws mirrors of them (MirrorStack),
// from the markup the server sends. A page opens no window of its own, so dialogs
// and the keys window show over the window, and the theme follows the
// page's prefers-color-scheme. The page is the program's BrowserPage: its
// title, downloads, files shown in a tab, the browser's storage and sound
// through Web Audio; its file input opens files (PagePicker); and the
// parts' clock (timers, playing sounds) ticks on the page's frames.

using System.Collections.Concurrent;
using System.Runtime.InteropServices.JavaScript;
using System.Runtime.Versioning;
using Clarity.Runtime;
using Clarity.Ui;
using SkiaSharp;

namespace Clarity.Ui.Browser;

[SupportedOSPlatform("browser")]
public static partial class BrowserCanvas
{
    const string Module = "clarity-ui";
    static Screen? screen;
    static PageInput? input;
    static AppScreen? app;
    static SKBitmap? pixels;
    static bool wanted;
    static readonly ConcurrentQueue<Action> later = new();
    static readonly FrameTicks ticks = new();
    static bool ticking;

    /// <summary>Shows <paramref name="shown"/> on the canvas <paramref name="canvas"/>, or on the whole page when there is none; it goes on while the page is open.</summary>
    public static async Task Run(Screen shown, string canvas = "clarity")
    {
        screen = shown;
        // The pointer's clicks counted, text from the page's field, and the browser's clipboard.
        input = new PageInput(shown, CopyText);
        // The pointer's shape is the page's cursor, as the element under it says.
        shown.CursorChanged += () =>
        {
            if (imported)
                SetCursor(PageCursor.Css(shown.Cursor));
        };
        shown.Invalidated += Want;
        shown.WakesChanged += WaitForWake;
        await Import();
        Prepare();
        await Start(canvas);
        Want();
        WaitForWake();
        await Task.Delay(Timeout.Infinite);
    }

    /// <summary>
    /// Shows a generated application from its main window on the canvas
    /// <paramref name="canvas"/> while the page is open: each window drawn
    /// from its markup over its view model, in the theme shown as the page's
    /// light or dark resolves it.
    /// </summary>
    public static async Task Run(WindowViewModel main, string canvas = "clarity")
    {
        Prepare();
        await Import();
        var assembly = main.GetType().Assembly;
        // A page has no system fonts: the fallback fonts the application carries draw what Inter lacks.
        Fonts.CarryFallbacks(assembly);
        var shown = new AppScreen(AppScreen.CarriedBy(assembly), PrefersDark(), OpenAddress, appFrame: AppScreen.AppFrameOf(assembly), files: files);
        app = shown;
        // What runs once the work at hand is done runs on the next frame.
        Later.Poster = action =>
        {
            later.Enqueue(action);
            Want();
        };
        DesktopProgram.Current = new PageProgram(shown);
        shown.Stack.TitleChanged += () => SetTitle(shown.Stack.Current?.Title ?? "");
        shown.Stack.Push(main);
        SetTitle(main.Title);
        await Run(shown.Screen, canvas);
    }

    static MirrorStack? mirror;
    static bool mirrorDark;

    /// <summary>
    /// Shows the windows of the server the page came from on the canvas
    /// <paramref name="canvas"/> while the page is open (DESIGN 7.6): their
    /// handlers run there, in a session of this tab, and each is drawn here
    /// from the markup the server sends, over a mirror of its state.
    /// <paramref name="app"/> is the application the program carries, whose
    /// view models the mirrors are; the person's <paramref name="settings"/>,
    /// which the browser keeps, go with each request.
    /// </summary>
    public static async Task Serve(System.Reflection.Assembly app, (Func<System.Text.Json.Nodes.JsonObject> Of, Action<System.Text.Json.Nodes.JsonObject> Apply)? settings = null, string canvas = "clarity")
    {
        Fonts.CarryFallbacks(app);
        Prepare();
        await Import();
        Later.Poster = action =>
        {
            later.Enqueue(action);
            Want();
        };
        var http = new HttpClient { BaseAddress = new Uri(Origin()) };
        var shown = await MirrorStack.Start(new HttpSessionLink(http), settings, AppScreen.CarriedBy(app), OpenAddress, files);
        mirror = shown;
        mirrorDark = PrefersDark();
        var served = new Screen(shown, AppLook.Of(shown.Carried, Themes.Current, mirrorDark)) { Keys = shown.KeyPressed, KeysUp = shown.KeyReleased };
        Themes.Changed += () => served.Show(AppLook.Of(shown.Carried, Themes.Current, mirrorDark));
        shown.TitleChanged += () => SetTitle(shown.Current?.Title ?? "");
        SetTitle(shown.Current?.Title ?? "");
        await Run(served, canvas);
    }

    /// <summary>An address a link or a handler opens: a web page in a new tab, mail and phone in the page; nothing else.</summary>
    static void OpenAddress(string address)
    {
        switch (BrowserAddress.How(address))
        {
            case BrowserAddress.Way.Tab:
                OpenTab(address);
                break;
            case BrowserAddress.Way.Page:
                OpenHere(address);
                break;
            default:
                Console.Error.WriteLine($"a page opens no address {address}");
                break;
        }
    }

    /// <summary>What a generated application asks of the page: the keys window over the window; a page does not quit.</summary>
    sealed class PageProgram(AppScreen shown) : IDesktop
    {
        public void Quit() => Console.Error.WriteLine("a page does not quit; closing its tab ends it");

        public void ShowKeys() => shown.ShowKeys();
    }

    static bool prepared;

    /// <summary>The page's file input and downloads, which the parts of its windows use.</summary>
    static IFilePicker? files;

    /// <summary>
    /// What the page gives the program: itself as the BrowserPage (sound
    /// included), its file input, and the clock of its frames. Run does it; a
    /// program that reads what the browser keeps before it runs (its language,
    /// its theme) calls it first.
    /// </summary>
    public static void Prepare()
    {
        if (prepared)
            return;
        prepared = true;
        var page = new Page();
        // The page's storage keeps the person's settings, so they last across reloads.
        PageServices.Use(page);
        SoundPart.Output = new PageSoundOutput(page);
        files = new PagePicker(page, OpenFile);
        Ticks.Source = ticks;
        ticks.Started += WantTick;
    }

    /// <summary>
    /// Prepares the page and brings in its script, so what the browser keeps
    /// can be read: a program that reads its language, theme or settings
    /// before its windows show awaits it first.
    /// </summary>
    public static async Task Ready()
    {
        Prepare();
        await Import();
    }

    /// <summary>The page's own: its title, files, storage and audio, through clarity-ui.js.</summary>
    sealed class Page : IBrowserPage
    {
        public void SetTitle(string title) => BrowserCanvas.SetTitle(title);

        public void Download(string name, byte[] data) => BrowserCanvas.Download(name, data);

        public void Show(string name, byte[] data) => BrowserCanvas.ShowFile(PagePicker.TypeOf(name), data);

        public string? Read(string key) => BrowserCanvas.Read(key);

        public void Write(string key, string value) => BrowserCanvas.Write(key, value);

        public int Play(byte[] file, double volume, bool repeat) => BrowserCanvas.Play(file, volume, repeat);

        public void Stop(int sound) => BrowserCanvas.StopSound(sound);
    }

    /// <summary>The file the person chose in the page's file input, its bytes brought in; none when they chose none.</summary>
    static async Task<ChosenFile?> OpenFile(string accept)
    {
        var name = await PickFile(accept);
        return name is null ? null : new ChosenFile(name, PickedBytes());
    }

    static void WantTick()
    {
        if (ticking)
            return;
        ticking = true;
        RequestTick();
    }

    static bool imported;

    /// <summary>The page's script, once.</summary>
    static async Task Import()
    {
        if (imported)
            return;
        await JSHost.ImportAsync(Module, ModuleUrl());
        imported = true;
    }

    // The page's script, carried in the package: no file of its own to serve.
    static string ModuleUrl()
    {
        using var stream = typeof(BrowserCanvas).Assembly.GetManifestResourceStream("Clarity.Ui.Browser.clarity-ui.js")!;
        using var bytes = new MemoryStream();
        stream.CopyTo(bytes);
        return "data:text/javascript;base64," + Convert.ToBase64String(bytes.ToArray());
    }

    static void Want()
    {
        if (wanted)
            return;
        wanted = true;
        RequestFrame();
    }

    [JSImport("start", Module)]
    private static partial Task Start(string canvas);

    [JSImport("requestFrame", Module)]
    private static partial void RequestFrame();

    [JSImport("origin", Module)]
    private static partial string Origin();

    [JSImport("prefersDark", Module)]
    private static partial bool PrefersDark();

    [JSImport("openTab", Module)]
    private static partial void OpenTab(string address);

    [JSImport("openHere", Module)]
    private static partial void OpenHere(string address);

    [JSImport("setTitle", Module)]
    private static partial void SetTitle(string title);

    [JSImport("wakeIn", Module)]
    private static partial void WakeIn(double ms);

    [JSImport("requestTick", Module)]
    private static partial void RequestTick();

    [JSImport("download", Module)]
    private static partial void Download(string name, [JSMarshalAs<JSType.MemoryView>] Span<byte> bytes);

    [JSImport("show", Module)]
    private static partial void ShowFile(string type, [JSMarshalAs<JSType.MemoryView>] Span<byte> bytes);

    [JSImport("read", Module)]
    private static partial string? Read(string key);

    [JSImport("write", Module)]
    private static partial void Write(string key, string value);

    [JSImport("play", Module)]
    private static partial int Play([JSMarshalAs<JSType.MemoryView>] Span<byte> bytes, double volume, bool repeat);

    [JSImport("stop", Module)]
    private static partial void StopSound(int sound);

    [JSImport("openFile", Module)]
    private static partial Task<string?> PickFile(string accept);

    [JSImport("pickedBytes", Module)]
    [return: JSMarshalAs<JSType.Array<JSType.Number>>]
    private static partial byte[] PickedBytes();

    [JSImport("present", Module)]
    private static partial void Present([JSMarshalAs<JSType.MemoryView>] Span<byte> pixels, int width, int height);

    [JSExport]
    internal static void Resized(double width, double height, double scale) => screen?.Resize(width, height, (float)scale);

    /// <summary>The page's light or dark changed: the theme shows its other mode.</summary>
    [JSExport]
    internal static void SystemDarkChanged(bool dark)
    {
        if (app is not null)
            app.SystemDark = dark;
        if (mirror is { } shown && screen is { } served && dark != mirrorDark)
        {
            mirrorDark = dark;
            served.Show(AppLook.Of(shown.Carried, Themes.Current, dark));
        }
    }

    /// <summary>Time went on (four times a second): info messages go when their time is up.</summary>
    [JSExport]
    internal static void Tick()
    {
        app?.Stack.Tick();
        mirror?.Tick();
    }

    /// <summary>The page waits for the screen's next wake; none when none waits.</summary>
    static void WaitForWake()
    {
        if (!imported || screen is null)
            return;
        WakeIn(screen.NextWake is { } at ? Math.Max(0, (at - screen.Now()).TotalMilliseconds) : -1);
    }

    /// <summary>The time of a wake came: the elements whose time came wake, and the page waits for the next.</summary>
    [JSExport]
    internal static void WakeNow()
    {
        screen?.WakeDue(screen.Now());
        WaitForWake();
    }

    /// <summary>A frame's time came (milliseconds since the page opened): the parts' clocks that are due run; the next frame is asked for while one runs.</summary>
    [JSExport]
    internal static void TimeTick(double ms)
    {
        ticking = false;
        ticks.Advance(TimeSpan.FromMilliseconds(ms));
        if (ticks.Running)
            WantTick();
    }

    [JSExport]
    internal static unsafe void Frame()
    {
        wanted = false;
        while (later.TryDequeue(out var action))
            action();
        if (screen is null)
            return;
        var width = Math.Max(1, (int)Math.Round(screen.Width * screen.Scale));
        var height = Math.Max(1, (int)Math.Round(screen.Height * screen.Scale));
        if (pixels is null || pixels.Width != width || pixels.Height != height)
        {
            pixels?.Dispose();
            pixels = new SKBitmap(new SKImageInfo(width, height, SKColorType.Rgba8888, SKAlphaType.Premul));
        }
        using (var canvas = new SKCanvas(pixels))
            screen.Draw(canvas);
        Present(new Span<byte>((void*)pixels.GetPixels(), pixels.ByteCount), width, height);
        // Something animates (a busy indicator): the next frame is drawn too.
        if (screen.Animating > 0)
            Want();
    }

    [JSExport]
    internal static void PointerMoved(double x, double y) => screen?.PointerMoved(x, y);

    [JSExport]
    internal static void PointerLeft() => screen?.PointerLeft();

    /// <summary>A press, at the page's time in milliseconds: a double or triple click is counted from the presses.</summary>
    [JSExport]
    internal static void PointerPressed(double x, double y, int button, double ms)
    {
        input?.Pressed(x, y, (PointerButton)button, (long)ms);
        ShowTextField();
    }

    /// <summary>A finger went down, moved and lifted: a tap clicks, a swipe pans, a rest then a move drags.</summary>
    [JSExport]
    internal static void TouchDown(double x, double y, double ms) => input?.TouchDown(x, y, (long)ms);

    [JSExport]
    internal static void TouchMoved(double x, double y, double ms) => input?.TouchMoved(x, y, (long)ms);

    [JSExport]
    internal static void TouchCancelled() => input?.TouchCancelled();

    [JSExport]
    internal static void TouchUp(double x, double y, double ms)
    {
        input?.TouchUp(x, y, (long)ms);
        ShowTextField();
    }

    [JSExport]
    internal static void PointerReleased(double x, double y, int button) => screen?.PointerReleased(x, y, (PointerButton)button);

    /// <summary>The wheel turned: a wheel event's deltas as the browser gave them, down and right positive, in its deltaMode, with the keys held; a notch is 50 of the screen's units (PageInput.WheelUnits).</summary>
    [JSExport]
    internal static void Wheel(double x, double y, double dx, double dy, int mode, double pageHeight, int modifiers) =>
        input?.Wheel(x, y, dx, dy, mode, pageHeight, (Modifiers)modifiers);

    /// <summary>The browser took the pointer away mid-press (pointercancel, the page lost the focus): a drag is dropped.</summary>
    [JSExport]
    internal static void PointerCancelled() => input?.PointerCancelled();

    /// <summary>A key went down: true when the page keeps the browser from doing anything with it.</summary>
    [JSExport]
    internal static bool KeyPressed(string key, int modifiers, bool altGraph, bool mac)
    {
        if (input is null)
            return true;
        var way = input.KeyDown(key, (Modifiers)modifiers, altGraph, mac);
        ShowTextField();
        return way == PageKeyWay.Prevent;
    }

    [JSExport]
    internal static void KeyReleased(string key, int modifiers, bool altGraph) => input?.KeyUp(key, (Modifiers)modifiers, altGraph);

    /// <summary>Text the page's field took: a key's character, AltGr's, a dead key's, an input method's.</summary>
    [JSExport]
    internal static void TextTyped(string text)
    {
        input?.Typed(text);
        ShowTextField();
    }

    /// <summary>The browser pasted text: the screen pastes it.</summary>
    [JSExport]
    internal static void Pasted(string text)
    {
        input?.Pasted(text);
        ShowTextField();
    }

    /// <summary>The page's text field goes where the focus is, so an input method shows its words there; a phone shows its keyboard only for a part that takes text.</summary>
    static void ShowTextField()
    {
        if (input is null)
            return;
        var place = input.FocusedPlace ?? new Box(0, 0, 0, 0);
        TextFieldAt(place.X, place.Y, place.Height, input.TakesText);
    }

    [JSImport("textFieldAt", Module)]
    private static partial void TextFieldAt(double x, double y, double height, bool takesText);

    [JSImport("setCursor", Module)]
    private static partial void SetCursor(string cursor);

    [JSImport("copyText", Module)]
    private static partial void CopyText(string text);
}

/// <summary>A browser's name of a key (KeyboardEvent.key), as Clarity.UI's.</summary>
public static class WebKeys
{
    public static Key Of(string key) => key switch
    {
        "Tab" => Key.Tab,
        "Enter" => Key.Enter,
        "Escape" => Key.Escape,
        " " => Key.Space,
        "Backspace" => Key.Backspace,
        "Delete" => Key.Delete,
        "Insert" => Key.Insert,
        "ArrowLeft" => Key.Left,
        "ArrowRight" => Key.Right,
        "ArrowUp" => Key.Up,
        "ArrowDown" => Key.Down,
        "Home" => Key.Home,
        "End" => Key.End,
        "PageUp" => Key.PageUp,
        "PageDown" => Key.PageDown,
        "Shift" => Key.Shift,
        "Control" => Key.Control,
        "Alt" => Key.Alt,
        ['F', .. var n] when int.TryParse(n, out var f) && f is >= 1 and <= 12 => Key.F1 + (f - 1),
        [var c] when char.IsAsciiLetter(c) => Key.A + (char.ToUpperInvariant(c) - 'A'),
        [var c] when char.IsAsciiDigit(c) => Key.D0 + (c - '0'),
        // A symbol is its character, from the keyboard or the number pad.
        [var c] when Shortcuts.Symbols.TryGetValue(c, out var symbol) => symbol,
        _ => Key.Other,
    };

    /// <summary>
    /// The browser's own name of a key Clarity.UI has no Key for (F13,
    /// Pause, PrintScreen, ContextMenu), so the person can make it a
    /// command's key; none for the keys it has, a typed character, and keys
    /// that only change others (Meta, Caps Lock, AltGraph) or make text.
    /// </summary>
    public static string? NameOf(string key) =>
        Of(key) == Key.Other && key.Length > 1 && key.All(char.IsAsciiLetterOrDigit)
            && key is not ("Meta" or "OS" or "Super" or "Hyper" or "Fn" or "FnLock" or "CapsLock" or "NumLock" or "Symbol" or "SymbolLock" or "AltGraph" or "Dead" or "Process" or "Unidentified" or "Compose")
            ? key
            : null;
}
