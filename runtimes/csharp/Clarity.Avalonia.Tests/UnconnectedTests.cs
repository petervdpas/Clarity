// The databases did not open (DESIGN 5.27): the window that keeps the
// connection shows first, or what the app's `on unconnected` opens; closing it
// connects again when a setting changed, and ends the application otherwise.

using System.ComponentModel;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class UnconnectedTests
{
    sealed class Setting : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged;
        public void Change() => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs("shopDb"));
    }

    sealed class HostViewModel(Db db) : WindowViewModel(db)
    {
        public void Show(WindowViewModel other) => Open(other);
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
        var run = new Run();
        run.Start(Db.Memory).Start("the secret setting shopDb is not kept yet");
        var shell = Assert.Single(run.Shown);
        Assert.Equal("Preferences", shell.Current!.Title);
        Assert.Equal(["the secret setting shopDb is not kept yet"], shell.Messages.Select(m => m.Shown));
        shell.Close();
        Assert.Equal((1, 0), (run.Ended, run.Connects));
    }

    [AvaloniaFact]
    public void A_changed_setting_connects_again_and_the_main_window_opens()
    {
        var run = new Run();
        var tries = 0;
        run.Start(() => ++tries == 1 ? throw new StorageException("still not kept") : Db.Memory()).Start("not kept");
        run.Settings.Change();
        run.Shown[0].Close();
        // Still not: the window shows again, saying why.
        Assert.Equal(2, run.Shown.Count);
        Assert.Equal(["storage: still not kept"], run.Shown[1].Messages.Select(m => m.Shown));
        run.Settings.Change();
        run.Shown[1].Close();
        Assert.Equal("Main", run.Shown[^1].Current!.Title);
        Assert.Equal((0, 2, 1), (run.Ended, run.Connects, run.Connected));
    }

    [AvaloniaFact]
    public void The_apps_handler_opens_what_shows_first_with_why()
    {
        var run = new Run();
        string? given = null;
        run.Start(Db.Memory, (db, unconnecting) =>
        {
            var host = new HostViewModel(db);
            host.RunUnconnected(unconnecting, reason =>
            {
                given = reason;
                host.Show(new CardViewModel(db, "Connect"));
            });
        }).Start("server down");
        Assert.Equal("server down", given);
        Assert.Equal("Connect", Assert.Single(run.Shown).Current!.Title);
        Assert.Empty(run.Shown[0].Messages);
    }

    [AvaloniaFact]
    public void Without_a_window_to_show_the_application_ends()
    {
        var run = new Run();
        run.Start(Db.Memory, window: false).Start("server down");
        Assert.Empty(run.Shown);
        Assert.Equal(1, run.Ended);
    }
}
