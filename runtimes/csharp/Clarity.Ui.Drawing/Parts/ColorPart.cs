// The color part, the same as on every stack (conformance/colors.json): a
// swatch that opens the palette every renderer shares (Palette.Rows) over
// the window, and the color as "#rrggbb" to type (at most 7 characters).
// Typed text is taken when the input is left or Enter is pressed (Enter then
// submits the form); text that is no color stays as typed and is marked, and
// the value stays. The palette works by keyboard too: the arrows and Tab
// move through its colors, Enter or Space picks one; each color says its
// "#rrggbb" when the pointer rests on it. Only a color that differs is a
// change; what code sets is never one. A bound part writes its field; an
// unbound named one keeps its color in its state's text.

using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

[PartKind("color")]
public sealed class ColorPart : Part, ITakesTyped
{
    const double Gap = 4;

    /// <summary>How wide the swatch is.</summary>
    public const double SwatchWidth = 32;

    TextEditor editor = null!;
    ColorSwatch swatch = null!;
    Overlay? open;
    string own = "";
    string? shown;

    /// <summary>Where the person types the color.</summary>
    public TextEditor Editor => editor;

    /// <summary>The swatch, which shows the color and opens the palette.</summary>
    public ColorSwatch Swatch => swatch;

    /// <summary>The palette, while it is open.</summary>
    public Overlay? Palette => open;

    /// <summary>The palette's colors, while it is open.</summary>
    public PaletteGrid? Grid { get; private set; }

    /// <summary>Whether what is typed is no color: the input is marked, the value stays.</summary>
    public bool Bad { get; private set; }


    /// <summary>The color it has, "#rrggbb", or "" for none.</summary>
    public string Value => Bound is { } path ? Host.Read(path) as string ?? "" : State?.Text ?? own;

    bool Usable => Enabled && !ReadOnly;

    protected internal override void Made()
    {
        swatch = new ColorSwatch(this);
        editor = new TextEditor
        {
            Placeholder = Spec.Text("placeholder") is { } placeholder ? Host.Text(placeholder) : null,
            Enabled = () => Enabled,
            ReadOnly = () => ReadOnly,
            Marked = () => Bad || State?.HasError == true,
            MaxLength = 7,
            Entered = () =>
            {
                Take();
                return PartTexts.Submit(this);
            },
        };
        editor.FocusMoved += focused =>
        {
            if (focused)
                Fire("enter");
            else
            {
                Take();
                Fire("leave");
            }
        };
        AddChild(swatch);
        AddChild(editor);
        PressLooks.Watch(this, Host, State);
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>A value set from elsewhere shows as it is, and what was marked is no longer.</summary>
    void Sync()
    {
        var now = Value;
        if (now == shown)
            return;
        shown = now;
        Bad = false;
        editor.Text = now;
    }

    /// <summary>Takes what is typed: a change when it is a color that differs from the value; text that is no color is marked.</summary>
    public void TakeTyped() => Take();

    /// <summary>Takes what is typed; see <see cref="TakeTyped"/>.</summary>
    public void Take()
    {
        Sync();
        if (Clarity.Runtime.Palette.Read(editor.Text) is not { } color)
        {
            Bad = true;
            Invalidate();
            return;
        }
        Offer(color);
    }

    /// <summary>The person picked <paramref name="color"/> from the palette: a change when it differs.</summary>
    public void Pick(string color)
    {
        Close();
        Offer(color);
    }

    void Offer(string color)
    {
        Bad = false;
        if (!Usable || color == Value)
        {
            shown = null;
            Sync();
            Invalidate();
            return;
        }
        if (Bound is { } path)
            Host.Write(Spec.Name ?? "", path, color);
        else if (State is { } state)
        {
            state.Text = color;
            Host.Edited(Spec.Name!);
        }
        else
            own = color;
        Sync();
        Fire("change");
        Invalidate();
    }

    /// <summary>Opens the palette below the swatch, or closes it when it is open.</summary>
    public void Toggle()
    {
        if (open is not null)
        {
            Close();
            return;
        }
        if (!Usable || Screen is not { } screen)
            return;
        var grid = new PaletteGrid(Value, Pick) { Holder = swatch };
        Grid = grid;
        var x = Math.Max(0, Math.Min(swatch.Bounds.X, screen.Width - PaletteGrid.Width));
        var below = Bounds.Bottom + 4;
        var y = below + PaletteGrid.Height > screen.Height && Bounds.Y - 4 - PaletteGrid.Height >= 0 ? Bounds.Y - 4 - PaletteGrid.Height : below;
        open = new Overlay(grid, new Box(x, y, PaletteGrid.Width, PaletteGrid.Height))
        {
            Anchor = () => swatch.Bounds,
            Closed = () =>
            {
                open = null;
                Grid = null;
            },
        };
        screen.Open(open);
    }

    public void Close()
    {
        if (open is { } was)
            Screen?.Close(was);
        open = null;
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var side = Math.Min(SwatchWidth, box.Width);
        swatch.Arrange(new Box(box.X, box.Y, side, box.Height));
        editor.Arrange(new Box(box.X + side + Gap, box.Y, Math.Max(0, box.Width - side - Gap), box.Height));
    }

    public override void Draw(Painter painter)
    {
        Sync();
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        base.Draw(painter);
    }

    /// <summary>The color of "#rrggbb"; none for "" or text that is no color.</summary>
    internal static SKColor? ColorOf(string? text) =>
        Clarity.Runtime.Palette.Read(text) is { Length: 7 } color && SKColor.TryParse(color, out var parsed) ? parsed : null;
}

/// <summary>A color part's swatch: the color in a framed square, a line across it for none; a click or Enter opens the palette, whose keys it then hands on.</summary>
public sealed class ColorSwatch(ColorPart part) : Element
{
    public override bool Focusable => part.Enabled && !part.ReadOnly;

