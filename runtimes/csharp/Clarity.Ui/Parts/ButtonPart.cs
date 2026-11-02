// The buttons (DESIGN 7.9): a button runs its click, a toggle stays pressed
// and tells its change, and a button group joins the buttons it holds in one
// row with only its outer corners round. Each is drawn in the theme's
// `button` look, with its hover and pressed looks, a style over it, the
// focus ring in the focus color and the dimmed look of a disabled part, as
// the shared screen draws them (runtimes/web/src/kinds/Button.svelte,
// Toggle.svelte, ButtonGroup.svelte).

using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>What a button and a toggle share: the push it is drawn as, its caption and its corners in a group.</summary>
public abstract class PressPart : Part
{
    /// <summary>Its round corners: all, or only the outer ones in a button group.</summary>
    public RoundCorners Corners { get; set; } = RoundCorners.All;

    public override bool Focusable => Enabled;

    /// <summary>Its caption as shown: what code set, else as the design wrote it, translated.</summary>
    public string Caption => State?.Caption is { Length: > 0 } set ? set : Host.Text(Spec.Text("caption"));


    /// <summary>Drawn pressed while it is not held: a toggle that is on.</summary>
    protected virtual bool On => false;

    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    /// <summary>How a push looks now: the theme's button look, its style over it, and the hover or pressed look over both.</summary>
    protected LookValues PushLook(Painter painter, bool hover, bool pressed)
    {
        // A toggle that is on whose style fills it shows its style pressed.
        pressed |= On && StyleFills(painter);
        var state = !Enabled || !(hover || pressed) ? null : pressed ? "pressed" : "hover";
        return painter.Look.Pressable("button", Spec.Text("style"), state);
    }

    /// <summary>Whether its style fills it: such a toggle that is on shows its style pressed, not the soft accent.</summary>
    bool StyleFills(Painter painter) => Spec.Text("style") is { Length: > 0 } style && painter.Look.Style(style).Background is not null;

    /// <summary>A push at <paramref name="box"/>: its plate, the ring of a default button or a toggle that is on, and its focus ring.</summary>
    protected void DrawPush(Painter painter, Box box, LookValues values, RoundCorners corners)
    {
        var look = painter.Look;
        var radius = values.Radius ?? look.Radius("button");
        var fill = On && !StyleFills(painter) ? look.Color("accent_soft") : values.Background ?? look.Color("raised");
        var line = On || Spec.Flag("default") ? look.Color("accent") : values.Border ?? look.Color("border");
        PressLooks.Plate(painter, box, fill, line, look.Border, radius, corners);
        if (Focused)
            PressLooks.FocusRing(painter, box, radius);
    }

    protected void DrawCaption(Painter painter, Box box, LookValues values) =>
        painter.Text(Caption, box.Inset(painter.Look.Padding), values.Text ?? painter.Look.Color("text"), values.Size ?? painter.Look.Size("text"), values.Font ?? "text", values.Bold, TextAlign.Center);
}

/// <summary>A button: its click runs its handlers; a split button opens its menu from the arrow beside it.</summary>
[PartKind("button")]
public sealed class ButtonPart : PressPart
{
    /// <summary>How wide a split button's arrow is.</summary>
    public const double ArrowWidth = 24;

    double pointerX;
    Overlay? menu;

    public bool Split => Spec.Flag("split");

    /// <summary>Where a split button's arrow is.</summary>
    public Box Arrow => new(Bounds.Right - ArrowWidth, Bounds.Y, ArrowWidth, Bounds.Height);

    /// <summary>The menu a split button's arrow opened; none while it is closed.</summary>
    public Overlay? Menu => menu;

    bool OnArrow => Split && pointerX >= Arrow.X;

    public override void OnPointerMoved(PointerInput at)
    {
        if (Split && pointerX != at.ScreenX)
        {
            pointerX = at.ScreenX;
            Invalidate();
        }
    }

    public override void OnPointerPressed(PointerInput at) => pointerX = at.ScreenX;

    public override void OnClick()
    {
        if (!Enabled)
            return;
        if (OnArrow)
            OpenMenu();
        else
            Click();
    }

    /// <summary>Clicked, as a click on it, Enter or Space, or its shortcut does: its handlers run and the window it opens opens.</summary>
    public void Click()
    {
        if (Enabled)
            Fire("click");
    }

