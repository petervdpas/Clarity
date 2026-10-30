// Undo (DESIGN 5.29) on Avalonia: every edit of a part goes over its
// window's edit bus as a message: the part, the scope it is undone in (the
// form or panel it is in, or the part itself), what changed in the window's
// data (each name with its value before and after), and the id of the
// database step its handler made. Ctrl+Z takes back the last message of the
// scope with the focus, so in a form it walks back field by field.
//
// The generated window marks each part with undo: `c:Edits.Scope="customerForm"`.

using System.Text.Json.Nodes;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.LogicalTree;
using Avalonia.Threading;

namespace Clarity.Avalonia;

/// <summary>One name of the window's data, before and after an edit.</summary>
public sealed record Changed(string Name, JsonNode? Before, JsonNode? After);

/// <summary>One edit of a part.</summary>
public sealed class Edit(string part, string scope, IReadOnlyList<Changed> changes, long step)
{
    public string Part { get; } = part;
    public string Scope { get; } = scope;
    public List<Changed> Changes { get; } = [.. changes];
    public long Step { get; set; } = step;
}

/// <summary>The edits of one window, in order.</summary>
public sealed class EditBus
{
    readonly List<Edit> done = [];
    readonly List<Edit> undone = [];

    /// <summary>An edit happened; one that changed nothing is no message. Ends what could be done again in its scope.</summary>
    public void Push(Edit edit)
    {
        if (edit.Changes.Count == 0 && edit.Step == 0)
            return;
        done.Add(edit);
        undone.RemoveAll(e => e.Scope == edit.Scope);
    }

    /// <summary>The last edit, when it is of <paramref name="part"/>: what a change handler right after it joins.</summary>
    public Edit? LastOf(string part) => done.Count > 0 && done[^1].Part == part ? done[^1] : null;

    public bool CanUndo(string scope) => done.Any(e => e.Scope == scope);

    public bool CanRedo(string scope) => undone.Any(e => e.Scope == scope);

    /// <summary>The last edit of <paramref name="scope"/>, to take back; it is then undone.</summary>
    public Edit? Undo(string scope) => Move(done, undone, scope);

    /// <summary>The last undone edit of <paramref name="scope"/>, to do again.</summary>
    public Edit? Redo(string scope) => Move(undone, done, scope);

    /// <summary>Taking an edit back or doing it again was refused: it stays where it was.</summary>
    public void Refused(Edit edit, bool wasUndo)
    {
        var (from, to) = wasUndo ? (undone, done) : (done, undone);
        from.Remove(edit);
        to.Add(edit);
    }

    static Edit? Move(List<Edit> from, List<Edit> to, string scope)
    {
        var at = from.FindLastIndex(e => e.Scope == scope);
        if (at < 0)
            return null;
        var edit = from[at];
        from.RemoveAt(at);
        to.Add(edit);
        return edit;
    }

    /// <summary>What changed between two copies of the window's data, by name.</summary>
    public static List<Changed> Between(JsonObject before, JsonObject after) =>
        [.. before.Select(p => p.Key).Union(after.Select(p => p.Key))
            .Where(name => !JsonNode.DeepEquals(before[name], after[name]))
            .Select(name => new Changed(name, before[name]?.DeepClone(), after[name]?.DeepClone()))];
}

/// <summary>The attached mark of a part with undo, and the keys that undo.</summary>
public static class Edits
{
    /// <summary>The scope a part's edits are undone in: its form or panel, or itself. Unset: no undo.</summary>
    public static readonly AttachedProperty<string?> ScopeProperty =
        AvaloniaProperty.RegisterAttached<Control, string?>("Scope", typeof(Edits));

    public static string? GetScope(Control control) => control.GetValue(ScopeProperty);

    public static void SetScope(Control control, string? value) => control.SetValue(ScopeProperty, value);

    /// <summary>A part with <c>no undo</c>: Ctrl+Z there takes back nothing, not even typing.</summary>
    public static readonly AttachedProperty<bool> OffProperty =
        AvaloniaProperty.RegisterAttached<Control, bool>("Off", typeof(Edits));

