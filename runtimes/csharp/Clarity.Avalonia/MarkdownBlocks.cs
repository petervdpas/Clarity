// The Markdown of the markdown part, read into blocks (headings, paragraphs,
// list items, code) and their spans of text, the way every renderer reads
// it: conformance/markdown.json says what each case gives, and the web
// renderer's app/src/lib/markdownBlocks.ts reads it the same way.

using System.Text;
using System.Text.RegularExpressions;

namespace Clarity.Avalonia;

/// A piece of a block's text with one look.
public sealed record MarkdownSpan(string Text, bool Bold = false, bool Italic = false, bool Code = false, string? Link = null);

/// A block: `heading` (with its level), `paragraph`, `item` (with its depth
/// and, numbered, its number) or `code` (with its text as written).
public sealed record MarkdownBlock(string Kind, IReadOnlyList<MarkdownSpan> Spans, int Level = 0, int Depth = 0, int? Number = null, string Text = "");

public static partial class MarkdownBlocks
{
    [GeneratedRegex(@"^(#{1,6})\s+(.*)$")]
    private static partial Regex Heading();

    [GeneratedRegex(@"^(\s*)([-*+]|(\d{1,9})[.)])\s+(.*)$")]
    private static partial Regex Item();

    const string Punctuation = "\\`*_[]()#+-.!{}<>";
    const string Fence = "```";

    /// The look a span gets from what holds it.
    readonly record struct Look(bool Bold, bool Italic, string? Link);

    /// Addresses a link may open; with any other, only the link's text shows.
    static bool Opens(string address) =>
        address.StartsWith("http://", StringComparison.Ordinal) || address.StartsWith("https://", StringComparison.Ordinal) || address.StartsWith("mailto:", StringComparison.Ordinal);

    static bool IsWordChar(string s, int at) => at >= 0 && at < s.Length && char.IsLetterOrDigit(s[at]);

    static char At(string s, int at) => at >= 0 && at < s.Length ? s[at] : '\0';

    /// Where the single `mark` that closes an emphasis from `from` on is, skipping doubled marks.
    static int ClosingSingle(string s, char mark, int from)
    {
        for (var j = from; j < s.Length; j++)
        {
            if (s[j] != mark) continue;
            if (At(s, j + 1) == mark)
            {
                j++;
                continue;
            }
            if (mark == '_' && IsWordChar(s, j + 1)) continue;
            return j;
        }
        return -1;
    }

    static void Inline(string s, Look look, List<MarkdownSpan> output)
    {
        var text = new StringBuilder();
        void Flush()
        {
            if (text.Length > 0) output.Add(new MarkdownSpan(text.ToString(), look.Bold, look.Italic, false, look.Link));
            text.Clear();
        }
        var i = 0;
        while (i < s.Length)
        {
            var c = s[i];
            var twoMarks = i + 1 < s.Length && s[i + 1] == c && (c == '*' || c == '_');
            if (c == '\\' && i + 1 < s.Length && Punctuation.Contains(s[i + 1]))
            {
                text.Append(s[i + 1]);
                i += 2;
            }
            else if (c == '`')
            {
                var end = s.IndexOf('`', i + 1);
                if (end < 0)
                {
                    text.Append(c);
                    i++;
                }
                else
                {
                    Flush();
                    output.Add(new MarkdownSpan(s[(i + 1)..end], look.Bold, look.Italic, true, look.Link));
                    i = end + 1;
                }
            }
            else if (twoMarks && i + 2 < s.Length && s[i + 2] != ' ' && !(c == '_' && IsWordChar(s, i - 1)))
            {
                var end = s.IndexOf(s.Substring(i, 2), i + 2, StringComparison.Ordinal);
                if (end < 0)
                {
                    text.Append(s, i, 2);
                    i += 2;
                }
                else
                {
                    Flush();
                    Inline(s[(i + 2)..end], look with { Bold = true }, output);
                    i = end + 2;
                }
            }
            else if ((c == '*' || c == '_') && i + 1 < s.Length && s[i + 1] != ' ' && !(c == '_' && IsWordChar(s, i - 1)))
            {
                var end = ClosingSingle(s, c, i + 1);
                if (end < 0)
                {
                    text.Append(c);
                    i++;
                }
                else
                {
                    Flush();
                    Inline(s[(i + 1)..end], look with { Italic = true }, output);
                    i = end + 1;
                }
            }
            else if (c == '[')
            {
                var close = s.IndexOf(']', i + 1);
                var end = close >= 0 && At(s, close + 1) == '(' ? s.IndexOf(')', close + 2) : -1;
                if (end < 0)
                {
                    text.Append(c);
                    i++;
                }
                else
                {
                    Flush();
                    var address = s[(close + 2)..end].Trim();
                    Inline(s[(i + 1)..close], Opens(address) ? look with { Link = address } : look, output);
                    i = end + 1;
                }
            }
            else
            {
                text.Append(c);
                i++;
            }
        }
        Flush();
    }

    /// The spans of a text: split where the look changes, merged where it does not.
    public static List<MarkdownSpan> Spans(string text)
    {
        var raw = new List<MarkdownSpan>();
        Inline(text, default, raw);
        var merged = new List<MarkdownSpan>();
        foreach (var span in raw)
        {
            if (merged.Count > 0 && merged[^1] is var last && last with { Text = "" } == span with { Text = "" })
                merged[^1] = last with { Text = last.Text + span.Text };
            else
                merged.Add(span);
        }
        return merged;
    }

    /// The blocks of a text in Markdown.
    public static List<MarkdownBlock> Read(string markdown)
    {
        var lines = markdown.Replace("\r\n", "\n").Split('\n');
        var blocks = new List<MarkdownBlock>();
        // The paragraph or item that following lines join: where it is, and its text so far.
        int? open = null;
        var openText = new StringBuilder();
        void Close()
        {
            if (open is int at) blocks[at] = blocks[at] with { Spans = Spans(openText.ToString()) };
            open = null;
            openText.Clear();
        }
        for (var i = 0; i < lines.Length; i++)
        {
            var line = lines[i];
            var trimmed = line.Trim();
            if (trimmed.StartsWith(Fence, StringComparison.Ordinal))
            {
                Close();
                var code = new List<string>();
                i++;
                while (i < lines.Length && !lines[i].Trim().StartsWith(Fence, StringComparison.Ordinal)) code.Add(lines[i++]);
                blocks.Add(new MarkdownBlock("code", [], Text: string.Join("\n", code)));
                continue;
            }
            if (trimmed.Length == 0)
            {
                Close();
                continue;
            }
            var heading = Heading().Match(trimmed);
            if (heading.Success)
            {
                Close();
                blocks.Add(new MarkdownBlock("heading", Spans(heading.Groups[2].Value.Trim()), Level: heading.Groups[1].Length));
                continue;
            }
            var item = Item().Match(line);
            if (item.Success)
            {
                Close();
                var depth = Math.Min(item.Groups[1].Length / 2, 3);
                int? number = item.Groups[3].Success ? int.Parse(item.Groups[3].Value) : null;
                blocks.Add(new MarkdownBlock("item", [], Depth: depth, Number: number));
                open = blocks.Count - 1;
                openText.Append(item.Groups[4].Value.Trim());
                continue;
            }
            if (open is not null)
            {
                openText.Append(' ').Append(trimmed);
            }
            else
            {
                blocks.Add(new MarkdownBlock("paragraph", []));
                open = blocks.Count - 1;
                openText.Append(trimmed);
            }
        }
        Close();
        return blocks;
    }
}
