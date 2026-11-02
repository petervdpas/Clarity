// The link (DESIGN 7.9, conformance/small-parts.json): underlined text in the
// accent color. It opens its address (`to`), or the address its bound field
// holds (an email address to write to, one with a scheme as it is), or runs
// its click when it has neither. The host opens an address in the
// system's program for it (a browser, a mail program). As the shared screen does it
// (runtimes/web/src/kinds/Link.svelte).

using System.Globalization;
using SkiaSharp;

namespace Clarity.Ui;

[PartKind("link")]
public sealed class LinkPart : Part
{
    public override bool Focusable => Enabled;

    /// <summary>What a link bound to a value opens: an email address to write to, an address with a scheme as it is, else none.</summary>
    public static string? Address(object? value)
    {
        var text = value is string s ? s.Trim() : "";
        if (text.Length == 0)
            return null;
        if (text.Contains('@') && !text.Contains(':'))
            return $"mailto:{text}";
        return HasScheme(text) ? text : null;
    }

    static bool HasScheme(string text)
    {
        var colon = text.IndexOf(':');
        if (colon < 1 || !char.IsAsciiLetter(text[0]))
            return false;
        for (var i = 1; i < colon; i++)
            if (!(char.IsAsciiLetterOrDigit(text[i]) || text[i] is '+' or '.' or '-'))
                return false;
        return true;
    }

    /// <summary>The text it shows: its bound value, else its caption.</summary>
    public string Shown => Bound is { } bound
        ? Convert.ToString(Host.Read(bound), CultureInfo.CurrentCulture) ?? ""
        : State?.Caption is { Length: > 0 } set ? set : Host.Text(Spec.Text("caption"));

    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    public override void OnClick() => Follow();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key != Key.Enter)
            return false;
        Follow();
        return true;
    }

    /// <summary>Opens its address, or the one its bound value holds; runs its click when it has neither.</summary>
    public void Follow()
    {
        if (!Enabled)
            return;
        var to = Spec.Text("to");
        var address = to ?? (Bound is { } bound ? Address(Host.Read(bound)) : null);
        if (address is not null)
            Host.OpenAddress(address);
        else if (to is null && Bound is null)
            Fire("click");
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var color = Enabled ? look.Color("accent") : look.Color("muted");
        var size = look.Size("text");
        var text = Shown;
        painter.Text(text, Bounds, color, size);
        var width = Math.Min(painter.Fonts.Measure(text, "text", size, false), Bounds.Width);
        var y = (float)(Bounds.Y + Bounds.Height / 2 + size * 0.55);
        using var paint = new SKPaint { Color = color, StrokeWidth = 1, IsAntialias = true };
        if (width > 0)
            painter.Canvas.DrawLine((float)Bounds.X, y, (float)(Bounds.X + width), y, paint);
        if (Focused)
            PressLooks.FocusRing(painter, new Box(Bounds.X - 2, Bounds.Y + Bounds.Height / 2 - size, width + 4, size * 2), 3);
    }
}
