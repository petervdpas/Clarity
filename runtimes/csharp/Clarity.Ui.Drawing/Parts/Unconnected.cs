// The databases did not open (DESIGN 5.27), as when a connection kept as a
// secret setting is not kept yet. The app's `on unconnected` handler runs
// with why, on a window that is not shown, and what it opens shows first;
// without one, or when it opens nothing, the window that keeps the
// connection shows, saying why. Until the databases open, the windows work on
// an empty database in memory. Closing that window connects again when a
// setting changed while it was open, and the main window opens once it does;
// closing it without a change ends the application. The host (a desktop
// window or a browser page) carries out the showing through IUnconnectedScreen.

using System.Collections.ObjectModel;
using System.ComponentModel;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>Where the unconnected flow shows its windows: the desktop or a browser page.</summary>
public interface IUnconnectedScreen
{
    /// <summary>Shows first in a desktop window (or page) of its own, with the messages said put in it; whenClosed runs once its last window closed.</summary>
    void Show(WindowViewModel first, IReadOnlyList<Message> said, Action whenClosed);

    /// <summary>The databases opened: the host shows main on them, as at a start that connected, and disposes db when the application ends.</summary>
    void Connected(Db db, WindowViewModel main);

    /// <summary>Nothing to show, or closed without a change: the application ends.</summary>
    void Ended();
}

/// <summary>What an application does when its databases did not open.</summary>
public sealed class Unconnected
{
    /// <summary>Opens the databases, as the application does at start.</summary>
    public required Func<Db> Connect { get; init; }

    /// <summary>The empty database in memory the windows work on until the databases open, such as <c>Sqlite.Memory</c>.</summary>
    public required Func<Db> Empty { get; init; }

    /// <summary>The main window, on the open databases.</summary>
    public required Func<Db, WindowViewModel> Main { get; init; }

    /// <summary>The window that keeps a secret connection; none when the project has none.</summary>
    public Func<Db, WindowViewModel>? Window { get; init; }

    /// <summary>The app's <c>on unconnected</c>: runs it, with why, on a window that is not shown.</summary>
    public Action<Db, Unconnecting>? Handler { get; init; }

    /// <summary>The settings, whose change while the window is open says to connect again.</summary>
    public INotifyPropertyChanged? Settings { get; init; }

    /// <summary>
    /// Why the databases did not open, as the person reads it (conformance/unconnected.json):
    /// in Clarity's own words, which the project translates, with this runtime's message in the slot.
    /// </summary>
    public static Text Reason(Exception e) => Unconnecting.Why(e);

    /// <summary>Shows what keeps the connection on <paramref name="screen"/>, saying <paramref name="problem"/>.</summary>
    public void Start(Text problem, IUnconnectedScreen screen)
    {
        var memory = Empty();
        var said = new ObservableCollection<Message>();
        WindowViewModel? first = null;
        if (Handler is { } handler)
        {
            var unconnecting = new Unconnecting(Locale.Show(problem), said, opened => first ??= opened);
            handler(memory, unconnecting);
        }
        // Without a handler, or one that opens nothing, the window that keeps the connection says why.
        if (first is null)
        {
            said.Add(new Message(Level.Error, problem));
            first = Window?.Invoke(memory);
        }
        if (first is null)
        {
            memory.Dispose();
            screen.Ended();
            return;
        }
        var changed = false;
        void Changed(object? sender, PropertyChangedEventArgs e) => changed = true;
        if (Settings is { } settings)
            settings.PropertyChanged += Changed;
        var closed = false;
        screen.Show(first, said, () =>
        {
            if (closed)
                return;
            closed = true;
            if (Settings is { } settings)
                settings.PropertyChanged -= Changed;
            memory.Dispose();
            if (!changed)
            {
                screen.Ended();
                return;
            }
            Db db;
            try
            {
                db = Connect();
            }
            catch (Exception e)
            {
                Start(Reason(e), screen);
                return;
            }
            screen.Connected(db, Main(db));
        });
    }
}
