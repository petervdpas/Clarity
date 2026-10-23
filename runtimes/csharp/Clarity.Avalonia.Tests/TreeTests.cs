// Trees against the cases every renderer shares (conformance/trees.json),
// and through the real ClarityTable: its toggle shows the rows under a row,
// and a row selected by code opens the rows above it.

using System.Collections.ObjectModel;
using System.Text.Json;
using Clarity.Avalonia;
using Avalonia.Headless.XUnit;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class TreeTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "trees.json"))).RootElement;

    static TreeRow[] Rows(JsonElement rows) =>
        [.. rows.EnumerateArray().Select(r => new TreeRow(r.GetProperty("id").GetInt64(), r.GetProperty("parent").ValueKind == JsonValueKind.Null ? null : r.GetProperty("parent").GetInt64()))];

    [AvaloniaFact]
    public void A_tree_shows_its_rows_as_every_renderer_does()
    {
        foreach (var c in File.GetProperty("cases").EnumerateArray())
        {
            var open = c.GetProperty("open").EnumerateArray().Select(o => (object?)o.GetInt64()).ToHashSet();
            var shown = Trees.Shown(Rows(c.GetProperty("rows")), open).Select(s => $"{s.Place},{s.Depth},{s.Children}").ToList();
            var expect = c.GetProperty("shows").EnumerateArray().Select(s => $"{s[0].GetInt32()},{s[1].GetInt32()},{s[2].GetBoolean()}").ToList();
            Assert.True(expect.SequenceEqual(shown), c.GetProperty("name").GetString());
        }
        foreach (var c in File.GetProperty("reveal").EnumerateArray())
        {
            var rows = Rows(c.GetProperty("rows"));
            var ids = Trees.Above(rows, c.GetProperty("at").GetInt32());
            Assert.Equal(c.GetProperty("opens").EnumerateArray().Select(o => (object?)o.GetInt64()), rows.Select(r => r.Id).Where(ids.Contains));
        }
    }

    public sealed class Folder
    {
        public long Id { get; set; }
        public string Name { get; set; } = "";
        public long? Parent { get; set; }
    }

    [AvaloniaFact]
    public void A_toggle_shows_the_rows_under_a_row_and_a_selected_row_opens_its_parents()
    {
        var folders = new ObservableCollection<Folder>
        {
            new() { Id = 1, Name = "Documents" },
            new() { Id = 2, Name = "Invoices", Parent = 1 },
            new() { Id = 3, Name = "2026", Parent = 2 },
            new() { Id = 4, Name = "Music" },
        };
        var table = new ClarityTable { TreeParent = "Parent", HeadersVisible = false };
        table.Columns.Add(new TableColumn { Field = "Name", Header = "Name" });
        table.ItemsSource = folders;
        List<string> Shown() => [.. table.Grid.ItemsSource!.Cast<Folder>().Select(f => f.Name)];
        Assert.Equal(["Documents", "Music"], Shown());
        Assert.Equal(new TreeShown(0, 0, true), table.TreePlace(folders[0]));
        table.Toggle(folders[0]);
        Assert.Equal(["Documents", "Invoices", "Music"], Shown());
        Assert.True(table.IsOpen(folders[0]));
        table.SelectedItem = folders[2];
        Assert.Equal(["Documents", "Invoices", "2026", "Music"], Shown());
        Assert.Equal(new TreeShown(2, 2, false), table.TreePlace(folders[2]));
        table.Toggle(folders[0]);
        Assert.Equal(["Documents", "Music"], Shown());
    }
}
