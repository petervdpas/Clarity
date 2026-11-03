// A window's state as JSON, which a window's undo keeps and takes back: its
// data, its parts and what it chooses from, out as JSON and back into
// another ViewModel of the same window, one made without a database.

using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Runtime.Tests;

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
        if (db is null)
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

    [RelayCommand]
    void ClickAt_Dial(Pointer at) => Handle(() =>
    {
        Dial.Pointer = at;
        Ran.Add($"click {Dial.X},{Dial.Y}");
    }, [at]);
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
    public void A_windows_state_crosses_into_another_of_the_same_window()
    {
        using var db = Sqlite.Memory();
        var server = Server(db);
        var state = Snapshot.Of(server);
        // One made without a database loads nothing itself.
        var copy = new SongsViewModel(null);
        Assert.Empty(copy.Songs);
        Snapshot.Apply(copy, (System.Text.Json.Nodes.JsonObject)System.Text.Json.Nodes.JsonNode.Parse(state.ToJsonString())!);

        Assert.Equal(["Blue", "Red"], copy.Songs.Select(s => s.Title));
        Assert.Equal(2.50m, copy.Songs[1].Price);
        // What it chooses from, though only the constructor sets it.
        Assert.Equal(["", "Blue", "Red"], copy.SongChoices.Select(c => c.Label));
        Assert.Null(copy.SongChoices[0].Value);
        Assert.Equal(2L, copy.SongChoices[2].Value);
        // The selected row is the row of its list, so the grid shows it selected.
        Assert.Same(copy.Songs[1], copy.Grid.SelectedItem);
        Assert.Equal("re", copy.Search.Text);
        Assert.False(copy.Search.IsEnabled);
        Assert.Equal("searchLabel", copy.Search.CaptionText.Key);
        Assert.Equal("tooShort", copy.Search.ErrorText.Key);
        Assert.Equal(3L, copy.Search.ErrorText.Values["min"].Value);
        Assert.Equal([new Shape.Circle(5, 6, 2, "accent")], copy.Dial.Drawing.Take(1));
        Assert.Equal("hi", Assert.IsType<Shape.Words>(copy.Dial.Drawing[1]).Text.Template);
    }

    [Fact]
    public void What_the_person_changed_crosses_back_and_keeps_the_rows()
    {
        using var db = Sqlite.Memory();
        var server = Server(db);
        var copy = new SongsViewModel(null);
        Snapshot.Apply(copy, Snapshot.Of(server));
        // The person types, edits a cell and selects the first row.
        copy.Search.Text = "blue";
        copy.Songs[0].Title = "Navy";
        copy.Grid.SelectedItem = copy.Songs[0];
        Snapshot.Apply(server, Snapshot.Of(copy));
        Assert.Equal("blue", server.Search.Text);
        Assert.Equal("Navy", server.Songs[0].Title);
        Assert.Same(server.Songs[0], server.Grid.SelectedItem);
    }
}
