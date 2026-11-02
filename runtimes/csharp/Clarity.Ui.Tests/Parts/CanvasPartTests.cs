// A canvas drawn by code (conformance/drawing.json): its shapes in its own
// units, fitted in proportion and centered; where a click, a drag, a drop and
// a bend are in its units; the view of a canvas that zooms; its grid. With a
// mode (conformance/canvas-modes.json) the person drags its objects, paints
// or draws, and the canvas sends what was done to the window as one action.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>A window that runs a canvas's commands, as a generated view model has them, and logs each.</summary>
public class CanvasPartHost : TestHost
{
    public readonly List<(string Part, string Event, Pointer At)> Points = [];
    public readonly List<(string Part, JsonObject Action)> Actions = [];
    public readonly List<(string Part, string Tell, JsonElement[] Values)> Tells = [];
    public readonly Dictionary<string, Func<IFrames>> Engines = [];

    /// <summary>What a pointer event does to the drawing, as a handler would draw anew.</summary>
    public Action<string, Pointer>? OnPoint;

    public override bool? Pointed(string part, string @event, Pointer at)
    {
        Points.Add((part, @event, at));
        State(part)!.Pointer = at;
        OnPoint?.Invoke(@event, at);
        return true;
    }

    public override bool? Acted(string part, JsonObject action)
    {
        Actions.Add((part, action));
        return true;
    }

    public override Func<IFrames>? Engine(string name) => Engines.GetValueOrDefault(name);

    public override void Told(string part, string tell, JsonElement[] values) => Tells.Add((part, tell, values));

    /// <summary>Draws <paramref name="shapes"/> on the canvas's state, as its draw handler does.</summary>
    public void Draw(string part, params Shape[] shapes)
    {
        var state = State(part)!;
        state.BeginDrawing();
        foreach (var shape in shapes)
            state.Draw(shape);
        state.EndDrawing();
        Change();
    }
}

public class CanvasPartTests
{
    static JsonElement Conformance(string name) =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", name))).RootElement;

    static readonly JsonElement Drawing = Conformance("drawing.json");

    static (double, double) Pair(JsonElement e) => (e[0].GetDouble(), e[1].GetDouble());

    static CanvasViewport View(JsonElement e) => new(e[0].GetDouble(), e[1].GetDouble(), e[2].GetDouble());

    static (CanvasPartHost Host, Screen Screen, CanvasPart Canvas) Canvas(string attributes, CanvasPartHost? host = null, double width = 800, double height = 400)
    {
        host ??= new CanvasPartHost();
        var (screen, canvas) = Alone($"""<Canvas name="pad" {attributes} />""", host, width, height);
        return (host, screen, (CanvasPart)canvas);
    }

    /// <summary>A part of <paramref name="markup"/> as the whole of a screen <paramref name="width"/> by <paramref name="height"/>, so its pixels are the screen's.</summary>
    internal static (Screen Screen, Part Part) Alone(string markup, IPartHost host, double width = 800, double height = 400)
    {
        var part = PartKinds.Make(WindowMarkup.Read($"<Window name=\"Test\">{markup}</Window>").Parts[0], host);
        var screen = new Screen(part, ThemeLook.Clarity(dark: false));
        screen.Resize(width, height, 1);
        return (screen, part);
    }

    static SKColor Pixel(Screen screen, int x, int y, int width = 800, int height = 400)
    {
        using var shot = Shot.Of(screen, width, height);
        return shot.PixelAt(x, y);
    }

    [Fact]
    public void A_click_is_where_in_the_units_as_on_every_renderer()
    {
        foreach (var c in Drawing.GetProperty("clicks").EnumerateArray())
        {
            var (uw, uh) = Pair(c.GetProperty("units"));
            var (pw, ph) = Pair(c.GetProperty("part"));
            var (x, y) = Pair(c.GetProperty("at"));
            var got = CanvasPart.PointOf(uw, uh, pw, ph, x, y);
            var point = c.GetProperty("point");
            Assert.Equal(point.ValueKind == JsonValueKind.Null ? null : new Pointer(point[0].GetInt64(), point[1].GetInt64()), got);
        }
    }

