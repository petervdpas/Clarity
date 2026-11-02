// A screen of Clarity.UI in the system's own window (DESIGN 7.9): Skia draws
// it through OpenGL when it changed, at the window's own scale, and what the
// person does with the mouse and keys goes to the screen.

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

    /// <summary>The size a window that names none opens at, on every stack (clarity_spec::layout).</summary>
    public static readonly Extent StandardSize = new(1100, 760);

    /// <summary>
    /// Shows the application's main window until the person closes it: its
    /// title and size. Drawing its parts from its markup comes with DESIGN
    /// 7.9, G7d; until then the window shows its theme's background.
    /// </summary>
    public static void Run(WindowViewModel main)
    {
        var size = main.WindowSize ?? StandardSize;
        var screen = new Screen(new GridPanel(columns: 12), ThemeLook.Clarity(dark: false));
        Run(screen, main.Title, (int)size.Width, (int)size.Height);
    }

    /// <summary>Shows <paramref name="screen"/> in a window until the person closes it.</summary>
    public static void Run(Screen screen, string title, int width, int height)
    {
        var options = WindowOptions.Default with
        {
            Title = title,
            Size = new Vector2D<int>(width, height),
            API = new GraphicsAPI(ContextAPI.OpenGL, ContextProfile.Core, ContextFlags.ForwardCompatible, new APIVersion(3, 3)),
            PreferredStencilBufferBits = 8,
            VSync = true,
            ShouldSwapAutomatically = false,
            FramesPerSecond = 60,
            UpdatesPerSecond = 60,
        };
        using var window = Window.Create(options);
        GRContext? skia = null;
        SKSurface? surface = null;
        IInputContext? input = null;
        var changed = true;
        screen.Invalidated += () => changed = true;

        void Fit()
        {
            if (skia is null)
                return;
            surface?.Dispose();
            var pixels = window.FramebufferSize;
            var target = new GRBackendRenderTarget(Math.Max(1, pixels.X), Math.Max(1, pixels.Y), 0, 8, new GRGlFramebufferInfo(0, Rgba8));
            surface = SKSurface.Create(skia, target, GRSurfaceOrigin.BottomLeft, SKColorType.Rgba8888);
            var scale = window.Size.X > 0 ? (float)pixels.X / window.Size.X : 1;
            screen.Resize(window.Size.X, window.Size.Y, scale);
            changed = true;
        }

        window.Load += () =>
        {
            // Skia finds OpenGL's functions itself (GLX or EGL, WGL, CGL) in the window's current context.
            window.MakeCurrent();
            skia = GRContext.CreateGl(GRGlInterface.Create());
            Fit();
            input = window.CreateInput();
            foreach (var mouse in input.Mice)
            {
                mouse.MouseMove += (_, at) => screen.PointerMoved(at.X, at.Y);
                mouse.MouseDown += (m, button) => screen.PointerPressed(m.Position.X, m.Position.Y, Button(button));
                mouse.MouseUp += (m, button) => screen.PointerReleased(m.Position.X, m.Position.Y, Button(button));
            }
            foreach (var keyboard in input.Keyboards)
            {
                keyboard.KeyDown += (k, key, _) => screen.KeyPressed(new KeyInput(KeyOf(key), ModifiersOf(k)));
                keyboard.KeyChar += (_, typed) =>
                {
                    if (!char.IsControl(typed))
                        screen.TextTyped(typed.ToString());
                };
            }
        };
        window.FramebufferResize += _ => Fit();
        window.Render += _ =>
        {
            if (!changed || surface is null || skia is null)
                return;
            changed = false;
            screen.Draw(surface.Canvas);
            skia.Flush();
            window.SwapBuffers();
        };
        window.Closing += () =>
        {
            input?.Dispose();
            surface?.Dispose();
            skia?.Dispose();
        };
        window.Run();
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

    static Clarity.Ui.Key KeyOf(SilkKey key) => key switch
    {
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
        >= SilkKey.F1 and <= SilkKey.F12 => Clarity.Ui.Key.F1 + (key - SilkKey.F1),
        >= SilkKey.A and <= SilkKey.Z => Clarity.Ui.Key.A + (key - SilkKey.A),
        >= SilkKey.Number0 and <= SilkKey.Number9 => Clarity.Ui.Key.D0 + (key - SilkKey.Number0),
        _ => Clarity.Ui.Key.Other,
    };
}
