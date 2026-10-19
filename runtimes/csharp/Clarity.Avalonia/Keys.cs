// Keys that press a button anywhere in its window, beside its HotKey:
// - Also: a second key gesture, such as the number pad's 7.
// - Typed: a character, such as `7` or `+`, which matches what is typed
//   from the keyboard or the number pad on any layout, as a browser reads
//   it. While the person types in a text input, the character is theirs.
// Both work only while the button can be pressed.

using System.Runtime.CompilerServices;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;

namespace Clarity.Avalonia;

public static class Keys
{
    public static readonly AttachedProperty<KeyGesture?> AlsoProperty =
        AvaloniaProperty.RegisterAttached<Button, KeyGesture?>("Also", typeof(Keys));

    public static readonly AttachedProperty<string?> TypedProperty =
        AvaloniaProperty.RegisterAttached<Button, string?>("Typed", typeof(Keys));

    public static KeyGesture? GetAlso(Button button) => button.GetValue(AlsoProperty);

    public static void SetAlso(Button button, KeyGesture? value) => button.SetValue(AlsoProperty, value);

    public static string? GetTyped(Button button) => button.GetValue(TypedProperty);

    public static void SetTyped(Button button, string? value) => button.SetValue(TypedProperty, value);

    /// What each button has added to its window, while it is in one.
    static readonly ConditionalWeakTable<Button, Added> Bound = new();

    sealed record Added(TopLevel Window, KeyBinding? Binding, EventHandler<KeyEventArgs>? Typed);

    static Keys()
    {
        AlsoProperty.Changed.AddClassHandler<Button>((button, _) => Rewire(button));
        TypedProperty.Changed.AddClassHandler<Button>((button, _) => Rewire(button));
    }

    static void Rewire(Button button)
    {
        button.AttachedToVisualTree -= Attached;
        button.DetachedFromVisualTree -= Detached;
        Remove(button);
        if (GetAlso(button) is null && string.IsNullOrEmpty(GetTyped(button))) return;
        button.AttachedToVisualTree += Attached;
        button.DetachedFromVisualTree += Detached;
        Add(button);
    }

    static void Attached(object? sender, VisualTreeAttachmentEventArgs e) => Add((Button)sender!);

    static void Detached(object? sender, VisualTreeAttachmentEventArgs e) => Remove((Button)sender!);

    static void Add(Button button)
    {
        if (TopLevel.GetTopLevel(button) is not { } window || Bound.TryGetValue(button, out _)) return;
        KeyBinding? binding = null;
        if (GetAlso(button) is { } gesture)
        {
            binding = new KeyBinding { Gesture = gesture, Command = new Press(button) };
            window.KeyBindings.Add(binding);
        }
        EventHandler<KeyEventArgs>? typed = null;
        if (GetTyped(button) is { Length: > 0 } character)
        {
            typed = (_, e) =>
            {
                if (e.Handled || !Matches(e, character) || Typing(window)) return;
                var press = new Press(button);
                if (!press.CanExecute(null)) return;
                press.Execute(null);
                e.Handled = true;
            };
            window.AddHandler(InputElement.KeyDownEvent, typed, RoutingStrategies.Bubble);
        }
        Bound.Add(button, new Added(window, binding, typed));
    }

    static void Remove(Button button)
    {
        if (!Bound.TryGetValue(button, out var added)) return;
        if (added.Binding is not null) added.Window.KeyBindings.Remove(added.Binding);
        if (added.Typed is not null) added.Window.RemoveHandler(InputElement.KeyDownEvent, added.Typed);
        Bound.Remove(button);
    }

    /// Whether the press typed `character`, without Ctrl, Alt or Meta. Shift
    /// makes some symbols (+, *), so it counts only for letters and digits.
    static bool Matches(KeyEventArgs e, string character)
    {
        if ((e.KeyModifiers & (KeyModifiers.Control | KeyModifiers.Alt | KeyModifiers.Meta)) != 0) return false;
        if (!string.Equals(e.KeySymbol, character, StringComparison.OrdinalIgnoreCase)) return false;
        var symbol = !char.IsLetterOrDigit(character[0]);
        return symbol || (e.KeyModifiers & KeyModifiers.Shift) == 0;
    }

    /// Whether the person is typing in a text input of the window.
    static bool Typing(TopLevel window) => window.FocusManager?.GetFocusedElement() is TextBox;

    /// Presses the button: runs its command, as a click does.
    sealed class Press(Button button) : ICommand
    {
        public event EventHandler? CanExecuteChanged { add { } remove { } }

        public bool CanExecute(object? parameter) =>
            button.IsEffectivelyEnabled && button.IsEffectivelyVisible && button.Command?.CanExecute(button.CommandParameter) == true;

        public void Execute(object? parameter) => button.Command?.Execute(button.CommandParameter);
    }
}
