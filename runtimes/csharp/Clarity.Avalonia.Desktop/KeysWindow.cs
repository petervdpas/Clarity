// Clarity's keys window on the desktop (DESIGN 5.28): the keys view in a
// window of its own over the window the person works in.

using Avalonia.Controls;

namespace Clarity.Avalonia;

public static class KeysWindow
{
    /// <summary>The application's commands, which the generated application gives at its start.</summary>
    public static IReadOnlyList<KeyLayerInfo> Catalog { get; set; } = [];

    /// <summary>Shows the keys window over <paramref name="owner"/>, in a window of Clarity's own.</summary>
    public static void Show(Window owner)
    {
        var view = new KeysView(Catalog);
        var window = new Window
        {
            Title = KeysView.Word("keysTitle", "Keys"),
            Content = view,
            SizeToContent = SizeToContent.WidthAndHeight,
            MaxHeight = 640,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        view.Closed += window.Close;
        // Sized once to what it shows first; after that it keeps its size.
        window.Opened += (_, _) => window.SizeToContent = SizeToContent.Manual;
        _ = window.ShowDialog(owner);
    }
}
