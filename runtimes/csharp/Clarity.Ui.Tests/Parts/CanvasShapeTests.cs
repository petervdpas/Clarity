// What a canvas draws, shape by shape, and how its pointer, toolbar and
// text tool behave, as Avalonia's canvas tests asked of it (CanvasShapeTests,
// CanvasDragTests, CanvasModeViewTests, DrawingTests). A canvas 100 by 100
// units fills 400 by 400 pixels of an 800 by 400 screen, from 200 across:
// unit x, y is pixel 200 + 4x, 4y.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui.Tests;

public class CanvasShapeTests
{
    const string Units = """units="{&quot;width&quot;:100,&quot;height&quot;:100}" """;

    static readonly ThemeLook Look = ThemeLook.Clarity(dark: false);

    static (CanvasPartHost Host, Screen Screen, CanvasPart Canvas) Canvas(string attributes, CanvasPartHost? host = null)
    {
        host ??= new CanvasPartHost();
        var (screen, part) = CanvasPartTests.Alone($"""<Canvas name="pad" {Units} {attributes} />""", host);
        return (host, screen, (CanvasPart)part);
    }

    static SKBitmap Shot(Screen screen) => Clarity.Ui.Tests.Shot.Of(screen, 800, 400);

    /// <summary>A PNG of one color, <paramref name="size"/> pixels square.</summary>
    static byte[] Png(SKColor color, int size = 4)
    {
        using var bitmap = new SKBitmap(new SKImageInfo(size, size, SKColorType.Rgba8888, SKAlphaType.Premul));
        bitmap.Erase(color);
        using var image = SKImage.FromBitmap(bitmap);
        using var png = image.Encode(SKEncodedImageFormat.Png, 100);
        return png.ToArray();
    }

    [Fact]
    public void A_canvas_draws_an_ellipse_and_a_path_and_an_object_draws_nothing()
    {
        var (host, screen, canvas) = Canvas("");
        host.Draw("pad",
            new Shape.Clear("panel"),
            new Shape.Ellipse(25, 25, 20, 10, "accent"),
            new Shape.Path([10, 70, 90, 70], "error", 4),
            new Shape.Object(1L, 50, 0, 50, 50));
        using var shot = Shot(screen);
        // Inside the ellipse across, not above it where only its box would be.
        Assert.Equal(Look.Color("accent"), shot.PixelAt(200 + 4 * 25, 4 * 25));
        Assert.Equal(Look.Color("accent"), shot.PixelAt(200 + 4 * 40, 4 * 25));
        Assert.Equal(Look.Color("panel"), shot.PixelAt(200 + 4 * 25, 4 * 12));
        Assert.Equal(Look.Color("error"), shot.PixelAt(200 + 4 * 50, 4 * 70));
        // The object's box shows the clear under it.
        Assert.Equal(Look.Color("panel"), shot.PixelAt(200 + 4 * 75, 4 * 25));
        Assert.Null(canvas.Failed);
    }

    [Fact]
    public void A_line_a_ring_and_words_are_drawn_in_their_colors()
    {
        var (host, screen, _) = Canvas("");
        host.Draw("pad",
            new Shape.Clear("panel"),
            new Shape.Line(10, 10, 90, 10, "accent", 3),
            new Shape.Ring(50, 50, 20, "error", 3),
            new Shape.Words(50, 85, Text.Plain("WWWW"), "text", 12));
        using var shot = Shot(screen);
        Assert.Equal(Look.Color("accent"), shot.PixelAt(200 + 4 * 50, 4 * 10));
        // The ring's line, not its middle.
        Assert.Equal(Look.Color("error"), shot.PixelAt(200 + 4 * 70, 4 * 50));
        Assert.Equal(Look.Color("panel"), shot.PixelAt(200 + 4 * 50, 4 * 50));
        // Somewhere along the words' middle is the text color.
        var row = Enumerable.Range(200 + 4 * 30, 4 * 40).Select(x => shot.PixelAt(x, 4 * 85));
        Assert.Contains(Look.Color("text"), row);
    }

    [Fact]
    public void A_canvas_clips_its_words_to_its_units_on_both_sides()
    {
        var (host, screen, _) = Canvas("");
        host.Draw("pad",
            new Shape.Clear("panel"),
            new Shape.Words(98, 50, Text.Plain("WWWW"), "text", 12),
            new Shape.Words(2, 20, Text.Plain("WWWW"), "text", 12));
        using var shot = Shot(screen);
        // The units are pixels 200 to 600 across; the screen's margins beside them stay free of the words.
        var right = Enumerable.Range(600, 200).Select(x => shot.PixelAt(x, 4 * 50));
        Assert.DoesNotContain(Look.Color("text"), right);
        var left = Enumerable.Range(0, 200).Select(x => shot.PixelAt(x, 4 * 20));
        Assert.DoesNotContain(Look.Color("text"), left);
        // Inside the units the words are drawn.
        Assert.Contains(Look.Color("text"), Enumerable.Range(500, 100).Select(x => shot.PixelAt(x, 4 * 50)));
    }

