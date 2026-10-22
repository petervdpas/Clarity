// A question a handler asks: ask ... then handler(values) (DESIGN 5.14).
// The window asks it in Clarity's own dialog when the handler has finished,
// as the web renderer does; the answer runs the handler named, with the
// values written for it and, for a value, the answer last. Cancelling runs
// nothing. Never a dialog of the system.

using Avalonia.Controls;
using Avalonia.Layout;
using Clarity.Runtime;

namespace Clarity.Avalonia;

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

/// <summary>
/// The content of the dialog that asks a question: the question, the input
/// its form needs, and the buttons. It says the answer, or that the person
/// cancelled; its window closes on either.
/// </summary>
public sealed class AskDialog : UserControl
{
    /// <summary>The person answered: yes (null) or the value.</summary>
    public event Action<object?>? Answered;

    /// <summary>The person cancelled: no, Escape, or the window closed.</summary>
    public event Action? Cancelled;

    public Question Question { get; }

    /// <summary>The input of a question for a value; none for yes or no.</summary>
    public Control? Input { get; }

    public Button Ok { get; }

    public Button Cancel { get; }

    public AskDialog(Question question)
    {
        Question = question;
        Classes.Add("ask");
        var (okText, cancelText) = question.Form is ConfirmForm confirm
            ? (confirm.Yes is { } yes ? Locale.Show(yes) : Locale.Translate("ok", "OK"), confirm.No is { } no ? Locale.Show(no) : Locale.Translate("cancel", "Cancel"))
            : (Locale.Translate("ok", "OK"), Locale.Translate("cancel", "Cancel"));
        Ok = new Button { Content = okText, IsDefault = true, Classes = { "accent" } };
        Cancel = new Button { Content = cancelText, IsCancel = true };
        Input = question.Form is ValueForm form ? InputFor(form) : null;
        Ok.Click += (_, _) => Answered?.Invoke(Value());
        Cancel.Click += (_, _) => Cancelled?.Invoke();
        var panel = new StackPanel { Spacing = 12, Margin = new(18), MinWidth = 320, MaxWidth = 520 };
        panel.Children.Add(new TextBlock { Text = Locale.Show(question.Text), TextWrapping = global::Avalonia.Media.TextWrapping.Wrap });
        if (Input is not null)
            panel.Children.Add(Input);
        panel.Children.Add(new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right, Children = { Cancel, Ok } });
        Content = panel;
    }

    static Control InputFor(ValueForm form)
    {
        if (form.Choices is { } choices)
            return new ChoiceInput { Choices = choices, Value = form.Default ?? choices.FirstOrDefault()?.Value };
        return form.Format?.Kind switch
        {
            FormatKind.Date => new DateInput { Value = form.Default as DateOnly? },
            FormatKind.DateTime => new DateTimeInput { Value = form.Default as DateTime? },
            FormatKind.Integer or FormatKind.Decimal or FormatKind.Money or FormatKind.Duration => new NumberInput { Format = form.Format, Value = form.Default },
            _ => new TextInput { Value = form.Default as string ?? "" },
        };
    }

    /// <summary>What the input holds now; null for a yes.</summary>
    public object? Value() => Input switch
    {
        ChoiceInput c => c.Value,
        DateInput d => d.Value,
        DateTimeInput t => t.Value,
        NumberInput n => n.Value,
        TextInput t => t.Value ?? "",
        _ => null,
    };
}

/// <summary>A warning or an error, said in Clarity's dialog and closed with OK.</summary>
public sealed class SaidDialog : UserControl
{
    /// <summary>The person read it: OK, Escape, or the window closed.</summary>
    public event Action? Done;

    public Message Message { get; }

    public Button Ok { get; }

    public SaidDialog(Message message)
    {
        Message = message;
        Ok = new Button { Content = Locale.Translate("ok", "OK"), IsDefault = true, IsCancel = true, HorizontalAlignment = HorizontalAlignment.Right };
        Ok.Classes.Add("accent");
        Ok.Click += (_, _) => Done?.Invoke();
        var text = new TextBlock { Text = message.Shown, TextWrapping = global::Avalonia.Media.TextWrapping.Wrap };
        text.Classes.Add("said");
        text.Classes.Add(message.Level.ToString().ToLowerInvariant());
        Content = new StackPanel { Spacing = 12, Margin = new(18), MinWidth = 320, MaxWidth = 520, Children = { text, Ok } };
    }
}
