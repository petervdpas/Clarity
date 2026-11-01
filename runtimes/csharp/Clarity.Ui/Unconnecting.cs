// The app's `on unconnected` running (DESIGN 5.27): the window it runs on
// says and opens through this, wherever the program shows its windows.

using System.Collections.ObjectModel;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>
/// The app's <c>on unconnected</c> running: why the databases did not open,
/// where what it says goes, and what it opens.
/// </summary>
public sealed record Unconnecting(string Reason, ObservableCollection<Message> Said, Action<WindowViewModel> Open)
{
    /// <summary>
    /// Why the databases did not open, as the person reads it (conformance/unconnected.json):
    /// in Clarity's own words, which the project translates, with this runtime's message in the slot.
    /// </summary>
    public static Text Why(Exception e) => e switch
    {
        NotKeptException => new Text("The database connection is not set yet") { Key = "unconnectedNotKept" },
        StorageException storage => Failed().With("problem", Slot.OfText(storage.What)),
        _ => Failed().With("problem", Slot.OfText(e.Message)),
    };

    /// <summary>Clarity's words, written in English as they read until the project gives them a value.</summary>
    static Text Failed() => new("The database did not open: {problem}") { Key = "unconnectedFailed" };
}
