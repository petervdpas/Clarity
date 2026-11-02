// What a desktop window needs of the system beyond Silk.NET's own (DESIGN
// 7.9): the OpenGL context it can get, drawing in software where Skia
// cannot use it, the window's real content scale, and its icon.
//
// OpenGL: a window asks for OpenGL 3.3 core first, as Skia draws best with
// it; a driver that has none (an old GPU, a virtual machine, Windows over
// Remote Desktop, whose own OpenGL is 1.1) gets the newest compatibility
// context it has, down to 1.1. Skia draws through the context when it can
// (OpenGL 2.0 and on); below that, or where CLARITY_RENDER=software says so,
// Skia draws in memory and the pixels go to the window with glDrawPixels,
// which every desktop OpenGL has. Only a system with no OpenGL at all shows
// no window, and says why.

using System.Runtime.InteropServices;
using Silk.NET.Core;
using Silk.NET.Windowing;
using SkiaSharp;

namespace Clarity.Ui.Desktop;

public static class WindowSystem
{
    /// <summary>The OpenGL contexts a window asks for, in turn, until the system gives one.</summary>
    public static readonly GraphicsAPI[] Contexts =
    [
        new(ContextAPI.OpenGL, ContextProfile.Core, ContextFlags.ForwardCompatible, new APIVersion(3, 3)),
        // Below 3.2 there are no profiles: GLFW is asked for any context of the version, which has everything of its own.
        new(ContextAPI.OpenGL, ContextProfile.Compatability, ContextFlags.Default, new APIVersion(2, 1)),
        new(ContextAPI.OpenGL, ContextProfile.Compatability, ContextFlags.Default, new APIVersion(1, 1)),
    ];

    /// <summary>
    /// The first of <see cref="Contexts"/> from <paramref name="first"/> on
    /// that the system gives, tried with a hidden window of GLFW's own (Silk.NET
    /// does not survive a context the system refuses); -1 for none.
    /// </summary>
    internal static unsafe int ContextFrom(int first)
    {
        var glfw = Silk.NET.GLFW.GlfwProvider.GLFW.Value;
        if (!glfw.Init())
            return -1;
        for (var i = first; i < Contexts.Length; i++)
        {
            var api = Contexts[i];
            glfw.DefaultWindowHints();
            glfw.WindowHint(Silk.NET.GLFW.WindowHintBool.Visible, false);
            glfw.WindowHint(Silk.NET.GLFW.WindowHintClientApi.ClientApi, Silk.NET.GLFW.ClientApi.OpenGL);
            glfw.WindowHint(Silk.NET.GLFW.WindowHintInt.ContextVersionMajor, api.Version.MajorVersion);
            glfw.WindowHint(Silk.NET.GLFW.WindowHintInt.ContextVersionMinor, api.Version.MinorVersion);
            var core = api.Profile == ContextProfile.Core;
            var profiled = api.Version.MajorVersion > 3 || (api.Version.MajorVersion == 3 && api.Version.MinorVersion >= 2);
            glfw.WindowHint(Silk.NET.GLFW.WindowHintOpenGlProfile.OpenGlProfile, core ? Silk.NET.GLFW.OpenGlProfile.Core : profiled ? Silk.NET.GLFW.OpenGlProfile.Compat : Silk.NET.GLFW.OpenGlProfile.Any);
            glfw.WindowHint(Silk.NET.GLFW.WindowHintBool.OpenGLForwardCompat, core);
            var probe = glfw.CreateWindow(1, 1, "", null, null);
            if (probe is not null)
            {
                glfw.DestroyWindow(probe);
                glfw.DefaultWindowHints();
                return i;
            }
            Console.Error.WriteLine($"OpenGL {api.Version.MajorVersion}.{api.Version.MinorVersion} is not there");
        }
        glfw.DefaultWindowHints();
        return -1;
    }

