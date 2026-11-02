// The system's light or dark choice (DESIGN 7.9), which Clarity's themes
// follow at start and when it changes. Each system answers in its own way:
// on Linux the desktop portal (org.freedesktop.appearance color-scheme), else
// GNOME's settings; on Windows the registry's AppsUseLightTheme; on macOS the
// defaults' AppleInterfaceStyle. A program that is missing, slow or says
// nothing leaves light, so reading it never stops the application. A browser
// page asks prefers-color-scheme instead (Clarity.Ui.Browser).

using System.Diagnostics;
using System.Text.RegularExpressions;

namespace Clarity.Ui;

public sealed partial class SystemTheme(Func<string, IReadOnlyList<string>, string?> run, string system)
{
    /// <summary>The system this runs on, asked through its own programs.</summary>
    public static SystemTheme Here { get; } = new(Run, OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : OperatingSystem.IsLinux() ? "linux" : "other");

    /// <summary>Whether the system shows dark now; light when it says nothing.</summary>
    public bool Dark()
    {
        try
        {
            return system switch
            {
                "linux" => Portal(run("gdbus", ["call", "--session", "--dest", "org.freedesktop.portal.Desktop", "--object-path", "/org/freedesktop/portal/desktop", "--method", "org.freedesktop.portal.Settings.Read", "org.freedesktop.appearance", "color-scheme"]))
                    ?? ColorScheme(run("gsettings", ["get", "org.gnome.desktop.interface", "color-scheme"]))
                    ?? GtkTheme(run("gsettings", ["get", "org.gnome.desktop.interface", "gtk-theme"]))
                    ?? false,
                "windows" => Registry(run("reg", ["query", @"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "/v", "AppsUseLightTheme"])) ?? false,
                "macos" => Defaults(run("defaults", ["read", "-g", "AppleInterfaceStyle"])),
                _ => false,
            };
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            // Fail safe: what cannot be asked is light.
            return false;
        }
    }

    /// <summary>
    /// Reads the choice every <paramref name="every"/> away from the screen's
    /// thread; <paramref name="changed"/> hears it when it differs from <paramref name="now"/>.
    /// Disposing it stops.
    /// </summary>
    public IDisposable Watch(bool now, TimeSpan every, Action<bool> changed)
    {
        var last = now;
        var busy = 0;
        return new Timer(_ =>
        {
            if (Interlocked.Exchange(ref busy, 1) == 1)
                return;
            try
            {
                var dark = Dark();
                if (dark != last)
                {
                    last = dark;
                    changed(dark);
                }
            }
            finally
            {
                busy = 0;
            }
        }, null, every, every);
    }

    /// <summary>The portal's color-scheme: 1 prefers dark, 2 prefers light, 0 says nothing.</summary>
    public static bool? Portal(string? said) => said is not null && PortalValue().Match(said) is { Success: true } m
        ? m.Groups[1].Value switch
        {
            "1" => true,
            "2" => false,
            _ => null,
        }
        : null;

    /// <summary>GNOME's color-scheme: <c>'prefer-dark'</c>, <c>'prefer-light'</c> or <c>'default'</c>.</summary>
    public static bool? ColorScheme(string? said) => said?.Trim().Trim('\'') switch
    {
        "prefer-dark" => true,
        "prefer-light" => false,
        _ => null,
    };

    /// <summary>A GTK theme named dark (<c>'Adwaita-dark'</c>) is dark; another says nothing.</summary>
    public static bool? GtkTheme(string? said) => said is not null && said.Contains("dark", StringComparison.OrdinalIgnoreCase) ? true : null;

    /// <summary>Windows' AppsUseLightTheme: 0 is dark, 1 light; none when the value is not there.</summary>
    public static bool? Registry(string? said) => said is not null && RegistryValue().Match(said) is { Success: true } m
        ? m.Groups[1].Value != "1"
        : null;

    /// <summary>macOS's AppleInterfaceStyle: <c>Dark</c>, else not set at all in light mode.</summary>
    public static bool Defaults(string? said) => said?.Trim() == "Dark";

    [GeneratedRegex(@"uint32 (\d+)")]
    private static partial Regex PortalValue();

    [GeneratedRegex(@"AppsUseLightTheme\s+REG_DWORD\s+0x([0-9a-fA-F]+)")]
    private static partial Regex RegistryValue();

    /// <summary>What a program prints when it ends well within two seconds; none when it is missing, fails or is slow.</summary>
    static string? Run(string program, IReadOnlyList<string> args)
    {
        var start = new ProcessStartInfo(program)
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        foreach (var arg in args)
            start.ArgumentList.Add(arg);
        try
        {
            using var process = Process.Start(start);
            if (process is null)
                return null;
            var output = process.StandardOutput.ReadToEndAsync();
            _ = process.StandardError.ReadToEndAsync();
            if (!process.WaitForExit(2000))
            {
                process.Kill(entireProcessTree: true);
                return null;
            }
            return process.ExitCode == 0 ? output.Result : null;
        }
        catch (Exception e) when (e is System.ComponentModel.Win32Exception or InvalidOperationException or IOException)
        {
            return null;
        }
    }
}

/// <summary>The look of an application's theme shown, as the system's light or dark choice resolves it.</summary>
public static class AppLook
{
    /// <summary>
    /// The theme named <paramref name="theme"/> as the application carries it
    /// (<paramref name="carried"/> gives a resource by its name), its
    /// <c>.dark.json</c> when the system is dark, else its <c>.light.json</c>,
    /// laid over Clarity's own of the same mode. Clarity's own, following the
    /// system, when the application carries none or it cannot be read.
    /// </summary>
    public static ThemeLook Of(Func<string, string?> carried, string? theme, bool systemDark)
    {
        var own = ThemeLook.Clarity(systemDark);
        if (theme is null)
            return own;
        var file = systemDark ? $"Themes/{ClassOf(theme)}.dark.json" : $"Themes/{ClassOf(theme)}.light.json";
        if (carried(file) is not { } resolved)
            return own;
        try
        {
            var look = ThemeLook.Over(resolved, ThemeLook.Clarity(dark: false));
            return look.Dark ? ThemeLook.Over(resolved, ThemeLook.Clarity(dark: true)) : look;
        }
        catch (System.Text.Json.JsonException e)
        {
            // Fail safe: a theme that cannot be read leaves Clarity's own.
            Console.Error.WriteLine($"the theme {theme} cannot be read: {e.Message}");
            return own;
        }
    }

    /// <summary>The file name a theme's resolved look goes by, as clarity_spec's class_name: <c>NightOwl</c> is <c>night-owl</c>.</summary>
    public static string ClassOf(string name)
    {
        var kebab = new System.Text.StringBuilder();
        for (var i = 0; i < name.Length; i++)
        {
            var c = name[i];
            if (char.IsAsciiLetterUpper(c))
            {
                if (i > 0 && kebab.Length > 0 && kebab[^1] != '-')
                    kebab.Append('-');
                kebab.Append(char.ToLowerInvariant(c));
            }
            else if (char.IsAsciiLetterOrDigit(c))
                kebab.Append(c);
            else if (kebab.Length > 0 && kebab[^1] != '-')
                kebab.Append('-');
        }
        return kebab.ToString().TrimEnd('-');
    }
}
