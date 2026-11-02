// A question a handler asks: ask ... then handler(values) (DESIGN 5.14),
// in Clarity's own dialog over the window, never a dialog of the system:
// the question, the input its form needs (a text box, a number box in the
// value's format, or a dropdown of its choices) and OK and Cancel. OK or
// Enter answers, and the window runs the handler the answer goes to;
// Cancel or Escape runs nothing.

using System.Xml.Linq;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class AskDialog : Element, IDialogKeys
{
    const double Width = 440;
    const double Line = 20;
    const double ButtonWidth = 104;

    readonly AskHost host;

    public AskDialog(Question question)
    {
        Question = question;
        var (ok, cancel) = question.Form is ConfirmForm confirm
            ? (confirm.Yes is { } yes ? Locale.Show(yes) : Locale.Translate("ok", "OK"), confirm.No is { } no ? Locale.Show(no) : Locale.Translate("cancel", "Cancel"))
            : (Locale.Translate("ok", "OK"), Locale.Translate("cancel", "Cancel"));
        host = new AskHost(question.Form as ValueForm);
        if (question.Form is ValueForm form)
        {
            Input = PartKinds.Make(WindowMarkup.PartOf(InputMarkup(form)), host);
            AddChild(Input);
        }
        Ok = new DialogButton(ok, accent: true, () => Answer(Value()));
        Cancel = new DialogButton(cancel, accent: false, () => Cancelled?.Invoke());
        AddChild(Cancel);
        AddChild(Ok);
    }

    public Question Question { get; }

    /// <summary>The input of a question for a value; none for yes or no.</summary>
    public Part? Input { get; }

    public DialogButton Ok { get; }
    public DialogButton Cancel { get; }

    /// <summary>The person answered: null for a yes, else the value.</summary>
    public Action<object?>? Answered { get; set; }

    /// <summary>The person cancelled: Cancel or Escape.</summary>
    public Action? Cancelled { get; set; }

    /// <summary>Where the dialog itself is, in the middle of the window.</summary>
    public Box Panel { get; private set; }

    /// <summary>What the person focuses first: the input, else OK.</summary>
    public Element First => Input?.InOrder().FirstOrDefault(e => e.Focusable) ?? Ok;

    /// <summary>The part of a value's form, bound to the answer.</summary>
    static XElement InputMarkup(ValueForm form)
    {
        var kind = form.Choices is not null
            ? "Dropdown"
            : form.Format?.Kind is FormatKind.Integer or FormatKind.Decimal or FormatKind.Money or FormatKind.Duration ? "Number" : "Textbox";
        return new XElement(kind, new XAttribute("name", "answer"), new XAttribute("bind", AskHost.Path), new XAttribute("at", "1,1"), new XAttribute("span", "12"));
    }

    /// <summary>What the input holds now, what is typed in it included; null for a yes.</summary>
    public object? Value()
    {
        switch (Input)
        {
            case TextBoxPart text:
                text.Take();
                break;
            case NumberPart number:
                number.Take();
                break;
        }
        return Input is null ? null : host.Read(AskHost.Path);
    }

    /// <summary>Answers with <paramref name="value"/>, as OK does.</summary>
    public void Answer(object? value) => Answered?.Invoke(value);

    /// <summary>Enter answers (a dropdown's open list has it first), Escape cancels.</summary>
    public bool KeyPressed(KeyInput key)
    {
        if (key.Modifiers != Modifiers.None)
            return false;
        if (key.Key == Key.Enter)
        {
            Answer(Value());
            return true;
        }
        if (key.Key == Key.Escape)
        {
            Cancelled?.Invoke();
            return true;
        }
        return false;
    }

    IReadOnlyList<string> Lines(double width) =>
        Screen is { } screen ? SaidDialog.Wrap(Locale.Show(Question.Text), width, screen.Fonts, screen.Look.Size("text")) : [Locale.Show(Question.Text)];

    public override void Arrange(Box box)
    {
        Bounds = box;
        var width = Math.Min(Width, Math.Max(200, box.Width - 32));
        var inner = width - 40;
        var lines = Lines(inner).Count;
        var height = 20 + lines * Line + 16 + (Input is null ? 0 : GridLayout.RowHeight + 16) + GridLayout.RowHeight + 20;
        Panel = new Box(box.X + (box.Width - width) / 2, box.Y + Math.Max(16, (box.Height - height) / 2), width, height);
        var y = Panel.Y + 20 + lines * Line + 16;
        if (Input is not null)
        {
            Input.Arrange(new Box(Panel.X + 20, y, inner, GridLayout.RowHeight));
            y += GridLayout.RowHeight + 16;
        }
        Ok.Arrange(new Box(Panel.Right - 20 - ButtonWidth, y, ButtonWidth, GridLayout.RowHeight));
        Cancel.Arrange(new Box(Ok.Bounds.X - 8 - ButtonWidth, y, ButtonWidth, GridLayout.RowHeight));
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        Dialogs.Backdrop(painter, Bounds, Panel);
        var size = look.Size("text");
        var lines = Lines(Panel.Width - 40);
        for (var i = 0; i < lines.Count; i++)
            painter.Text(lines[i], new Box(Panel.X + 20, Panel.Y + 20 + i * Line, Panel.Width - 40, Line), look.Color("text"), size);
        base.Draw(painter);
    }

    /// <summary>The answer's one value, which the input reads and writes; its format and choices are the question's.</summary>
    sealed class AskHost(ValueForm? form) : IPartHost
    {
        public const string Path = "answer";

        object? value = form?.Default ?? form?.Choices?.FirstOrDefault()?.Value;

        public object? Read(string path) => path == Path ? value : null;

        public IReadOnlyList<object> Rows(string path) => [];

        public void Write(string part, string path, object? written)
        {
            if (path != Path)
                return;
            value = written;
            Changed?.Invoke();
        }

        public PartState? State(string? name) => null;

        public bool Run(string? part, HandlerCall call) => true;

        public void Edited(string part)
        {
        }

        public string Text(string? written) => written ?? "";

        public Type? TypeOf(string path) => value?.GetType();

        public IReadOnlyList<Choice> Choices(string path) => path == Path ? form?.Choices ?? [] : [];

        public FieldFacts? Field(string path) => path == Path && form?.Format is { } format ? new FieldFacts("", format) : null;

        public Formats Formats => Locale.Formats;

        public void OpenAddress(string address)
        {
        }

        public byte[]? Resource(string name) => null;

        public string? IconPath(string name) => null;

        public event Action? Changed;

        public IFilePicker Files { get; } = new NoFilePicker();

        public Task<byte[]?> Fetch(string bind, string? storePath) => Task.FromResult<byte[]?>(null);

        public void Say(Level level, Clarity.Runtime.Text text)
        {
        }

        public bool? Pointed(string part, string @event, Clarity.Runtime.Pointer at) => null;

        public bool? Acted(string part, System.Text.Json.Nodes.JsonObject action) => null;

        public Func<IFrames>? Engine(string name) => null;

        public void Told(string part, string tell, System.Text.Json.JsonElement[] values)
        {
        }

        public IReadOnlyList<string> KeysOf(string command, string? designed) => designed is null ? [] : [designed];
    }
}

