// A screen of Clarity.UI on a browser page's canvas (DESIGN 7.9): the same
// program as on the desktop, compiled to WebAssembly. Skia draws the screen
// into pixels when it changed, on the page's next frame, at the page's pixel
// scale; clarity-ui.js puts them on the canvas and sends what the person does.

using System.Runtime.InteropServices.JavaScript;
using System.Runtime.Versioning;
using Clarity.Ui;
using SkiaSharp;

namespace Clarity.Ui.Browser;

[SupportedOSPlatform("browser")]
public static partial class BrowserCanvas
{
    const string Module = "clarity-ui";
    static Screen? screen;
    static SKBitmap? pixels;
    static bool wanted;

    /// <summary>Shows <paramref name="shown"/> on the canvas <paramref name="canvas"/>, or on the whole page when there is none; it goes on while the page is open.</summary>
    public static async Task Run(Screen shown, string canvas = "clarity")
    {
        screen = shown;
        shown.Invalidated += Want;
        await JSHost.ImportAsync(Module, ModuleUrl());
        await Start(canvas);
        Want();
        await Task.Delay(Timeout.Infinite);
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

    [JSImport("present", Module)]
    private static partial void Present([JSMarshalAs<JSType.MemoryView>] Span<byte> pixels, int width, int height);

    [JSExport]
    internal static void Resized(double width, double height, double scale) => screen?.Resize(width, height, (float)scale);

    [JSExport]
    internal static unsafe void Frame()
    {
        wanted = false;
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
    }

    [JSExport]
    internal static void PointerMoved(double x, double y) => screen?.PointerMoved(x, y);

    [JSExport]
    internal static void PointerLeft() => screen?.PointerLeft();

    [JSExport]
    internal static void PointerPressed(double x, double y, int button) => screen?.PointerPressed(x, y, (PointerButton)button);

    [JSExport]
    internal static void PointerReleased(double x, double y, int button) => screen?.PointerReleased(x, y, (PointerButton)button);

    [JSExport]
    internal static void KeyPressed(string key, int modifiers) => screen?.KeyPressed(new KeyInput(WebKeys.Of(key), (Modifiers)modifiers));

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
        ['F', .. var n] when int.TryParse(n, out var f) && f is >= 1 and <= 12 => Key.F1 + (f - 1),
        [var c] when char.IsAsciiLetter(c) => Key.A + (char.ToUpperInvariant(c) - 'A'),
        [var c] when char.IsAsciiDigit(c) => Key.D0 + (c - '0'),
        _ => Key.Other,
    };
}
