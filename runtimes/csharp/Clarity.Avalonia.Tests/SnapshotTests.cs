// A window's state as it crosses between the server, where its handlers
// run, and the browser, which shows it (DESIGN 7.6): its data, its parts
// and what it chooses from, out as JSON and back into another ViewModel of
// the same window, a mirror made without a database.

using Clarity.Avalonia;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed partial class Song : ObservableObject
{
    public long Id { get; set; }
    public string Title { get => title; set => SetProperty(ref title, value); }
    string title = "";
    public decimal Price { get; set; }
}

sealed partial class SongsViewModel : WindowViewModel
{
    public SongsViewModel(Db? db) : base(db)
    {
        if (IsMirror)
            return;
        _songs = [new Song { Id = 1, Title = "Blue", Price = 1.25m }, new Song { Id = 2, Title = "Red", Price = 2.50m }];
        SongChoices = [new Choice(null, ""), .. _songs.Select(s => new Choice(s.Id, s.Title))];
    }

    public override string Title => "Songs";

    public List<Song> Songs { get => _songs; set => SetProperty(ref _songs, value); }
    List<Song> _songs = [];

    public Choice[] SongChoices { get; } = [];

    public PartState Grid { get; } = new("grid");
    public PartState Search { get; } = new("search", caption: "searchLabel");
    public PartState Dial { get; } = new("dial");

    public List<string> Ran { get; } = [];

    [RelayCommand]
    void Find() => Handle(OnFind);

    void OnFind() => Ran.Add("find " + Search.Text);

    /// <summary>What an engine beside the screen told, as a generated window hears it.</summary>
    [Command]
    public void Told_Dial(string tell, System.Text.Json.JsonElement[] values) => Handle(() => Ran.Add($"told {tell} {values[0].GetInt64()}"), [tell, values]);

    [RelayCommand]
    void ClickAt_Dial(Pointer at) => Handle(() =>
    {
        Dial.Pointer = at;
        Ran.Add($"click {Dial.X},{Dial.Y}");
    }, [at]);
}

/// <summary>What a mirror sends instead of running a handler.</summary>
sealed class Sent : IRemote
{
    public List<(string Command, object?[] Args)> Commands { get; } = [];

    public void Command(WindowViewModel window, string command, object?[] args) => Commands.Add((command, args));

    public Task<byte[]?> Fetch(WindowViewModel window, string fetcher, object?[] args) => Task.FromResult<byte[]?>(null);
}

public class SnapshotTests
{
    static SongsViewModel Server(Db db)
    {
        var songs = new SongsViewModel(db);
        songs.Grid.SelectedItem = songs.Songs[1];
        songs.Search.Text = "re";
        songs.Search.IsEnabled = false;
        songs.Search.ErrorText = Text.Of("tooShort").With("min", Slot.OfInteger(3, null));
        songs.Dial.BeginDrawing();
        songs.Dial.Draw(new Shape.Circle(5, 6, 2, "accent"));
        songs.Dial.Draw(new Shape.Words(1, 2, Text.Plain("hi")));
        songs.Dial.EndDrawing();
        return songs;
    }

    [Fact]
    public void A_windows_state_crosses_into_a_mirror_of_it()
    {
        using var db = Db.Memory();
        var server = Server(db);
        var state = Snapshot.Of(server);
        // A mirror is made without a database and loads nothing itself.
        var mirror = new SongsViewModel(null);
        Assert.True(mirror.IsMirror);
        Assert.Empty(mirror.Songs);
        Snapshot.Apply(mirror, Snapshot.Parse(state.ToJsonString()));

        Assert.Equal(["Blue", "Red"], mirror.Songs.Select(s => s.Title));
        Assert.Equal(2.50m, mirror.Songs[1].Price);
        // What it chooses from, though only the constructor sets it.
        Assert.Equal(["", "Blue", "Red"], mirror.SongChoices.Select(c => c.Label));
        Assert.Null(mirror.SongChoices[0].Value);
        Assert.Equal(2L, mirror.SongChoices[2].Value);
        // The selected row is the row of its list, so the grid shows it selected.
        Assert.Same(mirror.Songs[1], mirror.Grid.SelectedItem);
        Assert.Equal("re", mirror.Search.Text);
        Assert.False(mirror.Search.IsEnabled);
        Assert.Equal("searchLabel", mirror.Search.CaptionText.Key);
        Assert.Equal("tooShort", mirror.Search.ErrorText.Key);
        Assert.Equal(3L, mirror.Search.ErrorText.Values["min"].Value);
        Assert.Equal([new Shape.Circle(5, 6, 2, "accent")], mirror.Dial.Drawing.Take(1));
        Assert.Equal("hi", Assert.IsType<Shape.Words>(mirror.Dial.Drawing[1]).Text.Template);
    }

