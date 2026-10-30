// A canvas drawn by code: its part's drawing, drawn anew after every
// handler, and the canvas view that shows it (conformance/drawing.json).

using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Media;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// A window with a canvas its draw handler draws: a dot per point of speed.
sealed class GaugeViewModel(Db db) : WindowViewModel(db)
{
    public PartState Dial { get; } = new("dial");
    public long Speed { get; set; } = 2;
    public int Draws { get; private set; }

    protected override void Redraw()
    {
        Dial.BeginDrawing();
        Draws++;
        Dial.Draw(new Shape.Clear("panel"));
        for (var i = 0; i < Speed; i++)
            Dial.Draw(new Shape.Circle(10 * i, 50, 4, "accent"));
        Dial.EndDrawing();
    }

    public void Faster() => Handle(() => Speed++);

    /// What a generated window gives a canvas with `on click`.
    public string Clicked { get; private set; } = "";

    public void DialClicked(Pointer at) => Handle(() =>
    {
        Dial.Pointer = at;
        Clicked = $"{Dial.X},{Dial.Y}";
    });
}

public class DrawingTests
{
    [AvaloniaFact]
    public void ACanvasIsDrawnAnewWhenItsWindowOpensAndAfterEveryHandler()
    {
        var vm = new GaugeViewModel(null!);
        vm.Opened();
        Assert.Equal(3, vm.Dial.Drawing.Count);
        var changes = 0;
        vm.Dial.PropertyChanged += (_, e) => changes += e.PropertyName == nameof(PartState.Drawing) ? 1 : 0;
        vm.Faster();
        Assert.Equal(4, vm.Dial.Drawing.Count);
        // One change per draw, however many shapes it has.
        Assert.Equal((2, 1), (vm.Draws, changes));
    }

    [AvaloniaFact]
    public void AColorIsTheThemesByNameOrWrittenOut()
    {
        var view = new CanvasView();
        view.Resources["ClarityAccent"] = new SolidColorBrush(Colors.Teal);
        view.Resources["ClarityText"] = new SolidColorBrush(Colors.White);
        view.Resources["ClarityLcdText"] = new SolidColorBrush(Colors.Lime);
        var window = new Window { Content = view };
        window.Show();
        Assert.Equal(Colors.Teal, ((ISolidColorBrush)view.BrushOf("accent")).Color);
        Assert.Equal(Colors.Lime, ((ISolidColorBrush)view.BrushOf("lcd_text")).Color);
        Assert.Equal(Color.Parse("#101810"), ((ISolidColorBrush)view.BrushOf("#101810")).Color);
        // A name the theme does not have is its text color.
        Assert.Equal(Colors.White, ((ISolidColorBrush)view.BrushOf("nothing")).Color);
    }

    [AvaloniaFact]
    public void ACanvasViewDrawsEveryShapeInItsUnits()
    {
        var view = new CanvasView
        {
            UnitsWidth = 200,
            UnitsHeight = 100,
            Width = 400,
            Height = 300,
            Drawing =
            [
                new Shape.Clear("#101810"),
                new Shape.Line(10, 20, 30, 40, "accent", 3),
                new Shape.Rect(5, 5, 20, 10, "text"),
                new Shape.Circle(50, 50, 40, "accent"),
                new Shape.Ring(50, 50, 40, "border", 2),
                new Shape.Words(50, 90, Text.Plain("km/h"), "muted", 8),
            ],
        };
        var window = new Window { Content = view };
        window.Show();
        // Kept in proportion and centered: 2 pixels a unit, 50 pixels above and below.
        Assert.Equal((2.0, 0.0, 50.0), view.Placement());
    }

