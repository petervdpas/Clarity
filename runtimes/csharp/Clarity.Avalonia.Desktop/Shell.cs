// A desktop window of the application. Clarity's windows open in it, one on
// top of the other: `open` shows a window in place, `close` goes back to the
// one below, which is told it is on top again. The first shell is the
// application's window: closing its last window ends the application. A
// desktop window of its own opens only where the design says so: a target
// marked `in new window` gets a shell of its own (closing its last window
// closes it and tells the window that opened it), and a dialog gets a window.
// Messages belong to the shell, so a message said just before closing stays
// on screen. When the language changes, every window's view is made again,
// so its texts and values show in the new language.

using System.Collections.ObjectModel;
using Avalonia.Controls;

namespace Clarity.Avalonia;

public sealed class Shell : Window
{
    readonly Func<WindowViewModel, Control> view;
    readonly Stack<(WindowViewModel Model, Control View)> stack = new();
    readonly ContentControl content = new();
    readonly MessageList messages;
    readonly TimeSpan infoDuration;
    bool closing;

    /// <summary>The window that opened this shell's first one; none for the application's window.</summary>
    WindowViewModel? opener;

    static readonly List<Shell> desktops = [];

    /// <summary>The open desktop windows, the application's first.</summary>
    public static IReadOnlyList<Shell> Desktops => desktops;

    /// <summary>The messages on screen, of every window.</summary>
    public ObservableCollection<Message> Messages { get; } = [];

    /// <summary>The window on top in the shell itself.</summary>
    public WindowViewModel? Current => stack.Count > 0 ? stack.Peek().Model : null;

    /// <summary>Every open window from bottom to top, dialogs last.</summary>
    public IReadOnlyList<WindowViewModel> Windows => [.. stack.Reverse().Select(s => s.Model), .. dialogs];

    readonly List<WindowViewModel> dialogs = [];

    /// <summary>The desktop window of each open dialog.</summary>
    readonly Dictionary<WindowViewModel, Window> dialogWindows = [];

    Shell(Func<WindowViewModel, Control> view, TimeSpan infoDuration)
    {
        this.view = view;
        this.infoDuration = infoDuration;
        Width = 1100;
        Height = 760;
        messages = new MessageList { Messages = Messages, InfoDuration = infoDuration };
        // A warning or an error is said in Clarity's dialog, one at a time.
        Messages.CollectionChanged += (_, e) =>
        {
            foreach (var message in e.NewItems?.OfType<Message>() ?? [])
            {
                if (message.Level != Level.Info)
                    Said(message);
            }
        };
        Content = new Panel { Children = { content, messages } };
        Locale.Changed += Refresh;
        Closing += (_, _) =>
        {
            if (closing)
                return;
            closing = true;
            Current?.Closing();
        };
        Closed += (_, _) =>
        {
            Locale.Changed -= Refresh;
            desktops.Remove(this);
            // The window that opened this desktop window is told, so it can reload its data.
            if (opener is { } back && desktops.Any(d => d.Windows.Contains(back)))
                back.Returned();
        };
    }

    /// <summary>
    /// Shows the application's first window, made into a view by <paramref name="view"/>;
    /// info messages stay for <paramref name="infoDuration"/>.
    /// </summary>
    public static Shell Start(WindowViewModel first, Func<WindowViewModel, Control> view, TimeSpan? infoDuration = null)
    {
        var shell = new Shell(view, infoDuration ?? TimeSpan.FromSeconds(5));
        shell.Fit(first);
        desktops.Add(shell);
        shell.Push(first);
        shell.Show();
        return shell;
    }

    /// <summary>Opens <paramref name="first"/> in a desktop window of its own, as the design marks it.</summary>
    void OpenApart(WindowViewModel first, WindowViewModel from)
    {
        var shell = new Shell(view, infoDuration) { opener = from };
        shell.Fit(first);
        desktops.Add(shell);
        shell.Push(first);
        shell.Show();
    }

    /// <summary>Opens at the size of the window it shows first, or the standard size.</summary>
    void Fit(WindowViewModel first)
    {
        if (first.WindowSize is { } size)
            (Width, Height) = (size.Width, size.Height);
    }

