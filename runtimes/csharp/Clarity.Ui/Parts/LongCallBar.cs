// A long engine call (DESIGN 5.12): while a handler's engine call runs, the
// window keeps drawing and shows how far the call is, as a note at the
// bottom right, clear of the messages' notes at the top: a bar, how much is
// done, and Stop, which ends the call at its next look and then greys, as it
// can stop it only once. It lies over the
// whole window, so nothing under it is pressed while the call runs; the
// window takes no other handler then anyway (LongCall.Waiting).

using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class LongCallBar : Element, IDialogKeys
{
    const double Width = 360;
    const double Height = 64;
    const double ButtonWidth = 88;

    readonly Func<(long Done, long Total)?> now;

    /// <summary>A bar that shows how far <paramref name="now"/> says the call is; Stop runs <paramref name="stop"/>.</summary>
    public LongCallBar(Func<(long Done, long Total)?> now, Action stop)
    {
        this.now = now;
        Stop = new DialogButton(Locale.Translate("stopCall", "Stop"), accent: false, () =>
        {
            Stopped = true;
            stop();
            Invalidate();
        })
        { Usable = () => !Stopped };
        AddChild(Stop);
    }

    /// <summary>The bar of the windows' long call <paramref name="call"/>, which Stop stops.</summary>
    public static LongCallBar Of(LongCall call) => new(() => call.Current, call.Stop);

    public DialogButton Stop { get; }

    /// <summary>Whether the person pressed Stop: it is then greyed and stops nothing more.</summary>
    public bool Stopped { get; private set; }

    /// <summary>Where the note is, at the bottom right of the window.</summary>
    public Box Note { get; private set; }

    /// <summary>How much is done, 0 to 1; 0 while the call says nothing.</summary>
    public double Fraction => now() is { Total: > 0 } at ? Math.Clamp((double)at.Done / at.Total, 0, 1) : 0;

    /// <summary>How much is done, as it shows: <c>25%</c>; nothing while the call says nothing.</summary>
    public string Said => now() is { Total: > 0 } ? $"{Math.Round(Fraction * 100)}%" : "";

    /// <summary>Escape stops, as Stop does.</summary>
    public bool KeyPressed(KeyInput key)
    {
        if (key.Modifiers != Modifiers.None || key.Key != Key.Escape)
            return false;
        Stop.Press();
        return true;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var pad = WindowView.ContentPadding;
        var width = Math.Min(Width, Math.Max(160, box.Width - 2 * pad));
        // At the bottom right, so the window's info notes at the top right stay readable while a call runs.
        Note = new Box(box.Right - pad - width, box.Bottom - pad - Height, width, Height);
        Stop.Arrange(new Box(Note.Right - 12 - ButtonWidth, Note.Y + (Height - GridLayout.RowHeight) / 2, ButtonWidth, GridLayout.RowHeight));
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var radius = look.Radius("panel");
        painter.Fill(Note, look.Color("raised"), radius);
        painter.Stroke(Note, look.Color("border"), look.Border, radius);
        painter.Fill(new Box(Note.X, Note.Y, 4, Note.Height), look.Color("accent"), radius);
        var track = new Box(Note.X + 16, Note.Y + Height / 2 - 4, Stop.Bounds.X - 16 - 56 - Note.X - 16, 8);
        painter.Fill(track, look.Color("border"), 4);
        if (Fraction > 0)
            painter.Fill(new Box(track.X, track.Y, track.Width * Fraction, track.Height), look.Color("accent"), 4);
        painter.Text(Said, new Box(track.Right + 8, Note.Y, 48, Height), look.Color("muted"), look.Size("small"));
        base.Draw(painter);
    }
}