    static System.Text.Json.JsonElement Cases(string name) =>
        System.Text.Json.JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "drawing.json"))).RootElement.GetProperty(name);

    static global::Avalonia.Size SizeOf(System.Text.Json.JsonElement v) => new(v[0].GetDouble(), v[1].GetDouble());
    static global::Avalonia.Point PointOf(System.Text.Json.JsonElement v) => new(v[0].GetDouble(), v[1].GetDouble());
    static CanvasViewport ViewOf(System.Text.Json.JsonElement v) => new(v[0].GetDouble(), v[1].GetDouble(), v[2].GetDouble());

    /// A canvas's grid and snap are as its line says until code sets them; the view follows the grid.
    [Fact]
    public void A_canvas_part_keeps_the_grid_and_snap_code_sets()
    {
        var sheet = new PartState("sheet") { Grid = 20, Snap = true };
        var changed = new List<string?>();
        sheet.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        sheet.Grid = 0;
        sheet.Snap = false;
        sheet.Background = "#f5f0e1";
        Assert.Equal((0L, false, "#f5f0e1"), (sheet.Grid, sheet.Snap, sheet.Background));
        Assert.Equal(["Grid", "Snap", "Background"], changed);
        // The grid's dots: 2 pixels in the theme's border, until code sets them.
        Assert.Equal((2L, ""), (sheet.DotSize, sheet.DotColor));
        sheet.DotSize = 3;
        sheet.DotColor = "muted";
        Assert.Equal((3L, "muted"), (sheet.DotSize, sheet.DotColor));
    }

    /// A grid thins its dots to 8 pixels apart or more, as every renderer draws it.
    [Fact]
    public void ACanvasGridThinsItsDotsAsEveryRendererDoes()
    {
        foreach (var c in Cases("grids").GetProperty("lines").EnumerateArray())
            Assert.Equal(c.GetProperty("every").GetDouble(), CanvasView.GridEvery(c.GetProperty("grid").GetDouble(), c.GetProperty("scale").GetDouble()));
    }

    /// A canvas that zooms starts whole, zooms around the pointer within its scales,
    /// pans past its edges, and finds the unit under a pixel, as every renderer does.
    [Fact]
    public void ACanvasZoomsAndPansAsEveryRendererDoes()
    {
        var views = Cases("views");
        foreach (var c in views.GetProperty("whole").EnumerateArray())
            Assert.Equal(ViewOf(c.GetProperty("view")), CanvasView.Whole(SizeOf(c.GetProperty("units")), SizeOf(c.GetProperty("part"))));
        foreach (var c in views.GetProperty("zoomed").EnumerateArray())
        {
            var found = CanvasView.Zoomed(SizeOf(c.GetProperty("units")), SizeOf(c.GetProperty("part")), ViewOf(c.GetProperty("view")), c.GetProperty("factor").GetDouble(), PointOf(c.GetProperty("at")));
            var wanted = ViewOf(c.GetProperty("result"));
            Assert.Equal((wanted.Scale, wanted.X, wanted.Y), (Math.Round(found.Scale, 9), Math.Round(found.X, 9), Math.Round(found.Y, 9)));
        }
        foreach (var c in views.GetProperty("scrolled").EnumerateArray())
            Assert.Equal(ViewOf(c.GetProperty("result")), CanvasView.Scrolled(ViewOf(c.GetProperty("view")), (global::Avalonia.Vector)PointOf(c.GetProperty("by"))));
        foreach (var c in views.GetProperty("points").EnumerateArray())
        {
            var point = c.GetProperty("point");
            var wanted = point.ValueKind == System.Text.Json.JsonValueKind.Null ? null : new Pointer(point[0].GetInt64(), point[1].GetInt64());
            Assert.Equal(wanted, CanvasView.PointIn(SizeOf(c.GetProperty("units")), ViewOf(c.GetProperty("view")), PointOf(c.GetProperty("at"))));
        }
    }

    [Fact]
    public void ACanvasFindsWhereItWasClickedAsEveryRendererDoes()
    {
        var file = System.Text.Json.JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "drawing.json")));
        var cases = file.RootElement.GetProperty("clicks").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var c in cases)
        {
            double At(string key, int i) => c.GetProperty(key)[i].GetDouble();
            var found = CanvasView.PointOf(new(At("units", 0), At("units", 1)), new(At("part", 0), At("part", 1)), new(At("at", 0), At("at", 1)));
            var point = c.GetProperty("point");
            Assert.Equal(point.ValueKind == System.Text.Json.JsonValueKind.Null ? null : new Pointer(point[0].GetInt64(), point[1].GetInt64()), found);
        }
    }

    [AvaloniaFact]
    public void AClickOnACanvasRunsItsHandlerWithWhereAndItIsDrawnAnew()
    {
        var vm = new GaugeViewModel(null!);
        vm.Opened();
        Assert.Equal((0L, 0L), (vm.Dial.X, vm.Dial.Y));
        vm.DialClicked(new Pointer(120, 150));
        Assert.Equal(("120,150", 2), (vm.Clicked, vm.Draws));
    }

    /// A drag gives where the pointer is and where it was pressed; a click has no start.
    [AvaloniaFact]
    public void ADragOnACanvasSaysWhereItStarted()
    {
        var vm = new GaugeViewModel(null!);
        vm.Opened();
        vm.DialClicked(new Pointer(900, 410) { FromX = 860, FromY = 400 });
        Assert.Equal((900L, 410L, 860L, 400L), (vm.Dial.X, vm.Dial.Y, vm.Dial.FromX, vm.Dial.FromY));
        vm.DialClicked(new Pointer(7, 3));
        Assert.Equal((0L, 0L), (vm.Dial.FromX, vm.Dial.FromY));
    }

    /// A foldable panel or form is open while it is not folded: the Expander
    /// binds to IsOpen, the code sets IsFolded (conformance/folding.json).
    [Fact]
    public void AFoldablePartIsOpenWhileItIsNotFolded()
    {
        var details = new PartState("details");
        var changed = new List<string?>();
        details.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        Assert.True(details.IsOpen);
        details.IsFolded = true;
        Assert.False(details.IsOpen);
        Assert.Equal(["IsFolded", "IsOpen"], changed);
        details.IsOpen = true;
        Assert.False(details.IsFolded);
    }
}