    [Fact]
    public void A_color_is_the_theme_s_by_name_or_written_out()
    {
        var (host, screen, _) = Canvas("");
        host.Draw("pad", new Shape.Rect(0, 0, 50, 50, "#101810"), new Shape.Rect(50, 0, 50, 50, "no_such_color"));
        using var shot = Shot(screen);
        Assert.Equal(new SKColor(0x10, 0x18, 0x10), shot.PixelAt(200 + 4 * 25, 4 * 25));
        Assert.Equal(Look.Color("text"), shot.PixelAt(200 + 4 * 75, 4 * 25));
    }

    [Fact]
    public void An_image_shape_draws_its_picture_stretched_to_its_box()
    {
        var host = new CanvasPartHost();
        host.Resources["dot.png"] = Png(SKColors.Red);
        var (_, screen, _) = Canvas("", host);
        host.Draw("pad", new Shape.Clear("panel"), new Shape.Image(10, 10, 30, 30, "dot.png"), new Shape.Image(60, 10, 30, 30, "missing.png"));
        using var shot = Shot(screen);
        Assert.Equal(SKColors.Red, shot.PixelAt(200 + 4 * 25, 4 * 25));
        // A picture it does not carry draws nothing.
        Assert.Equal(Look.Color("panel"), shot.PixelAt(200 + 4 * 75, 4 * 25));
    }

    /// <summary>An engine that draws one picture over the whole canvas.</summary>
    sealed class PictureEngine : IFrames
    {
        public Hears Hears => default;

        public Drawn Frame(double ms, IReadOnlyList<ScreenInput> input) =>
            new(new JsonArray { new JsonObject { ["op"] = "image", ["x"] = 0.0, ["y"] = 0.0, ["width"] = 100.0, ["height"] = 100.0, ["picture"] = "sprite.png" } }, []);

        public void Dispose() { }
    }

    [Fact]
    public void An_engine_canvas_draws_its_image_shapes()
    {
        var host = new CanvasPartHost();
        host.Resources["sprite.png"] = Png(SKColors.Lime);
        host.Engines["sky"] = () => new PictureEngine();
        var (_, screen, canvas) = Canvas("""runs="sky" """, host);
        canvas.Engine!.Background = work => work();
        using (Shot(screen)) { }
        using var shot = Shot(screen);
        Assert.Equal(SKColors.Lime, shot.PixelAt(400, 200));
    }

    /// <summary>An engine that hears keys and draws nothing.</summary>
    sealed class KeysEngine : IFrames
    {
        public Hears Hears => new(Pointer: false, Keys: true);

        public Drawn Frame(double ms, IReadOnlyList<ScreenInput> input) => new([], []);

        public void Dispose() { }
    }

    [Fact]
    public void An_engine_canvas_that_hears_keys_takes_the_focus_when_pressed()
    {
        var host = new CanvasPartHost();
        host.Engines["game"] = () => new KeysEngine();
        var (_, screen, canvas) = Canvas("""runs="game" """, host);
        canvas.Engine!.Background = work => work();
        using (Shot(screen)) { }
        screen.PointerPressed(400, 200, PointerButton.Left);
        screen.PointerReleased(400, 200, PointerButton.Left);
        Assert.Same(canvas.Engine, screen.Focused);
    }

    [Fact]
    public void On_a_canvas_that_zooms_the_clear_fills_the_whole_view()
    {
        var (host, screen, _) = Canvas("""zoom="true" """);
        host.Draw("pad", new Shape.Clear("accent"));
        using var shot = Shot(screen);
        // Beside the units too: the sheet has no edges.
        Assert.Equal(Look.Color("accent"), shot.PixelAt(50, 200));
    }

    // -- The pointer ---------------------------------------------------------------

    [Fact]
    public void A_press_moved_less_than_four_pixels_is_still_a_click_and_one_let_go_outside_is_none()
    {
        var (host, screen, _) = Canvas("""onclick="clicked" ondrag="dragged" """);
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(241, 41);
        screen.PointerReleased(241, 41, PointerButton.Left);
        var click = Assert.Single(host.Points);
        Assert.Equal(("click", 10L, 10L), (click.Event, click.At.X, click.At.Y));
        // Let go off the part, above it: nothing.
        var (other, otherScreen, _) = Canvas("""onclick="clicked" """);
        otherScreen.PointerPressed(240, 40, PointerButton.Left);
        otherScreen.PointerReleased(240, -10, PointerButton.Left);
        Assert.Empty(other.Points);
    }

