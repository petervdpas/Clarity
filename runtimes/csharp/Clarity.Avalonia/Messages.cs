// Messages a handler shows: at the top right of the window, an info message
// going away by itself, warnings and errors staying until they are clicked.

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
    /// <summary>The text with its slots filled.</summary>
    public string Shown => Text.Show();
}

/// <summary>The messages of a window, as buttons that dismiss them.</summary>
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

    void Rebuild()
    {
        Children.Clear();
        foreach (var message in Messages ?? [])
        {
            var button = new Button { Content = message.Shown, HorizontalAlignment = HorizontalAlignment.Stretch };
            button.Classes.Add("message");
            button.Classes.Add(message.Level.ToString().ToLowerInvariant());
            ToolTip.SetTip(button, "Dismiss");
            button.Click += (_, _) => Dismiss(message);
            Children.Add(button);
        }
    }
}
