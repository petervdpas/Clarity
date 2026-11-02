// The parts that show text and do not edit it: a label (with the required
// mark of its input), a heading, a help text (its input's error in the
// error color, else its help), a text (a bound value, on one line or wrapped
// over several rows), a separator and a badge. Also what the text and number
// parts share: their caption, the field they bind as the host knows it, and
// a value as the person reads it.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>A part that takes Enter in its inputs (a form): it runs its submit, true when it has one.</summary>
public interface ISubmits
{
    bool Submit();
}

/// <summary>What the text parts share.</summary>
public static class PartTexts
{
    /// <summary>
    /// A part's caption as shown: as code set it in its state, else as
    /// written, translated; null when it has none (the web's rule).
    /// </summary>
    public static string? Caption(this Part part)
    {
        if (part.State is { } state && (state.CaptionText.Key is { Length: > 0 } || state.CaptionText.Template.Length > 0))
            return state.Caption;
        return part.Spec.Text("caption") is { } written ? part.Host.Text(written) : null;
    }

    public static Clarity.Runtime.Formats FormatsOf(this Part part) => part.Host.Formats;

    /// <summary>The format of the value at <paramref name="path"/>: as the host knows it, else by its type.</summary>
    public static FieldFormat? FormatOf(this Part part, string path)
    {
        if (part.Host.Field(path)?.Format is { } known)
            return known;
        var type = part.Host.TypeOf(path);
        type = type is null ? null : Nullable.GetUnderlyingType(type) ?? type;
        return type == typeof(long) || type == typeof(int) ? new FieldFormat(FormatKind.Integer)
            : type == typeof(decimal) || type == typeof(double) ? new FieldFormat(FormatKind.Decimal)
            : type == typeof(DateOnly) ? new FieldFormat(FormatKind.Date)
            : type == typeof(DateTime) ? new FieldFormat(FormatKind.DateTime)
            : null;
    }

    /// <summary>
    /// A bound value as the person reads it: yes or no, an enum by its
    /// choice's label, numbers and dates in the person's format, the rest as it is.
    /// </summary>
    public static string Shown(this Part part, string path, object? value)
    {
        switch (value)
        {
            case null:
                return "";
            case bool b:
                return b ? Locale.Translate("yes", "Yes") : Locale.Translate("no", "No");
            case Enum:
                return part.Host.Choices(path).FirstOrDefault(c => Equals(c.Value, value))?.Label ?? value.ToString() ?? "";
            // A list field shows each item as one is shown, joined (Avalonia's Bindings).
            case System.Collections.IList items:
                return string.Join(", ", items.Cast<object?>().Select(item => part.Shown(path, item)));
        }
        if (part.FormatOf(path) is { } format && value is not "")
            return part.FormatsOf().Show(format, value);
        if (part.Host.Choices(path) is { Count: > 0 } choices && choices.FirstOrDefault(c => Equals(c.Value, value)) is { } choice)
            return choice.Label;
        return Convert.ToString(value, CultureInfo.InvariantCulture) ?? "";
    }

    /// <summary>The part named <paramref name="name"/> in the same window as <paramref name="from"/>.</summary>
    public static Part? Named(Element from, string? name)
    {
        if (string.IsNullOrEmpty(name))
            return null;
        var top = from;
        while (top.Parent is { } parent)
            top = parent;
        return Find(top, name);
    }

    static Part? Find(Element at, string name)
    {
        if (at is Part part && part.Spec.Name == name)
            return part;
        foreach (var child in at.Children)
            if (Find(child, name) is { } found)
                return found;
        return null;
    }

    /// <summary>Enter in an input: the form it is in runs its submit; false when it is in none, or that has none.</summary>
    public static bool Submit(Element from)
    {
        for (var e = from.Parent; e is not null; e = e.Parent)
        {
            if (e is ISubmits submits)
                return submits.Submit();
            if (e is Part { Spec.Kind: "form" } form)
            {
                var calls = form.Spec.On("submit").ToList();
                foreach (var call in calls)
                    form.Host.Run(form.Spec.Name, call);
                return calls.Count > 0;
            }
        }
        return false;
    }

    /// <summary>A line cut off with an ellipsis where it is wider than <paramref name="width"/>.</summary>
    public static string Fit(Painter painter, string text, double width, double size, string role = "text", bool bold = false)
    {
        if (text.Length == 0 || painter.Fonts.Measure(text, role, size, bold) <= width)
            return text;
        const string dots = "\u2026";
        var low = 0;
        var high = text.Length;
        while (low < high)
        {
            var mid = (low + high + 1) / 2;
            if (painter.Fonts.Measure(text[..mid] + dots, role, size, bold) <= width)
                low = mid;
            else
                high = mid - 1;
        }
        return text[..low].TrimEnd() + dots;
    }

    /// <summary>The input part a label or help text belongs to, by its <c>for</c>.</summary>
    internal static Part? Target(this Part part) => Named(part, part.Spec.Text("for"));
}

/// <summary>A label: its caption in the muted color, with a mark when its input's field is required.</summary>
[PartKind("label")]
public sealed class LabelPart : Part
{
    /// <summary>Whether its input's field is required.</summary>
    public bool Required => this.Target() is { Bound: { } path } && Host.Field(path)?.Required == true;

