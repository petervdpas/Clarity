// Keys that press a button anywhere in its window, beside its HotKey:
// - Also: a second key gesture, such as the number pad's 7.
// - Typed: a character, such as `7` or `+`, which matches what is typed
//   from the keyboard or the number pad on any layout, as a browser reads
//   it. While the person types in a text input, the character is theirs.
// Both work only while the button can be pressed.
// - Frame: the key of a menu item or a toolbar or ribbon button (DESIGN
//   5.28). The window's frame (FrameRoot) answers it when nothing inside the
//   window took the key first, whichever ribbon tab is open and while the
//   menu is shut; what code hid or disabled does not answer.
// - Id: the command's id among the person's keys (PersonKeys): where they
//   set one, a frame command and a button answer it instead of the design's.

using System.Runtime.CompilerServices;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.LogicalTree;
using Avalonia.Threading;

namespace Clarity.Avalonia;

public static class Keys
{
    public static readonly AttachedProperty<KeyGesture?> AlsoProperty =
        AvaloniaProperty.RegisterAttached<Button, KeyGesture?>("Also", typeof(Keys));

    public static readonly AttachedProperty<string?> TypedProperty =
        AvaloniaProperty.RegisterAttached<Button, string?>("Typed", typeof(Keys));

    public static readonly AttachedProperty<KeyGesture?> FrameProperty =
        AvaloniaProperty.RegisterAttached<Control, KeyGesture?>("Frame", typeof(Keys));

    public static readonly AttachedProperty<bool> FrameRootProperty =
        AvaloniaProperty.RegisterAttached<Control, bool>("FrameRoot", typeof(Keys));

    public static readonly AttachedProperty<string?> IdProperty =
        AvaloniaProperty.RegisterAttached<Control, string?>("Id", typeof(Keys));

    public static string? GetId(Control command) => command.GetValue(IdProperty);

    public static void SetId(Control command, string? value) => command.SetValue(IdProperty, value);

    /// <summary>The key a button's design gives it, before the person's took its place.</summary>
    static readonly ConditionalWeakTable<Button, Designed> DesignedKeys = new();

    sealed record Designed(KeyGesture? HotKey);

    /// <summary>The buttons with an id, which follow the person's keys.</summary>
    static readonly List<WeakReference<Button>> Followers = [];

    /// <summary>A button takes the person's key, or its design's again.</summary>
    static void Follow(Button button)
    {
        if (GetId(button) is not { } id) return;
        if (!DesignedKeys.TryGetValue(button, out var designed))
        {
            designed = new Designed(button.HotKey);
            DesignedKeys.Add(button, designed);
        }
        button.HotKey = PersonKeys.Choices.TryGetValue(id, out var keys) ? keys.Select(KeyRules.Gesture).FirstOrDefault(g => g is not null) : designed.HotKey;
    }

    /// <summary>The keys a frame command answers now: the person's, or its design's.</summary>
    static IEnumerable<KeyGesture> FrameGestures(Control command)
    {
        if (GetId(command) is { } id && PersonKeys.Choices.TryGetValue(id, out var keys))
            return keys.Select(KeyRules.Gesture).OfType<KeyGesture>();
        return GetFrame(command) is { } designed ? [designed] : [];
    }

    public static KeyGesture? GetFrame(Control command) => command.GetValue(FrameProperty);

    public static void SetFrame(Control command, KeyGesture? value) => command.SetValue(FrameProperty, value);

    public static bool GetFrameRoot(Control root) => root.GetValue(FrameRootProperty);

    public static void SetFrameRoot(Control root, bool value) => root.SetValue(FrameRootProperty, value);

    /// The key handler each frame has added to its window, while it is in one.
    static readonly ConditionalWeakTable<Control, FrameAdded> Frames = new();

    sealed record FrameAdded(TopLevel Window, EventHandler<KeyEventArgs> Handler);

