// A number box and a spin box, the same as on every stack
// (conformance/forms.json, formats.json): the value shows in the person's
// format and typed text is read the same way, when the input is left or
// Enter is pressed. Text that is no number stays as typed and is marked,
// and the value stays; a cleared input is zero; only a number that differs
// is a change, and what code sets is never one. Unbound, or bound to a value
// without a format, it keeps the text as typed. A spin box has a range
// (conformance/ranges.json): its arrows and keys move it a step, or ten,
// and a typed number is kept within the range.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("number", "spinbox")]
public sealed class NumberPart : Part
{
    const double ArrowWidth = 22;

    TextEditor editor = null!;
    string own = "";
    string? shown;

    public TextEditor Editor => editor;

    /// <summary>A spin box's arrows.</summary>
    public InputSideButton? Up { get; private set; }

    public InputSideButton? Down { get; private set; }

    /// <summary>What was typed while it is no number; null when the input shows its value.</summary>
    public string? Bad { get; private set; }


    /// <summary>How its value shows and reads: the bound field's format; null keeps the text.</summary>
    public FieldFormat? Format => Bound is { } path ? this.FormatOf(path) : null;

    /// <summary>A spin box's range: as written, else the bound field's own; null for a number box.</summary>
    public NumberRange? Range
    {
        get
        {
            if (Spec.Kind != "spinbox")
                return null;
            var facts = Bound is { } path ? Host.Field(path) : null;
            return NumberRange.Of(Spec.Text("min") ?? facts?.Min, Spec.Text("max") ?? facts?.Max, Spec.Text("step") ?? "1");
        }
    }

    /// <summary>Whether it holds whole numbers: its field is an integer or a duration; unbound, its range has no decimals.</summary>
    public bool Whole => Format is { } format
        ? format.Kind is FormatKind.Integer or FormatKind.Duration
        : !(Spec.Text("step") ?? "1").Contains('.') && !(Spec.Text("min") ?? "").Contains('.');

    /// <summary>The value: its field's, or the text it keeps unbound.</summary>
    public object? Value => Bound is { } path ? Host.Read(path) : State?.Text ?? own;

    /// <summary>The value as it shows for editing.</summary>
    string TextOf(object? value) => Format is { } format
        ? this.FormatsOf().EditText(format, value)
        : value is null ? "" : Convert.ToString(value, CultureInfo.InvariantCulture) ?? "";

    protected internal override void Made()
    {
        editor = new TextEditor
        {
            Placeholder = Spec.Text("placeholder") is { } placeholder ? Host.Text(placeholder) : null,
            Enabled = () => Enabled,
            ReadOnly = () => ReadOnly,
            Marked = () => Bad is not null || State?.HasError == true,
            Entered = Enter,
            KeyFirst = Keyed,
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
        AddChild(editor);
        if (Spec.Kind == "spinbox")
        {
            Up = new InputSideButton(InputIcon.Up, () => Spin(RangeKey.Up)) { Enabled = () => Enabled };
            Down = new InputSideButton(InputIcon.Down, () => Spin(RangeKey.Down)) { Enabled = () => Enabled };
            AddChild(Up);
            AddChild(Down);
            editor.RightInset = ArrowWidth + 2;
        }
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>A value set from elsewhere (code, the record read again) shows as it is, and is no longer marked.</summary>
    void Sync()
    {
        var now = TextOf(Value);
        if (now == shown)
            return;
        shown = now;
        Bad = null;
        editor.Text = now;
    }

    /// <summary>Takes what is typed: a change when it is a number that differs from the value.</summary>
    public void Take()
    {
        Sync();
        var typed = editor.Text;
        if (Bad is null && typed == shown)
            return;
        var next = Read(typed);
        if (next is null)
        {
            Bad = typed;
            Invalidate();
            return;
        }
        Bad = null;
        if (Range is { } range && NumberRange.Number(next) is { } number)
            next = Held(range.TypedIn(number));
        Offer(next);
    }

    /// <summary>What is typed as a value: in the field's format (cleared is zero), or as text; null when it is no number.</summary>
    object? Read(string typed)
    {
        if (Format is { } format)
            return typed.Trim().Length == 0 ? (Whole ? (object)0L : 0m) : this.FormatsOf().Read(format, typed);
        if (Range is null)
            return typed;
        if (typed.Trim().Length == 0)
            return "0";
        return decimal.TryParse(typed.Trim(), NumberStyles.Number, CultureInfo.InvariantCulture, out _) ? typed.Trim() : null;
    }

    /// <summary>A number as the value holds it: a long or a decimal for a field with a format, else text.</summary>
    object Held(decimal n) => Format is null ? n.ToString(CultureInfo.InvariantCulture) : NumberRange.Held(n, Whole);

    /// <summary>Makes <paramref name="next"/> the value, when it differs; else the input shows the value again.</summary>
    void Offer(object next)
    {
        var now = Value;
        var same = NumberRange.Number(next) is { } a && NumberRange.Number(now) is { } b ? a == b : Equals(next, now);
        if (same)
        {
            shown = null;
            Sync();
            return;
        }
        if (Bound is { } path)
            Host.Write(Spec.Name ?? "", path, next);
        else if (State is { } state)
        {
            state.Text = Convert.ToString(next, CultureInfo.InvariantCulture) ?? "";
            Host.Edited(Spec.Name!);
        }
        else
            own = Convert.ToString(next, CultureInfo.InvariantCulture) ?? "";
        Sync();
        Fire("change");
    }

    /// <summary>A spin box's arrow or key: a step, or ten, from the value.</summary>
    public void Spin(RangeKey key)
    {
        if (Range is not { } range || ReadOnly || !Enabled)
            return;
        Bad = null;
        Offer(Held(range.Keyed(NumberRange.Number(Value), key)));
    }

    /// <summary>Up and Down, Page Up and Page Down spin a spin box; Left and Right move the caret.</summary>
    bool Keyed(KeyInput key)
    {
        if (Range is null)
            return false;
        RangeKey? spin = key.Key switch
        {
            Key.Up => RangeKey.Up,
            Key.Down => RangeKey.Down,
            Key.PageUp => RangeKey.PageUp,
            Key.PageDown => RangeKey.PageDown,
            _ => null,
        };
        if (spin is not { } k)
            return false;
        Spin(k);
        return true;
    }

    /// <summary>Enter: the number is taken, then the form it is in submits.</summary>
    bool Enter()
    {
        Take();
        return PartTexts.Submit(this);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        editor.Arrange(box);
        if (Up is not null && Down is not null)
        {
            var half = Math.Max(0, (box.Height - 4) / 2);
            Up.Arrange(new Box(box.Right - 2 - ArrowWidth, box.Y + 2, ArrowWidth, half));
            Down.Arrange(new Box(box.Right - 2 - ArrowWidth, box.Y + 2 + half, ArrowWidth, half));
        }
    }

    public override void Draw(Painter painter)
    {
        Sync();
        base.Draw(painter);
    }
}
