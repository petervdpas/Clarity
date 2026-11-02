// The system's own event for a change of its light or dark choice, which
// SystemTheme.Follow listens to instead of reading the choice every few
// seconds. On Linux the desktop portal's SettingChanged signal for
// org.freedesktop.appearance color-scheme, over the session's D-Bus. On
// Windows a change of the registry value AppsUseLightTheme, the same change
// that sends WM_SETTINGCHANGE "ImmersiveColorSet" to every window, heard
// without a window procedure of its own. macOS gives none here, so its
// choice is read every few seconds.

using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace Clarity.Ui.Desktop;

static class SystemThemeEvents
{
    /// <summary>Listens to the system's change event, calling <paramref name="heard"/> each time; none where the system has none.</summary>
    public static IDisposable? Listen(Action heard)
    {
        if (OperatingSystem.IsLinux())
            return Portal(heard);
        if (OperatingSystem.IsWindows())
            return Registry(heard);
        return null;
    }

    // -- Linux: the desktop portal ---------------------------------------------------

    /// <summary>The portal's SettingChanged signals of the color scheme; none when the session has no bus within a few seconds.</summary>
    static IDisposable? Portal(Action heard)
    {
        // No session bus at all (a bare X server, a service): nothing to listen to.
        var runtime = Environment.GetEnvironmentVariable("XDG_RUNTIME_DIR");
        if (string.IsNullOrEmpty(Environment.GetEnvironmentVariable("DBUS_SESSION_BUS_ADDRESS"))
            && (string.IsNullOrEmpty(runtime) || !File.Exists(Path.Combine(runtime, "bus"))))
            return null;
        var connection = Tmds.DBus.Protocol.DBusConnection.Session;
        if (connection is null)
            return null;
        var rule = new Tmds.DBus.Protocol.MatchRule
        {
            Type = Tmds.DBus.Protocol.MessageType.Signal,
            Interface = "org.freedesktop.portal.Settings",
            Member = "SettingChanged",
            Path = "/org/freedesktop/portal/desktop",
        };
        var adding = connection.AddMatchAsync(
            rule,
            (Tmds.DBus.Protocol.Message m, object? _) =>
            {
                var reader = m.GetBodyReader();
                return (Namespace: reader.ReadString(), Key: reader.ReadString());
            },
            (Tmds.DBus.Protocol.Notification<(string Namespace, string Key)> told) =>
            {
                if (told.HasValue && told.Value is ("org.freedesktop.appearance", "color-scheme"))
                    heard();
            },
            false,
            Tmds.DBus.Protocol.ObserverFlags.None,
            null).AsTask();
        if (!adding.Wait(TimeSpan.FromSeconds(3)))
        {
            // A bus that does not answer: the choice is read every few seconds instead.
            _ = adding.ContinueWith(t => t.Result.Dispose(), TaskContinuationOptions.OnlyOnRanToCompletion);
            return null;
        }
        return adding.Result;
    }

    // -- Windows: the registry value -------------------------------------------------

    const string Personalize = @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    static readonly IntPtr CurrentUser = unchecked((IntPtr)(int)0x80000001);
    const int Read = 0x20019;
    const int ChangeLastSet = 0x4;

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
    static extern int RegOpenKeyEx(IntPtr key, string subKey, int options, int access, out IntPtr result);

    [DllImport("advapi32.dll")]
    static extern int RegNotifyChangeKeyValue(IntPtr key, bool watchSubtree, int filter, SafeWaitHandle changed, bool asynchronous);

    [DllImport("advapi32.dll")]
    static extern int RegCloseKey(IntPtr key);

    /// <summary>A thread of its own that waits for the Personalize key to change, until disposed; none when the key cannot be opened.</summary>
    static IDisposable? Registry(Action heard)
    {
        if (RegOpenKeyEx(CurrentUser, Personalize, 0, Read, out var key) != 0)
            return null;
        var changed = new AutoResetEvent(false);
        var stop = new ManualResetEvent(false);
        var thread = new Thread(() =>
        {
            try
            {
                // The notice is for one change: it is asked for again after each.
                while (RegNotifyChangeKeyValue(key, false, ChangeLastSet, changed.SafeWaitHandle, true) == 0
                    && WaitHandle.WaitAny([changed, stop]) == 0)
                    heard();
            }
            finally
            {
                RegCloseKey(key);
            }
        })
        { IsBackground = true, Name = "Clarity system theme" };
        thread.Start();
        return new Stop(() => stop.Set());
    }

    sealed class Stop(Action stop) : IDisposable
    {
        public void Dispose() => stop();
    }
}