    [Fact]
    public void A_drag_without_a_drag_handler_pans_a_canvas_that_zooms()
    {
        var (host, screen, canvas) = Canvas("""zoom="true" onclick="clicked" """);
        var whole = canvas.Viewport;
        screen.PointerPressed(400, 200, PointerButton.Left);
        screen.PointerMoved(380, 180);
        screen.PointerReleased(380, 180, PointerButton.Left);
        Assert.Equal(whole.X + 20 / whole.Scale, canvas.Viewport.X, 6);
        Assert.Equal(whole.Y + 20 / whole.Scale, canvas.Viewport.Y, 6);
        Assert.Empty(host.Points);
    }

    [Fact]
    public void A_double_click_on_a_bend_point_takes_it_away()
    {
        var (host, screen, _) = Canvas("""onbend="bent" """);
        host.Draw("pad", new Shape.Link("a-b", 10, 50, 90, 50, [50, 30], "text", 1));
        // The link in hand first, then a double click on its point at 50, 30.
        screen.PointerPressed(200 + 4 * 30, 4 * 40, PointerButton.Left);
        screen.PointerReleased(200 + 4 * 30, 4 * 40, PointerButton.Left);
        screen.PointerPressed(400, 120, PointerButton.Left, clicks: 2);
        screen.PointerReleased(400, 120, PointerButton.Left);
        var bent = Assert.Single(host.Points);
        Assert.Equal(("bend", "a-b"), (bent.Event, bent.At.Bent));
        Assert.Empty(bent.At.Via!);
    }

    [Fact]
    public void A_press_on_a_shape_drawn_over_a_link_drags_and_does_not_bend()
    {
        var (host, screen, _) = Canvas("""onbend="bent" ondrag="dragged" """);
        host.Draw("pad", new Shape.Link("a-b", 10, 50, 90, 50, [], "text", 1), new Shape.Rect(40, 40, 20, 20, "panel"));
        screen.PointerPressed(400, 200, PointerButton.Left);
        screen.PointerMoved(400, 120);
        screen.PointerReleased(400, 120, PointerButton.Left);
        Assert.Equal("drag", Assert.Single(host.Points).Event);
    }

    [Fact]
    public void The_hand_shows_over_a_canvas_with_a_click_and_its_buttons()
    {
        var (_, _, clicks) = Canvas("""onclick="clicked" """);
        Assert.Equal(Cursor.Hand, clicks.CursorAt(400, 200));
        var (_, _, plain) = Canvas("""zoom="true" """);
        Assert.Null(plain.CursorAt(400, 200));
        var (_, back, _) = plain.ZoomButtons();
        Assert.Equal(Cursor.Hand, plain.CursorAt(back.X + 5, back.Y + 5));
    }

    // -- Modes ---------------------------------------------------------------------

    [Fact]
    public void A_press_on_a_toolbar_button_picks_its_tool_and_tools_names_the_ones_shown()
    {
        var (host, screen, canvas) = Canvas("""mode="vector" bind="drawing" tools="select, rect" """);
        host.Values["drawing"] = "";
        Assert.Equal(["select", "rect"], canvas.ToolsShown());
        var rect = canvas.ToolButtons()[1].Box;
        screen.PointerPressed(rect.X + 4, rect.Y + 4, PointerButton.Left);
        screen.PointerReleased(rect.X + 4, rect.Y + 4, PointerButton.Left);
        Assert.Equal("rect", canvas.ToolInHand);
        Assert.Empty(host.Actions);
        // The tool code names, when the canvas does not have it, is its first.
        host.State("pad")!.Tool = "ellipse";
        Assert.Equal("select", canvas.ToolInHand);
    }

    [Fact]
    public void A_readonly_canvas_shows_its_toolbar_disabled_and_a_press_picks_nothing()
    {
        var (host, screen, canvas) = Canvas("""mode="vector" bind="drawing" readonly="true" """);
        host.Values["drawing"] = "";
        var (_, editable, editableCanvas) = Canvas("""mode="vector" bind="drawing" """, new CanvasPartHost { Values = { ["drawing"] = "" } });
        var select = canvas.ToolButtons()[0].Box;
        using var dimmed = Shot(screen);
        using var normal = Shot(editable);
        // The tool in hand shows in the accent only while the person may use it.
        Assert.Equal(Look.Color("accent"), normal.PixelAt((int)select.X + 3, (int)select.Bottom - 3));
        Assert.NotEqual(Look.Color("accent"), dimmed.PixelAt((int)select.X + 3, (int)select.Bottom - 3));
        var line = canvas.ToolButtons()[1].Box;
        screen.PointerPressed(line.X + 4, line.Y + 4, PointerButton.Left);
        Assert.Equal("select", canvas.ToolInHand);
        Assert.Null(canvas.CursorAt(line.X + 4, line.Y + 4));
        Assert.Equal(Cursor.Hand, editableCanvas.CursorAt(line.X + 4, line.Y + 4));
    }

