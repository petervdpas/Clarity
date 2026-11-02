// A text value a handler sets from a key (a status line: `status =
// Board.play(square)` gives @yourMove) shows again in the new language when
// the person picks another one, as long as nothing else changed it since.

using System.Globalization;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

[CollectionDefinition(nameof(SaidTextTests), DisableParallelization = true)]
public sealed class SaidTextAlone;

sealed partial class StatusViewModel(Db? db) : WindowViewModel(db)
{
    public string Status { get => status; set => SetProperty(ref status, value); }
    string status = "";

    public void Play(Text said) => ShowText(nameof(Status), said, shown => Status = shown);
}

[Collection(nameof(SaidTextTests))]
public class SaidTextTests
{
    static readonly Dictionary<string, string> Dutch = new() { ["yourMove"] = "Jouw zet", ["youWin"] = "Je wint" };
    static readonly Dictionary<string, string> English = new() { ["yourMove"] = "Your move", ["youWin"] = "You win" };

    static Dictionary<string, string> shown = English;

    static void Start() =>
        Locale.Start(null, [], null, (key, _) => shown.GetValueOrDefault(key), (_, value) => value, CultureInfo.InvariantCulture);

    [Fact]
    public void A_said_key_shows_again_in_the_new_language()
    {
        try
        {
            shown = English;
            Start();
            var window = new StatusViewModel(null);
            window.Play(Text.Of("yourMove"));
            Assert.Equal("Your move", window.Status);
            shown = Dutch;
            window.Reshow();
            Assert.Equal("Jouw zet", window.Status);
            // A text that is no key stays as it is.
            window.Play(Text.Of("Hello"));
            window.Reshow();
            Assert.Equal("Hello", window.Status);
        }
        finally
        {
            Locale.Reset();
        }
    }

    [Fact]
    public void A_value_changed_since_keeps_what_it_is()
    {
        try
        {
            shown = English;
            Start();
            var window = new StatusViewModel(null);
            window.Play(Text.Of("youWin"));
            window.Status = "typed by the person";
            shown = Dutch;
            window.Reshow();
            Assert.Equal("typed by the person", window.Status);
        }
        finally
        {
            Locale.Reset();
        }
    }
}
