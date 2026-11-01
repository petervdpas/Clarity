// The databases did not open (DESIGN 5.27): the window that keeps the
// connection shows first, or what the app's `on unconnected` opens; closing it
// connects again when a setting changed, and ends the application otherwise.

using System.ComponentModel;
using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class UnconnectedTests : IDisposable
{
    readonly List<Run> runs = [];

    /// <summary>Every desktop window a test left open closes, so the window cases count only their own.</summary>
    public void Dispose()
    {
        foreach (var shell in runs.SelectMany(r => r.Shown).Where(s => s.IsVisible).ToList())
            shell.Close();
    }

    sealed class Setting : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged;
        public void Change() => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs("shopDb"));
    }

    sealed class HostViewModel(Db db) : WindowViewModel(db)
    {
        public void Show(WindowViewModel other) => Open(other);
    }

    Run Track(Run run)
    {
        runs.Add(run);
        return run;
    }

    sealed class Run
    {
        public readonly List<Shell> Shown = [];
        public int Ended, Connects, Connected;
        public readonly Setting Settings = new();

        public Unconnected Start(Func<Db> connect, Action<Db, Unconnecting>? handler = null, bool window = true) => new()
        {
            Connect = () =>
            {
                Connects++;
                return connect();
            },
            Main = db => new CardViewModel(db, "Main"),
            Window = window ? db => new CardViewModel(db, "Preferences") : null,
            Handler = handler,
            View = _ => new TextBlock(),
            Settings = Settings,
            Shown = Shown.Add,
            Connected = _ => Connected++,
            Ended = () => Ended++,
        };
    }

    [AvaloniaFact]
    public void The_window_that_keeps_the_connection_says_why_and_closing_it_unchanged_ends()
    {
        var run = Track(new Run());
        run.Start(Sqlite.Memory).Start(Text.Plain("the secret setting shopDb is not kept yet"));
        var shell = Assert.Single(run.Shown);
        Assert.Equal("Preferences", shell.Current!.Title);
        Assert.Equal(["the secret setting shopDb is not kept yet"], shell.Messages.Select(m => m.Shown));
        shell.Close();
        Assert.Equal((1, 0), (run.Ended, run.Connects));
    }

    [AvaloniaFact]
    public void A_changed_setting_connects_again_and_the_main_window_opens()
    {
        var run = Track(new Run());
        var tries = 0;
        run.Start(() => ++tries == 1 ? throw new StorageException("still not kept") : Sqlite.Memory()).Start(Text.Plain("not kept"));
        run.Settings.Change();
        run.Shown[0].Close();
        // Still not: the window shows again, saying why.
        Assert.Equal(2, run.Shown.Count);
        // In Clarity's own words, which the project translates.
        var said = Assert.Single(run.Shown[1].Messages).Text;
        Assert.Equal(("unconnectedFailed", "still not kept"), (said.Key, said.Values["problem"].Value));
        run.Settings.Change();
        run.Shown[1].Close();
        Assert.Equal("Main", run.Shown[^1].Current!.Title);
        Assert.Equal((0, 2, 1), (run.Ended, run.Connects, run.Connected));
    }

    [AvaloniaFact]
    public void The_apps_handler_opens_what_shows_first_with_why()
    {
        var run = Track(new Run());
        string? given = null;
        run.Start(Sqlite.Memory, (db, unconnecting) =>
        {
            var host = new HostViewModel(db);
            host.RunUnconnected(unconnecting, reason =>
            {
                given = reason;
                host.Show(new CardViewModel(db, "Connect"));
            });
        }).Start(Text.Plain("server down"));
        Assert.Equal("server down", given);
        Assert.Equal("Connect", Assert.Single(run.Shown).Current!.Title);
        Assert.Empty(run.Shown[0].Messages);
    }

    [AvaloniaFact]
    public void Without_a_window_to_show_the_application_ends()
    {
        var run = Track(new Run());
        run.Start(Sqlite.Memory, window: false).Start(Text.Plain("server down"));
        Assert.Empty(run.Shown);
        Assert.Equal(1, run.Ended);
    }
}
