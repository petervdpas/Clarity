// Command-line programs (DESIGN 5.20): reading a command line against the
// program's commands, writing its help, printing a table and asking on a
// terminal, as conformance/command-line.json says every runtime does. A
// generated program describes itself as a CliProgram and runs the command
// Cli.Parse finds with the values it read.

using System.Globalization;
using System.Text;

namespace Clarity.Runtime;

/// <summary>What an argument or option holds; Bool only for an option.</summary>
public enum CliKind { Text, Integer, Decimal, Money, Date, DateTime, Duration, Enum, Bool }

/// <summary>An argument or an option; its name as typed (<c>call-after</c>).</summary>
public sealed record CliParam(string Name, CliKind Kind)
{
    /// <summary>The values of an enum, by name.</summary>
    public IReadOnlyList<string> Values { get; init; } = [];
    public string? Default { get; init; }
    public string? Min { get; init; }
    public string? Max { get; init; }
    public bool Required { get; init; }
    public string? Help { get; init; }
}

/// <summary>A command: the words typed, as <c>["customer", "add"]</c>.</summary>
public sealed record CliCommand(IReadOnlyList<string> Line, string? Help, IReadOnlyList<CliParam> Arguments, IReadOnlyList<CliParam> Options);

/// <summary>A program and its commands, in the order they are declared.</summary>
public sealed record CliProgram(string Name, string? Help, IReadOnlyList<CliCommand> Commands);

/// <summary>Clarity's words in the language shown: the translation of a key, or its English.</summary>
public sealed class CliWords(Func<string, string?>? translated = null)
{
    /// <summary>Clarity's own words of the command line, in English.</summary>
    public static readonly IReadOnlyList<(string Key, string English)> English =
    [
        ("cliUsage", "Usage"),
        ("cliCommands", "Commands"),
        ("cliOptions", "Options"),
        ("cliCommandWord", "command"),
        ("cliOptionsWord", "options"),
        ("cliHelpOption", "Shows this help"),
        ("cliDefault", "default {value}"),
        ("cliRequiredMark", "required"),
        ("cliUnknownCommand", "There is no command {command}"),
        ("cliUnknownOption", "There is no option {option}"),
        ("cliMissingArgument", "{argument} is missing"),
        ("cliTooManyArguments", "There are more values than arguments"),
        ("cliNeedsValue", "{option} needs a value"),
        ("cliWrongValue", "{name} cannot be {value}"),
        ("cliTooSmall", "{name} is at least {min}"),
        ("cliTooLarge", "{name} is at most {max}"),
        ("cliRequired", "{option} is required"),
        ("cliWrongAnswer", "{value} does not fit; try again"),
        ("cliNoTerminal", "There is nobody to answer: {question}"),
        ("cliChooseNumber", "Type a number from 1 to {count}"),
    ];

    /// <summary>The word of <paramref name="key"/> with its slots filled.</summary>
    public string Say(string key, params (string Name, string Value)[] slots)
    {
        var text = translated?.Invoke(key);
        if (string.IsNullOrEmpty(text))
            text = English.FirstOrDefault(w => w.Key == key).English ?? key;
        foreach (var (name, value) in slots)
            text = text.Replace("{" + name + "}", value);
        return text;
    }
}

/// <summary>
/// What a command line becomes: a command to run with every argument's value
/// and then every option's (null for the empty value), help to write to
/// standard output (exit code 0), or a wrong command line: its message's key
/// and slots, and what goes to standard error (exit code 2).
/// </summary>
public abstract record CliParsed
{
    public sealed record Run(int Command, IReadOnlyList<string?> Values) : CliParsed;
    public sealed record Help(string Text) : CliParsed;
    public sealed record Wrong(string Key, IReadOnlyList<(string Name, string Value)> Slots, string Said) : CliParsed;
}

/// <summary>What a command asks: yes or no by its two words, a value of a kind
/// with its default, or one of the choices by number from 1.</summary>
public abstract record CliAsked
{
    public sealed record Confirm(string Yes, string No) : CliAsked;
    public sealed record Value(CliParam Param) : CliAsked;
    public sealed record Choice(IReadOnlyList<string> Choices) : CliAsked;
}