    public override bool OnKey(KeyInput key)
    {
        if (!Enabled)
            return false;
        if (key.Key is Key.Enter or Key.Space)
        {
            Click();
            return true;
        }
        if (key.Key == Key.Down && Split)
        {
            OpenMenu();
            return true;
        }
        return false;
    }

    /// <summary>Opens a split button's menu below it: picking an item runs its handler.</summary>
    public void OpenMenu()
    {
        if (Screen is not { } screen || Spec.Menu is not { } items || menu is not null)
            return;
        var shown = items.Items.Where(i => i.Text("kind") != "separator").ToList();
        var list = new DropdownList([.. shown.Select(i => Host.Text(i.Text("caption")))], -1, at =>
        {
            CloseMenu();
            Pick(shown[at]);
        });
        menu = new Overlay(list, DropdownList.Below(screen, Bounds, Math.Max(Bounds.Width, 160), shown.Count)) { Closed = () => menu = null };
        screen.Open(menu);
    }

    void CloseMenu()
    {
        if (menu is { } open)
            Screen?.Close(open);
        menu = null;
    }

    public override void Draw(Painter painter)
    {
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        var held = Pressed && Hovered;
        if (!Split)
        {
            var values = PushLook(painter, Hovered, held);
            DrawPush(painter, Bounds, values, Corners);
            DrawCaption(painter, Bounds, values);
            return;
        }
        var main = Bounds with { Width = Math.Max(0, Bounds.Width - ArrowWidth) };
        var mainLook = PushLook(painter, Hovered && !OnArrow, held && !OnArrow);
        DrawPush(painter, main, mainLook, Corners & RoundCorners.Left);
        DrawCaption(painter, main, mainLook);
        var arrowLook = PushLook(painter, Hovered && OnArrow, held && OnArrow);
        DrawPush(painter, Arrow, arrowLook, Corners & RoundCorners.Right);
        PressLooks.DownArrow(painter, Arrow, arrowLook.Text ?? painter.Look.Color("text"));
    }
}

/// <summary>A toggle: a button that stays pressed, its value bound or kept in its state; pressing it is a change.</summary>
[PartKind("toggle")]
public sealed class TogglePart : PressPart
{
    bool kept;

    /// <summary>Whether it is pressed: its bound value, else its state's, else its own.</summary>
    public bool IsOn => Bound is { } bound ? Host.Read(bound) is true : State?.IsChecked ?? kept;

    protected override bool On => IsOn;

    public override void OnClick() => Flip();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Enter or Key.Space))
            return false;
        Flip();
        return true;
    }

    /// <summary>The person pressed it: it changes and tells its change.</summary>
    void Flip()
    {
        if (!Enabled || ReadOnly)
            return;
        var next = !IsOn;
        if (Bound is { } bound)
            Host.Write(Spec.Name ?? "", bound, next);
        else if (State is { } state)
        {
            state.IsChecked = next;
            Host.Edited(Spec.Name!);
        }
        else
            kept = next;
        Fire("change");
        Invalidate();
    }

    public override void Draw(Painter painter)
    {
        using var dimmed = PressLooks.Dimmed(painter, Enabled && !ReadOnly);
        var values = PushLook(painter, Hovered, Pressed && Hovered);
        DrawPush(painter, Bounds, values, Corners);
        DrawCaption(painter, Bounds, values);
    }
}

/// <summary>A button group: the buttons and toggles it holds, joined in one row; only its outer corners are round.</summary>
[PartKind("buttons")]
public sealed class ButtonsPart : Part
{
    GridPanel grid = null!;

    protected internal override void Made()
    {
        var columns = Math.Max(1, (int)(Spec.Number("width") ?? 1));
        grid = GridOf(Spec.Children, columns, gap: 0);
        AddChild(grid);
        var pushes = grid.Children.OfType<Part>().OrderBy(p => p.Spec.Number("col") ?? 1).ToList();
        for (var i = 0; i < pushes.Count; i++)
            if (pushes[i] is PressPart push)
                push.Corners = (i == 0 ? RoundCorners.Left : RoundCorners.None) | (i == pushes.Count - 1 ? RoundCorners.Right : RoundCorners.None);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        grid.Arrange(box);
    }
}
