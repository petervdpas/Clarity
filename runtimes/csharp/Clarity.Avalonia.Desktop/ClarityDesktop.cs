// The desktop program (DESIGN 7.6): native windows through Avalonia.Desktop,
// sound through SoundFlow, quitting and the keys window. A generated desktop
// program starts with UseClarityDesktop().

using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;

namespace Clarity.Avalonia;

/// <summary>Quitting and the keys window, through the desktop lifetime.</summary>
public sealed class ClarityDesktop : IDesktop
{
    static IClassicDesktopStyleApplicationLifetime? Lifetime => Application.Current?.ApplicationLifetime as IClassicDesktopStyleApplicationLifetime;

    /// <inheritdoc />
    public void Quit() => Lifetime?.Shutdown();

    /// <inheritdoc />
    public void ShowKeys()
    {
        if (Lifetime is { } desktop && (desktop.Windows.FirstOrDefault(w => w.IsActive) ?? desktop.MainWindow) is { } owner)
            KeysWindow.Show(owner);
    }
}

public static class ClarityDesktopBuilder
{
    /// <summary>The application on the desktop: the platform's windows, sound from the speakers, quit and keys.</summary>
    public static AppBuilder UseClarityDesktop(this AppBuilder builder)
    {
        DesktopProgram.Current = new ClarityDesktop();
        Sound.Output = new SoundFlowOutput();
        return builder.UsePlatformDetect();
    }
}
