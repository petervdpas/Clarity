// The app's `on unconnected` running (DESIGN 5.27): the window it runs on
// says and opens through this, wherever the program shows its windows.

using System.Collections.ObjectModel;

namespace Clarity.Avalonia;

/// <summary>
/// The app's <c>on unconnected</c> running: why the databases did not open,
/// where what it says goes, and what it opens.
/// </summary>
public sealed record Unconnecting(string Reason, ObservableCollection<Message> Said, Action<WindowViewModel> Open);
