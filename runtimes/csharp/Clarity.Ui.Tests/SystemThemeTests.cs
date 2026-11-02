// The system's light or dark choice (DESIGN 7.9): read the way each system
// answers it, never hard to run (a program that is missing or says nothing
// is light), and the theme shown is the theme's .light or .dark JSON.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class SystemThemeTests
{
    /// <summary>A system whose programs answer as given: by the program and its first word.</summary>
    static Func<string, IReadOnlyList<string>, string?> Answers(Dictionary<string, string?> answers) =>
        (program, args) => answers.GetValueOrDefault($"{program} {args.FirstOrDefault()}");

    [Fact]
    public void Linux_asks_the_desktop_portal_first()
    {
        Assert.True(SystemTheme.Portal("(<<uint32 1>>,)\n"));
        Assert.False(SystemTheme.Portal("(<<uint32 2>>,)"));
        Assert.Null(SystemTheme.Portal("(<<uint32 0>>,)"));
        Assert.Null(SystemTheme.Portal("Error: GDBus.Error"));
        var theme = new SystemTheme(Answers(new() { ["gdbus call"] = "(<<uint32 1>>,)" }), "linux");
        Assert.True(theme.Dark());
    }

    [Fact]
    public void Linux_without_the_portal_asks_gnome_s_settings()
    {
        Assert.True(SystemTheme.ColorScheme("'prefer-dark'\n"));
        Assert.False(SystemTheme.ColorScheme("'prefer-light'"));
        Assert.Null(SystemTheme.ColorScheme("'default'"));
        Assert.True(SystemTheme.GtkTheme("'Adwaita-dark'"));
        Assert.Null(SystemTheme.GtkTheme("'Adwaita'"));
        var scheme = new SystemTheme(Answers(new() { ["gsettings get"] = "'prefer-dark'" }), "linux");
        Assert.True(scheme.Dark());
        var nothing = new SystemTheme(Answers([]), "linux");
        Assert.False(nothing.Dark());
    }

    [Fact]
    public void Windows_reads_apps_use_light_theme_in_the_registry()
    {
        const string dark = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\r\n    AppsUseLightTheme    REG_DWORD    0x0\r\n";
        Assert.True(SystemTheme.Registry(dark));
        Assert.False(SystemTheme.Registry(dark.Replace("0x0", "0x1")));
        Assert.Null(SystemTheme.Registry("ERROR: The system was unable to find the specified registry key or value."));
        Assert.True(new SystemTheme(Answers(new() { ["reg query"] = dark }), "windows").Dark());
    }

    [Fact]
    public void Macos_reads_the_interface_style_in_the_defaults()
    {
        Assert.True(SystemTheme.Defaults("Dark\n"));
        Assert.False(SystemTheme.Defaults(null));
        Assert.True(new SystemTheme(Answers(new() { ["defaults read"] = "Dark" }), "macos").Dark());
        Assert.False(new SystemTheme(Answers([]), "macos").Dark());
    }

    [Fact]
    public void A_program_that_fails_is_light()
    {
        var failing = new SystemTheme((_, _) => throw new InvalidOperationException("no such program"), "linux");
        Assert.False(failing.Dark());
        Assert.False(new SystemTheme(Answers([]), "plan9").Dark());
    }

    [Fact]
    public void The_theme_shown_is_its_light_or_dark_json_over_clarity_s_own()
    {
        var carried = new Dictionary<string, string>
        {
            ["Themes/night-owl.light.json"] = """{ "dark": false, "colors": { "accent": "#112233" } }""",
            ["Themes/night-owl.dark.json"] = """{ "dark": true, "colors": { "accent": "#445566" } }""",
        };
        var light = AppLook.Of(carried.GetValueOrDefault, "NightOwl", systemDark: false);
        var dark = AppLook.Of(carried.GetValueOrDefault, "NightOwl", systemDark: true);
        Assert.Equal(SKColor.Parse("#112233"), light.Color("accent"));
        Assert.Equal(SKColor.Parse("#445566"), dark.Color("accent"));
        Assert.True(dark.Dark);
        // What the dark one leaves is Clarity's own dark.
        Assert.Equal(ThemeLook.Clarity(dark: true).Color("background"), dark.Color("background"));
        // Without a theme of the project, Clarity's own follows the system.
        Assert.True(AppLook.Of(_ => null, null, systemDark: true).Dark);
        Assert.False(AppLook.Of(_ => null, "Clarity", systemDark: false).Dark);
        // A theme that cannot be read leaves Clarity's own.
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("accent"), AppLook.Of(_ => "{ not json", "NightOwl", systemDark: false).Color("accent"));
        Assert.Equal("night-owl", AppLook.ClassOf("NightOwl"));
    }
}
