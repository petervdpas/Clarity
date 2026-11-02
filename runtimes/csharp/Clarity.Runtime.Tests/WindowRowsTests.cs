// What a window's model does without any screen, as Avalonia's WindowTests,
// SplitTests, PersonKeyTests and MessageTests checked it: a selected row read
// from its part, quit and keys without a desktop, the person's keys kept
// beside the settings, and messages shown again in another language.

using System.Globalization;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

sealed class PickingViewModel(Db? db) : WindowViewModel(db)
{
    public PartState Grid { get; } = new("grid");
    public string Picked() => Selected<string>(Grid);
    public void Quit() => Handle(QuitApplication);
    public void Keys() => Handle(ShowKeys);
}

[Collection(nameof(SaidTextTests))]
public class WindowRowsTests
{
    [Fact]
    public void ASelectedRowIsReadFromItsPart()
    {
        var window = new PickingViewModel(null);
        Assert.Equal("no row of grid is selected", Assert.Throws<InvalidValueException>(window.Picked).Message.Replace("invalid value: ", ""));
        window.Grid.SelectedItem = "b";
        Assert.Equal("b", window.Picked());
    }

    sealed class Recorded : IDesktop
    {
        public List<string> Did { get; } = [];
        public void Quit() => Did.Add("quit");
        public void ShowKeys() => Did.Add("keys");
    }

    [Fact]
    public void Without_the_desktop_quit_and_keys_do_nothing_and_with_it_they_reach_it()
    {
        var before = DesktopProgram.Current;
        try
        {
            DesktopProgram.Current = null;
            var window = new PickingViewModel(null);
            window.Quit();
            window.Keys();
            Assert.Empty(window.Messages);
            var desktop = new Recorded();
            DesktopProgram.Current = desktop;
            window.Quit();
            window.Keys();
            Assert.Equal(["quit", "keys"], desktop.Did);
        }
        finally
        {
            DesktopProgram.Current = before;
        }
    }

    [Fact]
    public void The_persons_keys_are_kept_beside_the_settings()
    {
        var home = Directory.CreateTempSubdirectory("clarity-keys-");
        var was = Environment.GetEnvironmentVariable("XDG_CONFIG_HOME");
        var store = KeptSettings.Store;
        Environment.SetEnvironmentVariable("XDG_CONFIG_HOME", home.FullName);
        KeptSettings.Store = null;
        try
        {
            KeptSettings.Keep("keys-test", new JsonObject { ["pageSize"] = 20 });
            PersonKeys.Start("keys-test");
            Assert.Empty(PersonKeys.Choices);
            PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>> { ["Card.saveButton"] = ["Ctrl+K"] });
            var kept = KeptSettings.Read("keys-test");
            Assert.Equal(20, kept["pageSize"]!.GetValue<int>());
            Assert.Equal("Ctrl+K", kept["keys"]!["Card.saveButton"]![0]!.GetValue<string>());
            PersonKeys.Start("keys-test");
            Assert.Equal(["Ctrl+K"], PersonKeys.Of("Card.saveButton", "Ctrl+S"));
            Assert.Equal(["Ctrl+Q"], PersonKeys.Of("quitItem", "Ctrl+Q"));
        }
        finally
        {
            PersonKeys.Stop();
            KeptSettings.Store = store;
            Environment.SetEnvironmentVariable("XDG_CONFIG_HOME", was);
            home.Delete(true);
        }
    }

    [Fact]
    public void Notes_show_again_in_another_language()
    {
        var shown = new Dictionary<string, string> { ["savedName"] = "Saved {name}" };
        Locale.Start(null, [], null, (key, _) => shown.GetValueOrDefault(key), (_, value) => value, CultureInfo.InvariantCulture);
        try
        {
            var window = new PickingViewModel(null);
            window.Say(Level.Info, Text.Of("savedName").With("name", Slot.OfText("Ada")));
            var note = Assert.Single(window.Messages);
            Assert.Equal("Saved Ada", note.Shown);
            shown["savedName"] = "{name} opgeslagen";
            Assert.Equal("Ada opgeslagen", note.Shown);
        }
        finally
        {
            Locale.Reset();
        }
    }
}
