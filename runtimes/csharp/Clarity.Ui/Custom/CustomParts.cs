// The custom parts the program's libraries bring, registered with the native
// library by the name their declaration gives them: each kind a table of
// callbacks over a C# CustomPart, the part itself held by a GC handle from
// `make` until `destroy`. Every callback catches what the part throws: it is
// said on standard error and the window goes on (fail safe per part).

using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;
using Clarity.Ui.Native;

namespace Clarity.Ui;

public static unsafe class CustomParts
{
    static readonly Dictionary<string, Func<CustomPart>> added = [];
    static readonly HashSet<string> registered = [];

    /// <summary>Adds a custom part by hand, beside those found in the program's assemblies; registered at once.</summary>
    public static void Add(string kind, Func<CustomPart> make)
    {
        lock (added)
            added[kind] = make;
        Register(kind, make);
    }

    /// <summary>Registers every custom part of the program's assemblies with the library: each class deriving from CustomPart, by its [PartKind].</summary>
    public static void RegisterAll()
    {
        foreach (var type in AppDomain.CurrentDomain.GetAssemblies().Where(a => !a.IsDynamic).SelectMany(Types))
        {
            if (type.IsAbstract || !typeof(CustomPart).IsAssignableFrom(type) || type.GetCustomAttribute<PartKindAttribute>() is not { } kinds)
                continue;
            if (type.GetConstructor(Type.EmptyTypes) is null)
                continue;
            foreach (var kind in kinds.Kinds)
                Register(kind, () => (CustomPart)Activator.CreateInstance(type)!);
        }
    }

    /// <summary>The types of an assembly, those that load when some do not.</summary>
    static IEnumerable<Type> Types(Assembly assembly)
    {
        try
        {
            return assembly.GetTypes();
        }
        catch (ReflectionTypeLoadException e)
        {
            return e.Types.OfType<Type>();
        }
    }

    static void Register(string kind, Func<CustomPart> make)
    {
        lock (registered)
            if (!registered.Add(kind))
                return;
        var callbacks = new CustomCallbacks
        {
            User = GCHandle.ToIntPtr(GCHandle.Alloc(make)),
            Make = &Make,
            Set = &Set,
            Hear = &Hear,
            Draw = &Draw,
            Start = &Start,
            Focusable = &Focusable,
            PointerPressed = &PointerPressed,
            PointerMoved = &PointerMoved,
            PointerReleased = &PointerReleased,
            Click = &Click,
            Wheel = &Wheel,
            Key = &Key,
            Text = &Text,
            FocusChanged = &FocusChanged,
            Wake = &Wake,
            Stop = &Stop,
            TooltipAt = &TooltipAt,
            Destroy = &Destroy,
        };
        var k = Interop.Utf8(kind);
        fixed (byte* kp = k)
            Interop.Check(Lib.CustomRegister(kp, (nuint)k.Length, &callbacks), "clarity_custom_register");
    }

    static CustomPart Part(nint part) => (CustomPart)GCHandle.FromIntPtr(part).Target!;

    /// <summary>Runs a callback on a part with its context: what waited goes first, a failure is said and the window goes on.</summary>
    static T On<T>(nint part, nint cx, T otherwise, Func<CustomPart, T> act, [CallerMemberName] string what = "")
    {
        try
        {
            var shown = Part(part);
            shown.Cx = cx;
            try
            {
                if (cx != 0)
                {
                    Interop.Check(Lib.CustomBounds(cx, out var x, out var y, out var w, out var h), "clarity_custom_bounds");
                    shown.Bounds = new Box(x, y, w, h);
                }
                shown.Flush();
                return act(shown);
            }
            finally
            {
                shown.Cx = 0;
            }
        }
        catch (Exception e)
        {
            Console.Error.WriteLine($"a custom part failed in {what}: {e.Message}");
            return otherwise;
        }
    }

    static void On(nint part, nint cx, Action<CustomPart> act, [CallerMemberName] string what = "") =>
        On(part, cx, 0, p =>
        {
            act(p);
            return 0;
        }, what);

