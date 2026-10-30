// A canvas the person zooms and drags on, driven by the mouse as a person
// would (conformance/drawing.json, views): a drag runs its command with
// where it was pressed, the drop when it is let go, a press that does not
// move is a click, and the wheel zooms and scrolls a canvas that zooms.

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
    /// A canvas of 1000 x 500 units in a window of 400 x 200 pixels: 2.5 units a pixel.
    static (Window, CanvasView) Shown(CanvasView canvas)
    {
        canvas.UnitsWidth = 1000;
        canvas.UnitsHeight = 500;
        var window = new Window { Width = 400, Height = 200, Content = canvas };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        return (window, canvas);
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
        // A press that does not move is a click.
        window.MouseDown(new Point(40, 20), MouseButton.Left);
        window.MouseMove(new Point(41, 21));
        window.MouseUp(new Point(41, 21), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal([new Pointer(102, 52)], clicks);
        window.Close();
    }

    /// On a canvas that zooms, a drag the handler does nothing with (the drawing
    /// stays the same) moves the view, and sends no drop.
    [AvaloniaFact]
    public void ADragTheHandlerDoesNothingWithMovesTheView()
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
        window.MouseWheel(new Point(200, 100), new Vector(0, 3 * Math.Log(2)), RawInputModifiers.Control);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal(250, canvas.View.X, 6);
        window.MouseDown(new Point(200, 100), MouseButton.Left);
        window.MouseMove(new Point(160, 100));
        window.MouseMove(new Point(120, 100));
        window.MouseUp(new Point(120, 100), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        // 80 pixels at 0.8 pixels a unit: the view moved 100 units to the right.
        Assert.Equal(1, drags);
        Assert.Equal(350, canvas.View.X, 6);
        Assert.Empty(drops);
        window.Close();
    }

    /// A canvas that snaps gives places on its grid: a click at 20, 30 pixels is 50, 75 units, snapped to 100, 100.
    [AvaloniaFact]
    public void ACanvasThatSnapsGivesPlacesOnItsGrid()
    {
        var clicks = new List<Pointer>();
        var (window, _) = Shown(new CanvasView { Grid = 100, Snap = true, Command = new RelayCommand<Pointer>(p => clicks.Add(p!)) });
        window.MouseDown(new Point(20, 30), MouseButton.Left);
        window.MouseUp(new Point(20, 30), MouseButton.Left);
        Dispatcher.UIThread.RunJobs();
        Assert.Equal([new Pointer(100, 100)], clicks);
        window.Close();
    }

    [AvaloniaFact]
    public void TheWheelZoomsAndScrollsACanvasThatZoomsAndARedrawKeepsTheView()
    {
        var (window, canvas) = Shown(new CanvasView { Zoom = true });
        Assert.Equal(new Rect(0, 0, 1000, 500), canvas.View);
        // Control and one notch of the wheel zoom in around the pointer, here the middle.
        window.MouseWheel(new Point(200, 100), new Vector(0, 3 * Math.Log(2)), RawInputModifiers.Control);
        Dispatcher.UIThread.RunJobs();
        var zoomed = canvas.View;
        Assert.Equal((250, 125, 500, 250), ((int)Math.Round(zoomed.X), (int)Math.Round(zoomed.Y), (int)Math.Round(zoomed.Width), (int)Math.Round(zoomed.Height)));
        // The wheel alone scrolls: a notch down is 50 pixels, 62.5 units at 0.8 pixels a unit.
        window.MouseWheel(new Point(200, 100), new Vector(0, -1));
        Dispatcher.UIThread.RunJobs();
        Assert.Equal(187.5, canvas.View.Y, 6);
        // A new drawing keeps the view; other units start over.
        canvas.Drawing = [];
        Assert.Equal(187.5, canvas.View.Y, 6);
        canvas.UnitsWidth = 2000;
        Assert.Equal(new Rect(0, 0, 2000, 500), canvas.View);
        window.Close();
    }
}
