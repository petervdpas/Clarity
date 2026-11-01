// Messages a handler shows: an info message, a warning or an error, with
// its text in the person's language and format. How they show is the
// screen's (Clarity.Avalonia's MessageList and dialog, the shared screen's).

using Clarity.Runtime;

namespace Clarity.Ui;

public enum Level
{
    Info,
    Warning,
    Error,
}

public sealed record Message(Level Level, Text Text)
{
    /// <summary>The text translated, its slots filled in the person's format.</summary>
    public string Shown => Locale.Show(Text);
}
