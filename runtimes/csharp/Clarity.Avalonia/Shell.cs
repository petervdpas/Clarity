// The application's one desktop window. Clarity's windows open in it, one on
// top of the other: `open` shows a window in place, `close` goes back to the
// one below, which is told it is on top again; closing the last one ends the
// application. Only a dialog opens a desktop window of its own. Messages
// belong to the shell, so a message said just before closing stays on screen.

using System.Collections.ObjectModel;
using Avalonia.Controls;

namespace Clarity.Avalonia;

public sealed class Shell : Window
{
    readonly Func<WindowViewModel, Control> view;
    readonly Stack<(WindowViewModel Model, Control View)> stack = new();
    readonly ContentControl content = new();
    readonly TimeSpan infoDuration;
    bool closing;

    /// <summary>The messages on screen, of every window.</summary>
    public ObservableCollection<Message> Messages { get; } = [];

    /// <summary>The window on top.</summary>
    public WindowViewModel? Current => stack.Count > 0 ? stack.Peek().Model : null;

    Shell(Func<WindowViewModel, Control> view, TimeSpan infoDuration)
    {
        this.view = view;
        this.infoDuration = infoDuration;
        Width = 1100;
        Height = 760;
        Content = new Panel { Children = { content, new MessageList { Messages = Messages, InfoDuration = infoDuration } } };
        Closing += (_, _) =>
        {
            if (closing)
                return;
            closing = true;
            Current?.Closing();
        };
    }

    /// <summary>
    /// Shows the application's first window, made into a view by <paramref name="view"/>;
    /// info messages stay for <paramref name="infoDuration"/>.
    /// </summary>
    public static Shell Start(WindowViewModel first, Func<WindowViewModel, Control> view, TimeSpan? infoDuration = null)
    {
        var shell = new Shell(view, infoDuration ?? TimeSpan.FromSeconds(5));
        shell.Push(first);
        shell.Show();
        return shell;
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
        model.OpenRequested += other => Open(other, model);
        model.CloseRequested += () => Close(model);
    }

    void ShowTop()
    {
        var (model, shown) = stack.Peek();
        content.Content = shown;
        Title = model.Title;
    }

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
            Content = new Panel { Children = { view(other), new MessageList { Messages = Messages, InfoDuration = infoDuration } } },
            SizeToContent = SizeToContent.WidthAndHeight,
            CanResize = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        var done = false;
        void Finish()
        {
            if (done)
                return;
            done = true;
            other.Closing();
            opener.Returned();
        }
        other.CloseRequested += () =>
        {
            Finish();
            dialog.Close();
        };
        dialog.Closing += (_, _) => Finish();
        _ = dialog.ShowDialog(this);
        other.Opened();
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
