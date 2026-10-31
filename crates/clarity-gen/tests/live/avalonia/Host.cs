// Starts the every-part app as its Program does, on Avalonia's headless
// platform: no window reaches the person's desktop. Then runs the steps
// (Steps.cs) on the UI thread. Arguments: the database file, the fixture folder.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Headless;
using Avalonia.Threading;

namespace Everything.Live;

public static class Host
{
    public static int Main(string[] args)
    {
        // Secrets in memory: the host runs apart from the person's desktop and its keyring.
        var secrets = new Dictionary<(string, string), string>();
        Clarity.Runtime.Secrets.Store = ((program, name) => secrets.GetValueOrDefault((program, name)), (program, name, value) => secrets[(program, name)] = value);
        // The databases made as the app makes them, and records put in, before it starts.
        using (Program.OpenDatabases())
        {
        }
        Steps.Seed(args[0]);
        var lifetime = new ClassicDesktopStyleApplicationLifetime { ShutdownMode = ShutdownMode.OnExplicitShutdown };
        // Drawn by the real renderer, as on a person's screen: a painted
        // picture is real pixels, as the app saves them.
        AppBuilder.Configure<App>()
            .UseSkia()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions { UseHeadlessDrawing = false })
            .SetupWithLifetime(lifetime);
        Dispatcher.UIThread.RunJobs();
        var main = lifetime.MainWindow ?? throw new InvalidOperationException("the app opened no window");
        var steps = new Steps(main, args[0], args[1]);
        var failed = steps.Run();
        lifetime.Shutdown();
        return failed;
    }
}
