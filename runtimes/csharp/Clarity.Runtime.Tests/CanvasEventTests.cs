// An action on a canvas with a mode (DESIGN 5.31), in the window: the
// runtime moves the window's own record, keeps the stroke's picture or the
// drawing's SVG, chooses what it touched, runs `on change`, and the whole
// is one edit Ctrl+Z takes back. A refused action runs nothing. And a canvas
// drawn by code: drawn anew when its window opens and after every handler,
// with the part state code reads and sets (grid, snap, dots, tool, brush,
// ink, where a click, a drag and a bend were). Ported from Avalonia's
// CanvasEventTests and DrawingTests.

using System.Text.Json.Nodes;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed partial class BoardBox : ObservableObject
{
    public long Id { get; set; }
    public long X { get => x; set => SetProperty(ref x, value); }
    long x;
    public long Y { get; set; }
}

public sealed partial class BoardNote : ObservableObject
{
    public long Id { get; set; }
    public Picture? Picture { get => picture; set => SetProperty(ref picture, value); }
    Picture? picture;
    public string Drawing { get; set; } = "";
}

sealed class BoardViewModel() : WindowViewModel(null)
{
    public List<BoardBox> Boxes { get; set; } = [new() { Id = 1 }, new() { Id = 2, X = 40, Y = 40 }];
    public BoardNote Note { get; set; } = new() { Id = 1 };
    public PartState Board { get; } = new("board");
    public PartState Sketch { get; } = new("sketch");
    public PartState Shapes { get; } = new("shapes");
    public int Changes { get; private set; }

    // As a generated canvas with a mode runs an action on it.
    public bool Moved(JsonObject action) =>
        CanvasEvent(Board, "board", new CanvasArgs("objects") { List = "Boxes", At = ["X", "Y"], Grid = Board.Grid, Snap = Board.Snap }, action, () => Changes++);

    public bool Painted(JsonObject action) => CanvasEvent(Sketch, "sketch", new CanvasArgs("paint") { Path = "Note.Picture" }, action, () => Changes++);

    public bool Drawn(JsonObject action) => CanvasEvent(Shapes, null, new CanvasArgs("vector") { Path = "Note.Drawing", Units = [400, 300] }, action, null);
}

/// <summary>A window with a canvas its draw handler draws: a dot per point of speed.</summary>
sealed class GaugeViewModel() : WindowViewModel(null)
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

    /// <summary>What a generated window gives a canvas with `on click`.</summary>
    public string Clicked { get; private set; } = "";

    public void DialClicked(Pointer at) => Handle(() =>
    {
        Dial.Pointer = at;
        Clicked = $"{Dial.X},{Dial.Y}";
    });
}

public class CanvasEventTests
{
    static JsonObject Action(string json) => JsonNode.Parse(json)!.AsObject();

    [Fact]
    public void A_dropped_object_moves_its_record_snapped_and_is_one_edit()
    {
        var window = new BoardViewModel();
        window.Board.Grid = 20;
        window.Board.Snap = true;
        window.Opened();
        Assert.True(window.Moved(Action("""{"move": {"id": 2, "x": 67, "y": 31}}""")));
        Assert.Equal((60L, 40L), (window.Boxes[1].X, window.Boxes[1].Y));
        Assert.Same(window.Boxes[1], window.Board.SelectedItem);
        Assert.Equal(1, window.Changes);
        // Put where it is: refused, nothing runs.
        Assert.False(window.Moved(Action("""{"move": {"id": 2, "x": 65, "y": 38}}""")));
        Assert.False(window.Moved(Action("""{"move": {"id": 9, "x": 0, "y": 0}}""")));
        Assert.Equal(1, window.Changes);
        window.UndoEdit("board", back: true);
        Assert.Equal((40L, 40L), (window.Boxes[1].X, window.Boxes[1].Y));
        window.UndoEdit("board", back: false);
        Assert.Equal(60L, window.Boxes[1].X);
    }

    [Fact]
    public void A_readonly_canvas_refuses_every_action()
    {
        var window = new BoardViewModel();
        window.Board.IsReadOnly = true;
        window.Opened();
        Assert.False(window.Moved(Action("""{"move": {"id": 1, "x": 10, "y": 10}}""")));
        Assert.Equal((0L, 0), (window.Boxes[0].X, window.Changes));
    }

    [Fact]
    public void A_stroke_is_the_picture_after_it_and_taken_back_as_one_edit()
    {
        var window = new BoardViewModel();
        window.Opened();
        byte[] png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        var stroke = new JsonObject { ["stroke"] = new JsonObject { ["data"] = Convert.ToBase64String(png) } };
        Assert.True(window.Painted(stroke));
        Assert.Equal(Picture.New(png), window.Note.Picture);
        Assert.Null(window.Sketch.SelectedItem);
        // The same picture again changes nothing.
        Assert.False(window.Painted(stroke));
        window.UndoEdit("sketch", back: true);
        Assert.Null(window.Note.Picture);
    }

    [Fact]
    public void A_vector_canvas_adds_moves_and_deletes_shapes_and_says_which()
    {
        var window = new BoardViewModel();
        window.Opened();
        Assert.True(window.Drawn(Action("""{"add": {"op": "rect", "x": 10, "y": 20, "width": 30, "height": 40, "color": "#ff0000"}}""")));
        Assert.Equal("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 300\">\n<rect x=\"10\" y=\"20\" width=\"30\" height=\"40\" fill=\"#ff0000\"/>\n</svg>\n", window.Note.Drawing);
        // A place from 0, as a table's chosen row; code reads it from 1.
        Assert.Equal((0L, 1L), (window.Shapes.SelectedItem, window.Shapes.SelectedShape));
        Assert.True(window.Drawn(Action("""{"move": {"index": 1, "x": 50, "y": 60}}""")));
        Assert.Contains("<rect x=\"50\" y=\"60\"", window.Note.Drawing);
        Assert.True(window.Drawn(Action("""{"delete": 1}""")));
        Assert.Equal((null, 0L), (window.Shapes.SelectedItem, window.Shapes.SelectedShape));
        Assert.False(window.Drawn(Action("""{"delete": 1}""")));
    }

