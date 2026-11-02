// A canvas an engine beside the screen draws (DESIGN 5.12,
// conformance/screen-engines.json): one instance from when it first shows,
// a frame at the screen's pace (FrameLoop), the frame's work off the
// screen's thread and its shapes shown on the next frame the screen draws.
// While it runs it asks the screen for every frame (Screen.Animate). When
// the engine stops, it says why in its place. Its pointer and keys go to the
// engine, when it takes them: it keeps the pointer while it is down (at the
// drawing's nearest edge beside it), and takes the keyboard when pressed or
// tabbed to. What handlers tell it through its state (hears) goes to the
// engine; what the engine tells runs the window's handler (IPartHost.Told).
// The canvas part it fills (CanvasPart) makes it for a canvas that `runs`
// an engine.

using System.Collections.Concurrent;
using System.Diagnostics;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class EngineCanvasPart : Part
{
    FrameLoop? loop;
    bool started;
    Action<TimeSpan>? next;
    readonly ConcurrentQueue<Action> onScreen = new();
    readonly Stopwatch watch = Stopwatch.StartNew();
    bool pointerDown;
    readonly HeldKeys held = new();

    /// <summary>The canvas it fills, whose units it draws in.</summary>
    public CanvasPart Canvas { get; internal set; } = null!;

    /// <summary>The engine that draws it: a new instance each call; none when the window has none.</summary>
    public Func<IFrames>? Runs { get; internal set; }

    /// <summary>The screen's clock, which gives each frame its time.</summary>
    public Func<TimeSpan> Clock { get; set; }

    /// <summary>Where a frame's work runs: off the screen's thread.</summary>
    public Action<Action> Background { get; set; } = work => Task.Run(work);

    public EngineCanvasPart() => Clock = () => watch.Elapsed;

    /// <summary>The shapes of its last frame, in its units.</summary>
    public IReadOnlyList<Shape> Drawing { get; private set; } = [];

    /// <summary>Why its engine stopped, which it shows in place of its drawing; none while it runs.</summary>
    public string? Stopped { get; private set; }

    /// <summary>Why its engine stopped, in Clarity's words in the person's language; none while it runs.</summary>
    public string? StoppedText => Stopped is { } why
        ? Locale.Show(new Text("The engine stopped: {problem}") { Key = "screenEngineFailed" }.With("problem", Slot.OfText(why)))
        : null;

    /// <summary>The pointer and keys its engine takes; none before it runs.</summary>
    Hears Hears => loop?.Hears ?? default;

    bool Pointing => Hears.Pointer && Stopped is null;

    public override bool Focusable => Hears.Keys && Stopped is null;

    /// <summary>Makes its engine and asks for the first frame; one that cannot be made says why.</summary>
    void Start()
    {
        started = true;
        if (Runs is not { } make)
        {
            Stopped = Locale.Translate("screenEngineMissing", "The window has no engine of this name.");
            return;
        }
        try
        {
            loop = new FrameLoop(make(), tick => next = tick, Background, onScreen.Enqueue, shapes => Drawing = shapes, Stop, Tell);
            loop.Start();
        }
        catch (EngineException failed)
        {
            Stop(failed.Message);
            return;
        }
        // What handlers told before it showed comes first.
        State?.Listen((_, _) => { }, (hear, values) => loop?.Hear(hear, values));
    }

    void Stop(string why)
    {
        Stopped = why;
        next = null;
        State?.StopListening();
        Screen?.Animate(this, false);
        Invalidate();
    }

    void Tell(ScreenTell tell)
    {
        if (Spec.Name is { } name)
            Host.Told(name, tell.Tell, tell.Values);
    }

    /// <summary>Runs what the engine's frames left for the screen: their shapes, what they told, a failure.</summary>
    void Settle()
    {
        while (onScreen.TryDequeue(out var then))
            then();
    }

    /// <summary>Ends its engine: the canvas left its screen. One that joins a screen again starts a new engine (OnAttached).</summary>
    public void End()
    {
        loop?.Dispose();
        loop = null;
        next = null;
        held.All();
        pointerDown = false;
        State?.StopListening();
        Screen?.Animate(this, false);
    }

    /// <summary>It joined a screen: its engine starts with the next frame drawn, a new instance each time.</summary>
    protected override void OnAttached()
    {
        started = false;
        Stopped = null;
    }

    /// <summary>It left its screen (its window closed or went under another): its engine ends.</summary>
    protected override void OnDetached() => End();

    public override void Draw(Painter painter)
    {
        if (!started)
            Start();
        Settle();
        // A frame at the screen's pace: the loop asked for this one.
        if (next is { } tick)
        {
            next = null;
            tick(Clock());
            Settle();
        }
        if (Stopped is null && loop is not null)
            Screen?.Animate(this);
        using (painter.Clip(Bounds))
        {
            if (StoppedText is { } said)
            {
                painter.Text(said, Bounds.Inset(8), painter.Look.Color("muted"), painter.Look.Size("text"), align: TextAlign.Center);
                return;
            }
            var v = CanvasPart.Whole(Canvas.UnitsWidth, Canvas.UnitsHeight, Bounds.Width, Bounds.Height);
            CanvasPart.Paint(painter, Bounds, Drawing, v, Canvas.UnitsWidth, Canvas.UnitsHeight, Canvas.Paper, false, _ => null);
        }
    }

    /// <summary>Where a pixel of the part is for the engine: on the drawing, or at its nearest edge while the pointer is down.</summary>
    Pointer? EnginePoint(double x, double y, bool near) => near
        ? CanvasPart.PointNear(Canvas.UnitsWidth, Canvas.UnitsHeight, Bounds.Width, Bounds.Height, x, y)
        : CanvasPart.PointOf(Canvas.UnitsWidth, Canvas.UnitsHeight, Bounds.Width, Bounds.Height, x, y);

    public override void OnPointerPressed(PointerInput at)
    {
        if (!Pointing || !Enabled || EnginePoint(at.X, at.Y, false) is not { } point)
            return;
        pointerDown = true;
        loop!.Pointer("down", point.X, point.Y);
    }

    public override void OnPointerMoved(PointerInput at)
    {
        if (Pointing && EnginePoint(at.X, at.Y, pointerDown) is { } point)
            loop!.Pointer("move", point.X, point.Y);
    }

    public override void OnPointerReleased(PointerInput at)
    {
        if (!pointerDown)
            return;
        pointerDown = false;
        if (Pointing && EnginePoint(at.X, at.Y, true) is { } point)
            loop!.Pointer("up", point.X, point.Y);
    }

    /// <summary>A key goes down once, however long it repeats; a shortcut with Control, Alt or the command key is the window's, but those keys themselves are held keys.</summary>
    public override bool OnKey(KeyInput key)
    {
        if (!Hears.Keys || Stopped is not null || loop is null || ScreenKeys.NameOf(key.Key) is not { } name)
            return false;
        var modifier = key.Key is Key.Shift or Key.Control or Key.Alt;
        if (!modifier && (key.Modifiers & (Modifiers.Control | Modifiers.Alt | Modifiers.Meta)) != 0)
            return false;
        if (held.Down(name))
            loop.Key("down", name);
        return true;
    }

    /// <summary>A key let go goes up, when it went down here.</summary>
    public override void OnKeyUp(KeyInput key)
    {
        if (loop is not null && ScreenKeys.NameOf(key.Key) is { } name && held.Up(name))
            loop.Key("up", name);
    }

    /// <summary>Losing the focus lets go of every key still down.</summary>
    public override void OnFocusChanged(bool focused)
    {
        if (focused)
            return;
        foreach (var name in held.All())
            loop?.Key("up", name);
    }
}

