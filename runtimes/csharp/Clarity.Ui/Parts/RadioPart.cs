// The radio group (DESIGN 7.9, conformance/forms.json): one round button
// per choice of its bound field (an enum's values, or what the host offers),
// side by side or one under the other. Only the person picking another
// choice is a change; what code sets is not. The arrow keys pick the next
// or the previous choice, as a system's radio group does. As the shared
// screen draws it (runtimes/web/src/parts/RadioInput.svelte).

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("radio")]
public sealed class RadioPart : Part
{
    /// <summary>How big a button's circle is, how high a line, and the room between choices.</summary>
    public const double Circle = 16, Line = 24, Across = 16, Down = 4;

    int focusAt = -1;

    public override bool Focusable => Enabled && Choices.Count > 0;

    public bool ReadOnly => Spec.Flag("readonly") || (State?.IsReadOnly ?? false);

    public bool Vertical => Spec.Text("orientation") == "vertical";

    public IReadOnlyList<Choice> Choices => DropdownPart.OfferedFor(Host, Bound);

    public object? Value => Bound is { } bound ? Host.Read(bound) : null;

    /// <summary>Where its value is among its choices; -1 when it is none of them.</summary>
    public int Chosen
    {
        get
        {
            var choices = Choices;
            var value = Value;
            for (var i = 0; i < choices.Count; i++)
                if (PressLooks.Same(choices[i].Value, value))
                    return i;
            return -1;
        }
    }

    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    /// <summary>Where each choice is: its circle and its label, flowing on in lines, or one under the other.</summary>
    public IReadOnlyList<Box> Places(Fonts fonts)
    {
        var size = Screen?.Look.Size("text") ?? 14;
        var choices = Choices;
        var widths = choices.Select(c => Circle + 6 + fonts.Measure(c.Label, "text", size, false)).ToList();
        var places = new List<Box>();
        double x = Bounds.X, y = 0;
        for (var i = 0; i < choices.Count; i++)
        {
            if (Vertical)
            {
                places.Add(new Box(Bounds.X, y, Math.Min(widths[i], Bounds.Width), Line));
                y += Line + Down;
                continue;
            }
            if (x > Bounds.X && x + widths[i] > Bounds.Right)
            {
                x = Bounds.X;
                y += Line + Down;
            }
            places.Add(new Box(x, y, widths[i], Line));
            x += widths[i] + Across;
        }
        // The lines in the middle of its height when they fit, else from its top.
        var height = places.Count == 0 ? 0 : places[^1].Bottom;
        var top = Bounds.Y + Math.Max(0, (Bounds.Height - height) / 2);
        return [.. places.Select(p => p.Offset(0, top))];
    }

    int ChoiceAt(double x, double y)
    {
        if (Screen is not { } screen)
            return -1;
        var places = Places(screen.Fonts);
        for (var i = 0; i < places.Count; i++)
            if (places[i].Contains(x, y))
                return i;
        return -1;
    }

    int pressedAt = -1;

    public override void OnPointerPressed(PointerInput at) => pressedAt = ChoiceAt(at.ScreenX, at.ScreenY);

    public override void OnClick()
    {
        if (pressedAt >= 0)
            Pick(pressedAt);
    }

    /// <summary>The person picked the choice at <paramref name="at"/>: a change when it differs from its value.</summary>
    public void Pick(int at)
    {
        var choices = Choices;
        if (!Enabled || ReadOnly || Bound is null || at < 0 || at >= choices.Count)
            return;
        focusAt = at;
        Invalidate();
        if (PressLooks.Same(choices[at].Value, Value))
            return;
        Host.Write(Spec.Name ?? "", Bound, choices[at].Value);
        Fire("change");
    }

    public override bool OnKey(KeyInput key)
    {
        var count = Choices.Count;
        if (count == 0)
            return false;
        var now = focusAt >= 0 ? focusAt : Math.Max(0, Chosen);
        switch (key.Key)
        {
            case Key.Right or Key.Down:
                Pick((now + 1) % count);
                return true;
            case Key.Left or Key.Up:
                Pick((now - 1 + count) % count);
                return true;
            case Key.Space:
                Pick(now);
                return true;
        }
        return false;
    }

    public override void OnFocusChanged(bool focused)
    {
        focusAt = focused ? Chosen : -1;
        Invalidate();
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        using var dimmed = PressLooks.Dimmed(painter, Enabled && !ReadOnly);
        var choices = Choices;
        var places = Places(painter.Fonts);
        var chosen = Chosen;
        var error = State?.HasError == true;
        using (painter.Clip(Bounds))
            for (var i = 0; i < choices.Count; i++)
            {
                var place = places[i];
                var circle = new Box(place.X + 1, place.Y + (Line - Circle) / 2, Circle, Circle);
                var on = i == chosen;
                painter.Fill(circle, look.Color("input"), Circle / 2);
                painter.Stroke(circle, error ? look.Color("error") : on ? look.Color("accent") : look.Color("strong_border"), on ? 1.5 : look.Border, Circle / 2);
                if (on)
                    painter.Fill(circle.Inset(4), look.Color("accent"), (Circle - 8) / 2);
                if (Focused && i == (focusAt >= 0 ? focusAt : Math.Max(0, chosen)))
                    PressLooks.FocusRing(painter, circle.Inset(-3), Circle / 2 + 3);
                painter.Text(choices[i].Label, place with { X = circle.Right + 6, Width = Math.Max(0, place.Right - circle.Right - 6) }, look.Color("text"), look.Size("text"));
            }
    }
}
