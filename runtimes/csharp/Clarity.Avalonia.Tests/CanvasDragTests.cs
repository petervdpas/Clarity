// A canvas the person zooms and drags on, driven by the mouse as a person
// would, moving as Clarity's own diagram designer does
// (conformance/drawing.json, views): a drag runs its command with where it
// was pressed, the drop when it is let go, a press that does not move is a
// click, the wheel pans and zooms, and the corner buttons zoom.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Headless.XUnit;
using Avalonia.Input;
using Avalonia.Threading;
using Clarity.Avalonia;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class CanvasDragTests
{
    /// A canvas of 1000 x 500 units in a window of 400 x 200 pixels: 0.4 pixels a unit.
    static (Window, CanvasView) Shown(CanvasView canvas)
    {
        canvas.UnitsWidth = 1000;
        canvas.UnitsHeight = 500;
        var window = new Window { Width = 400, Height = 200, Content = canvas };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        return (window, canvas);
    }

    static void Click(Window window, Point at)
    {
        window.MouseDown(at, MouseButton.Left);
        window.MouseUp(at, MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
    }

    [AvaloniaFact]
    public void ADragRunsItsCommandWithWhereItStartedAndTheDropAfter()
    {
        var drags = new List<Pointer>();
        var drops = new List<Pointer>();
        var clicks = new List<Pointer>();
        var (window, _) = Shown(new CanvasView
        {
            DragCommand = new RelayCommand<Pointer>(p => drags.Add(p!)),
            DropCommand = new RelayCommand<Pointer>(p => drops.Add(p!)),
            Command = new RelayCommand<Pointer>(p => clicks.Add(p!)),
        });
        window.MouseDown(new Point(40, 20), MouseButton.Left);
        window.MouseMove(new Point(60, 20));
        window.MouseMove(new Point(100, 40));
        window.MouseUp(new Point(100, 40), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal([new Pointer(150, 50) { FromX = 100, FromY = 50 }, new Pointer(250, 100) { FromX = 100, FromY = 50 }], drags);
        Assert.Equal([new Pointer(250, 100) { FromX = 100, FromY = 50 }], drops);
        Assert.Empty(clicks);
        // A press that does not move is a click, on the unit under it.
        window.MouseDown(new Point(40, 20), MouseButton.Left);
        window.MouseMove(new Point(41, 21));
        window.MouseUp(new Point(41, 21), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal([new Pointer(102, 52)], clicks);
        window.Close();
    }

    /// On a canvas that zooms, a drag the handler does nothing with (the drawing
    /// stays the same) pans, and sends no drop.
    [AvaloniaFact]
    public void ADragTheHandlerDoesNothingWithPans()
    {
        var drags = 0;
        var drops = new List<Pointer>();
        CanvasView canvas = null!;
        // The same drawing each time, with a text that has a slot, as a redraw makes it.
        IReadOnlyList<Shape> Drawn() => [new Shape.Words(10, 10, new Text("{n} km/h").With("n", Slot.OfInteger(5, null)))];
        canvas = new CanvasView
        {
            Zoom = true,
            Drawing = Drawn(),
            DragCommand = new RelayCommand<Pointer>(_ => { drags++; canvas.Drawing = Drawn(); }),
            DropCommand = new RelayCommand<Pointer>(p => drops.Add(p!)),
        };
        var (window, _) = Shown(canvas);
        var start = canvas.Viewport.X;
        window.MouseDown(new Point(200, 100), MouseButton.Left);
        window.MouseMove(new Point(160, 100));
        window.MouseMove(new Point(120, 100));
        window.MouseUp(new Point(120, 100), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        // 80 pixels at 0.4 pixels a unit: the view moved 200 units to the right.
        Assert.Equal(1, drags);
        Assert.Equal(200, canvas.Viewport.X - start, 6);
        Assert.Empty(drops);
        window.Close();
    }

    [AvaloniaFact]
    public void TheWheelPansAndZoomsTheButtonsZoomAndARedrawKeepsTheView()
    {
        var (window, canvas) = Shown(new CanvasView { Zoom = true });
        // Whole: 0.4 pixels a unit, the canvas filling the window.
        Assert.Equal(new CanvasViewport(0.4, 0, 0), canvas.Viewport);
        // Control and a notch of the wheel zoom in 15% around the pointer: the unit under it, 500, 250, stays.
        window.MouseWheel(new Point(200, 100), new Vector(0, 1), RawInputModifiers.Control);
        Dispatcher.UIThread.RunJobs();
        var v = canvas.Viewport;
        Assert.Equal(0.46, v.Scale, 9);
        Assert.Equal((500.0, 250.0), (Math.Round(v.X + 200 / v.Scale, 6), Math.Round(v.Y + 100 / v.Scale, 6)));
        // The wheel alone pans: a notch down is 50 pixels.
        window.MouseWheel(new Point(200, 100), new Vector(0, -1));
        Dispatcher.UIThread.RunJobs();
        Assert.Equal(50 / 0.46, canvas.Viewport.Y - v.Y, 6);
        // A new drawing keeps the view.
        canvas.Drawing = [];
        Assert.Equal(0.46, canvas.Viewport.Scale, 9);
        // The corner buttons, as the designer's: in by 20%, then the level shows it whole again.
        Click(window, new Point(400 - 6 - 13, 18));
        Assert.Equal(0.552, canvas.Viewport.Scale, 9);
        Click(window, new Point(400 - 6 - 26 - 2 - 26, 18));
        Assert.Equal(new CanvasViewport(0.4, 0, 0), canvas.Viewport);
        // Other units start over.
        Click(window, new Point(400 - 6 - 13, 18));
        canvas.UnitsWidth = 2000;
        Assert.Equal(0.2, canvas.Viewport.Scale, 9);
        window.Close();
    }

    /// On a canvas that zooms, the drawing's clear fills the whole control, past the canvas's edges.
    [AvaloniaFact]
    public void OnACanvasThatZoomsTheClearFillsTheWholeView()
    {
        var (window, canvas) = Shown(new CanvasView { Zoom = true, Drawing = [new Shape.Clear("#ff0000")] });
        canvas.UnitsWidth = 1000;
        canvas.UnitsHeight = 1000;
        Dispatcher.UIThread.RunJobs();
        // Whole, the canvas fills the middle 200 of 400 pixels; the clear still fills the corners.
        var frame = window.CaptureRenderedFrame()!;
        var pixels = new byte[frame.PixelSize.Width * frame.PixelSize.Height * 4];
        var handle = System.Runtime.InteropServices.GCHandle.Alloc(pixels, System.Runtime.InteropServices.GCHandleType.Pinned);
        try
        {
            frame.CopyPixels(new PixelRect(frame.PixelSize), handle.AddrOfPinnedObject(), pixels.Length, frame.PixelSize.Width * 4);
        }
        finally
        {
            handle.Free();
        }
        var middle = (frame.PixelSize.Height / 2 * frame.PixelSize.Width + frame.PixelSize.Width / 2) * 4;
        // Red, green, blue: the clear's red in the middle and in the corner, past the canvas's units.
        Assert.Equal((255, 0, 0, 255, 0, 0), (pixels[middle], pixels[middle + 1], pixels[middle + 2], pixels[0], pixels[1], pixels[2]));
        window.Close();
    }

    /// A clear without a color fills the canvas with its paper, the part's background.
    [AvaloniaFact]
    public void AClearWithoutAColorFillsTheCanvasWithItsPaper()
    {
        var (window, canvas) = Shown(new CanvasView { Paper = "#ff0000", Drawing = [new Shape.Clear("")] });
        Dispatcher.UIThread.RunJobs();
        var frame = window.CaptureRenderedFrame()!;
        var pixels = new byte[frame.PixelSize.Width * frame.PixelSize.Height * 4];
        var handle = System.Runtime.InteropServices.GCHandle.Alloc(pixels, System.Runtime.InteropServices.GCHandleType.Pinned);
        try
        {
            frame.CopyPixels(new PixelRect(frame.PixelSize), handle.AddrOfPinnedObject(), pixels.Length, frame.PixelSize.Width * 4);
        }
        finally
        {
            handle.Free();
        }
        var middle = (frame.PixelSize.Height / 2 * frame.PixelSize.Width + frame.PixelSize.Width / 2) * 4;
        Assert.Equal((255, 0, 0), (pixels[middle], pixels[middle + 1], pixels[middle + 2]));
        window.Close();
    }

    /// A link bends: a press on it adds a point there, dragging moves it, and letting go
    /// runs the bend command with the link's name and its points; a double click takes a point away.
    [AvaloniaFact]
    public void ALinkBendsAndTheBendCommandHearsItsPoints()
    {
        var bends = new List<Pointer>();
        // 0.4 pixels a unit: a link from 100,100 to 900,100 runs along y 40, from x 40 to 360.
        var (window, canvas) = Shown(new CanvasView
        {
            Drawing = [new Shape.Link("a-b", 100, 100, 900, 100, [], "border", 2)],
            BendCommand = new RelayCommand<Pointer>(p => bends.Add(p!)),
        });
        window.MouseDown(new Point(200, 40), MouseButton.Left);
        window.MouseMove(new Point(200, 80));
        window.MouseUp(new Point(200, 80), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Single(bends);
        Assert.Equal("a-b", bends[0].Bent);
        Assert.Equal([500L, 200L], bends[0].Via);
        // The drawing comes back with the point; a double click on it takes it away.
        canvas.Drawing = [new Shape.Link("a-b", 100, 100, 900, 100, [500, 200], "border", 2)];
        window.MouseDown(new Point(200, 80), MouseButton.Left, RawInputModifiers.None);
        window.MouseUp(new Point(200, 80), MouseButton.Left);
        canvas.Unbend(new Point(200, 80));
        Dispatcher.UIThread.RunJobs();
        Assert.Equal("a-b", bends[^1].Bent);
        Assert.Empty(bends[^1].Via);
        window.Close();
    }

    /// A press on a shape drawn over a link is not a bend: the box above the line takes it.
    [AvaloniaFact]
    public void APressOnAShapeDrawnOverALinkIsNotABend()
    {
        var bends = new List<Pointer>();
        var drags = new List<Pointer>();
        var (window, _) = Shown(new CanvasView
        {
            Drawing = [new Shape.Link("a-b", 100, 100, 900, 100, [], "border"), new Shape.Rect(400, 50, 200, 100, "raised")],
            BendCommand = new RelayCommand<Pointer>(p => bends.Add(p!)),
            DragCommand = new RelayCommand<Pointer>(p => drags.Add(p!)),
        });
        window.MouseDown(new Point(200, 40), MouseButton.Left);
        window.MouseMove(new Point(200, 80));
        window.MouseUp(new Point(200, 80), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Empty(bends);
        Assert.Single(drags);
        window.Close();
    }

    /// A canvas with undo takes the focus when pressed, so Ctrl+Z takes back its edits (DESIGN 5.29).
    [AvaloniaFact]
    public void A_canvas_with_undo_takes_the_focus_when_pressed()
    {
        var canvas = new CanvasView();
        Edits.SetScope(canvas, "diagram");
        var (window, _) = Shown(canvas);
        Click(window, new Point(40, 20));
        Assert.True(canvas.IsFocused);
        window.Close();
    }
}
