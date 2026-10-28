// Languages, formats and themes while the application runs: the language
// picked or chosen, texts translated and filled, views rebuilt when the
// language changes, the pickers of the `language` and `theme` click targets,
// and choices kept for the next start.

using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Headless.XUnit;
using Avalonia.Media;
using Avalonia.Styling;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// A window whose title is a text of the project.
sealed class TitledViewModel(Db db) : WindowViewModel(db)
{
    public override string Title => Locale.Translate("customers");
}

public sealed class LocaleTests : IDisposable
{
    /// The project's texts by key: English, the default, and Dutch, which leaves one empty.
    static readonly Dictionary<string, string> English = new()
    {
        ["customers"] = "Customers",
        ["save"] = "Save",
        ["savedOn"] = "Saved {name} on {when}",
        ["saveCount"] = "Save {n}",
        ["yes"] = "Yes",
        ["business"] = "Business",
        ["nameRequired"] = "Name is required",
        ["fullNameHelp"] = "The full name",
        ["onlyEnglish"] = "Only in English",
    };

    static readonly Dictionary<string, string> Dutch = new()
    {
        ["customers"] = "Klanten",
        ["save"] = "Opslaan",
        ["savedOn"] = "{name} opgeslagen op {when}",
        ["saveCount"] = "Bewaar {n}",
        ["yes"] = "Ja",
        ["business"] = "Zakelijk",
        ["nameRequired"] = "Naam is verplicht",
        ["fullNameHelp"] = "De volledige naam",
        ["onlyEnglish"] = "",
    };

    readonly string folder = Directory.CreateTempSubdirectory("clarity-locale-").FullName;

    public LocaleTests()
    {
        Remembered.Folder = folder;
        Start(new CultureInfo("en-US"));
    }

    public void Dispose()
    {
        Locale.Reset();
        Themes.Reset();
        Remembered.Folder = null;
        Directory.Delete(folder, recursive: true);
    }

    static void Start(CultureInfo system) => Locale.Start(
        baseLanguage: "en",
        available: ["nl"],
        currency: "EUR",
        // As .NET resources look them up: a language that leaves a key empty falls back to the default.
        lookup: (key, language) => language.Name == "nl" && Dutch.TryGetValue(key, out var t) && t != "" ? t : English.GetValueOrDefault(key),
        label: (enumName, value) => enumName == "Kind" && value == "Business" ? Locale.Translate("business") : value,
        system: system);

    [AvaloniaFact]
    public void TheSystemLocalePicksTheLanguageAndFormats()
    {
        Assert.Equal("Customers", Locale.Translate("customers"));
        Assert.Equal("en", Locale.Current);
        Start(new CultureInfo("nl-BE"));
        Assert.Equal("Klanten", Locale.Translate("customers"));
        Assert.Equal("nl", Locale.Current);
        Assert.Equal("nl-BE", Locale.Formats.Culture.Name);
        // A key a language leaves empty shows the default's; a text that is no key stays as it is.
        Assert.Equal("Only in English", Locale.Translate("onlyEnglish"));
        Assert.Equal("7", Locale.Translate("7"));
    }

    [AvaloniaFact]
    public void TextIsTranslatedThenFilledInThePersonsFormat()
    {
        Start(new CultureInfo("nl-NL"));
        var saved = Text.Of("savedOn")
            .With("name", Slot.OfText("Ada"))
            .With("when", Slot.OfDate(new DateOnly(2026, 10, 17), "long"));
        Assert.Equal("Ada opgeslagen op 17 oktober 2026", Locale.Show(saved));
        Assert.Equal("Zakelijk, Ja", Locale.Show(new Text("{k}, {b}").With("k", new Slot(SlotKind.Enum, "Business", EnumName: "Kind")).With("b", Slot.OfBoolean(true))));
        Assert.Equal("Ada opgeslagen op 17 oktober 2026", new Message(Level.Info, saved).Shown);
    }

    [AvaloniaFact]
    public void AChosenLanguageShowsItsTextsAndFormatsAndIsKept()
    {
        var changed = 0;
        Locale.Changed += Count;
        void Count() => changed++;
        try
        {
            Locale.Choose("nl");
        }
        finally
        {
            Locale.Changed -= Count;
        }
        Assert.Equal(1, changed);
        Assert.Equal("Klanten", Locale.Translate("customers"));
        Assert.Equal("nl", Locale.Formats.Culture.Name);
        Assert.Equal("nl", CultureInfo.CurrentUICulture.Name);
        // The next start keeps the choice, whatever the system says.
        Start(new CultureInfo("en-US"));
        Assert.Equal("Klanten", Locale.Translate("customers"));
        // Choosing the default language shows its texts, in the system's formats.
        Locale.Choose("en");
        Assert.Equal("Customers", Locale.Translate("customers"));
        Assert.Equal("en-US", Locale.Formats.Culture.Name);
        Start(new CultureInfo("nl-NL"));
        Assert.Equal("Customers", Locale.Translate("customers"));
    }

