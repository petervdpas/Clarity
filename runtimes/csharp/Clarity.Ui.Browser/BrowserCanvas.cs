// A screen of Clarity.UI on a browser page's canvas (DESIGN 7.9): the same
// program as on the desktop, compiled to WebAssembly. Skia draws the screen
// into pixels when it changed, on the page's next frame, at the page's pixel
// scale, and on every frame while something animates; clarity-ui.js puts
// them on the canvas and sends what the person does: the pointer, the wheel,
// keys going down and up and typed text, to the same screen methods the desktop sends them to
// (a dialog's keys and the window's own included, which are asked first).
// A generated application runs its windows here as on the desktop, through
// the same stack (AppScreen); a page opens no window of its own, so dialogs
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
        BrowserPage.Current = page;
        SoundPart.Output = new PageSoundOutput(page);
        files = new PagePicker(page, OpenFile);
        Ticks.Source = ticks;
        ticks.Started += WantTick;
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
    }

    /// <summary>Time went on (four times a second): info messages go when their time is up.</summary>
    [JSExport]
    internal static void Tick() => app?.Stack.Tick();

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

    [JSExport]
    internal static void PointerPressed(double x, double y, int button) => screen?.PointerPressed(x, y, (PointerButton)button);

    [JSExport]
    internal static void PointerReleased(double x, double y, int button) => screen?.PointerReleased(x, y, (PointerButton)button);

    /// <summary>The wheel turned, in the page's pixels, down and right positive (a wheel event's deltas), with the keys held.</summary>
    [JSExport]
    internal static void Wheel(double x, double y, double dx, double dy, int modifiers) => screen?.Wheel(x, y, dx, dy, (Modifiers)modifiers);

    [JSExport]
    internal static void KeyPressed(string key, int modifiers)
    {
        if (screen is not null)
            screen.KeyPressed(new KeyInput(WebKeys.Of(key), (Modifiers)modifiers));
    }

    [JSExport]
    internal static void KeyReleased(string key, int modifiers) => screen?.KeyReleased(new KeyInput(WebKeys.Of(key), (Modifiers)modifiers));

    [JSExport]
    internal static void TextTyped(string text) => screen?.TextTyped(text);
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
}