    [Fact]
    public void Tool_brush_and_ink_are_part_state_code_sets()
    {
        var part = new PartState("sketch") { Tool = "pen", Brush = 4, Ink = "accent" };
        var changed = new List<string?>();
        part.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        part.Tool = "eraser";
        part.Brush = 6;
        part.Ink = "#ff0000";
        Assert.Equal(["Tool", "Brush", "Ink"], changed);
        Assert.Equal(("text", 2L), (new PartState("x").Ink, new PartState("x").Brush));
    }

    [Fact]
    public void A_canvas_is_drawn_anew_when_its_window_opens_and_after_every_handler()
    {
        var window = new GaugeViewModel();
        window.Opened();
        Assert.Equal(3, window.Dial.Drawing.Count);
        var changes = 0;
        window.Dial.PropertyChanged += (_, e) => changes += e.PropertyName == nameof(PartState.Drawing) ? 1 : 0;
        window.Faster();
        Assert.Equal(4, window.Dial.Drawing.Count);
        // One change per draw, however many shapes it has.
        Assert.Equal((2, 1), (window.Draws, changes));
    }

    [Fact]
    public void A_click_on_a_canvas_runs_its_handler_with_where_and_it_is_drawn_anew()
    {
        var window = new GaugeViewModel();
        window.Opened();
        Assert.Equal((0L, 0L), (window.Dial.X, window.Dial.Y));
        window.DialClicked(new Pointer(120, 150));
        Assert.Equal(("120,150", 2), (window.Clicked, window.Draws));
    }

    [Fact]
    public void A_drag_on_a_canvas_says_where_it_started()
    {
        var window = new GaugeViewModel();
        window.Opened();
        window.DialClicked(new Pointer(900, 410) { FromX = 860, FromY = 400 });
        Assert.Equal((900L, 410L, 860L, 400L), (window.Dial.X, window.Dial.Y, window.Dial.FromX, window.Dial.FromY));
        // A click has no start.
        window.DialClicked(new Pointer(7, 3));
        Assert.Equal((0L, 0L), (window.Dial.FromX, window.Dial.FromY));
    }

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
        // A spin box's range: as its line writes it (the window sets it), then as code sets it, on the wire once set.
        var level = new PartState("level") { SpinMin = 1, SpinMax = 99 };
        level.SpinMax = 3;
        Assert.Equal((1L, 3L), (level.SpinMin, level.SpinMax));
        Assert.Equal(3L, (long?)WireJson.Part(level, null)["max"]);
        Assert.False(WireJson.Part(new PartState("free"), null).ContainsKey("max"));
        // A line tool: false until code sets canvas.connecting.
        Assert.False(sheet.Connecting);
        sheet.Connecting = true;
        Assert.True(sheet.Connecting);
        // Boxes people resize: true until code sets canvas.resizable, and only false goes to the screen.
        Assert.True(sheet.Resizable);
        Assert.False(WireJson.Part(sheet, null).ContainsKey("resizable"));
        sheet.Resizable = false;
        Assert.False(sheet.Resizable);
        Assert.Equal(false, (bool?)WireJson.Part(sheet, null)["resizable"]);
    }

    [Fact]
    public void A_bend_says_which_link_and_its_points()
    {
        var sheet = new PartState("sheet");
        Assert.Equal(("", 0), (sheet.Bent, sheet.Via.Count));
        sheet.Pointer = new Pointer(0, 0) { Bent = "a-b", Via = [500, 200] };
        Assert.Equal("a-b", sheet.Bent);
        Assert.Equal([500L, 200L], sheet.Via);
    }

    [Fact]
    public void A_foldable_part_is_open_while_it_is_not_folded()
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

    [Fact]
    public void A_links_ends_label_and_style_and_a_texts_style_go_to_the_screen()
    {
        var board = new PartState("board");
        board.BeginDrawing();
        board.Draw(new Shape.Link("r1", 10, 20, 300, 40, [], "muted", 2, "one", "zeroOrMany", Text.Plain("orders"), "dashed"));
        board.Draw(new Shape.Words(10, 20, Text.Plain("Id"), "accent", 11, "left", "bold mono"));
        board.Draw(new Shape.Link("r2", 10, 20, 300, 40, [], "muted"));
        board.EndDrawing();
        var drawing = (JsonArray)WireJson.Part(board, null)["drawing"]!;
        var link = (JsonObject)drawing[0]!;
        Assert.Equal(("link", "one", "zeroOrMany", "dashed"), (link["op"]!.GetValue<string>(), link["start"]!.GetValue<string>(), link["end"]!.GetValue<string>(), link["style"]!.GetValue<string>()));
        Assert.Equal("orders", WireJson.ReadText(link["label"]).Template);
        Assert.Equal("bold mono", drawing[1]!["style"]!.GetValue<string>());
        // Left out, a link's ends, label and style are empty.
        Assert.Equal(("", ""), (drawing[2]!["start"]!.GetValue<string>(), drawing[2]!["style"]!.GetValue<string>()));
        // A shape read back keeps them.
        var read = Assert.IsType<Shape.Link>(Shape.Of(link));
        Assert.Equal(("one", "zeroOrMany", "dashed"), (read.Start, read.End, read.Style));
    }
}