    [Fact]
    public void What_the_person_changed_crosses_back_and_keeps_the_rows()
    {
        using var db = Db.Memory();
        var server = Server(db);
        var mirror = new SongsViewModel(null);
        Snapshot.Apply(mirror, Snapshot.Of(server));
        // The person types, edits a cell and selects the first row.
        mirror.Search.Text = "blue";
        mirror.Songs[0].Title = "Navy";
        mirror.Grid.SelectedItem = mirror.Songs[0];
        Snapshot.Apply(server, Snapshot.Of(mirror));
        Assert.Equal("blue", server.Search.Text);
        Assert.Equal("Navy", server.Songs[0].Title);
        Assert.Same(server.Songs[0], server.Grid.SelectedItem);
    }

    [Fact]
    public void What_a_handler_tells_a_part_crosses_to_the_mirror_once()
    {
        using var db = Db.Memory();
        var server = new SongsViewModel(db);
        server.Dial.Hear("play");
        server.Dial.Hear("melody", "C4 E4", 120L);
        var mirror = new SongsViewModel(null);
        var heard = new List<string>();
        mirror.Dial.Listen((_, _) => { }, (hear, values) => heard.Add($"{hear} {string.Join(",", values)}"));
        Snapshot.Apply(mirror, Snapshot.Of(server, told: true));
        Assert.Equal(["play ", "melody C4 E4,120"], heard);
        // Told once: the next reply tells it nothing again.
        Snapshot.Apply(mirror, Snapshot.Of(server, told: true));
        Assert.Equal(2, heard.Count);
    }

    [Fact]
    public void A_mirror_sends_its_commands_instead_of_running_them()
    {
        var mirror = new SongsViewModel(null);
        var sent = new Sent();
        mirror.Remote = sent;
        mirror.FindCommand.Execute(null);
        mirror.ClickAt_DialCommand.Execute(new Pointer(3, 4));
        Assert.Empty(mirror.Ran);
        Assert.Equal(["Find", "ClickAt_Dial"], sent.Commands.Select(c => c.Command));
        Assert.Equal(new Pointer(3, 4), sent.Commands[1].Args[0]);
    }

    [Fact]
    public void The_server_runs_a_command_by_its_name()
    {
        using var db = Db.Memory();
        var server = Server(db);
        Snapshot.Run(server, "Find", []);
        Snapshot.Run(server, "ClickAt_Dial", [new Pointer(7, 8)]);
        Assert.Equal(["find re", "click 7,8"], server.Ran);
        // What an engine told, by the method that hears it, with its values as the mirror sends them.
        var told = Snapshot.Arguments([(object?)"hit", System.Text.Json.JsonSerializer.SerializeToElement(new[] { 30 }).EnumerateArray().ToArray()]);
        Snapshot.Run(server, "Told_Dial", Snapshot.Arguments(server, "Told_Dial", System.Text.Json.Nodes.JsonNode.Parse(told.ToJsonString())!.AsArray()));
        Assert.Equal("told hit 30", server.Ran[^1]);
        Assert.Throws<SessionException>(() => Snapshot.Run(server, "Nothing", []));
    }
}
