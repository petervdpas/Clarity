// The language and formats the application shows (DESIGN section 1.5), by
// the rules of the web renderer (app/src/lib/model.ts): the system's locale
// picks the translations and formats values; a language chosen with the
// `language` click target shows its texts and its formats, and is kept for
// the next start. Texts are looked up by their keys, through the
// application's generated `Texts` (its .resx files): a language that leaves
// a key empty shows the default language's value.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public static class Locale
{
    const string Key = "language";

    static string? baseLanguage;
    static string[] available = [];
    static string? currency;
    static Func<string, CultureInfo, string?> lookup = (_, _) => null;
    static Func<string, string, string> label = (_, value) => value;
    static CultureInfo system = CultureInfo.CurrentCulture;
    static bool chosen;
    static string? choice;

    /// <summary>The language shown changed: views show their texts and values again.</summary>
    public static event Action? Changed;

    /// <summary>The language whose translations are shown; null shows the texts as written.</summary>
    public static CultureInfo? Language { get; private set; }

    /// <summary>How values are shown and read.</summary>
    public static Formats Formats { get; private set; } = new(CultureInfo.CurrentCulture, null);

    /// <summary>The languages the application offers, the one its texts are written in first.</summary>
    public static string[] Offered => Languages.Offered(baseLanguage, available);

    /// <summary>The language the texts are shown in.</summary>
    public static string Current => Language?.Name ?? baseLanguage ?? "en";

    /// <summary>
    /// Starts with the project's languages: <paramref name="lookup"/> gives a
    /// text in a language, or null when it has no translation there;
    /// <paramref name="label"/> gives the label of an enum value by the enum's
    /// name. A choice kept from an earlier run wins over the system's locale.
    /// </summary>
    public static void Start(
        string? baseLanguage,
        string[] available,
        string? currency,
        Func<string, CultureInfo, string?> lookup,
        Func<string, string, string> label,
        CultureInfo? system = null)
    {
        Locale.baseLanguage = baseLanguage;
        Locale.available = available;
        Locale.currency = currency;
        Locale.lookup = lookup;
        Locale.label = label;
        Locale.system = system ?? CultureInfo.CurrentCulture;
        chosen = Languages.Remembered(Remembered.Get(Key), baseLanguage, available, out choice);
        Apply();
        Text.KeyValue = key => lookup(key, Language ?? CultureInfo.InvariantCulture);
    }

    /// <summary>Back to the texts as written, in the culture of the running thread: for tests.</summary>
    public static void Reset() => Start(null, [], null, (_, _) => null, (_, value) => value, CultureInfo.InvariantCulture);

    /// <summary>Shows <paramref name="language"/>, and keeps it for the next start.</summary>
    public static void Choose(string language)
    {
        chosen = true;
        choice = language == (baseLanguage ?? "en") ? null : language;
        Remembered.Set(Key, language);
        Apply();
        Changed?.Invoke();
    }

    static void Apply()
    {
        var picked = chosen ? choice : Languages.Pick(system.Name, available, baseLanguage);
        Language = picked is null ? null : CultureInfo.GetCultureInfo(picked);
        // A chosen language shows its own formats too.
        var culture = chosen && choice is not null ? CultureInfo.GetCultureInfo(choice) : system;
        Formats = new Formats(culture, currency);
        // Controls that format by themselves (a date picker) follow.
        CultureInfo.CurrentCulture = culture;
        CultureInfo.CurrentUICulture = Language ?? culture;
        CultureInfo.DefaultThreadCurrentCulture = culture;
        CultureInfo.DefaultThreadCurrentUICulture = Language ?? culture;
    }

    /// <summary>
    /// The value of a key in the language shown, or the default language's
    /// when that one leaves it empty; a text that is no key stays as it is.
    /// </summary>
    public static string Translate(string keyOrText) =>
        lookup(keyOrText, Language ?? CultureInfo.InvariantCulture) is { Length: > 0 } value ? value : keyOrText;

    /// <summary>One of Clarity's own words by its key, or <paramref name="fallback"/> when the project gives it no value.</summary>
    public static string Translate(string key, string fallback) => Translate(key) is var value && value != key ? value : fallback;

    /// <summary>Text as shown: a key by its value in the language shown, each slot in the person's format.</summary>
    public static string Show(Text text) =>
        Formats.Fill(text.WithTemplate(text.Key is { } key ? (Translate(key) is var value && value != key ? value : Unkept(text, key)) : text.Template), label, Translate);

    /// <summary>A key the project gives no value: Clarity's own words read as their English, written as the template; any other key as itself.</summary>
    static string Unkept(Text text, string key) => text.Template is { Length: > 0 } english ? english : Text.Plain(key).Template;
}
