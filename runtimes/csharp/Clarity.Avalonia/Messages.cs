// Messages a handler shows. An info message is a note at the top right of
// the window that goes away by itself, or by its small x; a warning or an
// error is said in Clarity's dialog (the shell's), read and closed with OK.

using System.Collections.ObjectModel;
using System.Collections.Specialized;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Threading;
using Clarity.Runtime;

namespace Clarity.Avalonia;

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

/// <summary>The info messages of a window, as notes with a small x that dismisses them.</summary>
public sealed class MessageList : StackPanel
{
    public static readonly StyledProperty<ObservableCollection<Message>?> MessagesProperty =
        AvaloniaProperty.Register<MessageList, ObservableCollection<Message>?>(nameof(Messages));

    /// <summary>How long an info message stays.</summary>
    public TimeSpan InfoDuration { get; set; } = TimeSpan.FromSeconds(5);

    public ObservableCollection<Message>? Messages
    {
        get => GetValue(MessagesProperty);
        set => SetValue(MessagesProperty, value);
    }

    public MessageList()
    {
        Spacing = 8;
        MaxWidth = 420;
        HorizontalAlignment = HorizontalAlignment.Right;
        VerticalAlignment = VerticalAlignment.Top;
        Margin = new Thickness(12);
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property != MessagesProperty)
            return;
        if (change.OldValue is ObservableCollection<Message> old)
            old.CollectionChanged -= Changed;
        if (change.NewValue is ObservableCollection<Message> added)
            added.CollectionChanged += Changed;
        Rebuild();
    }

    void Changed(object? sender, NotifyCollectionChangedEventArgs e)
    {
        foreach (var message in e.NewItems?.OfType<Message>() ?? [])
        {
            if (message.Level != Level.Info)
                continue;
            DispatcherTimer.RunOnce(() => Dismiss(message), InfoDuration);
        }
        Rebuild();
    }

    public void Dismiss(Message message) => Messages?.Remove(message);

    /// <summary>Shows the messages again, as in another language.</summary>
    public void Rebuild()
    {
        Children.Clear();
        // Warnings and errors are said in the shell's dialog, not here.
        foreach (var message in (Messages ?? []).Where(m => m.Level == Level.Info))
        {
            var close = new Button { Content = "×", VerticalAlignment = VerticalAlignment.Top };
            close.Classes.Add("close");
            ToolTip.SetTip(close, Locale.Translate("dismiss", "Dismiss"));
            global::Avalonia.Automation.AutomationProperties.SetName(close, Locale.Translate("dismiss", "Dismiss"));
            close.Click += (_, _) => Dismiss(message);
            var row = new DockPanel();
            DockPanel.SetDock(close, Dock.Right);
            row.Children.Add(close);
            row.Children.Add(new TextBlock { Text = message.Shown, TextWrapping = global::Avalonia.Media.TextWrapping.Wrap, VerticalAlignment = VerticalAlignment.Center });
            var note = new Border { Child = row };
            note.Classes.Add("message");
            note.Classes.Add(message.Level.ToString().ToLowerInvariant());
            Children.Add(note);
        }
    }
}
