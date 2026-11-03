// A sidebar docked to a window's edge (conformance/sidebars.json): its
// parts on a grid of its columns. The person drags a resizable one's inner
// edge between its min and max, folds a collapsible one to a narrow strip
// with its button and back, and lets a maximizable one fill the window and
// give it back. Whether it is folded and whether it fills the window are its
// part state (IsCollapsed, IsMaximized), so code reads and sets them; an
// unnamed sidebar keeps them itself. Folding leaves whether it fills the
// window as it was, as Avalonia's did: shown again, it fills it again. Its
// buttons take the focus and say what they do; its edge shows the resize
// pointer.

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
        // As Avalonia's header: the fill button, then the fold button, then its parts, in Tab order.
        AddChild(new SidebarButton(this, fills: true));
        AddChild(new SidebarButton(this, fills: false));
        holder = new Showing(() => !Collapsed);
        holder.Add(grid);
        AddChild(holder);
    }

    public SidebarSpec Sidebar { get; }

    /// <summary>Its fill and fold buttons.</summary>
    public IReadOnlyList<SidebarButton> Buttons => [.. Children.OfType<SidebarButton>()];

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

    /// <summary>Folds it to its strip or shows it again; whether it fills the window stays as it was, folded it does not show.</summary>
    public void Fold(bool folded)
    {
        if (!Sidebar.Collapsible)
            return;
        if (State is { } state)
            state.IsCollapsed = folded;
        else
            ownCollapsed = folded;
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
        foreach (var button in Buttons)
            button.Arrange((button.Fills ? FillButton : FoldButton) ?? default);
    }

    /// <summary>Over its edge, while the person may drag it, the pointer shows it resizes.</summary>
    public override Cursor? CursorAt(double x, double y) => Edge is { } edge && edge.Contains(x, y) ? Cursor.ResizeLeftRight : null;

    public override void OnPointerPressed(PointerInput at)
    {
        if (Edge is { } edge && edge.Contains(at.ScreenX, at.ScreenY))
            dragging = (at.ScreenX, Width);
    }

    public override void OnPointerMoved(PointerInput at)
    {
        base.OnPointerMoved(at);
        if (Pressed && dragging is { } from)
            DragTo(from.Width + (Sidebar.Left ? at.ScreenX - from.X : from.X - at.ScreenX));
    }

    public override void OnPointerReleased(PointerInput at) => dragging = null;

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var line = Sidebar.Left ? new Box(Bounds.Right - look.Border, Bounds.Y, look.Border, Bounds.Height) : new Box(Bounds.X, Bounds.Y, look.Border, Bounds.Height);
        Plate(painter, Bounds, line);
        if (Edge is { } edge && (Over(edge) || dragging is not null))
            painter.Fill(edge, look.Color("accent_soft"));
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

    internal static void Button(Painter painter, Box box, string mark, bool over, SkiaSharp.SKColor text)
    {
        var look = painter.Look;
        var values = look.Part("button", over ? "hover" : null);
        var radius = values.Radius ?? look.Radius("button");
        painter.Fill(box, values.Background ?? look.Color("panel"), radius);
        painter.Stroke(box, values.Border ?? look.Color("border"), look.Border, radius);
        painter.Text(mark, box, values.Text ?? text, look.Size("text"), align: TextAlign.Center);
    }
}

/// <summary>
/// A sidebar's fold or fill button: a click, Enter or Space folds it (or
/// shows it again) or lets it fill the window (or gives the window back).
/// It takes the focus, shows the focus ring, and says what it does.
/// </summary>
public sealed class SidebarButton : Element
{
    readonly FrameSidebar sidebar;

    internal SidebarButton(FrameSidebar sidebar, bool fills)
    {
        this.sidebar = sidebar;
        Fills = fills;
    }

    /// <summary>Whether it is the fill button; else the fold button.</summary>
    public bool Fills { get; }

    Box? Place => Fills ? sidebar.FillButton : sidebar.FoldButton;

    public override bool IsShown => Place is not null;

    public override bool Focusable => true;

    public override void OnClick() => Press();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Enter or Key.Space) || key.Modifiers != Modifiers.None)
            return false;
        Press();
        return true;
    }

    void Press()
    {
        if (Fills)
            sidebar.Fill(!sidebar.Maximized);
        else
            sidebar.Fold(!sidebar.Collapsed);
    }

    public override string? TooltipAt(double x, double y) => Fills
        ? sidebar.Maximized ? Locale.Translate("sidebarGiveBack", "Give the window back") : Locale.Translate("sidebarFill", "Fill the window")
        : sidebar.Collapsed ? Locale.Translate("sidebarShow", "Show the sidebar") : Locale.Translate("sidebarFold", "Fold the sidebar away");

    public override void OnPointerMoved(PointerInput at) => Invalidate();

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var text = look.Color("frame_text");
        // The fold button points the way the sidebar goes; the fill button shows the arrows out, or in while it fills.
        if (Fills)
        {
            FrameSidebar.Button(painter, Bounds, "", Hovered, text);
            FrameSidebar.FillMark(painter, Bounds, sidebar.Maximized, look.Part("button", Hovered ? "hover" : null).Text ?? text);
        }
        else
            FrameSidebar.Button(painter, Bounds, sidebar.Sidebar.Left != sidebar.Collapsed ? "\u2039" : "\u203A", Hovered, text);
        if (Focused)
            PressLooks.FocusRing(painter, Bounds, look.Radius("button"));
    }
}