    static void FrameAttached(object? sender, VisualTreeAttachmentEventArgs e)
    {
        var root = (Control)sender!;
        if (TopLevel.GetTopLevel(root) is not { } window || Frames.TryGetValue(root, out _)) return;
        EventHandler<KeyEventArgs> handler = (_, e) =>
        {
            // The window's own buttons and inputs hear a key first.
            if (e.Handled) return;
            foreach (var command in Commands(root))
            {
                if (!FrameGestures(command).Any(g => g.Matches(e)) || !Answers(command, root)) continue;
                var (run, parameter) = command switch
                {
                    Button b => (b.Command, b.CommandParameter),
                    MenuItem m => (m.Command, m.CommandParameter),
                    _ => (null, null),
                };
                if (run?.CanExecute(parameter) != true) continue;
                run.Execute(parameter);
                e.Handled = true;
                return;
            }
        };
        window.AddHandler(InputElement.KeyDownEvent, handler, RoutingStrategies.Bubble);
        Frames.Add(root, new FrameAdded(window, handler));
    }

    static void FrameDetached(object? sender, VisualTreeAttachmentEventArgs e)
    {
        var root = (Control)sender!;
        if (!Frames.TryGetValue(root, out var added)) return;
        added.Window.RemoveHandler(InputElement.KeyDownEvent, added.Handler);
        Frames.Remove(root);
    }

    /// A control's parent: logical, or the menu or tab control whose item it is.
    static Control? Parent(Control control) => control.Parent as Control ?? (control as ILogical).LogicalParent as Control ?? ItemsControl.ItemsControlFromItemContainer(control);

    /// Every control of a frame, also the items of a menu that is shut and
    /// the content of a ribbon tab that is not open.
    static IEnumerable<Control> Commands(Control root)
    {
        var seen = new HashSet<Control>();
        var waiting = new Stack<Control>([root]);
        while (waiting.TryPop(out var at))
        {
            if (!seen.Add(at)) continue;
            yield return at;
            foreach (var child in at.GetLogicalChildren().OfType<Control>())
                waiting.Push(child);
            if (at is ItemsControl items)
                foreach (var item in items.Items.OfType<Control>())
                    waiting.Push(item);
            if (at is ContentControl { Content: Control content })
                waiting.Push(content);
        }
    }

    /// Whether a frame command shows and takes clicks, up to its frame: a ribbon
    /// tab that is not open still answers, one code hid does not.
    static bool Answers(Control command, Control root)
    {
        for (var at = command; at is not null && !ReferenceEquals(at, root); at = Parent(at))
        {
            if (!at.IsVisible || !at.IsEnabled) return false;
        }
        return true;
    }

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
        IdProperty.Changed.AddClassHandler<Control>((control, change) =>
        {
            if (control is not Button button) return;
            lock (Followers)
            {
                Followers.RemoveAll(w => !w.TryGetTarget(out _));
                Followers.Add(new WeakReference<Button>(button));
            }
            Follow(button);
        });
        PersonKeys.Changed += () =>
        {
            // The buttons follow on the UI thread, whichever thread set the keys.
            if (!Dispatcher.UIThread.CheckAccess())
            {
                Dispatcher.UIThread.Post(FollowAll);
                return;
            }
            FollowAll();
        };
        static void FollowAll()
        {
            Button[] live;
            lock (Followers)
                live = [.. Followers.Select(w => w.TryGetTarget(out var b) ? b : null).OfType<Button>()];
            foreach (var button in live)
                Follow(button);
        }
        FrameRootProperty.Changed.AddClassHandler<Control>((root, _) =>
        {
            root.AttachedToVisualTree -= FrameAttached;
            root.DetachedFromVisualTree -= FrameDetached;
            FrameDetached(root, null!);
            if (!GetFrameRoot(root)) return;
            root.AttachedToVisualTree += FrameAttached;
            root.DetachedFromVisualTree += FrameDetached;
            FrameAttached(root, null!);
        });
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
