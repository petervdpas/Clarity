// The window model's "once the work at hand is done" on Avalonia: its UI
// thread, after what it is doing (Clarity.Ui' Later); and its wait
// for a long engine call: a frame of its own, so the window keeps painting.

using System.Runtime.CompilerServices;
using Avalonia.Threading;
using Clarity.Ui;

namespace Clarity.Avalonia;

static class LaterOnAvalonia
{
    // Set as the library loads, before any window model runs, whichever program (desktop, browser, tests) loads it.
#pragma warning disable CA2255
    [ModuleInitializer]
#pragma warning restore CA2255
    internal static void Start()
    {
        Later.Poster = action => Dispatcher.UIThread.Post(action, DispatcherPriority.Background);
        LongCall.Waiter = call =>
        {
            var frame = new DispatcherFrame();
            call.ContinueWith(_ => Dispatcher.UIThread.Post(() => frame.Continue = false), TaskScheduler.Default);
            Dispatcher.UIThread.PushFrame(frame);
        };
    }
}
