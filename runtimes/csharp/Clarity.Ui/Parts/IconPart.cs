// An icon: one of Clarity's Lucide icons, an SVG path in a 24 by 24 box,
// drawn with a round line in the text's color, scaled to 20 in the middle of
// its place. A generated C# application carries the icons it shows in its
// Icons class (`public const string TriangleAlert = "<svg path data>"`),
// which the host finds by the icon's name.

using SkiaSharp;

namespace Clarity.Ui;

[PartKind("icon")]
public sealed class IconPart : Part
{
    /// <summary>How big an icon part draws its icon.</summary>
    public const double IconSize = 20;

    /// <summary>The member of the generated Icons class that holds an icon: <c>triangle-alert</c> is <c>TriangleAlert</c>, as the generator names it.</summary>
    public static string Member(string name)
    {
        var member = new System.Text.StringBuilder();
        foreach (var part in System.Text.RegularExpressions.Regex.Split(name, @"[^\p{L}\p{N}]+"))
            if (part.Length > 0)
                member.Append(char.ToUpperInvariant(part[0])).Append(part[1..]);
        if (member.Length == 0 || char.IsAsciiDigit(member[0]))
            member.Insert(0, '_');
        return member.ToString();
    }

    /// <summary>The path of the icon it shows, as the host gives it; null when there is none.</summary>
    public string? PathOf() => Host.IconPath(Icon);

    /// <summary>The icon it shows, by its name.</summary>
    public string Icon => Spec.Text("icon") ?? "circle";

    /// <summary>Draws the icon of <paramref name="path"/> in <paramref name="box"/>, in <paramref name="color"/>; false when the path draws nothing.</summary>
    public static bool DrawIcon(Painter painter, string? path, Box box, SKColor color)
    {
        if (string.IsNullOrEmpty(path) || SKPath.ParseSvgPathData(path) is not { } shape)
            return false;
        using (shape)
        {
            var scale = (float)(Math.Min(box.Width, box.Height) / 24);
            using var paint = new SKPaint
            {
                Color = color,
                IsAntialias = true,
                Style = SKPaintStyle.Stroke,
                StrokeWidth = 2,
                StrokeCap = SKStrokeCap.Round,
                StrokeJoin = SKStrokeJoin.Round,
            };
            painter.Canvas.Save();
            painter.Canvas.Translate((float)(box.X + (box.Width - 24 * scale) / 2), (float)(box.Y + (box.Height - 24 * scale) / 2));
            painter.Canvas.Scale(scale);
            painter.Canvas.DrawPath(shape, paint);
            painter.Canvas.Restore();
        }
        return true;
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var box = new Box(Bounds.X + (Bounds.Width - IconSize) / 2, Bounds.Y + (Bounds.Height - IconSize) / 2, IconSize, IconSize);
        var color = painter.Look.Color(Enabled ? "text" : "muted");
        if (!DrawIcon(painter, PathOf(), box, color))
            painter.Stroke(box.Inset(2), painter.Look.Color("border"), 1, IconSize / 2);
    }
}
