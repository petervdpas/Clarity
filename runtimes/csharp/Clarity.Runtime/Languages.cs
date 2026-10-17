// Which language an application shows its texts in (DESIGN section 1.5,
// I1c), by the same rules as the web renderer (app/src/lib/translate.ts):
// the person's locale picks among the languages the project has
// translations for; the language chosen with the `language` click target
// wins, and is kept for the next start.

using System.Globalization;

namespace Clarity.Runtime;

public static class Languages
{
    static string Primary(string tag) => tag.Split('-')[0].ToLowerInvariant();

    /// <summary>
    /// The language whose translations to show for <paramref name="locale"/>,
    /// among those the project has; null to show the texts as written. The
    /// exact tag wins, then the base language, then the language without its
    /// region, then any region of the same language.
    /// </summary>
    public static string? Pick(string locale, IReadOnlyList<string> available, string? baseLanguage)
    {
        if (available.Contains(locale))
            return locale;
        var language = Primary(locale);
        if (language == Primary(baseLanguage ?? "en"))
            return null;
        return available.FirstOrDefault(a => a == language) ?? available.FirstOrDefault(a => Primary(a) == language);
    }

    /// <summary>The languages to switch between: the one the texts are written in first, then those translated.</summary>
    public static string[] Offered(string? baseLanguage, IReadOnlyList<string> available)
    {
        var written = baseLanguage ?? "en";
        return [written, .. available.Where(a => a != written).Order(StringComparer.Ordinal)];
    }

    /// <summary>A language named in itself, as people look for it: <c>nl</c> is Nederlands.</summary>
    public static string Name(string tag)
    {
        try
        {
            var culture = CultureInfo.GetCultureInfo(tag);
            var name = culture.NativeName;
            return name.Length == 0 ? tag : char.ToUpper(name[0], culture) + name[1..];
        }
        catch (CultureNotFoundException)
        {
            return tag;
        }
    }

    /// <summary>
    /// A language chosen in an earlier run: whether it still applies, and the
    /// language to show (null for the texts as written). It does not apply when
    /// nothing was chosen or the project no longer has that language; then the
    /// locale picks.
    /// </summary>
    public static bool Remembered(string? stored, string? baseLanguage, IReadOnlyList<string> available, out string? language)
    {
        language = null;
        if (string.IsNullOrEmpty(stored))
            return false;
        if (stored == (baseLanguage ?? "en"))
            return true;
        if (!available.Contains(stored))
            return false;
        language = stored;
        return true;
    }
}
