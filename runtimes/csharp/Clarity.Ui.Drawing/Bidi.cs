// The Unicode bidirectional algorithm (UAX #9) for one line of text, as
// Avalonia's TextFormatter ran it: the paragraph's direction from its first
// strong letter, the implicit levels of every character (rules W1 to W7, N1,
// N2, I1, I2), the line's end reset (L1), and the visual order of runs of
// one level (L2). Hebrew and Arabic run right to left, Latin and numbers
// left to right inside them. Explicit embedding and isolate marks (U+202A to
// U+202E, U+2066 to U+2069) are left out (rule X9), so they change nothing;
// the marks LRM, RLM and ALM count as strong letters.

using System.Globalization;
using System.Text;

namespace Clarity.Ui;

/// <summary>The bidirectional class of a character (UAX #9, table 4), as far as Clarity.UI needs it.</summary>
public enum BidiClass : byte
{
    L, R, AL, EN, ES, ET, AN, CS, NSM, BN, B, S, WS, ON,
}

public static class Bidi
{
    /// <summary>The bidirectional class of <paramref name="character"/>.</summary>
    public static BidiClass ClassOf(int character)
    {
        switch (character)
        {
            case 0x200E:
                return BidiClass.L;
            case 0x200F:
                return BidiClass.R;
            case 0x061C:
                return BidiClass.AL;
            case '\n' or '\r' or 0x1C or 0x1D or 0x1E or 0x85 or 0x2029:
                return BidiClass.B;
            case '\t' or 0x0B or 0x1F:
                return BidiClass.S;
            case ' ' or 0x0C or 0x1680 or 0x2028 or 0x205F or 0x3000:
                return BidiClass.WS;
            case >= 0x2000 and <= 0x200A:
                return BidiClass.WS;
            case >= '0' and <= '9' or >= 0x06F0 and <= 0x06F9 or >= 0xFF10 and <= 0xFF19
                or 0xB2 or 0xB3 or 0xB9 or 0x2070 or >= 0x2074 and <= 0x2079 or >= 0x2080 and <= 0x2089:
                return BidiClass.EN;
            case >= 0x0600 and <= 0x0605 or >= 0x0660 and <= 0x0669 or 0x066B or 0x066C or 0x06DD or 0x0890 or 0x0891 or 0x08E2:
                return BidiClass.AN;
            case '+' or '-' or 0x207A or 0x207B or 0x208A or 0x208B or 0x2212 or 0xFB29 or 0xFE62 or 0xFE63 or 0xFF0B or 0xFF0D:
                return BidiClass.ES;
            case '#' or '$' or '%' or 0xB0 or 0xB1 or 0x0609 or 0x060A or 0x066A or 0x2030 or 0x2031 or 0x2032 or 0x2033 or 0x2034
                or 0x212E or 0x2213 or 0xFE5F or 0xFE69 or 0xFE6A or 0xFF03 or 0xFF04 or 0xFF05:
                return BidiClass.ET;
            case ',' or '.' or '/' or ':' or 0xA0 or 0x060C or 0x202F or 0x2044 or 0xFE50 or 0xFE52 or 0xFE55 or 0xFF0C or 0xFF0E or 0xFF0F or 0xFF1A:
                return BidiClass.CS;
        }
        if (character > 0x10FFFF || character is >= 0xD800 and <= 0xDFFF)
            return BidiClass.ON;
        var category = CharUnicodeInfo.GetUnicodeCategory(character);
        switch (category)
        {
            case UnicodeCategory.NonSpacingMark or UnicodeCategory.EnclosingMark:
                return BidiClass.NSM;
            case UnicodeCategory.Control or UnicodeCategory.Format:
                return BidiClass.BN;
            case UnicodeCategory.CurrencySymbol:
                return BidiClass.ET;
        }
        if (IsArabicLetter(character))
            return BidiClass.AL;
        if (IsRightToLeft(character))
            return BidiClass.R;
        return category switch
        {
            UnicodeCategory.UppercaseLetter or UnicodeCategory.LowercaseLetter or UnicodeCategory.TitlecaseLetter
                or UnicodeCategory.ModifierLetter or UnicodeCategory.OtherLetter or UnicodeCategory.SpacingCombiningMark
                or UnicodeCategory.DecimalDigitNumber or UnicodeCategory.LetterNumber or UnicodeCategory.PrivateUse => BidiClass.L,
            _ => BidiClass.ON,
        };
    }

