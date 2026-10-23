// A custom part drawn by a native library (DESIGN 5.19 and 6.5): Avalonia
// makes a native child window where the part sits, and Clarity's part host
// (libclarity_parts beside the program) lets the library draw in it. The
// part's PartState carries what code sets and tells; what the part tells
// comes back on the UI thread.

using System.Globalization;
using System.Runtime.InteropServices;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Platform;
using Avalonia.Threading;

namespace Clarity.Avalonia;

public class NativePart : NativeControlHost
{
    /// <summary>How values cross to the part host and back: numbers written as text read too.</summary>
    public static readonly JsonSerializerOptions Options = new() { NumberHandling = System.Text.Json.Serialization.JsonNumberHandling.AllowReadingFromString };

    /// <summary>The kind of part: its declared name, as <c>Arena</c>.</summary>
    public static readonly StyledProperty<string> KindProperty = AvaloniaProperty.Register<NativePart, string>(nameof(Kind), "");

    /// <summary>What code sets and tells the part.</summary>
    public static readonly StyledProperty<PartState?> StateProperty = AvaloniaProperty.Register<NativePart, PartState?>(nameof(State));

    /// <summary>What the part binds; the person changes it through the part.</summary>
    public static readonly StyledProperty<object?> ValueProperty = AvaloniaProperty.Register<NativePart, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public string Kind { get => GetValue(KindProperty); set => SetValue(KindProperty, value); }
    public PartState? State { get => GetValue(StateProperty); set => SetValue(StateProperty, value); }
    public object? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>The part told something other than its value: the tell and its values.</summary>
    public event Action<string, JsonElement[]>? Told;

    /// <summary>The person changed the value through the part.</summary>
    public event EventHandler? ValueChanged;

    /// <summary>The part as the part host knows it.</summary>
    readonly ulong id = PartHost.NewId();
    nuint handle;
    bool telling;

    protected override IPlatformHandle CreateNativeControlCore(IPlatformHandle parent)
    {
        var made = base.CreateNativeControlCore(parent);
        handle = (nuint)made.Handle;
        if (PartHost.Start())
        {
            PartHost.Register(id, this);
            Place();
            Give("enabled", IsEnabled);
            if (State is { } state)
                foreach (var (name, value) in state.Values)
                    Give(name, value);
            Give("value", Value);
        }
        return made;
    }

    protected override void DestroyNativeControlCore(IPlatformHandle control)
    {
        PartHost.Close(id);
        handle = 0;
        base.DestroyNativeControlCore(control);
    }

    protected override void OnSizeChanged(SizeChangedEventArgs e)
    {
        base.OnSizeChanged(e);
        Place();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == StateProperty)
        {
            (change.OldValue as PartState)?.StopListening();
            (change.NewValue as PartState)?.Listen((name, value) => Give(name, value), (hear, values) => PartHost.Hear(id, hear, Json(values)));
        }
        else if (change.Property == ValueProperty && !telling)
            Give("value", change.NewValue);
        else if (change.Property == IsEnabledProperty)
            Give("enabled", IsEnabled);
    }

    void Place()
    {
        if (handle == 0)
            return;
        var scale = TopLevel.GetTopLevel(this)?.RenderScaling ?? 1;
        PartHost.Place(id, Kind, handle, (int)Math.Round(Bounds.Width * scale), (int)Math.Round(Bounds.Height * scale));
    }

    void Give(string name, object? value)
    {
        if (handle != 0)
            PartHost.Set(id, name, Json(value));
    }

    /// <summary>What the part told, on the UI thread.</summary>
    internal void Tell(string tell, JsonElement[] values)
    {
        if (tell != "change")
        {
            Told?.Invoke(tell, values);
            return;
        }
        telling = true;
        try
        {
            SetCurrentValue(ValueProperty, values.Length > 0 ? Plain(values[0]) : null);
        }
        finally
        {
            telling = false;
        }
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>A told value as the window's data holds it: a whole number, a number, text or a bool.</summary>
    static object? Plain(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Number when value.TryGetInt64(out var whole) => whole,
        JsonValueKind.Number => value.GetDecimal(),
        JsonValueKind.String => value.GetString(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => null,
    };

    /// <summary>A told value at <paramref name="at"/> as the handler's type; its empty value when missing.</summary>
    public static T Read<T>(JsonElement[] values, int at) =>
        at < values.Length ? values[at].Deserialize<T>(Options)! : default!;

    /// <summary>A value as the part host takes it: a whole number or a bool as JSON's own, a decimal,
    /// a date or a moment as text in its plain form.</summary>
    public static string Json(object? value) => value switch
    {
        null => "null",
        decimal d => JsonSerializer.Serialize(d.ToString(CultureInfo.InvariantCulture)),
        DateOnly d => JsonSerializer.Serialize(d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture)),
        DateTime t => JsonSerializer.Serialize(t.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture)),
        object?[] list => "[" + string.Join(",", list.Select(Json)) + "]",
        Enum e => JsonSerializer.Serialize(e.ToString()),
        _ => JsonSerializer.Serialize(value),
    };
}