    public static bool GetOff(Control control) => control.GetValue(OffProperty);

    public static void SetOff(Control control, bool value) => control.SetValue(OffProperty, value);

    /// <summary>The value properties of the inputs: a change of one is an edit.</summary>
    static readonly string[] Values = ["Value", "IsChecked", "Text", "SelectedItem", "SelectedDate", "Values", "Picture"];

    static Edits()
    {
        ScopeProperty.Changed.AddClassHandler<Control>((control, _) =>
        {
            control.PropertyChanged -= ValueChanged;
            control.LostFocus -= Left;
            if (GetScope(control) is null)
                return;
            control.PropertyChanged += ValueChanged;
            control.LostFocus += Left;
        });
    }

    static void ValueChanged(object? sender, AvaloniaPropertyChangedEventArgs e)
    {
        if (sender is not Control control || !Values.Contains(e.Property.Name) || GetScope(control) is not { } scope)
            return;
        // A text input's edit is its text as it is when it is left; its own undo has the typing.
        if (control.IsKeyboardFocusWithin && TopLevel.GetTopLevel(control)?.FocusManager?.GetFocusedElement() is TextBox)
            return;
        if (control.DataContext is WindowViewModel window)
            window.Edited(control.Name ?? scope, scope);
    }

    /// <summary>A text box is left: what was typed in it is taken, one edit.</summary>
    static void Left(object? sender, global::Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (sender is TextBox box && GetScope(box) is { } scope && box.DataContext is WindowViewModel window)
            window.Edited(box.Name ?? scope, scope);
    }

    /// <summary>
    /// Undo in a desktop window: its keys, before the part with the focus sees
    /// them, and the scope of the part that has the focus, for an Undo in a menu.
    /// </summary>
    public static void Listen(TopLevel window, Func<WindowViewModel?> model)
    {
        window.AddHandler(InputElement.KeyDownEvent, (_, e) =>
        {
            if (model() is { } shown && Key(e, shown, window.FocusManager?.GetFocusedElement()))
                e.Handled = true;
        }, global::Avalonia.Interactivity.RoutingStrategies.Tunnel);
        window.AddHandler(InputElement.GotFocusEvent, (_, e) =>
        {
            if (model() is { } shown && (e.Source as Control)?.GetSelfAndLogicalAncestors().OfType<Control>().Select(GetScope).FirstOrDefault(s => s is not null) is { } scope)
                shown.FocusedScope = scope;
        }, global::Avalonia.Interactivity.RoutingStrategies.Bubble);
    }

    /// <summary>
    /// Ctrl+Z, and Ctrl+Y or Ctrl+Shift+Z, on the part with the focus: its
    /// scope's last edit. Typing in a text box not yet taken is its own to take back.
    /// </summary>
    public static bool Key(KeyEventArgs e, WindowViewModel window, IInputElement? focused)
    {
        var command = e.KeyModifiers.HasFlag(KeyModifiers.Control) || e.KeyModifiers.HasFlag(KeyModifiers.Meta);
        if (!command || e.KeyModifiers.HasFlag(KeyModifiers.Alt))
            return false;
        var back = e.Key == global::Avalonia.Input.Key.Z && !e.KeyModifiers.HasFlag(KeyModifiers.Shift);
        var again = e.Key == global::Avalonia.Input.Key.Y || (e.Key == global::Avalonia.Input.Key.Z && e.KeyModifiers.HasFlag(KeyModifiers.Shift));
        if (!back && !again)
            return false;
        var part = (focused as Control)?.GetSelfAndLogicalAncestors().OfType<Control>().FirstOrDefault(c => GetScope(c) is not null || GetOff(c));
        // `no undo`: nothing, not even the typing.
        if (part is not null && GetOff(part))
            return true;
        if (focused is TextBox box && (part is null || (back ? box.CanUndo : box.CanRedo)))
            return false;
        if (part is null || GetScope(part) is not { } scope)
            return false;
        window.UndoEdit(scope, back);
        return true;
    }
}