    /// <summary>While the palette is open, Tab moves through its colors.</summary>
    public override bool WantsTab => part.Grid is not null;

    public override void OnClick() => part.Toggle();

    public override bool OnKey(KeyInput key)
    {
        if (part.Grid is { } grid)
            return grid.Keyed(key);
        if (key.Key is Key.Enter or Key.Space or Key.Down)
        {
            part.Toggle();
            return true;
        }
        return false;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var radius = look.Radius("input");
        var values = look.Part("input", Focused ? "focus" : null);
        painter.Fill(Bounds, values.Background ?? look.Color("input"), radius);
        var inner = Bounds.Inset(3);
        if (ColorPart.ColorOf(part.Value) is { } color)
            painter.Fill(inner, color, Math.Max(0, radius - 2));
        else
        {
            using var pen = new SKPaint { Color = look.Color("muted"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.2f };
            painter.Canvas.DrawLine((float)inner.X, (float)inner.Bottom, (float)inner.Right, (float)inner.Y, pen);
        }
        var line = Hovered && Focusable ? values.Border ?? look.Color("strong_border") : values.Border ?? look.Color("border");
        painter.Stroke(Bounds, line, look.Border, radius);
        if (Focused)
            PressLooks.FocusRing(painter, Bounds, radius);
    }
}

/// <summary>The palette every renderer shares, ten colors a row, over the window; a press on a color picks it, and so do Enter and Space on the color the keys marked.</summary>
public sealed class PaletteGrid : Element
{
    const double Cell = 22;
    const double Space = 3;
    const double Pad = 6;
    const int Across = 10;

    /// <summary>How wide and high it is.</summary>
    public static double Width => 2 * Pad + Across * Cell + (Across - 1) * Space;

    public static double Height => 2 * Pad + Rows * Cell + (Rows - 1) * Space;

    static int Rows => (Clarity.Runtime.Palette.Rows.Sum(r => r.Count) + Across - 1) / Across;

