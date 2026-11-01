// The sample's screen, the same on the desktop and in a browser page.

using Clarity.Ui.Drawing;

namespace Clarity.Ui.Samples;

public static class Hello
{
    public static Screen Make(bool dark = false)
    {
        var grid = new GridPanel(columns: 4);
        grid.Add(new TextLine("Clarity.UI") { Bold = true, SizeRole = "heading", FontRole = "heading" }, new GridItem(1, 1, 4, 1, false));
        var count = new TextLine("Not pressed yet") { Color = "muted" };
        var presses = 0;
        grid.Add(new PressButton("Press me", () => count.Text = $"Pressed {++presses}x"), new GridItem(1, 2, 1, 1, false));
        grid.Add(count, new GridItem(2, 2, 3, 1, false));
        grid.Add(new Card(), new GridItem(1, 3, 4, 3, true));
        return new Screen(grid, ThemeLook.Clarity(dark));
    }
}

/// <summary>A button in the theme's button look; the parts themselves come with G7c.</summary>
sealed class PressButton(string caption, Action pressed) : Element
{
    public override bool Focusable => true;

    public override void OnClick() => pressed();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Enter or Key.Space))
            return false;
        pressed();
        return true;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look.Part("button", Pressed ? "pressed" : Hovered ? "hover" : null);
        var radius = look.Radius ?? painter.Look.Radius("button");
        painter.Fill(Bounds, look.Background ?? painter.Look.Color("raised"), radius);
        if (Focused)
            painter.Stroke(Bounds, painter.Look.Color("focus"), 2, radius);
        else
            painter.Stroke(Bounds, look.Border ?? painter.Look.Color("border"), painter.Look.Border, radius);
        painter.Text(caption, Bounds, look.Text ?? painter.Look.Color("text"), painter.Look.Size("text"), align: TextAlign.Center);
    }
}

/// <summary>A panel in the theme's panel color, with its border.</summary>
sealed class Card : Element
{
    public override void Draw(Painter painter)
    {
        var radius = painter.Look.Radius("panel");
        painter.Fill(Bounds, painter.Look.Color("panel"), radius);
        painter.Stroke(Bounds, painter.Look.Color("border"), painter.Look.Border, radius);
        painter.Text("This panel grows with the window.", Bounds.Inset(12) with { Height = 36 }, painter.Look.Color("muted"), painter.Look.Size("text"));
    }
}