    [Fact]
    public void A_press_beside_every_object_drags_nothing()
    {
        var (host, screen, _) = Canvas("""mode="objects" """);
        host.Draw("pad", new Shape.Object(7L, 10, 10, 20, 20), new Shape.Rect(10, 10, 20, 20, "accent"));
        screen.PointerPressed(200 + 4 * 60, 4 * 60, PointerButton.Left);
        screen.PointerMoved(200 + 4 * 70, 4 * 70);
        screen.PointerReleased(200 + 4 * 70, 4 * 70, PointerButton.Left);
        Assert.Empty(host.Actions);
    }

    [Fact]
    public void A_drag_goes_on_while_the_drawing_is_drawn_anew()
    {
        var (host, screen, canvas) = Canvas("""mode="objects" """);
        host.Draw("pad", new Shape.Object("a", 10, 10, 20, 20), new Shape.Rect(10, 10, 20, 20, "accent"));
        screen.PointerPressed(260, 60, PointerButton.Left);
        screen.PointerMoved(280, 60);
        // A timer's handler draws anew, another object first: the drag stays on "a".
        host.Draw("pad", new Shape.Object("b", 60, 60, 20, 20), new Shape.Object("a", 10, 10, 20, 20), new Shape.Rect(10, 10, 20, 20, "accent"));
        screen.PointerMoved(300, 80);
        Assert.Equal(20, ((Shape.Rect)canvas.Shown().ToList()[2]).X);
        screen.PointerReleased(300, 80, PointerButton.Left);
        var action = Assert.Single(host.Actions).Action;
        Assert.True(JsonNode.DeepEquals(JsonNode.Parse("""{"move":{"id":"a","x":20,"y":15}}"""), action), action.ToJsonString());
    }

    [Fact]
    public void A_press_on_no_shape_with_select_chooses_none()
    {
        var (host, screen, _) = Canvas("""mode="vector" bind="drawing" """);
        host.Values["drawing"] = CanvasModes.Vector("", new CanvasArgs("vector"), JsonNode.Parse("""{"add":{"op":"rect","x":10,"y":10,"width":10,"height":10,"color":"accent"}}""")!.AsObject())!.Text;
        host.Change();
        host.State("pad")!.Tool = "select";
        host.State("pad")!.SelectedItem = 0L;
        screen.PointerPressed(200 + 4 * 80, 4 * 80, PointerButton.Left);
        screen.PointerReleased(200 + 4 * 80, 4 * 80, PointerButton.Left);
        Assert.Null(host.State("pad")!.SelectedItem);
        Assert.Empty(host.Actions);
    }

    // -- The text tool ---------------------------------------------------------------

    static (CanvasPartHost Host, Screen Screen, CanvasPart Canvas) Typing()
    {
        var (host, screen, canvas) = Canvas("""mode="vector" bind="drawing" """);
        host.Values["drawing"] = "";
        host.State("pad")!.Tool = "text";
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerReleased(240, 40, PointerButton.Left);
        return (host, screen, canvas);
    }

    [Fact]
    public void The_text_tool_types_in_a_text_input_whose_caret_moves()
    {
        var (host, screen, canvas) = Typing();
        Assert.NotNull(canvas.TypingEditor);
        Assert.Same(canvas.TypingEditor, screen.Focused);
        screen.TextTyped("Hlo");
        // Left twice and type between: a caret that moves, as in every text box.
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        screen.TextTyped("el");
        Assert.Equal("Hello", canvas.Typing);
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Null(canvas.TypingEditor);
        var action = Assert.Single(host.Actions).Action;
        Assert.Equal("Hello", (string?)action["add"]!["text"]);
        // The canvas has the keyboard again, for Delete.
        Assert.Same(canvas, screen.Focused);
    }

    [Fact]
    public void Escape_drops_the_text_being_typed()
    {
        var (host, screen, canvas) = Typing();
        screen.TextTyped("Gone");
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Null(canvas.Typing);
        Assert.Empty(host.Actions);
    }
}
