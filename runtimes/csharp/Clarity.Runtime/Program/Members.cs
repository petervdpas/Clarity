// A view model's members by their Clarity names, as the C# generator names
// them (crates/clarity-gen/src/csharp): `finishedBy` is FinishedBy, or
// FinishedBy_ where the name is taken; a handler `approve` is the command
// ApproveCommand; a call with values `digit(7)` the command Digit7. What
// each type has is found once, by reflection, and kept per type.

using System.Collections;
using System.Collections.Concurrent;
using System.Globalization;
using System.Reflection;
using System.Text;
using System.Text.Json;

namespace Clarity.Runtime;

internal static class Members
{
    static readonly ConcurrentDictionary<(Type, string), PropertyInfo?> members = new();

    /// <summary>A Clarity name in Pascal case, as clarity-gen's pascal: <c>finishedBy</c> is FinishedBy; nothing, or a digit first, starts with _.</summary>
    public static string Pascal(string name)
    {
        var pascal = new StringBuilder();
        var start = true;
        foreach (var c in name)
        {
            if (!char.IsLetterOrDigit(c))
            {
                start = true;
                continue;
            }
            pascal.Append(start ? char.ToUpperInvariant(c) : c);
            start = false;
        }
        if (pascal.Length == 0 || char.IsAsciiDigit(pascal[0]))
            pascal.Insert(0, '_');
        return pascal.ToString();
    }

    /// <summary>A member's Clarity name, as Clarity's JSON writes it: FinishedBy (or FinishedBy_) is <c>finishedBy</c>.</summary>
    public static string Named(string member) => ClarityNaming.Instance.ConvertName(member);

    /// <summary>A property by its Clarity name: <c>finishedBy</c> is FinishedBy, or FinishedBy_ where the name is taken.</summary>
    public static PropertyInfo? Member(Type type, string name) => members.GetOrAdd((type, name), key =>
    {
        var pascal = Pascal(key.Item2);
        return key.Item1.GetProperty(pascal, BindingFlags.Public | BindingFlags.Instance)
            ?? key.Item1.GetProperty(pascal + "_", BindingFlags.Public | BindingFlags.Instance);
    });

    /// <summary>The value at a path of Clarity names (<c>order.status</c>, <c>tasks[1].done</c>); none when there is none.</summary>
    public static object? Read(object root, string path)
    {
        object? at = root;
        foreach (var step in path.Split('.'))
        {
            var (name, place) = Step(step);
            if (at is null || Member(at.GetType(), name) is not { } property)
                return null;
            at = property.GetValue(at);
            if (place is { } i)
                at = at is IList list && i >= 0 && i < list.Count ? list[i] : null;
        }
        return at;
    }

    /// <summary>A step of a path: its name, and the place of a record when it has one (<c>tasks[1]</c>).</summary>
    static (string Name, int? Place) Step(string step)
    {
        var open = step.IndexOf('[');
        if (open < 0 || !step.EndsWith(']'))
            return (step, null);
        return int.TryParse(step.AsSpan(open + 1, step.Length - open - 2), NumberStyles.None, CultureInfo.InvariantCulture, out var place)
            ? (step[..open], place)
            : (step, null);
    }

    /// <summary>The type of a list's rows; none for what is no list.</summary>
    public static Type? RowType(Type type)
    {
        if (type == typeof(string))
            return null;
        if (type.IsArray)
            return type.GetElementType();
        return type.GetInterfaces().Append(type)
            .FirstOrDefault(i => i.IsGenericType && i.GetGenericTypeDefinition() == typeof(IEnumerable<>))?
            .GetGenericArguments()[0];
    }

    /// <summary>
    /// The command the generator names for a call with values, as
    /// clarity-gen's call_command does: <c>digit(7)</c> is Digit7,
    /// <c>scale(0.5, "half")</c> Scale0_5Half.
    /// </summary>
    public static string CallCommand(string handler, IReadOnlyList<string> args)
    {
        var name = new StringBuilder(Pascal(handler));
        foreach (var value in args)
        {
            if (value.Length >= 2 && value[0] == '"' && value[^1] == '"')
            {
                var words = new string([.. value[1..^1].Select(c => char.IsAsciiLetterOrDigit(c) ? c : '_')]);
                name.Append(Pascal(words));
            }
            else if (value is "true" or "false")
                name.Append(value == "true" ? "True" : "False");
            else if (double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out _))
                name.Append(value.Replace("-", "Minus").Replace(".", "_"));
            else
                name.Append(Pascal(value));
        }
        return name.ToString();
    }

    /// <summary>
    /// A call as Clarity writes it (conformance/calls.json), taken apart:
    /// <c>scale(0.5, "half")</c> is the handler scale with the values
    /// <c>0.5</c> and <c>"half"</c>, each as written.
    /// </summary>
    public static (string Handler, List<string> Args) Call(string written)
    {
        var open = written.IndexOf('(');
        if (open < 0 || !written.EndsWith(')'))
            return (written.Trim(), []);
        var args = new List<string>();
        var current = new StringBuilder();
        var quoted = false;
        foreach (var c in written[(open + 1)..^1])
        {
            if (c == '"')
                quoted = !quoted;
            if (c == ',' && !quoted)
            {
                args.Add(current.ToString().Trim());
                current.Clear();
                continue;
            }
            current.Append(c);
        }
        if (current.ToString().Trim() is { Length: > 0 } last)
            args.Add(last);
        return (written[..open].Trim(), args);
    }
}
