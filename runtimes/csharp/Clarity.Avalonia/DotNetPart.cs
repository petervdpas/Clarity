// A custom part drawn by a .NET library (DESIGN 5.19 and 6.5): an Avalonia
// control from a NuGet package or a DLL, shown through an adapter the
// developer writes, half of a partial class whose other half Clarity
// generates from the part's declaration. The part's PartState carries what
// code sets and tells it.

using System.Globalization;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;

namespace Clarity.Avalonia;

public abstract class DotNetPart : UserControl
{
    /// <summary>How values cross to a part and back: numbers written as text read too.</summary>
    public static readonly JsonSerializerOptions Options = new() { NumberHandling = System.Text.Json.Serialization.JsonNumberHandling.AllowReadingFromString };


    /// <summary>What code sets and tells the part.</summary>
    public static readonly StyledProperty<PartState?> StateProperty = AvaloniaProperty.Register<DotNetPart, PartState?>(nameof(State));

    /// <summary>What the part binds; the person changes it through the part.</summary>
    public static readonly StyledProperty<object?> ValueProperty = AvaloniaProperty.Register<DotNetPart, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public PartState? State { get => GetValue(StateProperty); set => SetValue(StateProperty, value); }
    public object? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>The part told something other than its value: the tell and its values.</summary>
    public event Action<string, JsonElement[]>? Told;

    /// <summary>The person changed the value through the part.</summary>
    public event EventHandler? ValueChanged;

    bool changing;

    /// <summary>A property, <c>value</c> or <c>enabled</c>, as code or the window sets it.</summary>
    protected abstract void Set(string name, object? value);

    /// <summary>A hear with its values.</summary>
    protected abstract void Hear(string hear, object?[] values);

    /// <summary>The part tells <paramref name="tell"/> with its values: its handler runs.</summary>
    protected void Tell(string tell, params object?[] values) =>
        Told?.Invoke(tell, values.Select(v => JsonDocument.Parse(Json(v)).RootElement.Clone()).ToArray());

    /// <summary>The person changed the value: what the part binds takes it, and <c>on change</c> runs.</summary>
    public void ChangeValue(object? value)
    {
        changing = true;
        try
        {
            SetCurrentValue(ValueProperty, value);
        }
        finally
        {
            changing = false;
        }
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>A value as type <typeparamref name="T"/>, as code or the window gave it.</summary>
    protected static T As<T>(object? value) =>
        value is T typed ? typed : JsonSerializer.Deserialize<T>(Json(value), Options)!;

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == StateProperty)
        {
            (change.OldValue as PartState)?.StopListening();
            if (change.NewValue is PartState state)
            {
                foreach (var (name, value) in state.Values)
                    Set(name, value);
                state.Listen(Set, Hear);
            }
        }
        else if (change.Property == ValueProperty && !changing)
            Set("value", change.NewValue);
        else if (change.Property == IsEnabledProperty)
            Set("enabled", IsEnabled);
    }

    /// <summary>A told value at <paramref name="at"/> as the handler's type; its empty value when missing.</summary>
    public static T Read<T>(JsonElement[] values, int at) =>
        at < values.Length ? values[at].Deserialize<T>(Options)! : default!;

    /// <summary>A value as a tell gives it: a whole number or a bool as JSON's own, a decimal,
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