/// <summary>
/// The keys an engine beside the screen gets, by Clarity's names, as the web
/// renderer names them too (conformance/screen-engines.json, keys): a letter
/// as itself in lower case, a digit, and the named keys.
/// </summary>
public static class ScreenKeys
{
    /// <summary>Clarity's name of <paramref name="key"/>; null for one an engine does not get.</summary>
    public static string? NameOf(Key key) => key switch
    {
        >= Key.A and <= Key.Z => ((char)('a' + (key - Key.A))).ToString(),
        >= Key.D0 and <= Key.D9 => ((char)('0' + (key - Key.D0))).ToString(),
        >= Key.F1 and <= Key.F12 => "F" + (key - Key.F1 + 1),
        Key.Space => "Space",
        Key.Enter => "Enter",
        Key.Escape => "Escape",
        Key.Backspace => "Backspace",
        Key.Delete => "Delete",
        Key.Left => "ArrowLeft",
        Key.Right => "ArrowRight",
        Key.Up => "ArrowUp",
        Key.Down => "ArrowDown",
        Key.Home => "Home",
        Key.End => "End",
        Key.PageUp => "PageUp",
        Key.PageDown => "PageDown",
        Key.Shift => "Shift",
        Key.Control => "Control",
        Key.Alt => "Alt",
        _ => null,
    };
}

/// <summary>The keys that are down, so each goes down once however long it repeats.</summary>
public sealed class HeldKeys
{
    readonly List<string> held = [];

    /// <summary>Whether <paramref name="name"/> went down now, rather than repeating.</summary>
    public bool Down(string name)
    {
        if (held.Contains(name))
            return false;
        held.Add(name);
        return true;
    }

    /// <summary>Whether <paramref name="name"/> was down.</summary>
    public bool Up(string name) => held.Remove(name);

    /// <summary>Lets go of every key: the canvas lost the keyboard.</summary>
    public List<string> All()
    {
        var all = held.ToList();
        held.Clear();
        return all;
    }
}