    [Fact]
    public void Its_view_starts_whole_zooms_around_the_pointer_and_pans_as_on_every_renderer()
    {
        var views = Drawing.GetProperty("views");
        foreach (var c in views.GetProperty("whole").EnumerateArray())
        {
            var (uw, uh) = Pair(c.GetProperty("units"));
            var (pw, ph) = Pair(c.GetProperty("part"));
            AssertView(View(c.GetProperty("view")), CanvasPart.Whole(uw, uh, pw, ph));
        }
        foreach (var c in views.GetProperty("zoomed").EnumerateArray())
        {
            var (uw, uh) = Pair(c.GetProperty("units"));
            var (pw, ph) = Pair(c.GetProperty("part"));
            var (ax, ay) = Pair(c.GetProperty("at"));
            AssertView(View(c.GetProperty("result")), CanvasPart.Zoomed(uw, uh, pw, ph, View(c.GetProperty("view")), c.GetProperty("factor").GetDouble(), ax, ay));
        }
        foreach (var c in views.GetProperty("scrolled").EnumerateArray())
        {
            var (bx, by) = Pair(c.GetProperty("by"));
            AssertView(View(c.GetProperty("result")), CanvasPart.Scrolled(View(c.GetProperty("view")), bx, by));
        }
        foreach (var c in views.GetProperty("points").EnumerateArray())
        {
            var (uw, uh) = Pair(c.GetProperty("units"));
            var (ax, ay) = Pair(c.GetProperty("at"));
            var point = c.GetProperty("point");
            Assert.Equal(point.ValueKind == JsonValueKind.Null ? null : new Pointer(point[0].GetInt64(), point[1].GetInt64()), CanvasPart.PointIn(uw, uh, View(c.GetProperty("view")), ax, ay));
        }
    }

    static void AssertView(CanvasViewport expected, CanvasViewport got)
    {
        Assert.Equal(expected.Scale, got.Scale, 6);
        Assert.Equal(expected.X, got.X, 6);
        Assert.Equal(expected.Y, got.Y, 6);
    }

    [Fact]
    public void Its_grid_is_thinned_to_dots_eight_pixels_apart_as_on_every_renderer()
    {
        foreach (var c in Drawing.GetProperty("grids").GetProperty("lines").EnumerateArray())
            Assert.Equal(c.GetProperty("every").GetDouble(), CanvasPart.GridEvery(c.GetProperty("grid").GetDouble(), c.GetProperty("scale").GetDouble()));
    }

    [Fact]
    public void Words_start_by_their_alignment() =>
        Assert.Equal((0.0, 40.0, 20.0), (CanvasPart.TextLeft("left", 40), CanvasPart.TextLeft("right", 40), CanvasPart.TextLeft("center", 40)));