    [AvaloniaFact]
    public void PartsShowTheirTextsTranslated()
    {
        var save = new PartState("save", caption: "save", help: "fullNameHelp");
        Locale.Choose("nl");
        Assert.Equal("Opslaan", save.Caption);
        Assert.Equal("De volledige naam", save.Shown);
        save.ErrorText = Text.Of("nameRequired");
        Assert.True(save.HasError);
        Assert.Equal("Naam is verplicht", save.Error);
        Assert.Equal("Naam is verplicht", save.Shown);
        save.CaptionText = Text.Of("saveCount").With("n", Slot.OfInteger(1234, null));
        Assert.Equal("Bewaar 1.234", save.Caption);
        // A caption without letters is no key: it shows as written.
        Assert.Equal("7", new PartState("seven", caption: "7").Caption);
        save.ErrorText = Text.Plain("");
        Assert.False(save.HasError);
    }

    [AvaloniaFact]
    public void ALanguageChangeRebuildsTheViewsOfEveryWindow()
    {
        using var db = Sqlite.Memory();
        var built = 0;
        var shell = Shell.Start(new TitledViewModel(db), vm =>
        {
            built++;
            return new ContentControl { DataContext = vm };
        });
        Assert.Equal(1, built);
        Assert.Equal("Customers", shell.Title);
        Locale.Choose("nl");
        Assert.Equal(2, built);
        Assert.Equal("Klanten", shell.Title);
        shell.Close();
        // A closed shell no longer listens.
        Locale.Choose("en");
        Assert.Equal(2, built);
    }

    [AvaloniaFact]
    public void TheLanguagePickerOffersEachLanguageByItsOwnName()
    {
        var button = new Button();
        Pickers.Language(button);
        var items = ((MenuFlyout)button.Flyout!).Items.Cast<MenuItem>().ToList();
        Assert.Equal(["English", "Nederlands"], items.Select(i => (string)i.Header!));
        Assert.True(items[0].IsChecked);
        Assert.False(items[1].IsChecked);

        var menu = new MenuItem();
        Pickers.Language(menu);
        var nl = menu.Items.Cast<MenuItem>().Last();
        nl.Command!.Execute(null);
        Assert.Equal("nl", Locale.Current);
        Assert.True(Remembered.Get("language") == "nl");
    }

    static ResourceDictionary Accent(string color) => new() { ["ClarityAccent"] = new SolidColorBrush(Color.Parse(color)) };

    [AvaloniaFact]
    public void AThemeChoiceSwapsItsResourcesAndIsKept()
    {
        Themes.Start([
            new Theme("Shop", "Shop", ThemeVariant.Default, () => Accent("#2f6fdf")),
            new Theme("Night", "Night", ThemeVariant.Dark, () => Accent("#5b8cff")),
        ]);
        var app = Application.Current!;
        Assert.Equal("Shop", Themes.Current);
        Assert.Equal(Color.Parse("#2f6fdf"), ((SolidColorBrush)app.FindResource("ClarityAccent")!).Color);

        var button = new Button();
        Pickers.Theme(button);
        var items = ((MenuFlyout)button.Flyout!).Items.Cast<MenuItem>().ToList();
        Assert.Equal(["Shop", "Night"], items.Select(i => (string)i.Header!));
        items[1].Command!.Execute(null);
        Assert.Equal("Night", Themes.Current);
        Assert.Equal(ThemeVariant.Dark, app.RequestedThemeVariant);
        Assert.Equal(Color.Parse("#5b8cff"), ((SolidColorBrush)app.FindResource("ClarityAccent")!).Color);
        Assert.Equal("Night", Remembered.Get("theme"));

        // The next start keeps it; a theme the application no longer offers falls back to the first.
        Themes.Start([new Theme("Shop", "Shop", ThemeVariant.Default, () => Accent("#2f6fdf")), new Theme("Night", "Night", ThemeVariant.Dark, () => Accent("#5b8cff"))]);
        Assert.Equal("Night", Themes.Current);
        Themes.Start([new Theme("Shop", "Shop", ThemeVariant.Light, () => Accent("#2f6fdf"))]);
        Assert.Equal("Shop", Themes.Current);
        Assert.Equal(ThemeVariant.Light, app.RequestedThemeVariant);
    }

    [AvaloniaFact]
    public void ValuesShowAndReadInThePersonsFormat()
    {
        Locale.Choose("nl");
        var money = new FormatConverter(new FieldFormat(FormatKind.Money));
        Assert.Equal("€ 1.250,50", money.Convert(1250.5m, typeof(string), null, CultureInfo.InvariantCulture));
        Assert.Equal("", money.Convert(null, typeof(string), null, CultureInfo.InvariantCulture));

        // A number input shows and reads in the person's format; text that is no number is marked.
        var edit = new NumberInput { Format = new FieldFormat(FormatKind.Money), Value = 1250.5m };
        Assert.Equal("1.250,50", edit.Text);
        edit.Text = "2.000,25";
        edit.Take();
        Assert.Equal(2000.25m, edit.Value);
        edit.Text = "abc";
        edit.Take();
        Assert.Equal(2000.25m, edit.Value);
        Assert.Contains("error", edit.Classes);

        var count = new NumberInput { Format = new FieldFormat(FormatKind.Integer) };
        count.Text = "1.234";
        count.Take();
        Assert.Equal(1234L, count.Value);
    }
}
