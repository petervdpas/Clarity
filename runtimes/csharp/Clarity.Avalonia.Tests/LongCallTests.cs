// A long engine call on Avalonia (DESIGN 5.12): while a handler's engine call
// runs on another thread, the window keeps painting, shows how far it is with
// a bar and a Stop button, and takes no other handler.

using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using Avalonia.LogicalTree;
using Avalonia.Threading;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class LongCallTests
{
    [AvaloniaFact]
    public void A_long_call_shows_a_bar_and_stops()
    {
        using var db = Sqlite.Memory();
        var vm = new CardViewModel(db);
        var list = new MessageList { Messages = vm.Messages };
        var window = new Window { Content = list };
        window.Show();
        ProgressBar? bar = null;
        Button? stop = null;
        var stopped = false;
        var otherRan = false;
        var handled = vm.Handle(() =>
        {
            // As an engine call does: it reports from its own thread and the screen waits its own way.
            var watch = Watching.Current!;
            var call = Task.Run(() =>
            {
                watch.Progress!(30, 120);
                while (!watch.Stop.IsCancellationRequested)
                    Thread.Sleep(5);
                stopped = true;
            });
            Dispatcher.UIThread.Post(() =>
            {
                // While it waits, the window paints the bar, and takes no other handler.
                Dispatcher.UIThread.RunJobs();
                bar = list.GetLogicalDescendants().OfType<ProgressBar>().FirstOrDefault();
                stop = list.GetLogicalDescendants().OfType<Button>().FirstOrDefault(b => b.Content as string == "Stop");
                otherRan = vm.Handle(() => { });
                // The person presses Stop.
                stop?.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            }, DispatcherPriority.Background);
            watch.Wait!(call);
        });
        Assert.True(handled);
        Assert.True(stopped);
        Assert.NotNull(bar);
        Assert.Equal(30, bar!.Value);
        Assert.Equal(120, bar.Maximum);
        Assert.NotNull(stop);
        Assert.False(otherRan);
        // Once the handler ended, the bar is gone.
        Dispatcher.UIThread.RunJobs();
        Assert.Empty(list.GetLogicalDescendants().OfType<ProgressBar>());
        window.Close();
    }
}
