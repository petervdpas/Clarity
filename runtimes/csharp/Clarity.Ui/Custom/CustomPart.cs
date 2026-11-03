// A custom part (DESIGN 5.19, 7.9): a control a .NET library of the program
// brings, placed in a window as <Custom part="Gauge">. The native library
// lays it out, gives it the pointer, keys and the focus, and asks it to draw
// on its canvas; it speaks with its window in the part's declared words:
// what it binds and its properties come in (Set), what code tells it comes
// in (Hear), the person's change of its value goes out (ChangeValue, which
// runs `on change`) and so do its own events (Tell). On C# it is a class
// deriving from CustomPart named by its declaration with [PartKind("Gauge")]:
// Clarity writes one half of it from the declaration, the developer's
// adapter the other. Each kind is registered with the library as a table of
// callbacks; nothing it throws crosses into native code.

using System.Text.Json;
using Clarity.Runtime;
using Clarity.Ui.Native;

namespace Clarity.Ui;

/// <summary>The kinds of part a class is, by the name its declaration gives it.</summary>
[AttributeUsage(AttributeTargets.Class)]
public sealed class PartKindAttribute(params string[] kinds) : Attribute
{
    public IReadOnlyList<string> Kinds { get; } = kinds;
}

public abstract unsafe class CustomPart
{
    /// <summary>The library's context of the callback that runs now; none between callbacks.</summary>
    internal nint Cx { get; set; }

    readonly List<Action<nint>> waiting = [];

    /// <summary>Where it is on the screen, as the library last laid it out.</summary>
    public Box Bounds { get; internal set; }

    /// <summary>Whether the person may use it.</summary>
    public bool Enabled
    {
        get
        {
            if (Cx == 0)
                return enabled;
            Interop.Check(Lib.CustomEnabled(Cx, out var on), "clarity_custom_enabled");
            enabled = on != 0;
            return enabled;
        }
    }

    bool enabled = true;

    /// <summary>A property (<c>value</c> for what it binds, <c>enabled</c>, or a declared one) as the window or code sets it.</summary>
    protected abstract void Set(string name, object? value);

    /// <summary>What code told it, with its values: <c>gauge.reset(1)</c>.</summary>
    protected abstract void Hear(string hear, object?[] values);

    /// <summary>Draws it in <see cref="Bounds"/>.</summary>
    public virtual void Draw(Painter painter)
    {
    }

    /// <summary>Whether it takes the focus, and so keys.</summary>
    public virtual bool Focusable => false;

    public virtual void OnStart()
    {
    }

    public virtual void OnPointerPressed(PointerInput at)
    {
    }

    public virtual void OnPointerMoved(PointerInput at)
    {
    }

    public virtual void OnPointerReleased(PointerInput at)
    {
    }

    public virtual void OnClick()
    {
    }

    public virtual bool OnWheel(double dx, double dy, Modifiers held) => false;

    public virtual bool OnKey(KeyInput key) => false;

    public virtual void OnText(string text)
    {
    }

    public virtual void OnFocusChanged(bool focused)
    {
    }

    /// <summary>The time it asked for came (<see cref="WakeIn"/>), or a frame while it animates.</summary>
    public virtual void OnWake()
    {
    }

    /// <summary>It left the screen: what it runs stops.</summary>
    public virtual void OnStop()
    {
    }

    public virtual string? TooltipAt(double x, double y) => null;

    /// <summary>It changed: the library draws it again.</summary>
    public void Invalidate() => Do(cx => Interop.Check(Lib.CustomInvalidate(cx), "clarity_custom_invalidate"));

    /// <summary>It wakes once (<see cref="OnWake"/>) after <paramref name="ms"/> milliseconds.</summary>
    public void WakeIn(double ms) => Do(cx => Interop.Check(Lib.CustomWakeIn(cx, ms), "clarity_custom_wake_in"));

    /// <summary>It wakes every frame while <paramref name="on"/>.</summary>
    public void Animate(bool on) => Do(cx => Interop.Check(Lib.CustomAnimate(cx, on ? 1 : 0), "clarity_custom_animate"));

    /// <summary>It takes the focus.</summary>
    public void Focus() => Do(cx => Interop.Check(Lib.CustomFocus(cx), "clarity_custom_focus"));

    /// <summary>The person changed its value: what it binds takes it (one edit, which Ctrl+Z takes back) and <c>on change</c> runs.</summary>
    protected void ChangeValue(object? value)
    {
        var json = Interop.Utf8(PartValues.Write(value));
        Do(cx =>
        {
            fixed (byte* jp = json)
                Interop.Check(Lib.CustomChange(cx, jp, (nuint)json.Length), "clarity_custom_change");
        });
    }

    /// <summary>The part told something: the tell and its values, wired to a handler or not.</summary>
    public event Action<string, JsonElement[]>? Told;

    /// <summary>It tells <paramref name="tell"/> with its values: <see cref="Told"/> hears it, and the handler wired to it runs with them.</summary>
    protected void Tell(string tell, params object?[] values)
    {
        var written = PartValues.Write(values);
        using (var parsed = JsonDocument.Parse(written))
            Told?.Invoke(tell, [.. parsed.RootElement.EnumerateArray().Select(e => e.Clone())]);
        var t = Interop.Utf8(tell);
        var v = Interop.Utf8(written);
        Do(cx =>
        {
            fixed (byte* tp = t)
            fixed (byte* vp = v)
                Interop.Check(Lib.CustomTell(cx, tp, (nuint)t.Length, vp, (nuint)v.Length), "clarity_custom_tell");
        });
    }

    /// <summary>A value as type <typeparamref name="T"/>, however the window or code gave it.</summary>
    protected static T As<T>(object? value) =>
        value is T typed ? typed : JsonSerializer.Deserialize<T>(value is JsonElement e ? e.GetRawText() : PartValues.Write(value), PartValues.Json)!;

    /// <summary>Does something through the library now, while a callback runs; else at the start of the next one.</summary>
    void Do(Action<nint> act)
    {
        if (Cx != 0)
            act(Cx);
        else
            lock (waiting)
                waiting.Add(act);
    }

    /// <summary>What waited for a callback, done with its context.</summary>
    internal void Flush()
    {
        if (Cx == 0)
            return;
        List<Action<nint>> now;
        lock (waiting)
        {
            now = [.. waiting];
            waiting.Clear();
        }
        foreach (var act in now)
            act(Cx);
    }

    internal void SetFromLibrary(string name, object? value)
    {
        if (name == "enabled" && value is bool on)
            enabled = on;
        Set(name, value);
    }

    internal void HearFromLibrary(string hear, object?[] values) => Hear(hear, values);
}
