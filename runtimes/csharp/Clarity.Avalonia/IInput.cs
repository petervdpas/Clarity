// What every input of Clarity's library has in common, the same as on the
// web (conformance/forms.json, tables.json): it tells when the person
// changed its value, and only then. A value the code sets (a handler, a
// record read again) or a value drawn for the first time is no change.

namespace Clarity.Avalonia;

/// <summary>An input of Clarity's library.</summary>
public interface IInput
{
    /// <summary>The person changed the value to one that differs: Clarity's <c>change</c>.</summary>
    event EventHandler? ValueChanged;
}