    /// <summary>Whether the person asked for drawing in software (CLARITY_RENDER=software), for a driver Skia draws wrongly on.</summary>
    public static bool SoftwareAsked(Func<string, string?> env) =>
        string.Equals(env("CLARITY_RENDER")?.Trim(), "software", StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// The scale a window draws at and how many of the system's window units
    /// one of the screen's units is: <paramref name="size"/> is the window in
    /// the system's units, <paramref name="pixels"/> its framebuffer, and
    /// <paramref name="content"/> the content scale the system gives the
    /// window (GLFW's, 0 when it gave none). On macOS and Wayland the window's
    /// units are already the screen's (the framebuffer is larger); on X11 and
    /// Windows they are pixels, and the content scale (150 percent, 200) says
    /// how large the screen's units are.
    /// </summary>
    public static (float Scale, double PerUnit) ScaleOf((int X, int Y) size, (int X, int Y) pixels, float content)
    {
        var ratio = size.X > 0 && pixels.X > 0 ? (float)pixels.X / size.X : 1;
        var scale = content > 0 ? Math.Max(content, ratio) : ratio;
        if (scale <= 0 || float.IsNaN(scale))
            scale = 1;
        return (scale, scale / ratio);
    }

    /// <summary>The content scale GLFW gives the window (its monitor's, as the system is set); 0 when it gives none.</summary>
    internal static unsafe float ContentScale(IWindow window)
    {
        try
        {
            if (window.Native?.Glfw is not { } handle || ContentScaleOf() is not { } get)
                return 0;
            float x = 0, y = 0;
            ((delegate* unmanaged<nint, float*, float*, void>)get)(handle, &x, &y);
            return x;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            return 0;
        }
    }

    static nint? contentScale;

    static nint? ContentScaleOf()
    {
        if (contentScale is null)
        {
            var glfw = Silk.NET.GLFW.GlfwProvider.GLFW.Value;
            contentScale = glfw.Context.TryGetProcAddress("glfwGetWindowContentScale", out var at) ? at : 0;
        }
        return contentScale == 0 ? null : contentScale;
    }

    /// <summary>The window's icon: the one the application carries (a resource named <c>icon.png</c>), else Clarity's; at the sizes systems show.</summary>
    internal static RawImage[] Icon(System.Reflection.Assembly app)
    {
        using var stream = app.GetManifestResourceStream("icon.png") ?? typeof(WindowSystem).Assembly.GetManifestResourceStream("Clarity.Ui.Desktop.icon.png");
        if (stream is null)
            return [];
        using var image = SKBitmap.Decode(stream);
        if (image is null)
            return [];
        return [.. new[] { 16, 32, 48, 64, 128 }.Where(s => s <= Math.Max(image.Width, 16)).Select(s => Sized(image, s))];
    }

    static RawImage Sized(SKBitmap image, int size)
    {
        using var sized = new SKBitmap(new SKImageInfo(size, size, SKColorType.Rgba8888, SKAlphaType.Unpremul));
        using (var canvas = new SKCanvas(sized))
        {
            canvas.Clear(SKColors.Transparent);
            using var paint = new SKPaint { IsAntialias = true };
            using var source = SKImage.FromBitmap(image);
            canvas.DrawImage(source, new SKRect(0, 0, size, size), new SKSamplingOptions(SKCubicResampler.Mitchell), paint);
        }
        return new RawImage(size, size, sized.Bytes);
    }

    /// <summary>The system's window, as its file dialogs name an owner.</summary>
    internal static SystemFilePicker.SystemWindow? Handle(IWindow window)
    {
        try
        {
            var native = window.Native;
            if (native?.X11 is { } x11)
                return new SystemFilePicker.SystemWindow(X11: x11.Window);
            if (native?.Win32 is { } win32)
                return new SystemFilePicker.SystemWindow(Win32: win32.Hwnd);
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            // A window the system names no handle of: the dialog belongs to none.
        }
        return null;
    }
}

/// <summary>
/// Skia's pixels drawn in memory and put on the window with OpenGL's
/// glDrawPixels: the window's picture where Skia cannot draw through the
/// window's OpenGL (a driver below OpenGL 2.0, Windows over Remote Desktop).
/// </summary>
internal sealed unsafe class SoftwareSurface(Func<string, nint> function) : IDisposable
{
    const uint Rgba = 0x1908, UnsignedByte = 0x1401, ColorBufferBit = 0x4000;
    readonly delegate* unmanaged<int, int, int, int, void> viewport = (delegate* unmanaged<int, int, int, int, void>)function("glViewport");
    readonly delegate* unmanaged<float, float, void> rasterPos = (delegate* unmanaged<float, float, void>)function("glRasterPos2f");
    readonly delegate* unmanaged<float, float, void> pixelZoom = (delegate* unmanaged<float, float, void>)function("glPixelZoom");
    readonly delegate* unmanaged<int, int, uint, uint, void*, void> drawPixels = (delegate* unmanaged<int, int, uint, uint, void*, void>)function("glDrawPixels");
    readonly delegate* unmanaged<uint, void> clear = (delegate* unmanaged<uint, void>)function("glClear");
    readonly delegate* unmanaged<void> flush = (delegate* unmanaged<void>)function("glFlush");
    SKBitmap? pixels;
    SKSurface? surface;

    /// <summary>Whether this OpenGL has what it takes.</summary>
    public bool Works => viewport != null && rasterPos != null && pixelZoom != null && drawPixels != null;

    /// <summary>The surface Skia draws on, as large as the framebuffer.</summary>
    public SKSurface Fit(int width, int height)
    {
        width = Math.Max(1, width);
        height = Math.Max(1, height);
        if (surface is not null && pixels is { } p && p.Width == width && p.Height == height)
            return surface;
        surface?.Dispose();
        pixels?.Dispose();
        pixels = new SKBitmap(new SKImageInfo(width, height, SKColorType.Rgba8888, SKAlphaType.Premul));
        surface = SKSurface.Create(pixels.Info, pixels.GetPixels(), pixels.RowBytes);
        return surface;
    }

    /// <summary>Puts what Skia drew on the window: from the top left, its rows downward.</summary>
    public void Present()
    {
        if (pixels is not { } p)
            return;
        viewport(0, 0, p.Width, p.Height);
        if (clear != null)
            clear(ColorBufferBit);
        // OpenGL's rows go up from the bottom: start at the top left and draw downward.
        rasterPos(-1, 1);
        pixelZoom(1, -1);
        drawPixels(p.Width, p.Height, Rgba, UnsignedByte, (void*)p.GetPixels());
        if (flush != null)
            flush();
    }

    public void Dispose()
    {
        surface?.Dispose();
        pixels?.Dispose();
    }
}
