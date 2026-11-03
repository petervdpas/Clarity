// What an application does when its databases did not open (DESIGN 5.27),
// as a generated Program.cs gives it to DesktopWindow.Run: the app's
// `on unconnected` handler runs with why, on a window that is not shown, and
// what it opens shows first; without one the window that keeps the
// connection shows, saying why. Until the databases open the windows work on
// an empty database in memory. The library runs the flow (closing that
// window connects again when a setting changed while it was open, and the
// main window opens once it does); the program answers it (ProgramWire).

using System.ComponentModel;
using Clarity.Runtime;

namespace Clarity.Ui;

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
}