/// <summary>Clarity's part host, in libclarity_parts beside the program.</summary>
static class PartHost
{
    const string Library = "clarity_parts";

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    delegate void ToldFn(ulong window, IntPtr part, IntPtr tell, IntPtr values, IntPtr context);

    [DllImport(Library)]
    [return: MarshalAs(UnmanagedType.U1)]
    static extern bool clarity_parts_start(ToldFn told, IntPtr context);

    [DllImport(Library)]
    static extern IntPtr clarity_parts_problem();

    [DllImport(Library)]
    static extern void clarity_parts_place(ulong window, string part, string kind, nuint parent, int x, int y, int width, int height, [MarshalAs(UnmanagedType.U1)] bool shown);

    [DllImport(Library)]
    static extern void clarity_parts_set(ulong window, string part, string name, string value);

    [DllImport(Library)]
    static extern void clarity_parts_hear(ulong window, string part, string hear, string values);

    [DllImport(Library)]
    static extern void clarity_parts_close(ulong window);

    /// <summary>Each part is a window of its own to the part host, named "part".</summary>
    const string Part = "part";

    // Kept alive as long as the part host may call it.
    static readonly ToldFn told = OnTold;
    static readonly Dictionary<ulong, NativePart> parts = [];
    static bool? started;
    static ulong next;

    /// <summary>Why custom parts cannot run, or null.</summary>
    public static string? Problem { get; private set; }

    public static ulong NewId() => Interlocked.Increment(ref next);

    public static bool Start()
    {
        if (started is bool known)
            return known;
        try
        {
            started = clarity_parts_start(told, IntPtr.Zero);
            if (started == false)
                Problem = Marshal.PtrToStringUTF8(clarity_parts_problem());
        }
        catch (Exception e) when (e is DllNotFoundException or EntryPointNotFoundException)
        {
            started = false;
            Problem = "the program's custom parts cannot load: " + e.Message;
        }
        if (Problem is not null)
            Console.Error.WriteLine(Problem);
        return started.Value;
    }

    public static void Register(ulong id, NativePart part) => parts[id] = part;

    public static void Place(ulong id, string kind, nuint parent, int width, int height) =>
        clarity_parts_place(id, Part, kind, parent, 0, 0, width, height, true);

    public static void Set(ulong id, string name, string value)
    {
        if (started == true)
            clarity_parts_set(id, Part, name, value);
    }

    public static void Hear(ulong id, string hear, string values)
    {
        if (started == true)
            clarity_parts_hear(id, Part, hear, values);
    }

    public static void Close(ulong id)
    {
        parts.Remove(id);
        if (started == true)
            clarity_parts_close(id);
    }

    static void OnTold(ulong window, IntPtr part, IntPtr tell, IntPtr values, IntPtr context)
    {
        var told = Marshal.PtrToStringUTF8(tell) ?? "";
        var given = JsonDocument.Parse(Marshal.PtrToStringUTF8(values) ?? "[]").RootElement.EnumerateArray().Select(v => v.Clone()).ToArray();
        Dispatcher.UIThread.Post(() =>
        {
            if (parts.TryGetValue(window, out var found))
                found.Tell(told, given);
        });
    }
}