    /// <summary>A value the library sent: a text, a whole number, a number, a bool, or the JSON of a list or record.</summary>
    static object? ValueOf(string json)
    {
        if (json.Length == 0)
            return null;
        using var document = JsonDocument.Parse(json);
        var e = document.RootElement;
        return e.ValueKind switch
        {
            JsonValueKind.Null or JsonValueKind.Undefined => null,
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.String => e.GetString(),
            JsonValueKind.Number when e.TryGetInt64(out var n) => n,
            JsonValueKind.Number => e.GetDouble(),
            _ => e.Clone(),
        };
    }

    static PointerInput Pointer(NativePointer* at) => new(at->X, at->Y, (Modifiers)at->Modifiers, (int)at->Clicks);

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static nint Make(nint user)
    {
        try
        {
            var make = (Func<CustomPart>)GCHandle.FromIntPtr(user).Target!;
            return GCHandle.ToIntPtr(GCHandle.Alloc(make()));
        }
        catch (Exception e)
        {
            // Fail safe: the library says in its place that the part is missing.
            Console.Error.WriteLine($"a custom part could not be made: {e.Message}");
            return 0;
        }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Set(nint part, byte* name, nuint nameLen, byte* value, nuint valueLen, nint cx) =>
        On(part, cx, p => p.SetFromLibrary(Interop.Read(name, nameLen), ValueOf(Interop.Read(value, valueLen))));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Hear(nint part, byte* hear, nuint hearLen, byte* values, nuint valuesLen, nint cx) =>
        On(part, cx, p =>
        {
            var json = Interop.Read(values, valuesLen);
            object?[] given = [];
            if (json.Length > 0)
            {
                using var document = JsonDocument.Parse(json);
                given = document.RootElement.ValueKind == JsonValueKind.Array ? [.. document.RootElement.EnumerateArray().Select(v => ValueOf(v.GetRawText()))] : [];
            }
            p.HearFromLibrary(Interop.Read(hear, hearLen), given);
        });

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Draw(nint part, nint canvas, double x, double y, double width, double height) =>
        On(part, 0, p =>
        {
            p.Bounds = new Box(x, y, width, height);
            p.Draw(new Painter(canvas));
        });

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Start(nint part, nint cx) => On(part, cx, p => p.OnStart());

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static int Focusable(nint part) => On(part, 0, 0, p => p.Focusable ? 1 : 0);

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void PointerPressed(nint part, NativePointer* at, nint cx) => On(part, cx, p => p.OnPointerPressed(Pointer(at)));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void PointerMoved(nint part, NativePointer* at, nint cx) => On(part, cx, p => p.OnPointerMoved(Pointer(at)));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void PointerReleased(nint part, NativePointer* at, nint cx) => On(part, cx, p => p.OnPointerReleased(Pointer(at)));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Click(nint part, nint cx) => On(part, cx, p => p.OnClick());

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static int Wheel(nint part, double dx, double dy, uint modifiers, nint cx) => On(part, cx, 0, p => p.OnWheel(dx, dy, (Modifiers)modifiers) ? 1 : 0);

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static int Key(nint part, byte* name, nuint nameLen, uint modifiers, nint cx) =>
        On(part, cx, 0, p => p.OnKey(new KeyInput(Interop.Read(name, nameLen), (Modifiers)modifiers)) ? 1 : 0);

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Text(nint part, byte* text, nuint textLen, nint cx) => On(part, cx, p => p.OnText(Interop.Read(text, textLen)));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void FocusChanged(nint part, int focused, nint cx) => On(part, cx, p => p.OnFocusChanged(focused != 0));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Wake(nint part, nint cx) => On(part, cx, p => p.OnWake());

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Stop(nint part) => On(part, 0, p => p.OnStop());

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void TooltipAt(nint part, double x, double y, nint tip) =>
        On(part, 0, p => Interop.Answer(tip, p.TooltipAt(x, y)));

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Destroy(nint part)
    {
        try
        {
            GCHandle.FromIntPtr(part).Free();
        }
        catch (Exception)
        {
            // A part freed twice is gone already.
        }
    }
}
