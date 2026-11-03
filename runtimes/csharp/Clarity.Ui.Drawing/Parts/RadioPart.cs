// The radio group (DESIGN 7.9, conformance/forms.json): one round button
// per choice of its bound field (an enum's values, or what the host offers),
// side by side or one under the other. Only the person picking another
// choice is a change; what code sets is not. Each button is a Tab stop of
// its own, which Space or Enter picks; the arrow keys pick the next or the
// previous choice, as a system's radio group does. Side by side, choices
// that do not fit on one line go on to the next. As the shared screen
// draws it (runtimes/web/src/parts/RadioInput.svelte).

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("radio")]
public sealed class RadioPart : Part
{
    /// <summary>How big a button's circle is, how high a line, and the room between choices.</summary>
    public const double Circle = 16, Line = 24, Across = 12, Down = 4;

    readonly List<RadioChoice> buttons = [];

    /// <summary>One element per choice, each a Tab stop of its own.</summary>
    public IReadOnlyList<RadioChoice> Buttons
    {
        get
        {
            Rebuild();
            return buttons;
        }
    }

    /// <summary>The choice whose button has the focus; -1 for none.</summary>
    int FocusAt => buttons.FindIndex(b => b.Focused);

    /// <summary>A button per choice, made again when the number of choices changed.</summary>
    void Rebuild()
    {
        var count = Choices.Count;
        if (count == buttons.Count)
            return;
        var focused = FocusAt;
        foreach (var button in buttons)
            RemoveChild(button);
        buttons.Clear();
        for (var i = 0; i < count; i++)
        {
            var button = new RadioChoice(this, i);
            buttons.Add(button);
            AddChild(button);
        }
        if (focused >= 0 && Screen is { } screen)
            screen.Focus(buttons[Math.Min(focused, count - 1)]);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        Rebuild();
        if (Screen is not { } screen)
            return;
        var places = Places(screen.Fonts);
        for (var i = 0; i < buttons.Count && i < places.Count; i++)
            buttons[i].Arrange(places[i]);
    }


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


    /// <summary>The person picked the choice at <paramref name="at"/>: a change when it differs from its value.</summary>
    public void Pick(int at)
    {
        var choices = Choices;
        if (!Enabled || ReadOnly || Bound is null || at < 0 || at >= choices.Count)
            return;
        // The focus follows the pick, as the arrow keys move it.
        if (FocusAt >= 0 && at < buttons.Count)
            Screen?.Focus(buttons[at]);
        Invalidate();
        if (PressLooks.Same(choices[at].Value, Value))
            return;
        Host.Write(Spec.Name ?? "", Bound, choices[at].Value);
        Fire("change");
    }

    /// <summary>A key on the button of choice <paramref name="at"/>: the arrows pick the next or the previous choice, Space or Enter this one.</summary>
    internal bool Keyed(int at, KeyInput key)
    {
        var count = Choices.Count;
        if (count == 0 || key.Control || key.Modifiers.HasFlag(Modifiers.Alt))
            return false;
        switch (key.Key)
        {
            case Key.Right or Key.Down:
                Pick((at + 1) % count);
                return true;
            case Key.Left or Key.Up:
                Pick((at - 1 + count) % count);
                return true;
            case Key.Space or Key.Enter:
                Pick(at);
                return true;
        }
        return false;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        using var dimmed = PressLooks.Dimmed(painter, Enabled && !ReadOnly);
        var choices = Choices;
        var places = Places(painter.Fonts);
        var chosen = Chosen;
        var focusAt = FocusAt;
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
                if (i == focusAt)
                    PressLooks.FocusRing(painter, circle.Inset(-3), Circle / 2 + 3);
                painter.Text(choices[i].Label, place with { X = circle.Right + 6, Width = Math.Max(0, place.Right - circle.Right - 6) }, look.Color("text"), look.Size("text"));
            }
    }
}

/// <summary>One button of a radio group: a Tab stop of its own; a click or Space or Enter picks its choice. The group draws it.</summary>
public sealed class RadioChoice(RadioPart group, int index) : Element
{
    /// <summary>Which choice of the group it is.</summary>
    public int Index { get; } = index;

    public override bool Focusable => group.Enabled && !group.ReadOnly;

    public override void OnClick() => group.Pick(Index);

    public override bool OnKey(KeyInput key) => group.Keyed(Index, key);

    public override void OnFocusChanged(bool focused) => Invalidate();

    // The group draws its buttons, so a choice's focus ring and its circle stay one picture.
    public override void Draw(Painter painter) { }
}
