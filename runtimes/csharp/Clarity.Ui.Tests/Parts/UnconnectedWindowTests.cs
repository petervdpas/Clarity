// The databases did not open (DESIGN 5.27): the window that keeps the
// connection shows first, or what the app's `on unconnected` opens; closing it
// connects again when a setting changed, and ends the application otherwise.
// The screen here is a fake that records what the flow asks of it.

using System.ComponentModel;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

public sealed class UnconnectedWindowTests
{
    sealed class Setting : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler? PropertyChanged;
        public void Change() => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs("shopDb"));
    }

    sealed class Card(Db db, string title) : WindowViewModel(db)
    {
        public override string Title => title;
    }

    sealed class HostViewModel(Db db) : WindowViewModel(db)
    {
        public void Show(WindowViewModel other) => Open(other);
    }

    /// <summary>What the flow showed, said, connected and ended, as a host would carry it out.</summary>
    sealed class Screen : IUnconnectedScreen
    {
        public readonly List<(WindowViewModel First, List<Message> Said, Action Closed)> Shown = [];
        public readonly List<(Db Db, WindowViewModel Main)> Connected = [];
        public int Ended;

        public void Show(WindowViewModel first, IReadOnlyList<Message> said, Action whenClosed) => Shown.Add((first, said.ToList(), whenClosed));
        void IUnconnectedScreen.Connected(Db db, WindowViewModel main) => Connected.Add((db, main));
        void IUnconnectedScreen.Ended() => Ended++;
    }

    sealed class Run
    {
        public readonly Screen Screen = new();
        public readonly Setting Settings = new();
        public readonly List<Db> Empties = [];
        public int Connects;

        public Unconnected Flow(Func<Db> connect, Action<Db, Unconnecting>? handler = null, bool window = true) => new()
        {
            Connect = () =>
            {
                Connects++;
                return connect();
            },
            Empty = () =>
            {
                var db = Db.Empty();
                Empties.Add(db);
                return db;
            },
            Main = db => new Card(db, "Main"),
            Window = window ? db => new Card(db, "Preferences") : null,
            Handler = handler,
            Settings = Settings,
        };
    }

    static Db Opened() => Db.Empty();

    [Fact]
    public void The_window_that_keeps_the_connection_says_why_and_closing_it_unchanged_ends()
    {
        var run = new Run();
        run.Flow(Opened).Start(Text.Plain("the secret setting shopDb is not kept yet"), run.Screen);
        var shown = Assert.Single(run.Screen.Shown);
        Assert.Equal("Preferences", shown.First.Title);
        Assert.Equal(["the secret setting shopDb is not kept yet"], shown.Said.Select(m => m.Shown));
        Assert.Equal(Level.Error, shown.Said[0].Level);
        shown.Closed();
        Assert.Equal((1, 0), (run.Screen.Ended, run.Connects));
        // The empty database in memory is let go with the window.
        Assert.True(Assert.Single(run.Empties).Disposed);
    }

    [Fact]
    public void A_changed_setting_connects_again_and_the_main_window_opens()
    {
        var run = new Run();
        var tries = 0;
        run.Flow(() => ++tries == 1 ? throw new StorageException("still not kept") : Opened()).Start(Text.Plain("not kept"), run.Screen);
        run.Settings.Change();
        run.Screen.Shown[0].Closed();
        // Still not: the window shows again, saying why.
        Assert.Equal(2, run.Screen.Shown.Count);
        // In Clarity's own words, which the project translates.
        var said = Assert.Single(run.Screen.Shown[1].Said).Text;
        Assert.Equal(("unconnectedFailed", "still not kept"), (said.Key, said.Values["problem"].Value));
        run.Settings.Change();
        run.Screen.Shown[1].Closed();
        var connected = Assert.Single(run.Screen.Connected);
        Assert.Equal("Main", connected.Main.Title);
        Assert.Equal((0, 2), (run.Screen.Ended, run.Connects));
    }

    [Fact]
    public void A_setting_changed_after_the_window_closed_does_not_count()
    {
        var run = new Run();
        run.Flow(Opened).Start(Text.Plain("not kept"), run.Screen);
        run.Screen.Shown[0].Closed();
        run.Settings.Change();
        Assert.Equal((1, 0), (run.Screen.Ended, run.Connects));
    }

    [Fact]
    public void The_apps_handler_opens_what_shows_first_with_why()
    {
        var run = new Run();
        string? given = null;
        run.Flow(Opened, (db, unconnecting) =>
        {
            var host = new HostViewModel(db);
            host.RunUnconnected(unconnecting, reason =>
            {
                given = reason;
                host.Show(new Card(db, "Connect"));
            });
        }).Start(Text.Plain("server down"), run.Screen);
        Assert.Equal("server down", given);
        var shown = Assert.Single(run.Screen.Shown);
        Assert.Equal("Connect", shown.First.Title);
        Assert.Empty(shown.Said);
    }

    [Fact]
    public void A_handler_that_opens_nothing_leaves_it_to_the_window_that_keeps_the_connection()
    {
        var run = new Run();
        run.Flow(Opened, (db, unconnecting) => new HostViewModel(db).RunUnconnected(unconnecting, _ => { })).Start(Text.Plain("server down"), run.Screen);
        var shown = Assert.Single(run.Screen.Shown);
        Assert.Equal("Preferences", shown.First.Title);
        Assert.Equal(["server down"], shown.Said.Select(m => m.Shown));
    }

    [Fact]
    public void Without_a_window_to_show_the_application_ends()
    {
        var run = new Run();
        run.Flow(Opened, window: false).Start(Text.Plain("server down"), run.Screen);
        Assert.Empty(run.Screen.Shown);
        Assert.Equal(1, run.Screen.Ended);
        Assert.True(Assert.Single(run.Empties).Disposed);
    }

    [Fact]
    public void Why_is_in_Clarity_own_words()
    {
        var why = Unconnected.Reason(new StorageException("the server did not answer"));
        Assert.Equal(("unconnectedFailed", "the server did not answer"), (why.Key, why.Values["problem"].Value));
    }
}
