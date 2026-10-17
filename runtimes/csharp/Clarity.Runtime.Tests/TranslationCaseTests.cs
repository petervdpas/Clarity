// Which translation an application shows, against the cases every renderer
// shares (conformance/translations.json); the web renderer runs the same
// cases in app/src/lib/translate.test.ts.

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class TranslationCaseTests
{
    [Fact]
    public void TheLanguageIsPickedAsTheCasesSay()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "translations.json");
        var cases = JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("picks").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var c in cases)
        {
            var available = c.GetProperty("available").EnumerateArray().Select(a => a.GetString()!).ToArray();
            var picked = Languages.Pick(c.GetProperty("locale").GetString()!, available, c.GetProperty("base").GetString());
            Assert.True(c.GetProperty("picks").GetString() == picked, $"{c.GetProperty("what")}: {picked}");
        }
    }

    [Fact]
    public void TheSwitchOffersTheBaseLanguageFirstEachByItsOwnName()
    {
        Assert.Equal(["en", "de", "nl"], Languages.Offered("en", ["nl", "de"]));
        Assert.Equal(["en", "nl"], Languages.Offered(null, ["en", "nl"]));
        Assert.Equal("Nederlands", Languages.Name("nl"));
        Assert.Equal("Deutsch", Languages.Name("de"));
    }

    [Fact]
    public void ARememberedChoiceAppliesWhileItsLanguageIsThere()
    {
        Assert.True(Languages.Remembered("nl", "en", ["nl"], out var nl));
        Assert.Equal("nl", nl);
        // The base language shows the texts as written.
        Assert.True(Languages.Remembered("en", "en", ["nl"], out var written));
        Assert.Null(written);
        // A language the project no longer has falls back to the locale.
        Assert.False(Languages.Remembered("fr", "en", ["nl"], out _));
        Assert.False(Languages.Remembered(null, "en", ["nl"], out _));
    }
}
