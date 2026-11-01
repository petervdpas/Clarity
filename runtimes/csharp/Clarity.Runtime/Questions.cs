// A question a handler asks: ask ... then handler(values) (DESIGN 5.14):
// what it asks for, and what its answer runs. The screen that shows the
// window asks it in Clarity's own dialog (Clarity.Avalonia's AskDialog, the
// shared screen's), never a dialog of the system.

using Clarity.Runtime;

namespace Clarity.Runtime;

/// <summary>What a question asks for, which decides its dialog.</summary>
public abstract record AskForm;

/// <summary>Yes or no; the handler runs on yes. Without texts the buttons say OK and Cancel.</summary>
public sealed record ConfirmForm(Text? Yes, Text? No) : AskForm;

/// <summary>
/// A value: typed in, in the format of <paramref name="Format"/> (text when
/// none), or one of <paramref name="Choices"/> (an enum's values, or records
/// by their ids), starting at <paramref name="Default"/>.
/// </summary>
public sealed record ValueForm(FieldFormat? Format, IReadOnlyList<Choice>? Choices, object? Default) : AskForm;

/// <summary>A question, and what its answer runs.</summary>
public sealed record Question(Text Text, AskForm Form, Action<object?> Answered);