/// <summary>Where a command asks: a terminal, or a pipe or script with nobody at it.</summary>
public interface ICliTerminal
{
    bool IsTerminal { get; }
    /// <summary>The next line typed, without its end; null at the end of input.</summary>
    string? ReadLine();
    /// <summary>Writes to standard error, where questions go.</summary>
    void Write(string text);
}

/// <summary>The terminal the program runs in.</summary>
public sealed class CliConsole : ICliTerminal
{
    /// <summary>A terminal, or Clarity's Output panel, which says so with <c>CLARITY_TERMINAL=1</c>.</summary>
    public bool IsTerminal => !Console.IsInputRedirected || Environment.GetEnvironmentVariable("CLARITY_TERMINAL") == "1";
    public string? ReadLine() => Console.In.ReadLine();
    public void Write(string text)
    {
        Console.Error.Write(text);
        Console.Error.Flush();
    }
}

public static class Cli
{
    static bool PlainDigits(string text) => text.Length > 0 && text.All(char.IsAsciiDigit);

    /// <summary><paramref name="value"/> as text of the parameter's kind; null when it is not one.</summary>
    public static string? ReadValue(CliParam param, string value)
    {
        switch (param.Kind)
        {
            case CliKind.Text:
                return value;
            case CliKind.Bool:
                return value is "true" or "false" ? value : null;
            case CliKind.Integer:
            {
                var digits = value.StartsWith('-') ? value[1..] : value;
                return PlainDigits(digits) && long.TryParse(value, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n)
                    ? n.ToString(CultureInfo.InvariantCulture) : null;
            }
            case CliKind.Decimal:
            case CliKind.Money:
            {
                var digits = value.StartsWith('-') ? value[1..] : value;
                var point = digits.IndexOf('.');
                var whole = point < 0 ? digits : digits[..point];
                if (!PlainDigits(whole) || (point >= 0 && !PlainDigits(digits[(point + 1)..])))
                    return null;
                if (!decimal.TryParse(value, NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var d))
                    return null;
                // Dividing by 1 with many decimals drops the trailing zeros: 7.50 is 7.5.
                return (d / 1.0000000000000000000000000000m).ToString(CultureInfo.InvariantCulture);
            }
            case CliKind.Date:
                return DateOnly.TryParseExact(value, "yyyy-MM-dd", CultureInfo.InvariantCulture, DateTimeStyles.None, out var date)
                    ? date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture) : null;
            case CliKind.DateTime:
                return System.DateTime.TryParseExact(value, ["yyyy-MM-ddTHH:mm:ss", "yyyy-MM-ddTHH:mm"], CultureInfo.InvariantCulture, DateTimeStyles.None, out var moment)
                    ? moment.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture) : null;
            case CliKind.Duration:
                return Duration(value)?.ToString(CultureInfo.InvariantCulture);
            case CliKind.Enum:
                return param.Values.FirstOrDefault(v => string.Equals(v, value, StringComparison.OrdinalIgnoreCase));
        }
        return null;
    }

    /// <summary>Whole seconds from 45, 3:45 or 1:02:03, as formats.json reads them.</summary>
    static long? Duration(string text)
    {
        var parts = text.Trim().Split(':');
        if (parts.Length > 3 || parts.Any(p => !PlainDigits(p) || p.Length > 15))
            return null;
        if (parts.Skip(1).Any(p => p.Length != 2 || long.Parse(p, CultureInfo.InvariantCulture) > 59))
            return null;
        return parts.Aggregate(0L, (seconds, p) => seconds * 60 + long.Parse(p, CultureInfo.InvariantCulture));
    }

    /// <summary>Whether the number <paramref name="a"/> is less than <paramref name="b"/>, exactly.</summary>
    static bool Below(string a, string b) =>
        decimal.TryParse(a, NumberStyles.Number, CultureInfo.InvariantCulture, out var x)
        && decimal.TryParse(b, NumberStyles.Number, CultureInfo.InvariantCulture, out var y)
        && x < y;

    static int Letters(string text) => text.EnumerateRunes().Count();

    static bool BeginsWith(IReadOnlyList<string> line, IReadOnlyList<string> path) =>
        line.Count >= path.Count && path.Select((w, i) => line[i] == w).All(same => same);

    /// <summary>A heading and its lines, the left column padded to the longest.</summary>
    static string Section(string heading, IReadOnlyList<(string Left, string Right)> rows)
    {
        var width = rows.Count == 0 ? 0 : rows.Max(r => Letters(r.Left));
        var out_ = new StringBuilder(heading + ":\n");
        foreach (var (left, right) in rows)
        {
            var line = "  " + left + (right.Length > 0 ? new string(' ', width - Letters(left) + 2) + right : "");
            out_.Append(line.TrimEnd(' ')).Append('\n');
        }
        return out_.ToString();
    }

    static string GroupUsage(CliProgram program, IReadOnlyList<string> path, CliWords words) =>
        $"{words.Say("cliUsage")}: {string.Join(' ', new[] { program.Name }.Concat(path))} <{words.Say("cliCommandWord")}> [{words.Say("cliOptionsWord")}]";

    static string CommandUsage(CliProgram program, CliCommand c, CliWords words) =>
        $"{words.Say("cliUsage")}: {string.Join(' ', new[] { program.Name }.Concat(c.Line).Concat(c.Arguments.Select(a => $"<{a.Name}>")))} [{words.Say("cliOptionsWord")}]";

    /// <summary>The help of the program (<paramref name="path"/> empty) or of a group.</summary>
    public static string GroupHelp(CliProgram program, IReadOnlyList<string> path, CliWords words)
    {
        var text = new StringBuilder();
        if (path.Count == 0 && program.Help is { } help)
            text.Append(help).Append("\n\n");
        text.Append(GroupUsage(program, path, words)).Append("\n\n");
        var commands = program.Commands.Where(c => c.Line.Count > path.Count && BeginsWith(c.Line, path)).Select(c => (string.Join(' ', c.Line), c.Help ?? "")).ToList();
        text.Append(Section(words.Say("cliCommands"), commands)).Append('\n');
        text.Append(Section(words.Say("cliOptions"), [("--help", words.Say("cliHelpOption"))]));
        return text.ToString();
    }

    /// <summary>The help of one command: what it does, its usage and its options.</summary>
    public static string CommandHelp(CliProgram program, CliCommand c, CliWords words)
    {
        var text = new StringBuilder();
        if (c.Help is { } help)
            text.Append(help).Append("\n\n");
        text.Append(CommandUsage(program, c, words)).Append("\n\n");
        var rows = c.Options.Select(o =>
        {
            var left = o.Kind == CliKind.Bool ? $"--{o.Name}" : $"--{o.Name} <{o.Name}>";
            var marks = new List<string>();
            if (o.Values.Count > 0)
                marks.Add(string.Join(", ", o.Values));
            if (o.Default is { } d && o.Kind != CliKind.Bool)
                marks.Add(words.Say("cliDefault", ("value", d)));
            if (o.Required)
                marks.Add(words.Say("cliRequiredMark"));
            var right = o.Help ?? "";
            if (marks.Count > 0)
                right += (right.Length > 0 ? " " : "") + "(" + string.Join("; ", marks) + ")";
            return (left, right);
        }).ToList();
        rows.Add(("--help", words.Say("cliHelpOption")));
        text.Append(Section(words.Say("cliOptions"), rows));
        return text.ToString();
    }

    static CliParsed Wrong(CliProgram program, IReadOnlyList<string> path, CliCommand? command, CliWords words, string key, params (string, string)[] slots)
    {
        var usage = command is null ? GroupUsage(program, path, words) : CommandUsage(program, command, words);
        return new CliParsed.Wrong(key, slots, words.Say(key, slots) + "\n" + usage + "\n");
    }

    static string OptionName(string word) => word.Split('=')[0];

    /// <summary>Reads <paramref name="args"/>, the words after the program's name.</summary>
    public static CliParsed Parse(CliProgram program, IReadOnlyList<string> args, CliWords words)
    {
        var end = args.ToList().IndexOf("--");
        if (end < 0)
            end = args.Count;
        var askedHelp = args.Take(end).Contains("--help");
        var rest = args.Where((a, i) => i >= end || a != "--help").ToList();
        if (rest.Count > 0 && rest[0] == "help")
        {
            askedHelp = true;
            rest.RemoveAt(0);
        }
        // The command, or the group read so far.
        var path = new List<string>();
        var at = 0;
        int? found = null;
        while (found is null)
        {
            if (at >= rest.Count || rest[at].StartsWith("--", StringComparison.Ordinal))
            {
                if (at < rest.Count && !askedHelp && path.Count == 0)
                    return Wrong(program, path, null, words, "cliUnknownOption", ("option", OptionName(rest[at])));
                return new CliParsed.Help(GroupHelp(program, path, words));
            }
            var line = path.Append(rest[at]).ToList();
            var group = false;
            for (var i = 0; i < program.Commands.Count; i++)
            {
                var candidate = program.Commands[i].Line;
                if (candidate.Count == line.Count && BeginsWith(candidate, line))
                    found = i;
                else if (candidate.Count > line.Count && BeginsWith(candidate, line))
                    group = true;
            }
            if (found is null && !group)
                return Wrong(program, path, null, words, "cliUnknownCommand", ("command", string.Join(' ', line)));
            path = line;
            at++;
        }
        var c = program.Commands[found.Value];
        if (askedHelp)
            return new CliParsed.Help(CommandHelp(program, c, words));
        CliParsed SayWrong(string key, params (string, string)[] slots) => Wrong(program, [], c, words, key, slots);

        var after = rest.Skip(at).ToList();
        var endsAt = after.IndexOf("--");
        if (endsAt < 0)
            endsAt = after.Count;
        (int Index, string? Value)? OptionOf(string word)
        {
            if (!word.StartsWith("--", StringComparison.Ordinal))
                return null;
            var equals = word.IndexOf('=');
            var name = equals < 0 ? word[2..] : word[2..equals];
            var index = c.Options.ToList().FindIndex(o => o.Name == name);
            return index < 0 ? null : (index, equals < 0 ? null : word[(equals + 1)..]);
        }
        for (var i = 0; i < endsAt; i++)
        {
            if (after[i].StartsWith("--", StringComparison.Ordinal) && OptionOf(after[i]) is null)
                return SayWrong("cliUnknownOption", ("option", OptionName(after[i])));
        }
        var given = new List<(int Index, string Value)>();
        var positional = new List<string>();
        for (var i = 0; i < after.Count; i++)
        {
            if (i == endsAt)
            {
                positional.AddRange(after.Skip(i + 1));
                break;
            }
            if (OptionOf(after[i]) is not { } option)
            {
                positional.Add(after[i]);
                continue;
            }
            var o = c.Options[option.Index];
            string value;
            if (option.Value is { } written)
                value = written;
            else if (o.Kind == CliKind.Bool)
                value = "true";
            else if (i + 1 < after.Count)
                value = after[++i];
            else
                return SayWrong("cliNeedsValue", ("option", $"--{o.Name}"));
            given.RemoveAll(g => g.Index == option.Index);
            given.Add((option.Index, value));
        }

        // Each value in the order it was typed: of its kind, then within its range.
        var reads = c.Arguments.Zip(positional, (a, v) => (Name: a.Name, Param: a, Typed: v))
            .Concat(given.Select(g => (Name: $"--{c.Options[g.Index].Name}", Param: c.Options[g.Index], Typed: g.Value)))
            .ToList();
        var values = new Dictionary<string, string>();
        foreach (var r in reads)
        {
            if (ReadValue(r.Param, r.Typed) is not { } v)
                return SayWrong("cliWrongValue", ("name", r.Name), ("value", r.Typed));
            values[r.Name] = v;
        }
        foreach (var r in reads)
        {
            var v = values[r.Name];
            if (r.Param.Min is { } min && Below(v, min))
                return SayWrong("cliTooSmall", ("name", r.Name), ("min", min));
            if (r.Param.Max is { } max && Below(max, v))
                return SayWrong("cliTooLarge", ("name", r.Name), ("max", max));
        }
        if (positional.Count < c.Arguments.Count)
            return SayWrong("cliMissingArgument", ("argument", c.Arguments[positional.Count].Name));
        if (positional.Count > c.Arguments.Count)
            return SayWrong("cliTooManyArguments");
        for (var i = 0; i < c.Options.Count; i++)
        {
            if (c.Options[i].Required && !given.Any(g => g.Index == i))
                return SayWrong("cliRequired", ("option", $"--{c.Options[i].Name}"));
        }
        var result = c.Arguments.Select(a => (string?)values[a.Name]).ToList();
        foreach (var o in c.Options)
            result.Add(values.TryGetValue($"--{o.Name}", out var v) ? v : o.Default ?? (o.Kind == CliKind.Bool ? "false" : null));
        return new CliParsed.Run(found.Value, result);
    }

    /// <summary>What <c>print list columns a, b</c> writes: a line of headings, then a
    /// line per row, the columns two spaces apart, a column of numbers aligned right.</summary>
    public static string Table(IReadOnlyList<string> headings, IReadOnlyList<bool> right, IReadOnlyList<IReadOnlyList<string>> rows)
    {
        var lines = new List<IReadOnlyList<string>> { headings };
        lines.AddRange(rows);
        var widths = headings.Select((_, i) => lines.Max(r => i < r.Count ? Letters(r[i]) : 0)).ToList();
        var text = new StringBuilder();
        foreach (var row in lines)
        {
            var cells = widths.Select((width, i) =>
            {
                var cell = i < row.Count ? row[i] : "";
                var pad = new string(' ', width - Letters(cell));
                return i < right.Count && right[i] ? pad + cell : cell + pad;
            });
            text.Append(string.Join("  ", cells).TrimEnd(' ')).Append('\n');
        }
        return text.ToString();
    }

    /// <summary>Asks <paramref name="question"/>: the answer as text (<c>true</c> or <c>false</c>
    /// for yes or no, the number of a choice) and 0, or null and the exit code the program ends with.</summary>
    public static (string? Answer, int Exit) Ask(string question, CliAsked asked, ICliTerminal terminal, CliWords words)
    {
        if (!terminal.IsTerminal)
        {
            if (asked is CliAsked.Value { Param.Default: { } given })
                return (given, 0);
            terminal.Write(words.Say("cliNoTerminal", ("question", question)) + "\n");
            return (null, 2);
        }
        switch (asked)
        {
            case CliAsked.Confirm confirm:
                while (true)
                {
                    terminal.Write($"{question} [{confirm.Yes}/{confirm.No}] ");
                    var line = terminal.ReadLine()?.Trim() ?? "";
                    if (line.Length == 0 || string.Equals(line, confirm.No, StringComparison.OrdinalIgnoreCase))
                        return ("false", 0);
                    if (string.Equals(line, confirm.Yes, StringComparison.OrdinalIgnoreCase))
                        return ("true", 0);
                }
            case CliAsked.Choice choice:
                terminal.Write(string.Concat(choice.Choices.Select((c, i) => $"{i + 1}. {c}\n")));
                while (true)
                {
                    terminal.Write(question + " ");
                    if (terminal.ReadLine() is not { } line)
                    {
                        terminal.Write("\n");
                        return (null, 1);
                    }
                    if (PlainDigits(line.Trim()) && int.TryParse(line.Trim(), CultureInfo.InvariantCulture, out var n) && n >= 1 && n <= choice.Choices.Count)
                        return (n.ToString(CultureInfo.InvariantCulture), 0);
                    terminal.Write(words.Say("cliChooseNumber", ("count", choice.Choices.Count.ToString(CultureInfo.InvariantCulture))) + "\n");
                }
            case CliAsked.Value { Param: var param }:
                var shown = param.Kind == CliKind.Enum ? string.Join('/', param.Values) : param.Default;
                while (true)
                {
                    terminal.Write(shown is null ? question + " " : $"{question} [{shown}] ");
                    if (terminal.ReadLine() is not { } typed)
                    {
                        terminal.Write("\n");
                        return (null, 1);
                    }
                    var line = typed.Trim();
                    if (line.Length == 0 && param.Default is { } fallback)
                        return (fallback, 0);
                    if (ReadValue(param, line) is { } value)
                        return (value, 0);
                    terminal.Write(words.Say("cliWrongAnswer", ("value", line)) + "\n");
                }
        }
        return (null, 1);
    }

    /// <summary><c>ask ... for Entity from list</c>: the record of the list the person chose.</summary>
    public static R Pick<R>(List<R> records, Func<List<R>, int> choose) => records[choose(records)];

    /// <summary>A text read from the command line; "" for none.</summary>
    public static string Text(string? text) => text ?? "";

    /// <summary>A whole number, or a duration's seconds, read from the command line; 0 for none.</summary>
    public static long Long(string? text) => long.TryParse(text, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? n : 0;

    /// <summary>A flag read from the command line.</summary>
    public static bool Bool(string? text) => text == "true";

    /// <summary>A decimal or money read from the command line; 0 for none.</summary>
    public static decimal Decimal(string? text) =>
        decimal.TryParse(text, NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var d) ? d : 0m;

    /// <summary>A date read from the command line; null for none.</summary>
    public static DateOnly? Date(string? text) =>
        DateOnly.TryParseExact(text, "yyyy-MM-dd", CultureInfo.InvariantCulture, DateTimeStyles.None, out var d) ? d : null;

    /// <summary>A moment read from the command line; null for none.</summary>
    public static DateTime? Moment(string? text) =>
        System.DateTime.TryParseExact(text, ["yyyy-MM-ddTHH:mm:ss", "yyyy-MM-ddTHH:mm"], CultureInfo.InvariantCulture, DateTimeStyles.None, out var m) ? m : null;

    /// <summary>An enum value read from the command line by its name; the first value for none.</summary>
    public static E Enum<E>(string? text) where E : struct, System.Enum =>
        System.Enum.TryParse<E>(text, ignoreCase: true, out var e) ? e : default;

    /// <summary>A value of code as the command line writes it: 42, 12.5, 2026-10-23, an enum value by its name; "" for none.</summary>
    public static string TextOf(object? value) => value switch
    {
        null => "",
        decimal d => (d / 1.0000000000000000000000000000m).ToString(CultureInfo.InvariantCulture),
        DateOnly d => d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
        System.DateTime m => m.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
        bool b => b ? "true" : "false",
        IFormattable f => f.ToString(null, CultureInfo.InvariantCulture),
        _ => value.ToString() ?? "",
    };

    /// <summary>The language to show for the system's <paramref name="locale"/> (<c>nl_NL.UTF-8</c>),
    /// among the project's translations; null for the language the project is written in, or one it has no translation for.</summary>
    public static string? PickLanguage(string locale, IReadOnlyList<string> available, string baseLanguage)
    {
        var tag = locale.Split('.', '@')[0].Replace('_', '-');
        static string Primary(string t) => t.Split('-')[0].ToLowerInvariant();
        if (tag.Length == 0 || tag is "C" or "POSIX" || Primary(tag) == Primary(baseLanguage))
            return null;
        return available.FirstOrDefault(a => a == tag) ?? available.FirstOrDefault(a => Primary(a) == Primary(tag));
    }

    /// <summary>Runs a command-line program: reads the command line, writes help or what is wrong
    /// with it, or runs the command it names with its values, and gives the exit code its run ends with (DESIGN 5.20).</summary>
    public static int RunProgram(CliProgram program, string[] args, Func<CliSession, int, IReadOnlyList<string?>, int> run)
    {
        switch (Parse(program, args, CliLanguage.Words))
        {
            case CliParsed.Help help:
                Console.Out.Write(help.Text);
                return 0;
            case CliParsed.Wrong wrong:
                Console.Error.Write(wrong.Said);
                return 2;
            case CliParsed.Run ran:
                try
                {
                    return Math.Clamp(run(new CliSession(new CliConsole()), ran.Command, ran.Values), 0, 255);
                }
                catch (RefusedException refused)
                {
                    Console.Error.WriteLine(CliLanguage.Show(refused.Text));
                    return 1;
                }
                catch (Exception err) when (err is ClarityException or IOException)
                {
                    Console.Error.WriteLine(err.Message);
                    return 1;
                }
        }
        return 2;
    }
}