    /// <summary>Arabic, Syriac, Thaana and their presentation forms: the letters rule W2 reads digits after as Arabic.</summary>
    static bool IsArabicLetter(int c) =>
        c is >= 0x0600 and <= 0x07BF or >= 0x0860 and <= 0x08FF or >= 0xFB50 and <= 0xFDFF or >= 0xFE70 and <= 0xFEFF or >= 0x1EE00 and <= 0x1EEFF;

    /// <summary>The blocks of scripts written right to left: Hebrew, N'Ko, Samaritan, Mandaic, and the old ones beyond the first plane.</summary>
    static bool IsRightToLeft(int c) =>
        c is >= 0x0590 and <= 0x08FF or >= 0xFB1D and <= 0xFB4F or >= 0x10800 and <= 0x10FFF or >= 0x1E800 and <= 0x1EFFF;

    /// <summary>Whether <paramref name="text"/> has a character written right to left.</summary>
    public static bool HasRightToLeft(string text)
    {
        foreach (var rune in text.EnumerateRunes())
            if (rune.Value >= 0x0590 && (IsArabicLetter(rune.Value) || IsRightToLeft(rune.Value) || rune.Value == 0x200F || rune.Value == 0x061C))
                return true;
        return false;
    }

    /// <summary>The paragraph's level: 1 (right to left) when its first strong letter is right to left, else 0 (rules P2, P3).</summary>
    public static int ParagraphLevel(string text)
    {
        foreach (var rune in text.EnumerateRunes())
            switch (ClassOf(rune.Value))
            {
                case BidiClass.L:
                    return 0;
                case BidiClass.R or BidiClass.AL:
                    return 1;
            }
        return 0;
    }

