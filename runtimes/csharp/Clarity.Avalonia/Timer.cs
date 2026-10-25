// A timer (DESIGN 5.26), the same as on the web: it shows nothing and has
// no place in the window; while its window is open and it is enabled, it
// runs its tick handler every so many seconds. A tick that comes while the
// handler still runs is left out.

using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Threading;

namespace Clarity.Avalonia;

public class Timer : Control
{
    public static readonly StyledProperty<ICommand?> CommandProperty = AvaloniaProperty.Register<Timer, ICommand?>(nameof(Command));

    /// <summary>The tick handler.</summary>
    public ICommand? Command { get => GetValue(CommandProperty); set => SetValue(CommandProperty, value); }

    /// <summary>Seconds between its ticks: `every 0.5`.</summary>
    public double Every { get; set; } = 1;

    DispatcherTimer? timer;
    bool ticking;

    public Timer()
    {
        // Nothing to see, nothing to take room.
        Width = 0;
        Height = 0;
        IsHitTestVisible = false;
    }

    /// <summary>One tick: runs the handler when it is enabled and the last one is done.</summary>
    public void Tick()
    {
        if (ticking || !IsEnabled || Command is not { } command || !command.CanExecute(null))
            return;
        ticking = true;
        try
        {
            command.Execute(null);
        }
        finally
        {
            ticking = false;
        }
    }

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(Math.Max(0.1, Every)) };
        timer.Tick += (_, _) => Tick();
        timer.Start();
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        timer?.Stop();
        timer = null;
        base.OnDetachedFromVisualTree(e);
    }
}
