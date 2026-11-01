// Windows the tests open: a shelf of books, a book, a pick and a window that
// keeps a setting. Clarity.Ui's tests and Clarity.Avalonia's (the mirror
// shell) both use them.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Ui.Tests;

sealed partial class ShelfViewModel(Db? db) : WindowViewModel(db)
{
    public override string Title => "Shelf";

    public List<string> Log { get; set; } = [];
    public PartState Name { get; } = new("name");

    protected override void OnOpen() => Log.Add("opened");

    protected override void OnReturn() => Log.Add("back");

    [RelayCommand]
    void Greet() => Handle(() => Say(Level.Info, Text.Plain("Hello " + Name.Text)));

    [RelayCommand]
    void CopyName() => Handle(() =>
    {
        Copy(Text.Plain("first"));
        Copy(Text.Plain("Name: " + Name.Text));
    });

    [RelayCommand]
    void Detail() => Handle(() => Open(new BookViewModel(Db)));

    [RelayCommand]
    void Pick() => Handle(() => Ask(new PickViewModel(Db), picked => Log.Add($"picked {picked}")));

    [RelayCommand]
    void Remove() => Handle(() => Ask(Text.Plain("Remove it?"), new ConfirmForm(null, null), _ => Log.Add("removed")));
}

sealed partial class BookViewModel(Db? db) : WindowViewModel(db)
{
    /// <summary>The book's cover, from the server's database as a generated window fetches it.</summary>
    public Func<Task<byte[]?>> CoverPicture => () => Fetched(nameof(CoverPicture), () => Db is null ? null : [1, 2, 3]);

    /// <summary>A file of the store, by its path.</summary>
    public Func<string, Task<byte[]?>> CoverFile => path => Fetched(nameof(CoverFile), () => path == "a.png" ? [4] : null, path);

    public override string Title => "Book";

    [RelayCommand]
    void Back() => Handle(Close);
}

sealed partial class PickViewModel(Db? db) : WindowViewModel(db)
{
    public override bool IsDialog => true;

    [RelayCommand]
    void Choose(string what) => Handle(() => CloseWith(what), [what]);
}

/// <summary>A person's settings, one at a time on the server, as a generated Settings is.</summary>
static class Prefs
{
    public static int Speed { get; set; } = 1;

    public static (Func<JsonObject> Of, Action<JsonObject> Apply) Hooks => (() => new JsonObject { ["speed"] = Speed }, sent => Speed = sent["speed"]?.GetValue<int>() ?? Speed);
}

sealed partial class FasterViewModel(Db? db) : WindowViewModel(db)
{
    public int Seen { get; set; }

    [RelayCommand]
    void Faster() => Handle(() =>
    {
        Seen = Prefs.Speed;
        Prefs.Speed++;
    });
}