    [Fact]
    public void It_draws_its_shapes_in_its_units_fitted_and_centered_in_the_theme_s_colors()
    {
        // 100 x 100 units in 800 x 400 pixels: 4 pixels a unit, starting 200 pixels in.
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" """);
        host.Draw("pad", new Shape.Clear(""), new Shape.Rect(10, 10, 20, 20, "accent"), new Shape.Circle(75, 75, 5, "#ff0000"));
        var look = ThemeLook.Clarity(dark: false);
        Assert.Equal(look.Color("accent"), Pixel(screen, 200 + 80, 80));
        Assert.Equal(new SKColor(255, 0, 0), Pixel(screen, 200 + 300, 300));
        // A clear without a color fills the units with the theme's panel; beside them is the window.
        Assert.Equal(look.Color("panel"), Pixel(screen, 200 + 200, 200));
        Assert.Equal(look.Color("background"), Pixel(screen, 100, 200));
    }

    [Fact]
    public void A_clear_without_a_color_fills_it_with_its_background()
    {
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" """);
        host.State("pad")!.Background = "accent";
        host.Draw("pad", new Shape.Clear(""));
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("accent"), Pixel(screen, 400, 200));
    }

    [Fact]
    public void A_name_the_theme_has_not_is_its_text_color() =>
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("text"), CanvasPart.ColorOf(ThemeLook.Clarity(dark: false), "no_such_role"));

    [Fact]
    public void A_click_runs_its_handler_with_where_it_was_in_its_units()
    {
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" onclick="paint" """);
        screen.PointerPressed(204, 8, PointerButton.Left);
        screen.PointerReleased(204, 8, PointerButton.Left);
        Assert.Equal([("pad", "click", new Pointer(1, 2))], host.Points);
        // Beside the drawing it runs nothing.
        screen.PointerPressed(100, 8, PointerButton.Left);
        screen.PointerReleased(100, 8, PointerButton.Left);
        Assert.Single(host.Points);
    }

    [Fact]
    public void Without_the_window_s_commands_a_click_sets_its_pointer_and_runs_its_handler()
    {
        var host = new TestHost();
        var (screen, _) = Alone("""<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" onclick="paint" />""", host);
        screen.PointerPressed(204, 8, PointerButton.Left);
        screen.PointerReleased(204, 8, PointerButton.Left);
        Assert.Equal(new Pointer(1, 2), host.States["pad"].Pointer);
        Assert.Contains("run pad click paint", host.Log);
    }

    [Fact]
    public void A_drag_runs_its_handler_with_where_it_started_and_the_drop_after()
    {
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" ondrag="moved" ondrop="dropped" """);
        // A handler that draws anew: the drag moves something.
        host.OnPoint = (e, at) => host.Draw("pad", new Shape.Rect(at.X, at.Y, 1, 1, "accent"));
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(260, 40);
        screen.PointerMoved(280, 80);
        screen.PointerReleased(280, 80, PointerButton.Left);
        Assert.Equal(["drag", "drag", "drop"], host.Points.Select(p => p.Event));
        Assert.Equal(new Pointer(20, 20) { FromX = 10, FromY = 10 }, host.Points[^1].At);
        Assert.Equal(new Pointer(15, 10) { FromX = 10, FromY = 10 }, host.Points[0].At);
    }

    [Fact]
    public void A_drag_the_handler_does_nothing_with_pans_a_canvas_that_zooms()
    {
        var (host, screen, canvas) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" zoom="true" ondrag="moved" """);
        var before = canvas.Viewport;
        screen.PointerPressed(300, 200, PointerButton.Left);
        screen.PointerMoved(320, 200);
        screen.PointerMoved(340, 200);
        screen.PointerReleased(340, 200, PointerButton.Left);
        Assert.Single(host.Points);
        Assert.Equal(before.X - 40 / before.Scale, canvas.Viewport.X, 6);
    }

    [Fact]
    public void The_wheel_pans_the_buttons_zoom_and_a_redraw_keeps_the_view()
    {
        var (host, screen, canvas) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" zoom="true" """);
        var whole = canvas.Viewport;
        screen.Wheel(400, 200, 0, 50);
        Assert.Equal(whole.Y + 50 / whole.Scale, canvas.Viewport.Y, 6);
        var (zoomOut, back, zoomIn) = canvas.ZoomButtons();
        screen.PointerPressed(zoomIn.X + 5, zoomIn.Y + 5, PointerButton.Left);
        screen.PointerReleased(zoomIn.X + 5, zoomIn.Y + 5, PointerButton.Left);
        Assert.Equal(whole.Scale * 1.2, canvas.Viewport.Scale, 6);
        host.Draw("pad", new Shape.Clear(""));
        Assert.Equal(whole.Scale * 1.2, canvas.Viewport.Scale, 6);
        screen.PointerPressed(back.X + 5, back.Y + 5, PointerButton.Left);
        screen.PointerReleased(back.X + 5, back.Y + 5, PointerButton.Left);
        Assert.Equal(whole, canvas.Viewport);
        Assert.True(zoomOut.X < back.X);
    }

    /// <summary>Shift and the wheel pan a canvas sideways (Screen.Wheel turns it, data-2), as Avalonia's ScrollViewer did.</summary>
    [Fact]
    public void Shift_and_the_wheel_pan_the_canvas_sideways()
    {
        var (_, screen, canvas) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" zoom="true" """);
        var whole = canvas.Viewport;
        screen.Wheel(400, 200, 0, 50, Modifiers.Shift);
        Assert.Equal(whole.X + 50 / whole.Scale, canvas.Viewport.X, 6);
        Assert.Equal(whole.Y, canvas.Viewport.Y, 6);
    }

    [Fact]
    public void Ctrl_and_the_wheel_zoom_at_the_pointer_and_its_buttons_say_what_they_do()
    {
        var (_, screen, canvas) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" zoom="true" """);
        var whole = canvas.Viewport;
        screen.PointerMoved(400, 200);
        // A notch (50 pixels) zooms 1.15 closer, as Avalonia's canvas did.
        screen.Wheel(400, 200, 0, -50, Modifiers.Control);
        Assert.Equal(whole.Scale * 1.15, canvas.Viewport.Scale, 6);
        // The point under the pointer stays where it was.
        Assert.Equal(whole.X + 400 / whole.Scale, canvas.Viewport.X + 400 / canvas.Viewport.Scale, 6);
        // Cmd on macOS zooms as Ctrl does.
        screen.Wheel(400, 200, 0, 50, Modifiers.Meta);
        Assert.Equal(whole.Scale, canvas.Viewport.Scale, 6);
        // A touchpad's ten small turns zoom as far as the one notch they make.
        for (var i = 0; i < 10; i++)
            screen.Wheel(400, 200, 0, -5, Modifiers.Control);
        Assert.Equal(whole.Scale * 1.15, canvas.Viewport.Scale, 6);
        screen.Wheel(400, 200, 0, 50, Modifiers.Control);
        Assert.Equal(whole.Scale, canvas.Viewport.Scale, 6);
        var (zoomOut, back, zoomIn) = canvas.ZoomButtons();
        Assert.Equal(CanvasPart.ZoomWord("zoomIn"), canvas.TooltipAt(zoomIn.X + 5, zoomIn.Y + 5));
        Assert.Equal(CanvasPart.ZoomWord("zoomWhole"), canvas.TooltipAt(back.X + 5, back.Y + 5));
        Assert.Null(canvas.TooltipAt(100, 300));
    }

    [Fact]
    public void Its_grid_shows_dots_in_the_theme_s_border_color()
    {
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" """);
        host.State("pad")!.Grid = 10;
        host.State("pad")!.DotSize = 4;
        host.Draw("pad", new Shape.Clear(""));
        // A dot every 10 units: at unit 10, 10 is pixel 240, 40.
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("border"), Pixel(screen, 240, 40));
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("panel"), Pixel(screen, 260, 60));
    }

    [Fact]
    public void A_link_bends_and_its_handler_hears_its_points()
    {
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" onbend="bent" """);
        host.Draw("pad", new Shape.Link("a-b", 10, 50, 90, 50, [], "text", 1));
        // A press on the link at unit 50, 50 adds a point there; dragging moves it to 50, 30.
        screen.PointerPressed(400, 200, PointerButton.Left);
        screen.PointerMoved(400, 160);
        screen.PointerMoved(400, 120);
        screen.PointerReleased(400, 120, PointerButton.Left);
        var bent = Assert.Single(host.Points);
        Assert.Equal("bend", bent.Event);
        Assert.Equal("a-b", bent.At.Bent);
        Assert.Equal([50L, 30L], bent.At.Via);
    }

    [Fact]
    public void A_press_on_a_shape_drawn_over_a_link_is_no_bend()
    {
        var (host, screen, _) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" onbend="bent" """);
        host.Draw("pad", new Shape.Link("a-b", 10, 50, 90, 50, [], "text", 1), new Shape.Rect(40, 40, 20, 20, "panel"));
        screen.PointerPressed(400, 200, PointerButton.Left);
        screen.PointerMoved(400, 120);
        screen.PointerReleased(400, 120, PointerButton.Left);
        Assert.Empty(host.Points);
    }

    // -- Modes (conformance/canvas-modes.json) -----------------------------------

    const string Units = """units="{&quot;width&quot;:100,&quot;height&quot;:100}" """;

    [Fact]
    public void An_object_is_dragged_by_its_box_and_its_drop_is_sent()
    {
        var (host, screen, canvas) = Canvas($"""{Units} mode="objects" """);
        host.Draw("pad", new Shape.Object(7L, 10, 10, 20, 20), new Shape.Rect(10, 10, 20, 20, "accent"));
        screen.PointerPressed(260, 60, PointerButton.Left);
        screen.PointerMoved(280, 60);
        screen.PointerMoved(300, 80);
        // While dragged its box and what is drawn after it move with the pointer.
        Assert.Equal(20, ((Shape.Rect)canvas.Shown().ToList()[1]).X);
        screen.PointerReleased(300, 80, PointerButton.Left);
        var (part, action) = Assert.Single(host.Actions);
        Assert.Equal("pad", part);
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"move":{"id":7,"x":20,"y":15}}"""), action), action.ToJsonString());
    }

    [Fact]
    public void A_vector_canvas_draws_moves_and_deletes_with_its_tools()
    {
        var (host, screen, canvas) = Canvas($"""{Units} mode="vector" bind="drawing" """);
        host.Values["drawing"] = "";
        host.State("pad")!.Tool = "rect";
        host.State("pad")!.Ink = "accent";
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(280, 80);
        screen.PointerReleased(280, 80, PointerButton.Left);
        var added = Assert.Single(host.Actions).Action;
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"add":{"op":"rect","x":10,"y":10,"width":10,"height":10,"color":"accent"}}"""), added), added.ToJsonString());

        // What the runtime kept comes back as the bound text; select and drag moves it.
        host.Values["drawing"] = CanvasModes.Vector("", new CanvasArgs("vector"), added)!.Text;
        host.Change();
        host.State("pad")!.Tool = "select";
        screen.PointerPressed(260, 60, PointerButton.Left);
        Assert.Equal(0L, host.State("pad")!.SelectedItem);
        screen.PointerMoved(280, 60);
        screen.PointerReleased(280, 60, PointerButton.Left);
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"move":{"index":1,"x":15,"y":10}}"""), host.Actions[^1].Action), host.Actions[^1].Action.ToJsonString());

        // Delete deletes the chosen one.
        screen.KeyPressed(new KeyInput(Key.Delete, Modifiers.None));
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"delete":1}"""), host.Actions[^1].Action));
        Assert.Equal(["Select", "Line", "Rectangle", "Ellipse", "Path", "Text"], canvas.ToolsShown().Select(CanvasPart.ToolWord));
    }

    [Fact]
    public void The_text_tool_takes_its_text_in_place_and_enter_adds_it()
    {
        var (host, screen, canvas) = Canvas($"""{Units} mode="vector" bind="drawing" """);
        host.Values["drawing"] = "";
        host.State("pad")!.Tool = "text";
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerReleased(240, 40, PointerButton.Left);
        Assert.Equal("", canvas.Typing);
        screen.TextTyped("Hi");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Null(canvas.Typing);
        var action = Assert.Single(host.Actions).Action;
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"add":{"op":"text","x":10,"y":10,"text":"Hi","color":"text","size":10}}"""), action), action.ToJsonString());
    }

    [Fact]
    public void A_stroke_is_sent_as_the_picture_after_it()
    {
        var (host, screen, _) = Canvas($"""{Units} mode="paint" bind="picture" """);
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(280, 80);
        screen.PointerReleased(280, 80, PointerButton.Left);
        var action = Assert.Single(host.Actions).Action;
        Assert.True(CanvasModes.IsPicture(action));
        using var picture = SKBitmap.Decode(CanvasModes.StrokeBytes(action));
        Assert.Equal((100, 100), (picture.Width, picture.Height));
        // The pen painted the theme's text color along the line, and nothing far from it.
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("text"), picture.PixelAt(15, 15));
        Assert.Equal(0, picture.PixelAt(80, 20).Alpha);
    }

    [Fact]
    public void A_readonly_canvas_sends_nothing()
    {
        var (host, screen, _) = Canvas($"""{Units} mode="paint" bind="picture" readonly="true" """);
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(280, 80);
        screen.PointerReleased(280, 80, PointerButton.Left);
        Assert.Empty(host.Actions);
    }

    [Fact]
    public void A_shape_of_no_number_draws_nothing_and_the_rest_draws()
    {
        var (host, screen, canvas) = Canvas(Units);
        host.Draw("pad", new Shape.Rect(double.NaN, 0, 10, 10, "accent"), new Shape.Rect(10, 10, 20, 20, "accent"));
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("accent"), Pixel(screen, 280, 80));
        Assert.Null(canvas.Failed);
    }

    [Fact]
    public void A_canvas_text_keeps_its_spaces_and_width_at_every_zoom()
    {
        var fonts = new Fonts(ThemeLook.Clarity(dark: true));
        using var whole = CanvasPart.WordsFont(fonts, 11);
        using var small = CanvasPart.WordsFont(fonts, 11 * 0.52);
        const string text = "string? medewerkerBk and DateTimeOffset? begindatum";
        Assert.Equal(whole.MeasureText(text) * 0.52, small.MeasureText(text), 0.5);
        Assert.True(small.MeasureText(" ") > 1);
    }

    [Fact]
    public void A_canvas_whose_drags_and_bends_are_edits_takes_the_focus_so_its_undo_key_reaches_it()
    {
        var (_, screen, canvas) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" undoScope="pad" ondrag="moved" ondrop="dropped" onbend="bent" """);
        Assert.True(canvas.Focusable);
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerReleased(240, 40, PointerButton.Left);
        Assert.Same(canvas, screen.Focused);
        // Without undo, or with nothing the person changes on it, it takes no focus.
        var (_, _, plain) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" noUndo="true" ondrop="dropped" """);
        Assert.False(plain.Focusable);
        var (_, _, drawn) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" ondraw="drawn" """);
        Assert.False(drawn.Focusable);
    }

    [Fact]
    public void A_canvas_grows_as_every_stack_grows_it_and_so_do_tabs_that_hold_one()
    {
        var (_, _, canvas) = Canvas("""units="{&quot;width&quot;:100,&quot;height&quot;:100}" ondraw="drawn" """);
        Assert.True(canvas.Grows);
        var (_, _, parts) = Made.Window("""<Tabs at="1,1" span="12" rows="3"><Tab title="One"><Canvas name="pad" at="1,1" span="12" rows="2" /></Tab></Tabs><Button at="1,4" span="2" />""", new CanvasPartHost(), width: 808, height: 800);
        Assert.True(parts[0].Grows);
        Assert.True(parts[0].Bounds.Height > 600);
    }
}
