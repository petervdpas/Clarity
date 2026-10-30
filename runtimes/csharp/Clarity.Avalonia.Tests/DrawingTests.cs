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

    static System.Text.Json.JsonElement Views() =>
        System.Text.Json.JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "drawing.json"))).RootElement.GetProperty("views");

    static global::Avalonia.Rect RectOf(System.Text.Json.JsonElement v) => new(v[0].GetDouble(), v[1].GetDouble(), v[2].GetDouble(), v[3].GetDouble());

    /// A canvas that zooms keeps the point under the pointer, within the canvas.
    [Fact]
    public void ACanvasZoomsAndScrollsAsEveryRendererDoes()
    {
        foreach (var c in Views().GetProperty("zoomed").EnumerateArray())
        {
            var units = new global::Avalonia.Size(c.GetProperty("units")[0].GetDouble(), c.GetProperty("units")[1].GetDouble());
            var at = new global::Avalonia.Point(c.GetProperty("at")[0].GetDouble(), c.GetProperty("at")[1].GetDouble());
            Assert.Equal(RectOf(c.GetProperty("result")), CanvasView.Zoomed(units, RectOf(c.GetProperty("view")), c.GetProperty("factor").GetDouble(), at));
        }
        foreach (var c in Views().GetProperty("scrolled").EnumerateArray())
        {
            var units = new global::Avalonia.Size(c.GetProperty("units")[0].GetDouble(), c.GetProperty("units")[1].GetDouble());
            var by = new global::Avalonia.Vector(c.GetProperty("by")[0].GetDouble(), c.GetProperty("by")[1].GetDouble());
            Assert.Equal(RectOf(c.GetProperty("result")), CanvasView.Scrolled(units, RectOf(c.GetProperty("view")), by));
        }
        foreach (var c in Views().GetProperty("points").EnumerateArray())
        {
            var part = new global::Avalonia.Size(c.GetProperty("part")[0].GetDouble(), c.GetProperty("part")[1].GetDouble());
            var at = new global::Avalonia.Point(c.GetProperty("at")[0].GetDouble(), c.GetProperty("at")[1].GetDouble());
            var point = c.GetProperty("point");
            Assert.Equal(point.ValueKind == System.Text.Json.JsonValueKind.Null ? null : new Pointer(point[0].GetInt64(), point[1].GetInt64()), CanvasView.PointIn(RectOf(c.GetProperty("view")), part, at));
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
