// A screen of Clarity.UI in the system's own window (DESIGN 7.9): Skia draws
// it through OpenGL when it changed, at the window's own scale, and what the
// person does with the mouse and keys goes to the screen.

using Clarity.Ui.Drawing;
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

    static Drawing.Key KeyOf(SilkKey key) => key switch
    {
        SilkKey.Tab => Drawing.Key.Tab,
        SilkKey.Enter or SilkKey.KeypadEnter => Drawing.Key.Enter,
        SilkKey.Escape => Drawing.Key.Escape,
        SilkKey.Space => Drawing.Key.Space,
        SilkKey.Backspace => Drawing.Key.Backspace,
        SilkKey.Delete => Drawing.Key.Delete,
        SilkKey.Insert => Drawing.Key.Insert,
        SilkKey.Left => Drawing.Key.Left,
        SilkKey.Right => Drawing.Key.Right,
        SilkKey.Up => Drawing.Key.Up,
        SilkKey.Down => Drawing.Key.Down,
        SilkKey.Home => Drawing.Key.Home,
        SilkKey.End => Drawing.Key.End,
        SilkKey.PageUp => Drawing.Key.PageUp,
        SilkKey.PageDown => Drawing.Key.PageDown,
        >= SilkKey.F1 and <= SilkKey.F12 => Drawing.Key.F1 + (key - SilkKey.F1),
        >= SilkKey.A and <= SilkKey.Z => Drawing.Key.A + (key - SilkKey.A),
        >= SilkKey.Number0 and <= SilkKey.Number9 => Drawing.Key.D0 + (key - SilkKey.Number0),
        _ => Drawing.Key.Other,
    };
}
