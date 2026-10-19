// A second key for a button, beside its HotKey: the number pad's 7 beside
// the keyboard's, which a browser treats alike. Like a HotKey it works
// anywhere in the window, and only while the button can be pressed.

using System.Runtime.CompilerServices;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;

namespace Clarity.Avalonia;

public static class Keys
{
    public static readonly AttachedProperty<KeyGesture?> AlsoProperty =
        AvaloniaProperty.RegisterAttached<Button, KeyGesture?>("Also", typeof(Keys));

    public static KeyGesture? GetAlso(Button button) => button.GetValue(AlsoProperty);

    public static void SetAlso(Button button, KeyGesture? value) => button.SetValue(AlsoProperty, value);

    /// The window each button is in and its binding there, while it is in one.
    static readonly ConditionalWeakTable<Button, Tuple<TopLevel, KeyBinding>> Bound = new();

    static Keys()
    {
        AlsoProperty.Changed.AddClassHandler<Button>((button, _) =>
        {
            button.AttachedToVisualTree -= Attached;
            button.DetachedFromVisualTree -= Detached;
            Remove(button);
            if (GetAlso(button) is null) return;
            button.AttachedToVisualTree += Attached;
            button.DetachedFromVisualTree += Detached;
            Add(button);
        });
    }

    static void Attached(object? sender, VisualTreeAttachmentEventArgs e) => Add((Button)sender!);

    static void Detached(object? sender, VisualTreeAttachmentEventArgs e) => Remove((Button)sender!);

    static void Add(Button button)
    {
        if (GetAlso(button) is not { } gesture || TopLevel.GetTopLevel(button) is not { } window || Bound.TryGetValue(button, out _)) return;
        var binding = new KeyBinding { Gesture = gesture, Command = new Press(button) };
        window.KeyBindings.Add(binding);
        Bound.Add(button, Tuple.Create(window, binding));
    }

    static void Remove(Button button)
    {
        if (!Bound.TryGetValue(button, out var bound)) return;
        bound.Item1.KeyBindings.Remove(bound.Item2);
        Bound.Remove(button);
    }

    /// Presses the button: runs its command, as a click does.
    sealed class Press(Button button) : ICommand
    {
        public event EventHandler? CanExecuteChanged { add { } remove { } }

        public bool CanExecute(object? parameter) =>
            button.IsEffectivelyEnabled && button.IsEffectivelyVisible && button.Command?.CanExecute(button.CommandParameter) == true;

        public void Execute(object? parameter) => button.Command?.Execute(button.CommandParameter);
    }
}