    void Push(WindowViewModel model)
    {
        Attach(model);
        stack.Push((model, view(model)));
        ShowTop();
        model.Opened();
    }

    void Attach(WindowViewModel model)
    {
        model.Messages = Messages;
        model.OpenRequested += (other, newWindow) =>
        {
            if (newWindow)
                OpenApart(other, model);
            else
                Open(other, model);
        };
        model.CloseRequested += () => Close(model);
        model.AskRequested += question => Ask(model, question);
    }

    void ShowTop()
    {
        var (model, shown) = stack.Peek();
        content.Content = shown;
        Title = model.Title;
    }

    /// <summary>Makes every window's view again, as in another language.</summary>
    void Refresh()
    {
        var models = stack.Select(s => s.Model).Reverse().ToList();
        stack.Clear();
        foreach (var model in models)
            stack.Push((model, view(model)));
        if (stack.Count > 0)
            ShowTop();
        messages.Rebuild();
        foreach (var (model, window) in dialogWindows)
        {
            window.Title = model.Title;
            window.Content = DialogContent(model);
        }
    }

    Panel DialogContent(WindowViewModel model) =>
        new() { Children = { view(model), new MessageList { Messages = Messages, InfoDuration = infoDuration } } };

    void Open(WindowViewModel other, WindowViewModel opener)
    {
        if (!other.IsDialog)
        {
            Push(other);
            return;
        }
        // Only a dialog gets a desktop window of its own, on top of this one.
        Attach(other);
        var dialog = new Window
        {
            Title = other.Title,
            Content = DialogContent(other),
            // A dialog fits its content, unless it says its size.
            SizeToContent = other.WindowSize is null ? SizeToContent.WidthAndHeight : SizeToContent.Manual,
            Width = other.WindowSize?.Width ?? double.NaN,
            Height = other.WindowSize?.Height ?? double.NaN,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        var done = false;
        void Finish()
        {
            if (done)
                return;
            done = true;
            dialogs.Remove(other);
            dialogWindows.Remove(other);
            other.Closing();
            // The window below it is told: the one that opened it, if it is still open.
            (Windows.Contains(opener) ? opener : Windows.LastOrDefault())?.Returned();
        }
        other.CloseRequested += () =>
        {
            Finish();
            dialog.Close();
        };
        dialog.Closing += (_, _) => Finish();
        dialogs.Add(other);
        dialogWindows[other] = dialog;
        _ = dialog.ShowDialog(this);
        other.Opened();
    }

    /// <summary>A warning or an error, in a dialog of Clarity's own on top of the window; OK takes it away.</summary>
    void Said(Message message)
    {
        if (!IsVisible)
            return;
        var said = new SaidDialog(message);
        var dialog = new Window
        {
            Title = Title,
            Content = said,
            SizeToContent = SizeToContent.WidthAndHeight,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        said.Done += () => dialog.Close();
        dialog.Closed += (_, _) => Messages.Remove(message);
        _ = dialog.ShowDialog(this);
    }

    /// <summary>A question a window asks, in a dialog of Clarity's own on top of the window; the answer runs its handler.</summary>
    void Ask(WindowViewModel model, Question question)
    {
        var ask = new AskDialog(question);
        var dialog = new Window
        {
            Title = Title,
            Content = ask,
            SizeToContent = SizeToContent.WidthAndHeight,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        // Closing the dialog any other way is a cancel: nothing runs.
        ask.Answered += answer =>
        {
            dialog.Close();
            model.Answer(question, answer);
        };
        ask.Cancelled += () => dialog.Close();
        _ = dialog.ShowDialog(this);
    }

    void Close(WindowViewModel model)
    {
        // A window closes only from the top; a dialog closes its own window.
        if (model.IsDialog || Current != model)
            return;
        stack.Pop();
        model.Closing();
        if (stack.Count == 0)
        {
            closing = true;
            Close();
            return;
        }
        ShowTop();
        stack.Peek().Model.Returned();
    }
}