/// <summary>A dialog's button: OK in the accent's color, others in the button's; it has the focus by Tab, and Enter or Space presses it.</summary>
public sealed class DialogButton(string text, bool accent, Action pressed) : Element
{
    public string Text { get; } = text;

    public override bool Focusable => true;

    public override void OnClick() => pressed();

    public override bool OnKey(KeyInput key)
    {
        if (key.Modifiers != Modifiers.None || key.Key is not (Key.Enter or Key.Space))
            return false;
        pressed();
        return true;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var values = look.Part("button", Pressed ? "pressed" : Hovered ? "hover" : null);
        var radius = values.Radius ?? look.Radius("button");
        var fill = accent ? look.Color("accent") : values.Background ?? look.Color("panel");
        painter.Fill(Bounds, fill, radius);
        painter.Stroke(Bounds, values.Border ?? look.Color("border"), look.Border, radius);
        if (Focused)
            painter.Stroke(Bounds.Inset(-2), look.Color("focus"), 2, radius + 2);
        var color = accent ? look.Color("on_accent") : values.Text ?? look.Color("text");
        painter.Text(Text, Bounds, color, values.Size ?? look.Size("text"), "text", values.Bold, TextAlign.Center);
    }
}


/// <summary>What Clarity's own dialogs share: their keys, and their dimmed window and panel.</summary>
public static class Dialogs
{
    /// <summary>Opens <paramref name="dialog"/> over the whole screen as a modal overlay; <paramref name="closed"/> runs when it closes.</summary>
    public static Overlay Open(Screen screen, Element dialog, Element? first = null, Action? closed = null)
    {
        var overlay = new Overlay(dialog, screen.Whole) { Modal = true, Covers = true, Closed = closed };
        screen.Open(overlay);
        screen.Focus(first);
        return overlay;
    }

    /// <summary>The window under a dialog dimmed (it takes nothing while the dialog is open) and the dialog's panel.</summary>
    public static void Backdrop(Painter painter, Box window, Box panel)
    {
        var look = painter.Look;
        painter.Fill(window, look.Color("text").WithAlpha(56));
        var radius = look.Radius("panel");
        painter.Fill(panel, look.Color("panel"), radius);
        painter.Stroke(panel, look.Color("strong_border"), look.Border, radius);
    }
}