/// <summary>The language a command-line program shows its texts in: the translation of a key,
/// in the language the system or the settings name, with each slot in that language's formats.</summary>
public static class CliLanguage
{
    static Func<string, CultureInfo, string?> lookup = (_, _) => null;
    static CultureInfo? language;

    /// <summary>Clarity's own words of the command line in the language shown.</summary>
    public static CliWords Words => new(key => Translate(key) is var value && value != key ? value : null);

    /// <summary>Starts with the project's translations: <paramref name="lookup"/> gives a text in a language,
    /// or null; <paramref name="chosen"/> is the language the settings keep, if any.</summary>
    public static void Start(string? baseLanguage, string[] available, string? currency, Func<string, CultureInfo, string?> lookup, Func<string, string, string> label, string? chosen)
    {
        CliLanguage.lookup = lookup;
        _ = (currency, label);
        var locale = new[] { "LC_ALL", "LC_MESSAGES", "LANG" }.Select(Environment.GetEnvironmentVariable).FirstOrDefault(l => !string.IsNullOrEmpty(l)) ?? CultureInfo.CurrentCulture.Name;
        var picked = !string.IsNullOrEmpty(chosen) && available.Contains(chosen) ? chosen : Cli.PickLanguage(locale, available, baseLanguage ?? "en");
        language = picked is null ? null : CultureInfo.GetCultureInfo(picked);
        Text.KeyValue = key => lookup(key, language ?? CultureInfo.InvariantCulture);
    }

