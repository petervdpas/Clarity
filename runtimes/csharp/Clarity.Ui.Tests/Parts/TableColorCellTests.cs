// A column of colors ("#rrggbb") in a table: its cells show a swatch of the
// color beside the color's text, shown or edited, as the shared web table does.
// The field's facts say it is a color (FieldFacts.Color, from Fields.cs).

using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui.Tests;

public class TableColorCellTests
{
    static List<object> People() =>
    [
        new Dictionary<string, object?> { ["name"] = "Ada", ["color"] = "#ff0000" },
        new Dictionary<string, object?> { ["name"] = "Bob", ["color"] = "" },
    ];

    static DataHost ColorHost()
    {
        var host = new DataHost();
        host.Lists["people"] = People();
        host.Fields["people.color"] = new FieldFacts("Color", Color: true);
        return host;
    }

    [Fact]
    public void A_column_of_a_color_field_shows_a_swatch_of_it()
    {
        var host = ColorHost();
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        Assert.Equal(CellKind.Color, table.Columns.First(c => c.Field == "color").Kind);
        Assert.Equal("#ff0000", table.Shown(0, "color"));
        using var pixels = Shot.Of(screen, 800, 400);
        var swatch = table.SwatchBox(0, "color")!.Value;
        Assert.Equal(new SKColor(255, 0, 0), pixels.GetPixel((int)(swatch.X + swatch.Width / 2), (int)(swatch.Y + swatch.Height / 2)));
        // A cell without a color has no swatch.
        Assert.Null(table.SwatchBox(1, "color"));
    }

    [Fact]
    public void An_edited_color_cell_shows_its_swatch_and_takes_typed_text()
    {
        var host = ColorHost();
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" edit="color" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        using var pixels = Shot.Of(screen, 800, 400);
        var swatch = table.SwatchBox(0, "color")!.Value;
        Assert.Equal(new SKColor(255, 0, 0), pixels.GetPixel((int)(swatch.X + swatch.Width / 2), (int)(swatch.Y + swatch.Height / 2)));
    }

    [Fact]
    public void A_column_of_a_field_that_is_no_color_stays_text()
    {
        var host = new DataHost();
        host.Lists["people"] = People();
        var (_, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        Assert.Equal(CellKind.Text, table.Columns.First(c => c.Field == "color").Kind);
        Assert.Null(table.SwatchBox(0, "color"));
    }
}
