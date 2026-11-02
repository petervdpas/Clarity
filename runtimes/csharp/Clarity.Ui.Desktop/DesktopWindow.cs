// A screen of Clarity.UI in the system's own window (DESIGN 7.9): Skia draws
// it through OpenGL when it changed, at the window's own scale, and what the
// person does with the mouse, the wheel and keys goes to the screen. A
// generated application runs its windows here, from the markup and themes
// its assembly carries.

using System.Collections.Concurrent;
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

    /// <summary>The size a window that names none opens at, on every stack (clarity_spec::layout).</summary>
    public static readonly Extent StandardSize = new(1100, 760);

    /// <summary>
    /// Shows a generated application from its main window until the person
    /// closes it: each window drawn from its markup (<c>Windows/Main.cxaml</c>,
    /// carried in the application's assembly) over its view model, in the
    /// theme shown laid over Clarity's own (<c>Themes/*.json</c>). Windows open
    /// in place over the one before and close back to it.
    /// </summary>
    public static void Run(WindowViewModel main)
    {
        var app = main.GetType().Assembly;
        var stack = new WindowStack(model => Carried(app, $"Windows/{WindowStack.NameOf(model)}.cxaml")) { OpenAddress = SystemAddress.Open };
        // The window's own keys (shortcuts, default and cancel buttons, undo) come before the part with the focus.
        var screen = new Screen(stack, LookOf(app)) { Keys = stack.KeyPressed };
        // What runs once the work at hand is done (a change handler joining its edit) runs on the next frame.
        var later = new ConcurrentQueue<Action>();
        Later.Poster = later.Enqueue;
        Themes.Changed += () => screen.Show(LookOf(app));
        var size = main.WindowSize ?? StandardSize;
        IWindow? shown = null;
        var ended = false;
        stack.TitleChanged += () =>
        {
            if (shown is not null && stack.Current is { } top)
                shown.Title = top.Title;
        };
        stack.Ended += () =>
        {
            ended = true;
            shown?.Close();
        };
        DesktopProgram.Current = new Program(() => shown?.Close());
        stack.Push(main);
        Run(screen, main.Title, (int)size.Width, (int)size.Height, new Hooks
        {
            Loaded = window => shown = window,
            Frame = () =>
            {
                while (later.TryDequeue(out var action))
                    action();
                stack.Tick();
            },
            Closing = () =>
            {
                if (!ended)
                    stack.Closing();
            },
        });
    }

    /// <summary>What a generated application gives the desktop window: quitting; the keys window comes with the frame.</summary>
    sealed class Program(Action quit) : IDesktop
    {
        public void Quit() => quit();

        public void ShowKeys() => Console.Error.WriteLine("Clarity.UI's keys window comes with the frame (G7d)");
    }

    /// <summary>A text the application carries as a resource, by its logical name; none when it carries none.</summary>
    static string? Carried(Assembly app, string name)
    {
        using var stream = app.GetManifestResourceStream(name);
        if (stream is null)
            return null;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }

    /// <summary>
    /// The theme shown (Themes.Current) as the application carries it,
    /// resolved for light mode, laid over Clarity's own; Clarity's own when
    /// the application offers none.
    /// </summary>
    static ThemeLook LookOf(Assembly app)
    {
        if (Themes.Current is not { } name || Carried(app, $"Themes/{ClassOf(name)}.light.json") is not { } resolved)
            return ThemeLook.Clarity(dark: false);
        try
        {
            var look = ThemeLook.Over(resolved, ThemeLook.Clarity(dark: false));
            return look.Dark ? ThemeLook.Over(resolved, ThemeLook.Clarity(dark: true)) : look;
        }
        catch (System.Text.Json.JsonException e)
        {
            // Fail safe: a theme that cannot be read leaves Clarity's own.
            Console.Error.WriteLine($"the theme {name} cannot be read: {e.Message}");
            return ThemeLook.Clarity(dark: false);
        }
    }

    /// <summary>The file name a theme's resolved look goes by, as clarity_spec's class_name: <c>NightOwl</c> is <c>night-owl</c>.</summary>
    internal static string ClassOf(string name)
    {
        var kebab = new System.Text.StringBuilder();
        for (var i = 0; i < name.Length; i++)
        {
            var c = name[i];
            if (char.IsAsciiLetterUpper(c))
            {
                if (i > 0 && kebab.Length > 0 && kebab[^1] != '-')
                    kebab.Append('-');
                kebab.Append(char.ToLowerInvariant(c));
            }
            else if (char.IsAsciiLetterOrDigit(c))
                kebab.Append(c);
            else if (kebab.Length > 0 && kebab[^1] != '-')
                kebab.Append('-');
        }
        return kebab.ToString().TrimEnd('-');
    }

    /// <summary>What a program does beside drawing its screen: when its window is made, every frame, and as it closes.</summary>
    sealed class Hooks
    {
        public Action<IWindow>? Loaded { get; init; }
        public Action? Frame { get; init; }
        public Action? Closing { get; init; }
    }

    /// <summary>Shows <paramref name="screen"/> in a window until the person closes it.</summary>
    public static void Run(Screen screen, string title, int width, int height) => Run(screen, title, width, height, new Hooks());

    static void Run(Screen screen, string title, int width, int height, Hooks hooks)
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
                // A turn of the wheel scrolls three lines of 40; up is toward the top.
                mouse.Scroll += (m, wheel) => screen.Wheel(m.Position.X, m.Position.Y, -wheel.X * 40, -wheel.Y * 40);
            }
            foreach (var keyboard in input.Keyboards)
            {
                keyboard.KeyDown += (k, key, _) =>
                {
                    var held = ModifiersOf(k);
                    screen.KeyPressed(new KeyInput(KeyOf(key, held.HasFlag(Modifiers.Shift)), held));
                };
                keyboard.KeyChar += (_, typed) =>
                {
                    if (!char.IsControl(typed))
                        screen.TextTyped(typed.ToString());
                };
            }
            hooks.Loaded?.Invoke(window);
        };
        window.FramebufferResize += _ => Fit();
        window.Update += _ => hooks.Frame?.Invoke();
        window.Render += _ =>
        {
            // While something animates, every frame is drawn.
            if ((!changed && screen.Animating == 0) || surface is null || skia is null)
                return;
            changed = false;
            screen.Draw(surface.Canvas);
            skia.Flush();
            window.SwapBuffers();
        };
        window.Closing += () =>
        {
            hooks.Closing?.Invoke();
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
        >= SilkKey.F1 and <= SilkKey.F12 => Clarity.Ui.Key.F1 + (key - SilkKey.F1),
        >= SilkKey.A and <= SilkKey.Z => Clarity.Ui.Key.A + (key - SilkKey.A),
        >= SilkKey.Number0 and <= SilkKey.Number9 => Clarity.Ui.Key.D0 + (key - SilkKey.Number0),
        _ => Clarity.Ui.Key.Other,
    };
}
