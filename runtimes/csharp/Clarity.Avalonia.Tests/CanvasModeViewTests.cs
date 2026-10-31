// A canvas with a mode on screen (DESIGN 5.31), driven by the mouse and keys
// as a person would: it sends what was done as one action and the runtime
// decides. Objects drag by their box, paint strokes come back as the picture
// after them, a vector canvas draws, moves and deletes shapes with its tools.

using System.Text.Json.Nodes;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Clarity.Avalonia;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class CanvasModeViewTests
{
    /// A canvas of 200 x 100 units in a window of 400 x 200 pixels: 2 pixels a unit.
    static (Window, CanvasView, List<JsonObject>) Shown(string mode, Action<CanvasView>? set = null)
    {
        var actions = new List<JsonObject>();
        var canvas = new CanvasView { Mode = mode, UnitsWidth = 200, UnitsHeight = 100, ActionCommand = new RelayCommand<JsonObject>(a => actions.Add(a!)) };
        set?.Invoke(canvas);
        var window = new Window { Width = 400, Height = 200, Content = canvas };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        return (window, canvas, actions);
    }

    static void Drag(Window window, params Point[] points)
    {
        window.MouseDown(points[0], MouseButton.Left);
        foreach (var p in points.Skip(1))
            window.MouseMove(p);
        window.MouseUp(points[^1], MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
    }

    [AvaloniaFact]
    public void An_object_is_dragged_by_its_box_and_its_drop_is_sent()
    {
        var (window, canvas, actions) = Shown("objects", c => c.Drawing =
        [
            new Shape.Object(7L, 20, 20, 40, 20),
            new Shape.Rect(20, 20, 40, 20, "accent"),
            new Shape.Object(8L, 100, 50, 40, 20),
            new Shape.Rect(100, 50, 40, 20, "text"),
        ]);
        window.MouseDown(new Point(60, 50), MouseButton.Left);
        window.MouseMove(new Point(80, 70));
        window.MouseMove(new Point(100, 90));
        // While dragged, the box and what is drawn after it move; the next object stays.
        var shown = CanvasView.WithObjectMoved(canvas.Drawing, 0, new Vector(20, 20)).ToList();
        Assert.Equal(new Shape.Rect(40, 40, 40, 20, "accent"), shown[1]);
        Assert.Equal(new Shape.Rect(100, 50, 40, 20, "text"), shown[3]);
        window.MouseUp(new Point(100, 90), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        var move = Assert.Single(actions)["move"]!;
        Assert.Equal((7L, 40.0, 40.0), ((long)move["id"]!, (double)move["x"]!, (double)move["y"]!));
        // A press beside every box drags nothing.
        Drag(window, new Point(390, 10), new Point(380, 30));
        Assert.Single(actions);
    }

    // The draw handler runs after every handler of the window, a timer's
    // tick among them: a drawing drawn anew while an object is dragged keeps
    // the drag, on the same object in the new drawing.
    [AvaloniaFact]
    public void A_drag_goes_on_while_the_drawing_is_drawn_anew()
    {
        Shape[] Drawn() =>
        [
            new Shape.Object(7L, 20, 20, 40, 20),
            new Shape.Rect(20, 20, 40, 20, "accent"),
        ];
        var (window, canvas, actions) = Shown("objects", c => c.Drawing = Drawn());
        window.MouseDown(new Point(60, 50), MouseButton.Left);
        window.MouseMove(new Point(80, 70));
        canvas.Drawing = [new Shape.Clear(""), .. Drawn()];
        window.MouseMove(new Point(100, 90));
        window.MouseUp(new Point(100, 90), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        var move = Assert.Single(actions)["move"]!;
        Assert.Equal((7L, 40.0, 40.0), ((long)move["id"]!, (double)move["x"]!, (double)move["y"]!));
    }

    [AvaloniaFact]
    public void A_stroke_is_sent_as_the_picture_after_it()
    {
        var (window, canvas, actions) = Shown("paint", c =>
        {
            c.Ink = "#ff0000";
            c.Brush = 6;
        });
        Assert.Equal(["Pen", "Eraser"], canvas.ToolButtons.Select(b => (string)b.Content!));
        Drag(window, new Point(20, 100), new Point(200, 100), new Point(380, 100));
        var data = (string)Assert.Single(actions)["stroke"]!["data"]!;
        using var picture = new Bitmap(new MemoryStream(Convert.FromBase64String(data)));
        // The picture is the canvas's units, painted in its ink along the stroke.
        Assert.Equal(new PixelSize(200, 100), picture.PixelSize);
        var shown = CanvasShapeTests.Rendered(new Image { Source = picture, Width = 200, Height = 100 }, 200, 100);
        Assert.Equal((255, 0, 0), CanvasShapeTests.Pixel(shown, 100, 50));
        Assert.NotEqual((255, 0, 0), CanvasShapeTests.Pixel(shown, 100, 80));
        // A readonly canvas paints nothing.
        canvas.IsReadOnly = true;
        Drag(window, new Point(20, 150), new Point(380, 150));
        Assert.Single(actions);
    }

    [AvaloniaFact]
    public void A_vector_canvas_draws_moves_and_deletes_with_its_tools()
    {
        var (window, canvas, actions) = Shown("vector", c =>
        {
            c.Tools = "select,rect";
            c.Ink = "#00ff00";
        });
        Assert.Equal(["Select", "Rectangle"], canvas.ToolButtons.Select(b => (string)b.Content!));
        canvas.Tool = "rect";
        // Below the toolbar, which sits in the top left corner.
        Drag(window, new Point(20, 60), new Point(60, 80), new Point(100, 100));
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"add": {"op": "rect", "x": 10, "y": 30, "width": 40, "height": 20, "color": "#00ff00"}}"""), actions[^1]), actions[^1].ToJsonString());
        // The drawing comes back from the runtime, its shape chosen.
        canvas.Svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 200 100\">\n<rect x=\"10\" y=\"30\" width=\"40\" height=\"20\" fill=\"#00ff00\"/>\n</svg>\n";
        canvas.Tool = "select";
        Drag(window, new Point(40, 70), new Point(60, 80), new Point(80, 90));
        Assert.Equal(0L, canvas.Chosen);
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"move": {"index": 1, "x": 30, "y": 40}}"""), actions[^1]), actions[^1].ToJsonString());
        window.KeyPressQwerty(PhysicalKey.Delete, RawInputModifiers.None);
        Dispatcher.UIThread.RunJobs();
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"delete": 1}"""), actions[^1]), actions[^1].ToJsonString());
        // A press on no shape chooses none.
        Drag(window, new Point(390, 190));
        Assert.Null(canvas.Chosen);
    }

    [AvaloniaFact]
    public void The_text_tool_asks_for_its_text_in_place_and_enter_adds_it()
    {
        var (window, canvas, actions) = Shown("vector", c => c.Tool = "text");
        // Without tools, every tool of the mode, in its order.
        Assert.Equal(["Select", "Line", "Rectangle", "Ellipse", "Path", "Text"], canvas.ToolButtons.Select(b => (string)b.Content!));
        Drag(window, new Point(100, 120));
        var typing = Assert.IsType<TextBox>(canvas.Typing);
        Assert.Equal("The text", typing.PlaceholderText);
        typing.Text = "Hello";
        typing.Focus();
        window.KeyPressQwerty(PhysicalKey.Enter, RawInputModifiers.None);
        Dispatcher.UIThread.RunJobs();
        Assert.Null(canvas.Typing);
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"add": {"op": "text", "x": 50, "y": 60, "text": "Hello", "color": "text", "size": 10}}"""), Assert.Single(actions)), actions[0].ToJsonString());
    }
}
