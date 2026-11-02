// A sidebar docked to a window's edge (conformance/sidebars.json): its
// parts on a grid of its columns. The person drags a resizable one's inner
// edge between its min and max, folds a collapsible one to a narrow strip
// with its button and back, and lets a maximizable one fill the window and
// give it back. Whether it is folded and whether it fills the window are its
// part state (IsCollapsed, IsMaximized), so code reads and sets them; an
// unnamed sidebar keeps them itself.

using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class FrameSidebar : FrameRegion
{
    /// <summary>The space inside it, and how high the row of its buttons is.</summary>
    public const double Padding = 8, ButtonsHeight = 24;

    /// <summary>How wide the edge the person drags is.</summary>
    public const double EdgeWidth = 6;

    readonly Showing holder;
    readonly GridPanel grid;
    readonly List<Part> parts = [];
    double? dragged;
    bool ownCollapsed;
    bool ownMaximized;
    (double X, double Width)? dragging;
    string? pressedOn;

    public FrameSidebar(WindowFrame frame, SidebarSpec sidebar) : base(frame)
    {
        Sidebar = sidebar;
        ownCollapsed = sidebar.Collapsed;
        grid = new GridPanel(Math.Max(1, sidebar.Columns));
        foreach (var spec in sidebar.Parts)
        {
            var part = PartKinds.Make(spec, frame.Host);
            parts.Add(part);
            grid.Add(part, spec.Place(part.Grows));
        }
        holder = new Showing(() => !Collapsed);
        holder.Add(grid);
        AddChild(holder);
    }

    public SidebarSpec Sidebar { get; }

    /// <summary>Its parts, in the order of its markup.</summary>
    public IReadOnlyList<Part> Parts => parts;

    PartState? State => Sidebar.Name is null ? null : Host.State(Sidebar.Name);

    public bool Shows => FrameRules.Shows(Host, Sidebar.Name);

    /// <summary>It shows, unless the other sidebar fills the window.</summary>
    public override bool IsShown => Shows && (Frame.Filling is null || Frame.Filling == this);

    public bool Collapsed => FrameRules.SidebarNow(Sidebar, State?.IsCollapsed ?? ownCollapsed, null).Collapsed;

    public bool Maximized => FrameRules.SidebarNow(Sidebar, null, State?.IsMaximized ?? ownMaximized).Maximized && !Collapsed;

    /// <summary>Its width while it is neither folded nor filling the window: as designed, or as the person dragged it.</summary>
    public double Width => dragged ?? Sidebar.Width;

    public override double Wanted => Collapsed ? FrameRules.StripWidth : Width;

    bool HasButtons => Sidebar.Collapsible || Sidebar.Maximizable;

    /// <summary>The edge the person drags, on the side toward the window's parts; none when it cannot be dragged now.</summary>
    public Box? Edge => Sidebar.Resizable && !Collapsed && !Maximized
        ? new Box(Sidebar.Left ? Bounds.Right - EdgeWidth : Bounds.X, Bounds.Y, EdgeWidth, Bounds.Height)
        : null;

    /// <summary>The fold button, at the end toward the window's parts; none for a sidebar that does not fold.</summary>
    public Box? FoldButton => Sidebar.Collapsible ? ButtonAt(0) : null;

    /// <summary>The button that lets it fill the window; none while it is folded or cannot.</summary>
    public Box? FillButton => Sidebar.Maximizable && !Collapsed ? ButtonAt(Sidebar.Collapsible ? 1 : 0) : null;

    Box ButtonAt(int place)
    {
        var inner = Collapsed ? Bounds.Inset(2) : Bounds.Inset(Padding);
        var x = Sidebar.Left ? inner.Right - (place + 1) * (ButtonsHeight + 2) + 2 : inner.X + place * (ButtonsHeight + 2);
        return new Box(Collapsed ? inner.X : x, inner.Y, ButtonsHeight, ButtonsHeight);
    }

    /// <summary>Folds it to its strip or shows it again; folded, it no longer fills the window.</summary>
    public void Fold(bool folded)
    {
        if (!Sidebar.Collapsible)
            return;
        if (State is { } state)
        {
            state.IsCollapsed = folded;
            state.IsMaximized = false;
        }
        else
        {
            ownCollapsed = folded;
            ownMaximized = false;
        }
        Invalidate();
    }

    /// <summary>Lets it fill the window, or gives the window back.</summary>
    public void Fill(bool filling)
    {
        if (!Sidebar.Maximizable)
            return;
        if (State is { } state)
            state.IsMaximized = filling;
        else
            ownMaximized = filling;
        Invalidate();
    }

    /// <summary>The person dragged its edge to make it <paramref name="width"/> wide: within its limits.</summary>
    public void DragTo(double width)
    {
        dragged = FrameRules.SidebarWidth(width, Sidebar.Min, Sidebar.Max);
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var inner = box.Inset(Padding);
        var top = HasButtons ? ButtonsHeight + 6 : 0;
        holder.Arrange(new Box(inner.X, inner.Y + top, inner.Width, Math.Max(0, inner.Height - top)));
    }

    public override void OnPointerPressed(PointerInput at)
    {
        pressedOn = null;
        if (Edge is { } edge && edge.Contains(at.ScreenX, at.ScreenY))
            dragging = (at.ScreenX, Width);
        else if (FoldButton is { } fold && fold.Contains(at.ScreenX, at.ScreenY))
            pressedOn = "fold";
        else if (FillButton is { } fill && fill.Contains(at.ScreenX, at.ScreenY))
            pressedOn = "fill";
    }

    public override void OnPointerMoved(PointerInput at)
    {
        base.OnPointerMoved(at);
        if (Pressed && dragging is { } from)
            DragTo(from.Width + (Sidebar.Left ? at.ScreenX - from.X : from.X - at.ScreenX));
    }

    public override void OnPointerReleased(PointerInput at) => dragging = null;

    public override void OnClick()
    {
        if (pressedOn == "fold")
            Fold(!Collapsed);
        else if (pressedOn == "fill")
            Fill(!Maximized);
        pressedOn = null;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var line = Sidebar.Left ? new Box(Bounds.Right - look.Border, Bounds.Y, look.Border, Bounds.Height) : new Box(Bounds.X, Bounds.Y, look.Border, Bounds.Height);
        Plate(painter, Bounds, line);
        if (Edge is { } edge && (Over(edge) || dragging is not null))
            painter.Fill(edge, look.Color("accent_soft"));
        var text = look.Color("frame_text");
        // The fold button points the way the sidebar goes; the fill button shows the arrows out, or in while it fills.
        if (FoldButton is { } fold)
            Button(painter, fold, Sidebar.Left != Collapsed ? "\u2039" : "\u203A", Over(fold), text);
        if (FillButton is { } fill)
        {
            Button(painter, fill, "", Over(fill), text);
            FillMark(painter, fill, Maximized, look.Part("button", Over(fill) ? "hover" : null).Text ?? text);
        }
        if (!Collapsed)
            using (painter.Clip(Bounds))
                base.Draw(painter);
    }

    /// <summary>
    /// The fill button's mark, drawn as lines so every font shows it: a
    /// double arrow from the bottom left to the top right to fill the window
    /// (as U+2922), from the top left to the bottom right while it fills (as U+2921).
    /// </summary>
    public static void FillMark(Painter painter, Box box, bool filling, SkiaSharp.SKColor color)
    {
        var side = (float)Math.Min(box.Width, box.Height) * 0.28f;
        var (cx, cy) = ((float)(box.X + box.Width / 2), (float)(box.Y + box.Height / 2));
        var (dx, dy) = filling ? (side, side) : (side, -side);
        using var pen = new SkiaSharp.SKPaint { Color = color, IsAntialias = true, Style = SkiaSharp.SKPaintStyle.Stroke, StrokeWidth = 1.6f, StrokeCap = SkiaSharp.SKStrokeCap.Round };
        var canvas = painter.Canvas;
        canvas.DrawLine(cx - dx, cy - dy, cx + dx, cy + dy, pen);
        var head = side * 0.55f;
        foreach (var s in new[] { 1f, -1f })
        {
            // Each end's arrowhead: two short lines back along the sides of the box.
            var (ex, ey) = (cx + s * dx, cy + s * dy);
            canvas.DrawLine(ex, ey, ex - s * Math.Sign(dx) * head, ey, pen);
            canvas.DrawLine(ex, ey, ex, ey - s * Math.Sign(dy) * head, pen);
        }
    }

    static void Button(Painter painter, Box box, string mark, bool over, SkiaSharp.SKColor text)
    {
        var look = painter.Look;
        var values = look.Part("button", over ? "hover" : null);
        var radius = values.Radius ?? look.Radius("button");
        painter.Fill(box, values.Background ?? look.Color("panel"), radius);
        painter.Stroke(box, values.Border ?? look.Color("border"), look.Border, radius);
        painter.Text(mark, box, values.Text ?? text, look.Size("text"), align: TextAlign.Center);
    }
}