    /// <summary>
    /// The embedding level of each UTF-16 unit of <paramref name="text"/>, a
    /// line of one paragraph: even is left to right, odd right to left. The
    /// paragraph's level is <paramref name="paragraph"/>, or found from the
    /// text when it is null.
    /// </summary>
    public static byte[] Levels(string text, int? paragraph = null)
    {
        var para = paragraph ?? ParagraphLevel(text);
        // One entry per character (a rune), mapped back to UTF-16 units at the end.
        var starts = new List<int>(text.Length);
        var original = new List<BidiClass>(text.Length);
        for (var i = 0; i < text.Length;)
        {
            // A lone surrogate reads as the replacement character, one unit long.
            Rune.DecodeFromUtf16(text.AsSpan(i), out var rune, out var used);
            starts.Add(i);
            original.Add(ClassOf(rune.Value));
            i += Math.Max(1, used);
        }
        var n = original.Count;
        var types = original.ToArray();
        var levels = new byte[n];
        var embedding = para % 2 == 0 ? BidiClass.L : BidiClass.R;

        // X9: marks that embed and BN are left out of the rules that follow.
        var kept = new List<int>(n);
        for (var i = 0; i < n; i++)
            if (types[i] != BidiClass.BN)
                kept.Add(i);
        var m = kept.Count;
        BidiClass T(int k) => types[kept[k]];
        void Set(int k, BidiClass t) => types[kept[k]] = t;

        // W1: a mark takes the class of what it marks.
        for (var k = 0; k < m; k++)
            if (T(k) == BidiClass.NSM)
                Set(k, k == 0 ? embedding : T(k - 1));
        // W2: a European digit after Arabic letters is an Arabic one.
        var lastStrong = embedding;
        for (var k = 0; k < m; k++)
        {
            var t = T(k);
            if (t is BidiClass.L or BidiClass.R or BidiClass.AL)
                lastStrong = t;
            else if (t == BidiClass.EN && lastStrong == BidiClass.AL)
                Set(k, BidiClass.AN);
        }
        // W3: Arabic letters are right to left.
        for (var k = 0; k < m; k++)
            if (T(k) == BidiClass.AL)
                Set(k, BidiClass.R);
        // W4: one separator between two numbers of a kind joins them.
        for (var k = 1; k < m - 1; k++)
        {
            var (before, t, after) = (T(k - 1), T(k), T(k + 1));
            if (t == BidiClass.ES && before == BidiClass.EN && after == BidiClass.EN)
                Set(k, BidiClass.EN);
            else if (t == BidiClass.CS && before == after && before is BidiClass.EN or BidiClass.AN)
                Set(k, before);
        }
        // W5: terminators (%, $) next to a European number join it.
        for (var k = 0; k < m; k++)
        {
            if (T(k) != BidiClass.ET)
                continue;
            var end = k;
            while (end < m && T(end) == BidiClass.ET)
                end++;
            if ((k > 0 && T(k - 1) == BidiClass.EN) || (end < m && T(end) == BidiClass.EN))
                for (var j = k; j < end; j++)
                    Set(j, BidiClass.EN);
            k = end - 1;
        }
        // W6: the separators and terminators left are neutral.
        for (var k = 0; k < m; k++)
            if (T(k) is BidiClass.ES or BidiClass.ET or BidiClass.CS)
                Set(k, BidiClass.ON);
        // W7: a European number after a left-to-right letter is left to right.
        lastStrong = embedding;
        for (var k = 0; k < m; k++)
        {
            var t = T(k);
            if (t is BidiClass.L or BidiClass.R)
                lastStrong = t;
            else if (t == BidiClass.EN && lastStrong == BidiClass.L)
                Set(k, BidiClass.L);
        }
        // N1, N2: neutrals between letters of one direction take it; else the paragraph's.
        static bool Neutral(BidiClass t) => t is BidiClass.B or BidiClass.S or BidiClass.WS or BidiClass.ON;
        static BidiClass Strong(BidiClass t) => t is BidiClass.EN or BidiClass.AN ? BidiClass.R : t;
        for (var k = 0; k < m; k++)
        {
            if (!Neutral(T(k)))
                continue;
            var end = k;
            while (end < m && Neutral(T(end)))
                end++;
            var lead = k == 0 ? embedding : Strong(T(k - 1));
            var trail = end == m ? embedding : Strong(T(end));
            var resolved = lead == trail ? lead : embedding;
            for (var j = k; j < end; j++)
                Set(j, resolved);
            k = end - 1;
        }
        // I1, I2: the levels.
        for (var i = 0; i < n; i++)
        {
            var t = types[i];
            levels[i] = (byte)(para % 2 == 0
                ? t == BidiClass.R ? para + 1 : t is BidiClass.AN or BidiClass.EN ? para + 2 : para
                : t is BidiClass.L or BidiClass.EN or BidiClass.AN ? para + 1 : para);
        }
        // What X9 left out takes the level before it.
        for (var i = 0; i < n; i++)
            if (original[i] == BidiClass.BN)
                levels[i] = (byte)(i == 0 ? para : levels[i - 1]);
        // L1: tabs, paragraph ends and the spaces before them and at the line's end go to the paragraph's level.
        var trailing = true;
        for (var i = n - 1; i >= 0; i--)
        {
            var t = original[i];
            if (t is BidiClass.S or BidiClass.B)
            {
                levels[i] = (byte)para;
                trailing = true;
            }
            else if (trailing && t is BidiClass.WS or BidiClass.BN)
                levels[i] = (byte)para;
            else
                trailing = false;
        }

        var units = new byte[text.Length];
        for (var i = 0; i < n; i++)
        {
            var end = i + 1 < n ? starts[i + 1] : text.Length;
            for (var u = starts[i]; u < end; u++)
                units[u] = levels[i];
        }
        return units;
    }

    /// <summary>
    /// The visual order, from the left, of runs at <paramref name="levels"/>
    /// (rule L2): from the highest level down to the lowest odd one, every
    /// sequence of runs at that level or higher is turned around.
    /// </summary>
    public static int[] VisualOrder(IReadOnlyList<int> levels)
    {
        var order = Enumerable.Range(0, levels.Count).ToArray();
        if (levels.Count == 0)
            return order;
        var highest = levels.Max();
        var lowestOdd = levels.Where(l => l % 2 == 1).DefaultIfEmpty(highest + 1).Min();
        for (var level = highest; level >= lowestOdd && level > 0; level--)
        {
            for (var i = 0; i < order.Length; i++)
            {
                if (levels[order[i]] < level)
                    continue;
                var end = i;
                while (end < order.Length && levels[order[end]] >= level)
                    end++;
                Array.Reverse(order, i, end - i);
                i = end;
            }
        }
        return order;
    }
}
