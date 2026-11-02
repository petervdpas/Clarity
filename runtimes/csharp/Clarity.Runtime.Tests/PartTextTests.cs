// A part's caption, help and error are kept by their keys and shown in the
// person's language, their slots in the person's format (Avalonia's
// LocaleTests.PartsShowTheirTextsTranslated, now on the runtime's PartState).

using System.Globalization;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

[Collection(nameof(SaidTextTests))]
public sealed class PartTextTests : IDisposable
{
    static readonly Dictionary<string, string> English = new()
    {
        ["save"] = "Save",
        ["saveCount"] = "Save {n}",
        ["nameRequired"] = "Name is required",
        ["fullNameHelp"] = "The full name",
    };

    static readonly Dictionary<string, string> Dutch = new()
    {
        ["save"] = "Opslaan",
        ["saveCount"] = "Bewaar {n}",
        ["nameRequired"] = "Naam is verplicht",
        ["fullNameHelp"] = "De volledige naam",
    };

    readonly string folder = Directory.CreateTempSubdirectory("clarity-part-texts-").FullName;

    public PartTextTests()
    {
        Remembered.Folder = folder;
        Locale.Start("en", ["nl"], "EUR",
            (key, language) => language.Name == "nl" && Dutch.TryGetValue(key, out var t) ? t : English.GetValueOrDefault(key),
            (_, value) => value, new CultureInfo("en-US"));
    }

    public void Dispose()
    {
        Locale.Reset();
        Remembered.Folder = null;
        Directory.Delete(folder, recursive: true);
    }

    [Fact]
    public void PartsShowTheirTextsTranslated()
    {
        var save = new PartState("save", caption: "save", help: "fullNameHelp");
        Assert.Equal("Save", save.Caption);
        Assert.Equal("The full name", save.Shown);
        Locale.Choose("nl");
        Assert.Equal("Opslaan", save.Caption);
        Assert.Equal("De volledige naam", save.Shown);
        save.ErrorText = Text.Of("nameRequired");
        Assert.True(save.HasError);
        Assert.Equal("Naam is verplicht", save.Error);
        // An error shows before the help.
        Assert.Equal("Naam is verplicht", save.Shown);
        save.CaptionText = Text.Of("saveCount").With("n", Slot.OfInteger(1234, null));
        Assert.Equal("Bewaar 1.234", save.Caption);
        // What code set by a key follows the language the person picks next.
        Locale.Choose("en");
        Assert.Equal("Save 1,234", save.Caption);
        Assert.Equal("Name is required", save.Error);
        // A caption without letters is no key: it shows as written.
        Assert.Equal("7", new PartState("seven", caption: "7").Caption);
        save.ErrorText = Text.Plain("");
        Assert.False(save.HasError);
        Assert.Equal("The full name", save.Shown);
    }
}
