// A generated application on the desktop (DESIGN 7.9): the native Clarity.UI
// library shows its windows in the system's own windows, draws them, lays
// them out and takes what the person does; the application's view models
// answer it (ProgramWire). The windows' markup, themes, model and resources
// come from the application's assembly. With CLARITY_HEADLESS set the
// library drives the program from standard input instead (DESIGN 1.6).

using System.Reflection;
using Clarity.Runtime;
using Clarity.Ui.Native;

namespace Clarity.Ui;

public static class DesktopWindow
{
    /// <summary>
    /// Shows a generated application from its main window until the person
    /// closes it: each window drawn from its markup (<c>Windows/Main.cxaml</c>,
    /// carried in the application's assembly) over its view model.
    /// </summary>
    public static void Run(WindowViewModel main)
    {
        var app = main.GetType().Assembly;
        var wire = new ProgramWire(app, main, Markup(app));
        var size = main.WindowSize is { } s ? (s.Width, s.Height) : ((double, double)?)null;
        Show(app, wire, main.Title, size);
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
        var wire = new ProgramWire(app, null, Markup(app), unconnected, problem);
        try
        {
            Show(app, wire, null, null);
        }
        finally
        {
            // The databases that opened meanwhile close as the application ends, however it ends.
            wire.Connected?.Dispose();
        }
    }

    /// <summary>The markup of each window the application carries, by its name.</summary>
    internal static Func<string, string?> Markup(Assembly app) => name =>
    {
        using var stream = app.GetManifestResourceStream($"Windows/{name}.cxaml");
        if (stream is null)
            return null;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    };

    static void Show(Assembly app, ProgramWire wire, string? title, (double, double)? size)
    {
        CustomParts.RegisterAll();
        // Quitting from code goes through the program's replies.
        DesktopProgram.Current = wire;
        using var files = NativeFiles.Of(app, string.IsNullOrEmpty(title) ? null : title, size);
        using var program = new NativeProgram(wire.Answer);
        Interop.Check(Lib.DesktopRun(program.Handle, files.Handle, out var exit), "clarity_desktop_run");
        if (exit != 0)
            Environment.ExitCode = exit;
    }

    /// <summary>The library's version, and whether its interface is the one this layer is written for.</summary>
    public static unsafe (string Version, bool Fits) Library()
    {
        var version = System.Runtime.InteropServices.Marshal.PtrToStringUTF8((nint)Lib.Version()) ?? "";
        return (version, Lib.InterfaceVersion() == Lib.Interface);
    }
}
