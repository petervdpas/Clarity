// Messages a handler shows: an info message, a warning or an error, with
// its text in the person's language and format. How they show is the
// screen's (Clarity.UI's message list and dialog, the shared screen's).

using Clarity.Runtime;

namespace Clarity.Runtime;

public enum Level
{
    Info,
    Warning,
    Error,
}

/// <summary>A toast the project declares (<c>toast saved</c>), as the screen shows it: its name,
/// level (info, success, warning, error), how long it shows in milliseconds, its icon (or null) and
/// its corner (topRight, topLeft, bottomRight, bottomLeft).</summary>
public sealed record ToastLook(string Name, string Level, int Lifespan, string? Icon, string Corner);

public sealed record Message(Level Level, Text Text, ToastLook? Toast = null)
{
    /// <summary>The text translated, its slots filled in the person's format.</summary>
    public string Shown => Locale.Show(Text);
}
