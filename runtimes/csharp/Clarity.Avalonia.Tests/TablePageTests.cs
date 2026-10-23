// A table shown a page at a time against the cases every renderer shares
// (conformance/table-pages.json), run through the real ClarityTable: its
// bar turns the pages, a sort shows the first page, and a row selected by
// code shows its page.

using System.Collections.ObjectModel;
using System.Text.Json;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class TablePageTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "table-pages.json"))).RootElement;

    public sealed class Album
    {
        public string Title { get; set; } = "";
    }

    [AvaloniaFact]
    public void A_page_shows_the_rows_every_renderer_shows()
    {
        foreach (var c in File.GetProperty("pages").EnumerateArray())
        {
            var got = TablePages.Of(c.GetProperty("count").GetInt32(), c.GetProperty("size").GetInt32(), c.GetProperty("page").GetInt32());
            Assert.Equal(new Paged(c.GetProperty("shows").GetInt32(), c.GetProperty("pages").GetInt32(), c.GetProperty("from").GetInt32(), c.GetProperty("to").GetInt32(), c.GetProperty("text").GetString()!), got);
        }
        foreach (var c in File.GetProperty("selected").EnumerateArray())
            Assert.Equal(c.GetProperty("page").GetInt32(), TablePages.With(c.GetProperty("size").GetInt32(), c.GetProperty("at").GetInt32()));
    }

    static List<string> Shown(ClarityTable table) => [.. table.Grid.ItemsSource!.Cast<Album>().Select(a => a.Title)];

    [AvaloniaFact]
    public void The_bar_turns_the_pages_and_a_selected_row_shows_its_page()
    {
        var albums = new ObservableCollection<Album>(Enumerable.Range(1, 7).Select(i => new Album { Title = $"Album {i:00}" }));
        var table = new ClarityTable { Sortable = true, PageSize = 3, NextText = "Next page" };
        table.Columns.Add(new TableColumn { Field = "Title", Header = "Title" });
        table.ItemsSource = albums;
        Assert.Equal(["Album 01", "Album 02", "Album 03"], Shown(table));
        Assert.Equal("1-3 / 7", table.Shown.Text);
        Assert.False(table.First.IsEnabled);
        Assert.Equal("Next page", table.NextText);
        table.Next.RaiseEvent(new RoutedEventArgs(global::Avalonia.Controls.Button.ClickEvent));
        Assert.Equal(["Album 04", "Album 05", "Album 06"], Shown(table));
        table.Last.RaiseEvent(new RoutedEventArgs(global::Avalonia.Controls.Button.ClickEvent));
        Assert.Equal(["Album 07"], Shown(table));
        Assert.False(table.Next.IsEnabled);
        // Rows that come again keep the page, within the pages there are.
        albums.RemoveAt(6);
        Assert.Equal("4-6 / 6", table.Shown.Text);
        // A sort shows the first page.
        table.SortBy("Title");
        Assert.Equal("1-3 / 6", table.Shown.Text);
        // A row selected by code shows its page.
        table.SelectedItem = albums[4];
        Assert.Equal(["Album 04", "Album 05", "Album 06"], Shown(table));
    }
}