    readonly Action<string> picked;
    int pressedAt = -1;
    int hoveredAt = -1;

    public PaletteGrid(string? chosen, Action<string> picked)
    {
        Chosen = chosen;
        this.picked = picked;
        Marked = Math.Max(0, Colors.ToList().IndexOf(chosen ?? ""));
    }

    /// <summary>The color the keys move and Enter picks: at first the one the part has, else the first.</summary>
    public int Marked { get; private set; }

    /// <summary>A key from the swatch while it is open: the arrows a color or a row on, Tab the next color, Home and End the ends, Enter or Space picks.</summary>
    public bool Keyed(KeyInput key)
    {
        var count = Colors.Count;
        int? to = key.Key switch
        {
            Key.Left => Marked - 1,
            Key.Right => Marked + 1,
            Key.Up => Marked - Across,
            Key.Down => Marked + Across,
            Key.Tab => Marked + (key.Shift ? -1 : 1),
            Key.Home => 0,
            Key.End => count - 1,
            _ => null,
        };
        if (to is { } next)
        {
            // Tab goes round; the arrows stop at the edges.
            Marked = key.Key == Key.Tab ? (next + count) % count : Math.Clamp(next, 0, count - 1);
            Invalidate();
            return true;
        }
        if (key.Key is Key.Enter or Key.Space)
        {
            picked(Colors[Marked]);
            return true;
        }
        return false;
    }

    /// <summary>A color says its "#rrggbb" when the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) => ColorAt(x, y) is var at and >= 0 ? Colors[at] : null;

    /// <summary>The colors it offers, in order: row by row.</summary>
    public IReadOnlyList<string> Colors { get; } = [.. Clarity.Runtime.Palette.Rows.SelectMany(r => r)];

    /// <summary>The color it marks as the one the part has.</summary>
    public string? Chosen { get; }

    /// <summary>The part it belongs to, which keeps the focus while the person picks.</summary>
    public Element? Holder { get; init; }

    /// <summary>Where the color <paramref name="color"/> lies on the screen.</summary>
    public Box Place(string color) => PlaceOf(Colors.ToList().IndexOf(color));

    Box PlaceOf(int at) => new(Bounds.X + Pad + at % Across * (Cell + Space), Bounds.Y + Pad + at / Across * (Cell + Space), Cell, Cell);

    int ColorAt(double x, double y)
    {
        for (var i = 0; i < Colors.Count; i++)
            if (PlaceOf(i).Contains(x, y))
                return i;
        return -1;
    }

    public override void OnPointerMoved(PointerInput at)
    {
        var over = ColorAt(at.ScreenX, at.ScreenY);
        if (over != hoveredAt)
        {
            hoveredAt = over;
            Invalidate();
        }
    }

    public override void OnPointerPressed(PointerInput at)
    {
        pressedAt = ColorAt(at.ScreenX, at.ScreenY);
        if (Holder is not null)
            Screen?.Focus(Holder);
    }

    public override void OnClick()
    {
        if (pressedAt >= 0)
            picked(Colors[pressedAt]);
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var radius = look.Radius("input");
        painter.Fill(Bounds, look.Color("panel"), radius);
        painter.Stroke(Bounds, look.Color("border"), look.Border, radius);
        for (var i = 0; i < Colors.Count; i++)
        {
            var place = PlaceOf(i);
            if (ColorPart.ColorOf(Colors[i]) is { } color)
                painter.Fill(place, color, 3);
            painter.Stroke(place, look.Color(i == hoveredAt ? "strong_border" : "border"), 1, 3);
            if (Colors[i] == Chosen)
                painter.Stroke(new Box(place.X - 2, place.Y - 2, place.Width + 4, place.Height + 4), look.Color("accent"), 2, 4);
            // The color the keys marked has the focus ring.
            if (i == Marked && Holder is { Focused: true })
                PressLooks.FocusRing(painter, place.Inset(-1), 4);
        }
    }
}
