// The databases did not open (DESIGN 5.27), as when a connection kept as a
// secret setting is not kept yet. The app's `on unconnected` handler runs
// with why, on a window that is not shown, and what it opens shows first;
// without one, or when it opens nothing, the window that keeps the
// connection shows, saying why. Until the databases open, the windows work on
// an empty database in memory. Closing that desktop window connects again when
// a setting changed while it was open, and the main window opens once it does;
// closing it without a change ends the application.

using System.Collections.ObjectModel;
using System.ComponentModel;
using Avalonia.Controls;
using Clarity.Runtime;

namespace Clarity.Avalonia;

/// <summary>What an application does when its databases did not open.</summary>
public sealed class Unconnected
{
    /// <summary>Opens the databases, as the application does at start.</summary>
    public required Func<Db> Connect { get; init; }

    /// <summary>The main window, on the open databases.</summary>
    public required Func<Db, WindowViewModel> Main { get; init; }

    /// <summary>The window that keeps a secret connection; none when the project has none.</summary>
    public Func<Db, WindowViewModel>? Window { get; init; }

    /// <summary>The app's <c>on unconnected</c>: runs it, with why, on a window that is not shown.</summary>
    public Action<Db, Unconnecting>? Handler { get; init; }

    public required Func<WindowViewModel, Control> View { get; init; }

    public TimeSpan? InfoDuration { get; init; }

    /// <summary>The settings, whose change while the window is open says to connect again.</summary>
    public INotifyPropertyChanged? Settings { get; init; }

    /// <summary>The desktop window that is now the application's.</summary>
    public required Action<Shell> Shown { get; init; }

    /// <summary>The databases opened and the main window shows.</summary>
    public Action<Db>? Connected { get; init; }

    /// <summary>The person closed the window without changing a setting: the application ends.</summary>
    public required Action Ended { get; init; }

    /// <summary>Shows what keeps the connection, saying <paramref name="problem"/>.</summary>
    public void Start(string problem)
    {
        var memory = Db.Memory();
        var said = new ObservableCollection<Message>();
        WindowViewModel? first = null;
        if (Handler is { } handler)
        {
            var unconnecting = new Unconnecting(problem, said, opened => first ??= opened);
            handler(memory, unconnecting);
        }
        // Without a handler, or one that opens nothing, the window that keeps the connection says why.
        if (first is null)
        {
            said.Add(new Message(Level.Error, Text.Plain(problem)));
            first = Window?.Invoke(memory);
        }
        if (first is null)
        {
            memory.Dispose();
            Ended();
            return;
        }
        var changed = false;
        void Changed(object? sender, PropertyChangedEventArgs e) => changed = true;
        if (Settings is { } settings)
            settings.PropertyChanged += Changed;
        var shell = Shell.Start(first, View, InfoDuration);
        foreach (var message in said)
            shell.Messages.Add(message);
        Shown(shell);
        shell.Closed += (_, _) =>
        {
            if (Settings is { } settings)
                settings.PropertyChanged -= Changed;
            memory.Dispose();
            if (!changed)
            {
                Ended();
                return;
            }
            Db db;
            try
            {
                db = Connect();
            }
            catch (Exception e)
            {
                Start(e.Message);
                return;
            }
            Connected?.Invoke(db);
            Shown(Shell.Start(Main(db), View, InfoDuration));
        };
    }
}

/// <summary>
/// The app's <c>on unconnected</c> running: why the databases did not open,
/// where what it says goes, and what it opens.
/// </summary>
public sealed record Unconnecting(string Reason, ObservableCollection<Message> Said, Action<WindowViewModel> Open);
