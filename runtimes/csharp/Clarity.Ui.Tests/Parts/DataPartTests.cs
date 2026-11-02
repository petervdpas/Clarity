// What the tests of the data parts share (a list, a table, a tree): a host
// whose lists are records, which applies the person's edit of a record's
// field (tasks[0].done) or of a whole list (a move) as a generated view model
// does; and how a field of a record is read and written.

using System.Globalization;
using System.Text.RegularExpressions;

namespace Clarity.Ui.Tests;

/// <summary>A window's view model of lists of records: an edit of a field writes it into its record.</summary>
public partial class DataHost : TestHost
{
    public override void Write(string part, string path, object? value)
    {
        if (FieldPath().Match(path) is { Success: true } m && Lists.TryGetValue(m.Groups[1].Value, out var rows))
        {
            RowFields.Set(rows[int.Parse(m.Groups[2].Value, CultureInfo.InvariantCulture)], m.Groups[3].Value, value);
            Log.Add($"write {part} {path} {Written(value)}");
            Change();
            return;
        }
        if (value is IEnumerable<object> list && Lists.ContainsKey(path))
        {
            Lists[path] = [.. list];
            Log.Add($"write {part} {path}");
            Change();
            return;
        }
        base.Write(part, path, value);
    }

    /// <summary>Hears each handler the parts run, as it runs.</summary>
    public Action<HandlerCall>? Ran { get; set; }

    public override bool Run(string? part, HandlerCall call)
    {
        var ran = base.Run(part, call);
        Ran?.Invoke(call);
        return ran;
    }

    /// <summary>A value as the cases write it.</summary>
    public static string Written(object? value) => value switch
    {
        null => "null",
        DateTime t => t.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
        DateOnly d => d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
        decimal n => n.ToString(CultureInfo.InvariantCulture),
        bool b => b ? "true" : "false",
        _ => Convert.ToString(value, CultureInfo.InvariantCulture)!,
    };

    /// <summary>The events the parts ran, as the cases write them: <c>select</c>, <c>change</c>.</summary>
    public List<string> Events => [.. Log.Where(l => l.StartsWith("run ")).Select(l => l.Split(' ')[2])];

    [GeneratedRegex(@"^(.+)\[(\d+)\]\.(\w+)$")]
    private static partial Regex FieldPath();
}

/// <summary>A folder, as a generated record: in a tree under its parent.</summary>
public sealed class Folder
{
    public long Id { get; set; }
    public string Name { get; set; } = "";
    public long? Parent { get; set; }
    public long Position { get; set; }
}

public class DataPartRowFieldTests
{
    [Fact]
    public void A_field_is_read_and_written_in_any_case_and_made_its_type()
    {
        var folder = new Folder { Id = 1, Name = "Music" };
        Assert.Equal("Music", RowFields.Read(folder, "name"));
        RowFields.Set(folder, "parent", 3);
        Assert.Equal(3L, folder.Parent);
        Assert.Equal(typeof(long), RowFields.TypeOf(folder, "parent"));
        var row = new Dictionary<string, object?> { ["title"] = "Milk" };
        Assert.Equal("Milk", RowFields.Read(row, "Title"));
        RowFields.Set(row, "title", "Oat milk");
        Assert.Equal("Oat milk", row["title"]);
        Assert.Equal("tasks[2].done", RowFields.Path("tasks", 2, "done"));
    }
}
