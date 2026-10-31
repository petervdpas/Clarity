// An action on a canvas with a mode (DESIGN 5.31), in the window: the
// runtime moves the window's own record, keeps the stroke's picture or the
// drawing's SVG, chooses what it touched, runs `on change`, and the whole
// is one edit Ctrl+Z takes back. A refused action runs nothing.

using System.Text.Json.Nodes;
using Clarity.Avalonia;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;
using Avalonia.Headless.XUnit;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed partial class Box : ObservableObject
{
    public long Id { get; set; }
    public long X { get => x; set => SetProperty(ref x, value); }
    long x;
    public long Y { get; set; }
}

public sealed partial class Note : ObservableObject
{
    public long Id { get; set; }
    public Picture? Picture { get => picture; set => SetProperty(ref picture, value); }
    Picture? picture;
    public string Drawing { get; set; } = "";
}

sealed class BoardViewModel() : WindowViewModel(null!)
{
    public List<Box> Boxes { get; set; } = [new() { Id = 1 }, new() { Id = 2, X = 40, Y = 40 }];
    public Note Note { get; set; } = new() { Id = 1 };
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

public class CanvasEventTests
{
    static JsonObject Action(string json) => JsonNode.Parse(json)!.AsObject();

    [AvaloniaFact]
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

    [AvaloniaFact]
    public void A_readonly_canvas_refuses_every_action()
    {
        var window = new BoardViewModel();
        window.Board.IsReadOnly = true;
        window.Opened();
        Assert.False(window.Moved(Action("""{"move": {"id": 1, "x": 10, "y": 10}}""")));
        Assert.Equal((0L, 0), (window.Boxes[0].X, window.Changes));
    }

    [AvaloniaFact]
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

    [AvaloniaFact]
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

    [AvaloniaFact]
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
}