    /// <summary>The value of a key in the language shown, or the default language's; a text that is no key stays as it is.</summary>
    public static string Translate(string keyOrText) =>
        lookup(keyOrText, language ?? CultureInfo.InvariantCulture) is { Length: > 0 } value ? value : keyOrText;

    /// <summary>One of Clarity's own words by its key, or <paramref name="fallback"/> when the project gives it no value.</summary>
    public static string Translate(string key, string fallback) => Translate(key) is var value && value != key ? value : fallback;

    /// <summary>Text as shown: a key by its value in the language shown, each slot as the command line
    /// writes values, the same on every stack and in every language (DESIGN 5.20).</summary>
    public static string Show(Text text) => text.Show();
}

/// <summary>What a running command prints, says and asks through: standard output, standard error
/// and the terminal, in the language shown.</summary>
public sealed class CliSession(ICliTerminal terminal)
{
    /// <summary><c>print value</c>: one line on standard output.</summary>
    public void Print(Text text) => Console.Out.WriteLine(CliLanguage.Show(text));

    /// <summary><c>print list columns a, b</c>: the records as a table on standard output.</summary>
    public void PrintTable(IReadOnlyList<Text> headings, IReadOnlyList<bool> right, IReadOnlyList<IReadOnlyList<string>> rows) =>
        Console.Out.Write(Cli.Table([.. headings.Select(CliLanguage.Show)], right, rows));

