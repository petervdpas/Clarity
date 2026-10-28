// What only a desktop program has (DESIGN 7.6): quitting, and Clarity's keys
// window in a window of its own. Clarity.Avalonia.Desktop gives it as the
// program starts; in a browser and on a server there is none, and the
// built-in `quit` and `keys` do nothing.

namespace Clarity.Avalonia;

/// <summary>The desktop program the windows run in.</summary>
public interface IDesktop
{
    /// <summary>Ends the application.</summary>
    void Quit();

    /// <summary>Shows Clarity's keys window over the window the person works in.</summary>
    void ShowKeys();
}

/// <summary>The desktop program, once it has started; none in a browser.</summary>
public static class DesktopProgram
{
    public static IDesktop? Current { get; set; }
}