    public string Text => this.Caption() ?? "";

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var size = look.Size("text");
        var mark = Required ? " *" : "";
        var markWidth = mark.Length == 0 ? 0 : painter.Fonts.Measure(mark, "text", size, false);
        var shown = PartTexts.Fit(painter, Text, Bounds.Width - markWidth, size);
        painter.Text(shown, Bounds, look.Color("muted"), size);
        if (mark.Length > 0)
        {
            var x = Bounds.X + painter.Fonts.Measure(shown, "text", size, false);
            painter.Text(mark, Bounds with { X = x, Width = markWidth + 1 }, look.Color("error"), size);
        }
    }
}

/// <summary>A heading: its caption in the theme's heading font and size.</summary>
[PartKind("heading")]
public sealed class HeadingPart : Part
{
    public string Text => this.Caption() ?? "";

    public override void Draw(Painter painter)
    {
        var size = painter.Look.Size("heading");
        painter.Text(PartTexts.Fit(painter, Text, Bounds.Width, size, "heading", true), Bounds, painter.Look.Color("text"), size, "heading", bold: true);
    }
}

/// <summary>A help text under an input: the input's error in the error color, else its own caption, else the input's help.</summary>
[PartKind("help")]
public sealed class HelpPart : Part
{
    /// <summary>Whether it shows its input's error.</summary>
    public bool ShowsError => Host.State(Spec.Text("for")) is { HasError: true } target && target.Error.Length > 0;

    public string Text
    {
        get
        {
            var target = Host.State(Spec.Text("for"));
            if (ShowsError)
                return target!.Error;
            return this.Caption() ?? target?.Help ?? "";
        }
    }

    public override void Draw(Painter painter)
    {
        var size = painter.Look.Size("small");
        painter.Text(PartTexts.Fit(painter, Text, Bounds.Width, size), Bounds, painter.Look.Color(ShowsError ? "error" : "muted"), size);
    }
}

/// <summary>
/// A value to read: what it binds, as the person reads it, on one line cut
/// off with an ellipsis; over several rows wrapped from the top, keeping
/// its line breaks, and scrolling when it is longer than the rows.
/// </summary>
[PartKind("text")]
public sealed class TextPart : Part
{
    TextEditor? lines;

    public string Text => Bound is { } path ? this.Shown(path, Host.Read(path)) : "";

    /// <summary>Over several rows: the wrapped lines, which scroll.</summary>
    public TextEditor? Lines => lines;

    protected internal override void Made()
    {
        if ((Spec.Number("height") ?? 1) <= 1)
            return;
        lines = new TextEditor(multiline: true) { Framed = false, TakesFocus = false, ReadOnly = () => true, Text = Text };
        AddChild(lines);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        if (lines is not null)
        {
            lines.Text = Text;
            lines.Arrange(box);
        }
    }

    public override void Draw(Painter painter)
    {
        if (lines is not null)
        {
            lines.Text = Text;
            lines.Draw(painter);
            return;
        }
        var size = painter.Look.Size("text");
        painter.Text(PartTexts.Fit(painter, Text, Bounds.Width, size), Bounds, painter.Look.Color("text"), size);
    }
}

/// <summary>A line between parts: across the middle of its place, or down it when vertical.</summary>
[PartKind("separator")]
public sealed class SeparatorPart : Part
{
    public bool Vertical => Spec.Text("orientation") == "vertical";

    public override void Draw(Painter painter)
    {
        var width = Math.Max(1, painter.Look.Border);
        var line = Vertical
            ? new Box(Math.Round(Bounds.X + (Bounds.Width - width) / 2), Bounds.Y, width, Bounds.Height)
            : new Box(Bounds.X, Math.Round(Bounds.Y + (Bounds.Height - width) / 2), Bounds.Width, width);
        painter.Fill(line, painter.Look.Color("strong_border"));
    }
}

/// <summary>A short text or number in a pill: what it binds, or its caption; in the accent color unless its style says otherwise.</summary>
[PartKind("badge")]
public sealed class BadgePart : Part
{
    /// <summary>Its style colors its pill.</summary>
    protected override bool DrawsOwnStyle => true;

    public string Text => Bound is { } path ? this.Shown(path, Host.Read(path)) : this.Caption() ?? "";

    /// <summary>Where the pill is drawn: as wide as its text, in the middle of its place's height.</summary>
    public Box Pill(Painter painter)
    {
        var size = painter.Look.Size("small");
        var height = Math.Min(Bounds.Height, Math.Ceiling(size * 1.5) + 2);
        var width = Math.Min(Bounds.Width, painter.Fonts.Measure(Text, "text", size, false) + 16);
        return new Box(Bounds.X, Math.Round(Bounds.Y + (Bounds.Height - height) / 2), width, height);
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var style = Spec.Text("style") is { } name ? look.Style(name) : LookValues.None;
        var pill = Pill(painter);
        var size = look.Size("small");
        painter.Fill(pill, style.Background ?? look.Color("accent"), pill.Height / 2);
        if (style.Border is { } border)
            painter.Stroke(pill, border, look.Border, pill.Height / 2);
        var inside = pill with { X = pill.X + 8, Width = Math.Max(0, pill.Width - 16) };
        painter.Text(PartTexts.Fit(painter, Text, inside.Width, size), inside, style.Text ?? look.Color("on_accent"), size);
    }
}