    /// <summary><c>message</c>: one line on standard error.</summary>
    public void Message(Text text) => Console.Error.WriteLine(CliLanguage.Show(text));

    /// <summary>Clarity's word for yes or no, as a table shows a bool.</summary>
    public static string YesNo(bool value) => value ? CliLanguage.Translate("yes", "Yes") : CliLanguage.Translate("no", "No");

    string Answer(Text question, CliAsked asked)
    {
        var (answer, exit) = Cli.Ask(CliLanguage.Show(question), asked, terminal, CliLanguage.Words);
        if (answer is null)
        {
            // Nobody to answer, or no answer: the program ends here.
            Environment.Exit(exit);
        }
        return answer!;
    }

    /// <summary><c>ask ... yes @a no @b</c>: whether the person said yes. Without the words the question takes Clarity's OK and Cancel.</summary>
    public bool Confirm(Text question, Text? yes, Text? no) =>
        Answer(question, new CliAsked.Confirm(yes is null ? CliLanguage.Translate("ok", "OK") : CliLanguage.Show(yes), no is null ? CliLanguage.Translate("cancel", "Cancel") : CliLanguage.Show(no))) == "true";

    /// <summary><c>ask ... for type [default value]</c>: the value typed, as text of its kind.</summary>
    public string Ask(Text question, CliKind kind, IReadOnlyList<string> values, string? fallback) =>
        Answer(question, new CliAsked.Value(new CliParam("", kind) { Values = values, Default = fallback }));

    /// <summary><c>ask ... for Entity from list</c>: the position, from 0, of the record chosen.</summary>
    public int Choose(Text question, IReadOnlyList<string> choices) =>
        int.Parse(Answer(question, new CliAsked.Choice(choices)), CultureInfo.InvariantCulture) - 1;
}
