// Rows of the data area the check found untested: the chart's axis lines,
// a canvas, chart and display whose drawing fails saying why in place, the
// bend handles of the link in hand, a vector canvas's dashed frame round the
// chosen shape and the preview of the one being drawn, a theme colour of
// two words, a picture field leaving an older fetch's answer, a relative
// link address the hosts refuse, and a gesture whose pointer was taken
// away (Avalonia's lost capture) dropped instead of committed. A canvas of
// 100 by 100 units fills 400 by 400 pixels of an 800 by 400 screen, from 200
// across: unit x, y is pixel 200 + 4x, 4y.

using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Clarity.Ui.Browser;
using SkiaSharp;

namespace Clarity.Ui.Tests;

public class CheckFixDataTests
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

    /// <summary>How far apart two colours are, channel by channel.</summary>
    static int Distance(SKColor a, SKColor b) => Math.Abs(a.Red - b.Red) + Math.Abs(a.Green - b.Green) + Math.Abs(a.Blue - b.Blue);

    /// <summary>A pixel drawn in <paramref name="color"/>, at least partly: nearer to it than to <paramref name="behind"/>.</summary>
    static bool Toward(SKColor pixel, SKColor color, SKColor behind) => Distance(pixel, color) < Distance(pixel, behind);

    /// <summary>A pixel mostly of <paramref name="color"/>: a thin line's edge blends a little of what is under it.</summary>
    static bool Near(SKColor pixel, SKColor color) => Distance(pixel, color) < 120;

    // -- The chart's axis -------------------------------------------------------------

    public sealed class Order
    {
        public string Status { get; set; } = "";
        public decimal Total { get; set; }
    }

    /// <summary>ChartView.cs drew its axis lines muted, the zero line 1 pixel, the others 0.4.</summary>
    [Fact]
    public void The_axis_lines_are_muted_the_zero_line_whole_and_the_others_thin()
    {
        var host = new TestHost();
        host.Lists["orders"] = [new Order { Status = "open", Total = 2.5m }, new Order { Status = "sent", Total = 6 }];
        var (screen, _, parts) = Made.Window("""<Chart name="totals" at="1,1" span="6" rows="6" bind="orders" by="status" measure="{&quot;kind&quot;:&quot;sum&quot;,&quot;field&quot;:&quot;total&quot;}" chart="bar" />""", host);
        var chart = Assert.IsType<ChartPart>(parts[0]);
        using var shot = Clarity.Ui.Tests.Shot.Of(screen, 800, 400);
        // A column beside the first bar, inside the plot: only the axis lines cross it.
        var x = (int)chart.Marks(chart.Bounds)[0].Box.X - 3;
        var behind = shot.PixelAt(x, (int)chart.Bounds.Y + 2);
        var muted = screen.Look.Color("muted");
        var rows = Enumerable.Range((int)chart.Bounds.Y, (int)chart.Bounds.Height).Select(y => shot.PixelAt(x, y)).ToList();
        // The zero line: one row, the whole muted colour.
        Assert.Equal(1, rows.Count(p => p == muted));
        // The others: thin, so lighter than muted yet drawn.
        var thin = rows.Where(p => p != muted && p != behind).ToList();
        Assert.True(thin.Count >= 2, $"{thin.Count} thin lines");
        Assert.All(thin, p => Assert.True(Distance(p, behind) < Distance(muted, behind), $"{p} is not thinner than {muted}"));
    }

    // -- Fail safe, per part ----------------------------------------------------------------

    /// <summary>A host whose resources, rows and values fail once it is told to, as a broken view model would.</summary>
    sealed class FailingHost : CanvasPartHost
    {
        public bool Failing;

        public override byte[]? Resource(string name) => Failing ? throw new InvalidOperationException("the picture store is gone") : base.Resource(name);

        public override IReadOnlyList<object> Rows(string path) => Failing ? throw new InvalidOperationException("the rows are gone") : base.Rows(path);

        public override object? Read(string path) => Failing ? throw new InvalidOperationException("the value is gone") : base.Read(path);
    }

    [Fact]
    public void A_canvas_whose_drawing_fails_says_why_in_its_place()
    {
        var host = new FailingHost();
        var (_, screen, canvas) = Canvas("", host);
        host.Draw("pad", new Shape.Clear("panel"), new Shape.Image(10, 10, 30, 30, "sprite.png"));
        host.Failing = true;
        using var shot = Shot(screen);
        Assert.Equal("the picture store is gone", canvas.Failed);
        // Its red outline, where it is.
        Assert.Contains(Enumerable.Range(0, 4).Select(y => shot.PixelAt(400, y)), p => Toward(p, Look.Color("error"), Look.Color("background")));
    }

    [Fact]
    public void A_chart_and_a_display_whose_drawing_fails_say_why_and_the_window_draws_on()
    {
        var host = new FailingHost();
        host.Lists["orders"] = [new Order { Status = "open", Total = 1 }];
        host.Values["total"] = 12L;
        var (screen, _, parts) = Made.Window("""
            <Chart name="totals" at="1,1" span="4" rows="4" bind="orders" by="status" chart="bar" />
            <Display name="shown" at="5,1" span="4" rows="2" bind="total" digits="4" />
            <Label name="beside" at="9,1" span="4" caption="Still here" />
            """, host);
        host.Failing = true;
        using var shot = Clarity.Ui.Tests.Shot.Of(screen, 800, 400);
        Assert.Equal("the rows are gone", parts[0].Failed);
        Assert.Equal("the value is gone", parts[1].Failed);
        // The label beside them still draws its words.
        var label = parts[2].Bounds;
        Assert.Contains(
            Enumerable.Range((int)label.X, (int)label.Width).SelectMany(x => Enumerable.Range((int)label.Y, (int)label.Height).Select(y => shot.PixelAt(x, y))),
            p => Toward(p, screen.Look.Color("text"), screen.Look.Color("background")));
    }

    // -- Bend handles ------------------------------------------------------------------

    [Fact]
    public void The_bend_points_of_the_link_in_hand_are_drawn_as_handles()
    {
        var (host, screen, _) = Canvas("""onbend="bent" """);
        host.Draw("pad", new Shape.Clear("panel"), new Shape.Link("a-b", 10, 50, 90, 50, [50, 30], "text", 0.5));
        // The bend point at 50, 30 is pixel 400, 120.
        using (var before = Shot(screen))
        {
            Assert.True(Toward(before.PixelAt(400, 120), Look.Color("text"), Look.Color("panel")));
            Assert.DoesNotContain(Enumerable.Range(402, 6).Select(x => before.PixelAt(x, 120)), p => Near(p, Look.Color("accent")));
        }
        // The link in hand: a press on it.
        screen.PointerPressed(200 + 4 * 30, 4 * 40, PointerButton.Left);
        screen.PointerReleased(200 + 4 * 30, 4 * 40, PointerButton.Left);
        Assert.Empty(host.Points);
        using var after = Shot(screen);
        // A handle: the panel's face over the line, ringed in the accent.
        Assert.Equal(Look.Color("panel"), after.PixelAt(400, 120));
        Assert.Contains(Enumerable.Range(402, 6).Select(x => after.PixelAt(x, 120)), p => Near(p, Look.Color("accent")));
    }

    // -- A vector canvas ---------------------------------------------------------------------

    static (CanvasPartHost Host, Screen Screen, CanvasPart Canvas) Vector(string shapes = "")
    {
        var (host, screen, canvas) = Canvas("""mode="vector" bind="drawing" """);
        host.Values["drawing"] = shapes;
        host.Change();
        return (host, screen, canvas);
    }

    static string Drawn(params string[] adds)
    {
        var drawing = "";
        foreach (var add in adds)
            drawing = CanvasModes.Vector(drawing, new CanvasArgs("vector"), JsonNode.Parse($$"""{"add":{{add}}}""")!.AsObject())!.Text;
        return drawing;
    }

    [Fact]
    public void The_chosen_shape_shows_in_a_dashed_accent_frame()
    {
        var (host, screen, _) = Vector(Drawn("""{"op":"rect","x":40,"y":60,"width":20,"height":20,"color":"text"}"""));
        SKColor behind;
        using (var none = Shot(screen))
        {
            behind = none.PixelAt(200 + 4 * 90, 4 * 95);
            Assert.Equal(behind, none.PixelAt(400, 4 * 60 - 3));
        }
        host.State("pad")!.SelectedItem = 0L;
        using var chosen = Shot(screen);
        // The frame runs 3 pixels outside the shape, along its top edge, in dashes.
        var top = Enumerable.Range(4 * 40 + 200 - 3, 4 * 20 + 6).Select(x => chosen.PixelAt(x, 4 * 60 - 3)).ToList();
        Assert.Contains(top, p => Near(p, Look.Color("accent")));
        Assert.Contains(top, p => p == behind);
    }

    [Fact]
    public void The_shape_being_drawn_shows_before_it_is_added()
    {
        var (host, screen, _) = Vector();
        host.State("pad")!.Tool = "rect";
        screen.PointerPressed(200 + 4 * 30, 4 * 50, PointerButton.Left);
        screen.PointerMoved(200 + 4 * 60, 4 * 80);
        using (var drawing = Shot(screen))
            Assert.Equal(Look.Color("text"), drawing.PixelAt(200 + 4 * 45, 4 * 65));
        Assert.Empty(host.Actions);
        screen.PointerReleased(200 + 4 * 60, 4 * 80, PointerButton.Left);
        Assert.Single(host.Actions);
    }

    // -- Colours -------------------------------------------------------------------------------

    /// <summary>DrawingTests.AColorIsTheThemesByNameOrWrittenOut: lcd_text, a role of two words, is the theme's.</summary>
    [Fact]
    public void A_colour_of_two_words_is_the_theme_s()
    {
        var (host, screen, _) = Canvas("");
        screen.Show(ThemeLook.Over("""{"colors":{"lcd_text":"#00ff00"}}""", Look));
        host.Draw("pad", new Shape.Rect(0, 0, 50, 50, "lcd_text"), new Shape.Rect(50, 0, 50, 50, "on_accent"));
        using var shot = Shot(screen);
        Assert.Equal(new SKColor(0, 0xff, 0), shot.PixelAt(200 + 4 * 25, 4 * 25));
        Assert.Equal(Look.Color("on_accent"), shot.PixelAt(200 + 4 * 75, 4 * 25));
    }

    // -- A picture field's fetch -----------------------------------------------------------------

    static byte[] Png(int size)
    {
        using var bitmap = new SKBitmap(new SKImageInfo(size, size, SKColorType.Rgba8888, SKAlphaType.Premul));
        bitmap.Erase(SKColors.Red);
        using var image = SKImage.FromBitmap(bitmap);
        using var png = image.Encode(SKEncodedImageFormat.Png, 100);
        return png.ToArray();
    }

    [Fact]
    public void A_picture_field_leaves_the_answer_to_an_older_fetch()
    {
        var host = new TestHost();
        host.Values["person"] = "first";
        host.Values["person.photo"] = Picture.Stored;
        var asked = new Dictionary<string, TaskCompletionSource<byte[]?>>();
        host.Fetcher = (_, _) =>
        {
            var answer = new TaskCompletionSource<byte[]?>();
            asked[(string)host.Values["person"]!] = answer;
            return answer.Task;
        };
        var (_, _, parts) = VariantMade.Window("""<Image at="1,1" span="6" rows="4" name="photo" bind="person.photo" />""", host);
        var part = Assert.IsType<PictureFieldPart>(parts[0]);
        host.Values["person"] = "second";
        host.Change();
        // The second record's picture comes first, then the first record's, late.
        asked["second"].SetResult(Png(3));
        asked["first"].SetResult(Png(5));
        Assert.Equal(3, part.Shown!.Width);
    }

    // -- A link's address ----------------------------------------------------------------------------

    /// <summary>
    /// Avalonia's Markdown opened only an absolute address. Here a relative
    /// one is no link at all (MarkdownBlocks takes http, https and mailto),
    /// and every host refuses one besides, should a link part or a handler
    /// give it.
    /// </summary>
    [Fact]
    public void A_relative_link_address_is_opened_by_no_host()
    {
        var host = new TestHost();
        host.Values["notes"] = "Read [the guide](docs/guide.md) or [the site](https://example.org/guide).";
        var (screen, _, parts) = Made.Window("""<Markdown name="rules" at="1,1" span="6" rows="4" bind="notes" />""", host);
        using (Clarity.Ui.Tests.Shot.Of(screen, 800, 400)) { }
        var part = Assert.IsType<MarkdownPart>(parts[0]);
        Assert.Equal(["https://example.org/guide"], part.Page.Runs.Where(r => r.Link is not null).Select(r => r.Link).Distinct());
        foreach (var os in new[] { OSPlatform.Linux, OSPlatform.Windows, OSPlatform.OSX })
            foreach (var relative in new[] { "docs/guide.md", "/about", "../up" })
                Assert.Null(SystemAddress.CommandFor(os, relative));
        foreach (var relative in new[] { "docs/guide.md", "/about", "//example.org/page" })
            Assert.Equal(BrowserAddress.Way.None, BrowserAddress.How(relative));
        Assert.NotNull(SystemAddress.CommandFor(OSPlatform.Linux, "https://example.org/guide"));
        Assert.Equal(BrowserAddress.Way.Tab, BrowserAddress.How("https://example.org/guide"));
    }

    // -- The pointer taken away ---------------------------------------------------------------------

    [Fact]
    public void A_drag_whose_pointer_is_taken_away_is_dropped_not_committed()
    {
        var (host, screen, _) = Canvas("""ondrag="dragged" ondrop="dropped" """);
        screen.PointerPressed(240, 40, PointerButton.Left);
        screen.PointerMoved(280, 80);
        Assert.Equal(["drag"], host.Points.Select(p => p.Event));
        screen.PointerCancelled();
        // The release that may still come is no drop either.
        screen.PointerReleased(280, 80, PointerButton.Left);
        Assert.Equal(["drag"], host.Points.Select(p => p.Event));
    }

    [Fact]
    public void An_object_dragged_when_the_pointer_is_taken_away_stays_where_it_was()
    {
        var (host, screen, canvas) = Canvas("""mode="objects" """);
        host.Draw("pad", new Shape.Object("a", 10, 10, 20, 20), new Shape.Rect(10, 10, 20, 20, "accent"));
        screen.PointerPressed(260, 60, PointerButton.Left);
        screen.PointerMoved(300, 80);
        Assert.Equal(20, ((Shape.Rect)canvas.Shown().ToList()[1]).X);
        screen.PointerCancelled();
        Assert.Equal(10, ((Shape.Rect)canvas.Shown().ToList()[1]).X);
        screen.PointerReleased(300, 80, PointerButton.Left);
        Assert.Empty(host.Actions);
    }

    [Fact]
    public void A_shape_being_drawn_or_a_bend_is_dropped_when_the_pointer_is_taken_away()
    {
        var (host, screen, _) = Vector();
        host.State("pad")!.Tool = "rect";
        screen.PointerPressed(200 + 4 * 30, 4 * 50, PointerButton.Left);
        screen.PointerMoved(200 + 4 * 60, 4 * 80);
        screen.PointerCancelled();
        using (var shot = Shot(screen))
            Assert.Equal(shot.PixelAt(200 + 4 * 90, 4 * 95), shot.PixelAt(200 + 4 * 45, 4 * 65));
        Assert.Empty(host.Actions);

        var (bends, bendScreen, bendCanvas) = Canvas("""onbend="bent" """);
        bends.Draw("pad", new Shape.Link("a-b", 10, 50, 90, 50, [], "text", 1));
        bendScreen.PointerPressed(200 + 4 * 50, 4 * 50, PointerButton.Left);
        bendScreen.PointerMoved(200 + 4 * 50, 4 * 20);
        Assert.NotEmpty(bendCanvas.Shown().OfType<Shape.Link>().Single().Via);
        bendScreen.PointerCancelled();
        Assert.Empty(bendCanvas.Shown().OfType<Shape.Link>().Single().Via);
        Assert.Empty(bends.Points);
    }

    [Fact]
    public void A_touch_the_browser_takes_over_drops_the_drag_it_was_making()
    {
        var (host, screen, _) = Canvas("""ondrag="dragged" ondrop="dropped" """);
        var input = new PageInput(screen, _ => { });
        input.TouchDown(240, 40, 0);
        input.TouchMoved(280, 80, PageInput.Hold);
        Assert.Equal(["drag"], host.Points.Select(p => p.Event));
        input.TouchCancelled();
        Assert.Equal(["drag"], host.Points.Select(p => p.Event));
    }

    [Fact]
    public void A_row_dragged_when_the_pointer_is_taken_away_stays_where_it_was()
    {
        var host = new DataHost();
        host.Lists["folders"] = [.. Enumerable.Range(1, 3).Select(i => (object)new Folder { Id = i, Name = $"Folder {i}", Position = i })];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name" reorder="position" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        var before = host.Lists["folders"].ToList();
        var from = table.CellBox(0, "name")!.Value;
        var onto = table.RowBox(2)!.Value;
        screen.PointerPressed(from.X + 20, from.Y + from.Height / 2, PointerButton.Left);
        screen.PointerMoved(from.X + 20, from.Y + from.Height / 2 + 10);
        screen.PointerMoved(onto.X + 20, onto.Bottom - 3);
        Assert.NotNull(table.Drop);
        screen.PointerCancelled();
        Assert.Null(table.Drop);
        screen.PointerReleased(onto.X + 20, onto.Bottom - 3, PointerButton.Left);
        Assert.Equal(before, host.Lists["folders"]);
    }
}
