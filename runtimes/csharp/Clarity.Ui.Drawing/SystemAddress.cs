// Opening an address in the system's own program for it (a link's web page
// in the browser, a mailto: in the mail program), as the desktop host does:
// xdg-open on Linux, open on macOS, the shell on Windows. Only addresses a
// link opens are passed on: web pages, mail and phone numbers.

using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Clarity.Ui;

public static class SystemAddress
{
    /// <summary>What runs to open an address: a program and its arguments, or the address itself through the system's shell.</summary>
    public sealed record Command(string Program, IReadOnlyList<string> Arguments, bool Shell);

    /// <summary>The schemes an address may have to be opened.</summary>
    static readonly string[] Schemes = ["http:", "https:", "mailto:", "tel:"];

    /// <summary>Whether <paramref name="address"/> is one the system is asked to open.</summary>
    public static bool Opens(string address) =>
        Schemes.Any(s => address.StartsWith(s, StringComparison.OrdinalIgnoreCase)) && !address.Any(char.IsControl);

    /// <summary>What opens <paramref name="address"/> on the system <paramref name="os"/>; none for an address it is not asked to open.</summary>
    public static Command? CommandFor(OSPlatform os, string address)
    {
        if (!Opens(address))
            return null;
        if (os == OSPlatform.Windows)
            return new Command(address, [], Shell: true);
        if (os == OSPlatform.OSX)
            return new Command("open", [address], Shell: false);
        return new Command("xdg-open", [address], Shell: false);
    }

    /// <summary>The system this runs on.</summary>
    public static OSPlatform Current =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? OSPlatform.Windows
        : RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? OSPlatform.OSX
        : OSPlatform.Linux;

    /// <summary>Opens <paramref name="address"/> in the system's program for it; one it may not open, or a program that fails, is only said.</summary>
    public static void Open(string address)
    {
        if (CommandFor(Current, address) is not { } command)
        {
            Console.Error.WriteLine($"an address that is opened has no scheme of a web page, mail or phone: {address}");
            return;
        }
        try
        {
            var start = new ProcessStartInfo(command.Program) { UseShellExecute = command.Shell };
            foreach (var argument in command.Arguments)
                start.ArgumentList.Add(argument);
            using var _ = Process.Start(start);
        }
        catch (Exception e) when (e is System.ComponentModel.Win32Exception or InvalidOperationException or PlatformNotSupportedException)
        {
            Console.Error.WriteLine($"{address} could not be opened: {e.Message}");
        }
    }
}
