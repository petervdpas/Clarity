// A text box, a memo and a password, the same as on every stack
// (conformance/forms.json). Typed text is taken when the input is left or
// Enter is pressed (a memo: only when it is left, Enter is a new line); only
// text that differs from the value is a change, and what code sets is never
// one. A bound input writes its field; an unbound named one keeps its text
// in its state. A password has an eye that shows what is typed or hides it
// again, and a button that copies it, named in Clarity's own words.

using SkiaSharp;

namespace Clarity.Ui;

/// <summary>The icons of the small buttons inside an input.</summary>
public enum InputIcon
{
    Eye,
    EyeShut,
    Copy,
    Up,
    Down,
}

/// <summary>A small flat button inside an input's right edge: a password's eye and copy, a spin box's arrows.</summary>
public sealed class InputSideButton(InputIcon icon, Action clicked) : Element
{
    public InputIcon Icon { get; set; } = icon;

    /// <summary>What it does, in the person's language.</summary>
    public string Name { get; set; } = "";

    public Func<bool> Enabled { get; set; } = () => true;

    /// <summary>It says what it does when the pointer rests on it.</summary>
    public override string? TooltipAt(double x, double y) => Name.Length > 0 ? Name : null;

    public override void OnClick()
    {
        if (Enabled())
            clicked();
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        if (Enabled() && (Hovered || Pressed))
            painter.Fill(Bounds, look.Color(Pressed ? "accent_soft" : "raised"), look.Radius("input"));
        var color = look.Color(Enabled() ? "text" : "muted");
        using var pen = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.6f, StrokeCap = SKStrokeCap.Round };
        var canvas = painter.Canvas;
        var cx = (float)(Bounds.X + Bounds.Width / 2);
        var cy = (float)(Bounds.Y + Bounds.Height / 2);
        switch (Icon)
        {
            case InputIcon.Eye:
            case InputIcon.EyeShut:
                canvas.DrawOval(new SKRect(cx - 8, cy - 5, cx + 8, cy + 5), pen);
                canvas.DrawCircle(cx, cy, 2.5f, pen);
                if (Icon == InputIcon.EyeShut)
                    canvas.DrawLine(cx - 7, cy - 7, cx + 7, cy + 7, pen);
                break;
            case InputIcon.Copy:
                canvas.DrawRoundRect(new SKRect(cx - 3, cy - 3, cx + 6, cy + 6), 1.5f, 1.5f, pen);
                canvas.DrawLine(cx - 6, cy + 3, cx - 6, cy - 6, pen);
                canvas.DrawLine(cx - 6, cy - 6, cx + 3, cy - 6, pen);
                break;
            case InputIcon.Up:
                canvas.DrawLine(cx - 4, cy + 2, cx, cy - 2, pen);
                canvas.DrawLine(cx, cy - 2, cx + 4, cy + 2, pen);
                break;
            case InputIcon.Down:
                canvas.DrawLine(cx - 4, cy - 2, cx, cy + 2, pen);
                canvas.DrawLine(cx, cy + 2, cx + 4, cy - 2, pen);
                break;
        }
    }
}

[PartKind("textbox", "memo", "password")]
public sealed class TextBoxPart : Part, ITakesTyped
{
    const double ButtonWidth = 28;

    TextEditor editor = null!;
    string own = "";
    string? shown;

    /// <summary>Where the person types.</summary>
    public TextEditor Editor => editor;

    /// <summary>A password's eye, which shows what is typed or hides it again.</summary>
    public InputSideButton? Eye { get; private set; }

    /// <summary>A password's copy button.</summary>
    public InputSideButton? Copy { get; private set; }


    /// <summary>The value it shows: its field, or the text it keeps unbound.</summary>
    public string Value => Bound is { } path ? ValueText(Host.Read(path)) : State?.Text ?? own;

    static string ValueText(object? value) => value is null ? "" : Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture) ?? "";

    protected internal override void Made()
    {
        editor = new TextEditor(multiline: Spec.Kind == "memo")
        {
            Placeholder = Spec.Text("placeholder") is { } placeholder ? Host.Text(placeholder) : null,
            Password = Spec.Kind == "password",
            Enabled = () => Enabled,
            ReadOnly = () => ReadOnly,
            Marked = () => State?.HasError == true,
            Entered = Enter,
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
        if (Spec.Kind == "password")
        {
            Eye = new InputSideButton(InputIcon.Eye, Reveal) { Enabled = () => Enabled };
            Copy = new InputSideButton(InputIcon.Copy, () => Screen?.Clipboard.SetText(editor.Text))
            {
                Name = Clarity.Runtime.Locale.Translate("copySecret", "Copy"),
                Enabled = () => Enabled,
            };
            Named();
            AddChild(Eye);
            AddChild(Copy);
            editor.RightInset = 2 * ButtonWidth + 2;
        }
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>The eye: the password shows as typed, or hides again.</summary>
    void Reveal()
    {
        editor.Password = !editor.Password;
        Named();
    }

    /// <summary>The eye says what it does next.</summary>
    void Named()
    {
        if (Eye is null)
            return;
        Eye.Icon = editor.Password ? InputIcon.Eye : InputIcon.EyeShut;
        Eye.Name = editor.Password ? Clarity.Runtime.Locale.Translate("showSecret", "Show") : Clarity.Runtime.Locale.Translate("hideSecret", "Hide");
    }

    /// <summary>A value set from elsewhere (code, the record read again) shows as it is.</summary>
    void Sync()
    {
        var now = Value;
        if (now == shown)
            return;
        shown = now;
        editor.Text = now;
    }

    /// <summary>Takes what is typed: a change when it differs from the value.</summary>
    public void Take()
    {
        Sync();
        var typed = editor.Text;
        if (typed == shown)
            return;
        shown = typed;
        if (Bound is { } path)
            Host.Write(Spec.Name ?? "", path, typed);
        else if (State is { } state)
        {
            state.Text = typed;
            Host.Edited(Spec.Name!);
        }
        else
            own = typed;
        Fire("change");
    }

    /// <summary>A key that runs a command has it take what is typed first.</summary>
    public void TakeTyped() => Take();

    /// <summary>Enter in one line: the text is taken, then the form it is in submits.</summary>
    bool Enter()
    {
        Take();
        return PartTexts.Submit(this);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        editor.Arrange(box);
        if (Eye is not null && Copy is not null)
        {
            var height = Math.Max(0, box.Height - 4);
            Eye.Arrange(new Box(box.Right - 2 - 2 * ButtonWidth, box.Y + 2, ButtonWidth, height));
            Copy.Arrange(new Box(box.Right - 2 - ButtonWidth, box.Y + 2, ButtonWidth, height));
        }
    }

    public override void Draw(Painter painter)
    {
        Sync();
        base.Draw(painter);
    }
}
