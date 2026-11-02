// The system's own file dialogs on the desktop (DESIGN 7.9): choose a file to
// open, where a copy is saved, a folder; and a file opened in the program the
// system uses for its kind. Each system its own way, with no package of its
// own: Linux asks the desktop portal over D-Bus
// (org.freedesktop.portal.FileChooser), then zenity, then kdialog; macOS
// asks osascript (AppleScript's choose file, choose file name, choose
// folder); Windows shows the common item dialog through COM. The dialog
// waits off the screen's thread, and what the person chose comes back on it
// (Later), so the window goes on drawing while the dialog is open.

using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using Clarity.Runtime;

namespace Clarity.Ui.Desktop;

public sealed class SystemFilePicker : IFilePicker
{
    /// <summary>What a dialog chooses.</summary>
    public enum Kind
    {
        Open,
        Save,
        Folder,
    }

    /// <summary>A dialog to show: what it chooses, its title, the name it suggests (saving) and the kinds of file it offers (opening).</summary>
    public sealed record Ask(Kind Kind, string Title, string? Name = null, FileFilter? Filter = null);

    /// <summary>How a system shows its dialogs, in the order they are tried.</summary>
    public enum Way
    {
        Portal,
        Commands,
        CommonItemDialog,
    }

    /// <summary>The ways <paramref name="os"/> shows its dialogs, the first that answers wins.</summary>
    public static IReadOnlyList<Way> Ways(OSPlatform os) =>
        os == OSPlatform.Windows ? [Way.CommonItemDialog]
        : os == OSPlatform.OSX ? [Way.Commands]
        : [Way.Portal, Way.Commands];

    readonly Func<Ask, string?> choose;
    readonly Action<SystemAddress.Command> run;
    readonly Action<Action> post;

    /// <summary>The dialogs of the system this runs on; answers come back through Later.</summary>
    public SystemFilePicker()
        : this(ask => Choose(SystemAddress.Current, ask), Start, Later.Post)
    {
    }

    /// <summary>
    /// A picker whose dialogs <paramref name="choose"/> answers, whose
    /// programs <paramref name="run"/> starts, and whose answers
    /// <paramref name="post"/> brings to the screen's thread: a test's.
    /// </summary>
    internal SystemFilePicker(Func<Ask, string?> choose, Action<SystemAddress.Command> run, Action<Action> post)
    {
        this.choose = choose;
        this.run = run;
        this.post = post;
    }

    public bool PicksFolders => true;

    public Task<ChosenFile?> OpenFile(string title, FileFilter? filter = null) =>
        OffScreen(() => choose(new Ask(Kind.Open, title, Filter: filter)) is { } path ? new ChosenFile(Path.GetFileName(path), File.ReadAllBytes(path)) : null);

    public Task<string?> SaveFile(string title, string name, byte[] bytes) =>
        OffScreen(() =>
        {
            if (choose(new Ask(Kind.Save, title, Name: name)) is not { } path)
                return null;
            File.WriteAllBytes(path, bytes);
            return path;
        });

    public Task<string?> ChooseFolder(string title) => OffScreen(() => choose(new Ask(Kind.Folder, title)));

    /// <summary>Opens a copy of the file, in a folder of its own among the system's temporary files, in the system's program for its kind.</summary>
    public void Show(string name, byte[] bytes)
    {
        try
        {
            var folder = Path.Combine(Path.GetTempPath(), "clarity-shown", Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(folder);
            var path = Path.Combine(folder, SafeName(name));
            File.WriteAllBytes(path, bytes);
            run(ShowCommand(SystemAddress.Current, path));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            Console.Error.WriteLine($"{name} could not be shown: {e.Message}");
        }
    }

    /// <summary>A file's name as given, without folders or characters a file name cannot have.</summary>
    internal static string SafeName(string name)
    {
        var bare = Path.GetFileName(name.Replace('\\', '/'));
        var safe = new string([.. bare.Select(c => Path.GetInvalidFileNameChars().Contains(c) || c is '/' or '\\' or ':' ? '_' : c)]);
        return safe is "" or "." or ".." ? "file" : safe;
    }

    /// <summary>What opens the file at <paramref name="path"/> in the system's program for its kind.</summary>
    public static SystemAddress.Command ShowCommand(OSPlatform os, string path) =>
        os == OSPlatform.Windows ? new(path, [], Shell: true)
        : os == OSPlatform.OSX ? new("open", [path], Shell: false)
        : new("xdg-open", [path], Shell: false);

    static void Start(SystemAddress.Command command)
    {
        try
        {
            var start = new ProcessStartInfo(command.Program) { UseShellExecute = command.Shell };
            foreach (var argument in command.Arguments)
                start.ArgumentList.Add(argument);
            using var _ = Process.Start(start);
        }
        catch (Exception e) when (e is Win32Exception or InvalidOperationException or PlatformNotSupportedException)
        {
            Console.Error.WriteLine($"{command.Program} could not be started: {e.Message}");
        }
    }

    /// <summary>Runs <paramref name="work"/> off the screen's thread; its answer, or what went wrong, comes back on the screen's thread.</summary>
    Task<T?> OffScreen<T>(Func<T?> work)
    {
        var done = new TaskCompletionSource<T?>();
        Task.Run(() =>
        {
            try
            {
                var answer = work();
                post(() => done.TrySetResult(answer));
            }
            catch (Exception e)
            {
                post(() => done.TrySetException(e));
            }
        });
        return done.Task;
    }

    // -- Asking the system -----------------------------------------------------------

    /// <summary>The path the person chose in <paramref name="os"/>'s dialog; none when they chose none, or no dialog could be shown.</summary>
    static string? Choose(OSPlatform os, Ask ask)
    {
        foreach (var way in Ways(os))
        {
            var (answered, path) = way switch
            {
                Way.Portal => Portal.Choose(ask),
                Way.CommonItemDialog when OperatingSystem.IsWindows() => CommonItemDialog.Choose(ask),
                Way.Commands => FirstCommand(os, ask),
                _ => (false, null),
            };
            if (answered)
                return path;
        }
        Console.Error.WriteLine("no file dialog could be shown on this system");
        return null;
    }

    /// <summary>The first program that shows the dialog: its path, or none when the person chose none.</summary>
    static (bool Answered, string? Path) FirstCommand(OSPlatform os, Ask ask)
    {
        foreach (var command in Commands(os, ask))
        {
            try
            {
                var start = new ProcessStartInfo(command.Program) { RedirectStandardOutput = true, UseShellExecute = false, StandardOutputEncoding = Encoding.UTF8 };
                foreach (var argument in command.Arguments)
                    start.ArgumentList.Add(argument);
                using var process = Process.Start(start);
                if (process is null)
                    continue;
                var output = process.StandardOutput.ReadToEnd();
                process.WaitForExit();
                // 0 is a choice, 1 is Cancel; anything else is a program that could not show its dialog.
                if (process.ExitCode == 0)
                    return (true, Answer(output));
                if (process.ExitCode == 1)
                    return (true, null);
            }
            catch (Win32Exception)
            {
                // The program is not installed: the next one is asked.
            }
        }
        return (false, null);
    }

    /// <summary>A program's answer as a path: its first line, a folder without the slash it ends with; none for nothing.</summary>
    internal static string? Answer(string output)
    {
        var line = output.Split('\n')[0].TrimEnd('\r');
        if (line.Length > 1 && line.EndsWith('/'))
            line = line[..^1];
        return line.Length == 0 ? null : line;
    }

    /// <summary>The programs that show the dialog on <paramref name="os"/>, in the order they are tried: zenity and kdialog on Linux, osascript on macOS; none on Windows.</summary>
    public static IReadOnlyList<SystemAddress.Command> Commands(OSPlatform os, Ask ask)
    {
        if (os == OSPlatform.Windows)
            return [];
        if (os == OSPlatform.OSX)
            return [new("osascript", ["-e", AppleScript(ask)], Shell: false)];
        return [Zenity(ask), KDialog(ask)];
    }

    static SystemAddress.Command Zenity(Ask ask)
    {
        List<string> args = ["--file-selection", $"--title={ask.Title}"];
        switch (ask.Kind)
        {
            case Kind.Save:
                args.AddRange(["--save", "--confirm-overwrite", $"--filename={ask.Name}"]);
                break;
            case Kind.Folder:
                args.Add("--directory");
                break;
            default:
                if (ask.Filter is { } filter)
                    args.Add($"--file-filter={filter.Name} | {string.Join(' ', filter.Patterns)}");
                break;
        }
        return new("zenity", args, Shell: false);
    }

    static SystemAddress.Command KDialog(Ask ask)
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        List<string> args = ["--title", ask.Title];
        switch (ask.Kind)
        {
            case Kind.Save:
                args.AddRange(["--getsavefilename", Path.Combine(home, ask.Name ?? "")]);
                break;
            case Kind.Folder:
                args.AddRange(["--getexistingdirectory", home]);
                break;
            default:
                args.AddRange(["--getopenfilename", home]);
                if (ask.Filter is { } filter)
                    args.Add($"{filter.Name} ({string.Join(' ', filter.Patterns)})");
                break;
        }
        return new("kdialog", args, Shell: false);
    }

    /// <summary>The AppleScript of the dialog, its texts quoted as AppleScript strings.</summary>
    static string AppleScript(Ask ask)
    {
        var prompt = $"with prompt {Quoted(ask.Title)}";
        return ask.Kind switch
        {
            Kind.Save => $"POSIX path of (choose file name {prompt} default name {Quoted(ask.Name ?? "")})",
            Kind.Folder => $"POSIX path of (choose folder {prompt})",
            _ when Extensions(ask.Filter) is { Count: > 0 } kinds => $"POSIX path of (choose file {prompt} of type {{{string.Join(", ", kinds.Select(Quoted))}}})",
            _ => $"POSIX path of (choose file {prompt})",
        };
    }

    static string Quoted(string text) => "\"" + text.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";

    /// <summary>The extensions of a filter's patterns: <c>*.png</c> is png; a pattern of every file gives none.</summary>
    internal static List<string> Extensions(FileFilter? filter) =>
        filter is null || filter.Patterns.Any(p => p is "*" or "*.*")
            ? []
            : [.. filter.Patterns.Where(p => p.StartsWith("*.") && p.Length > 2).Select(p => p[2..])];

    // -- The desktop portal (Linux) ------------------------------------------------

    /// <summary>org.freedesktop.portal.FileChooser over the session's D-Bus: the dialog of the desktop the person runs, in a sandbox too.</summary>
    static class Portal
    {
        const string Bus = "org.freedesktop.portal.Desktop";
        const string ObjectPath = "/org/freedesktop/portal/desktop";
        const string Chooser = "org.freedesktop.portal.FileChooser";

        public static (bool Answered, string? Path) Choose(Ask ask)
        {
            try
            {
                return Ask(ask).GetAwaiter().GetResult();
            }
            catch (Exception e) when (e is not OutOfMemoryException)
            {
                // No session bus, or no portal on it: the next way is tried.
                Console.Error.WriteLine($"the desktop portal showed no file dialog: {e.Message}");
                return (false, null);
            }
        }

        static async Task<(bool, string?)> Ask(Ask ask)
        {
            var connection = Tmds.DBus.Protocol.DBusConnection.Session ?? throw new InvalidOperationException("there is no D-Bus session");
            var token = "clarity" + Guid.NewGuid().ToString("N");
            var responses = new Dictionary<string, (uint Code, string? Uri)>();
            var answered = new TaskCompletionSource<(uint Code, string? Uri)>(TaskCreationOptions.RunContinuationsAsynchronously);
            string? handle = null;
            var gate = new object();
            // Every Response of a request is heard, kept by its request until the call says which is this dialog's.
            var rule = new Tmds.DBus.Protocol.MatchRule
            {
                Type = Tmds.DBus.Protocol.MessageType.Signal,
                Interface = "org.freedesktop.portal.Request",
                Member = "Response",
            };
            using var watch = await connection.AddMatchAsync(
                rule,
                (Tmds.DBus.Protocol.Message m, object? _) =>
                {
                    var reader = m.GetBodyReader();
                    var code = reader.ReadUInt32();
                    var results = reader.ReadDictionaryOfStringToVariantValue();
                    var uri = results.TryGetValue("uris", out var uris) ? uris.GetArray<string>().FirstOrDefault() : null;
                    return (m.PathAsString ?? "", code, uri);
                },
                (Tmds.DBus.Protocol.Notification<(string Path, uint Code, string? Uri)> heard) =>
                {
                    if (!heard.HasValue)
                        return;
                    var response = heard.Value;
                    lock (gate)
                    {
                        if (handle == response.Path)
                            answered.TrySetResult((response.Code, response.Uri));
                        else
                            responses[response.Path] = (response.Code, response.Uri);
                    }
                },
                false,
                Tmds.DBus.Protocol.ObserverFlags.None,
                null);
            var request = await connection.CallMethodAsync(Message(connection, ask, token), (Tmds.DBus.Protocol.Message m, object? _) => m.GetBodyReader().ReadObjectPathAsString(), null);
            lock (gate)
            {
                handle = request;
                if (responses.TryGetValue(request, out var early))
                    answered.TrySetResult(early);
            }
            var (code, chosen) = await answered.Task;
            // 0 is a choice, 1 is Cancel; 2 is a dialog that ended another way, also no choice.
            return (true, code == 0 && chosen is { } uri && Uri.TryCreate(uri, UriKind.Absolute, out var address) && address.IsFile ? address.LocalPath : null);
        }

        static Tmds.DBus.Protocol.MessageBuffer Message(Tmds.DBus.Protocol.DBusConnection connection, Ask ask, string token)
        {
            using var writer = connection.GetMessageWriter();
            writer.WriteMethodCallHeader(Bus, ObjectPath, Chooser, ask.Kind == Kind.Save ? "SaveFile" : "OpenFile", "ssa{sv}");
            // No parent window: the system's own windows have no handle the portal knows.
            writer.WriteString("");
            writer.WriteString(ask.Title);
            var options = writer.WriteDictionaryStart();
            writer.WriteDictionaryEntryStart();
            writer.WriteString("handle_token");
            writer.WriteVariantString(token);
            writer.WriteDictionaryEntryStart();
            writer.WriteString("modal");
            writer.WriteVariantBool(true);
            if (ask.Kind == Kind.Folder)
            {
                writer.WriteDictionaryEntryStart();
                writer.WriteString("directory");
                writer.WriteVariantBool(true);
            }
            if (ask.Kind == Kind.Save)
            {
                writer.WriteDictionaryEntryStart();
                writer.WriteString("current_name");
                writer.WriteVariantString(ask.Name ?? "");
            }
            if (ask.Kind == Kind.Open && ask.Filter is { } filter)
            {
                // filters: a(sa(us)), each a name and its patterns (0 a glob).
                writer.WriteDictionaryEntryStart();
                writer.WriteString("filters");
                writer.WriteSignature("a(sa(us))");
                var filters = writer.WriteArrayStart(Tmds.DBus.Protocol.DBusType.Struct);
                writer.WriteStructureStart();
                writer.WriteString(filter.Name);
                var patterns = writer.WriteArrayStart(Tmds.DBus.Protocol.DBusType.Struct);
                foreach (var pattern in filter.Patterns)
                {
                    writer.WriteStructureStart();
                    writer.WriteUInt32(0);
                    writer.WriteString(pattern);
                }
                writer.WriteArrayEnd(patterns);
                writer.WriteArrayEnd(filters);
            }
            writer.WriteDictionaryEnd(options);
            return writer.CreateMessage();
        }
    }

    // -- The common item dialog (Windows) ------------------------------------------

    /// <summary>Windows' open, save and folder dialog (IFileDialog), on a thread of its own as COM wants.</summary>
    [System.Runtime.Versioning.SupportedOSPlatform("windows")]
    static class CommonItemDialog
    {
        static readonly Guid OpenDialog = new("DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7");
        static readonly Guid SaveDialog = new("C0B4E2F3-BA21-4773-8DBA-335EC946EB8B");

        const uint OverwritePrompt = 0x2;
        const uint PickFolders = 0x20;
        const uint ForceFileSystem = 0x40;
        const uint PathMustExist = 0x800;
        const uint FileMustExist = 0x1000;
        const uint FileSystemPath = 0x80058000;
        const int Cancelled = unchecked((int)0x800704C7);

        public static (bool Answered, string? Path) Choose(Ask ask)
        {
            (bool, string?) answer = (false, null);
            var thread = new Thread(() =>
            {
                try
                {
                    answer = Show(ask);
                }
                catch (Exception e) when (e is COMException or InvalidCastException or TypeLoadException)
                {
                    Console.Error.WriteLine($"the file dialog could not be shown: {e.Message}");
                }
            });
            thread.SetApartmentState(ApartmentState.STA);
            thread.Start();
            thread.Join();
            return answer;
        }

        static (bool, string?) Show(Ask ask)
        {
            var type = Type.GetTypeFromCLSID(ask.Kind == Kind.Save ? SaveDialog : OpenDialog, throwOnError: true)!;
            var dialog = (IFileDialog)Activator.CreateInstance(type)!;
            try
            {
                dialog.GetOptions(out var options);
                options |= ForceFileSystem | PathMustExist;
                options |= ask.Kind switch
                {
                    Kind.Save => OverwritePrompt,
                    Kind.Folder => PickFolders,
                    _ => FileMustExist,
                };
                dialog.SetOptions(options);
                dialog.SetTitle(ask.Title);
                if (ask.Kind == Kind.Save)
                    dialog.SetFileName(ask.Name ?? "");
                if (ask.Kind == Kind.Open && ask.Filter is { } filter)
                    dialog.SetFileTypes(1, [new FilterSpec { Name = filter.Name, Spec = string.Join(';', filter.Patterns) }]);
                var shown = dialog.Show(IntPtr.Zero);
                if (shown == Cancelled)
                    return (true, null);
                Marshal.ThrowExceptionForHR(shown);
                dialog.GetResult(out var item);
                try
                {
                    item.GetDisplayName(FileSystemPath, out var name);
                    try
                    {
                        return (true, Marshal.PtrToStringUni(name));
                    }
                    finally
                    {
                        Marshal.FreeCoTaskMem(name);
                    }
                }
                finally
                {
                    Marshal.ReleaseComObject(item);
                }
            }
            finally
            {
                Marshal.ReleaseComObject(dialog);
            }
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        struct FilterSpec
        {
            [MarshalAs(UnmanagedType.LPWStr)]
            public string Name;

            [MarshalAs(UnmanagedType.LPWStr)]
            public string Spec;
        }

        [ComImport]
        [Guid("42f85136-db7e-439c-85f1-e4075d135fc8")]
        [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        interface IFileDialog
        {
            [PreserveSig]
            int Show(IntPtr parent);

            void SetFileTypes(uint count, [MarshalAs(UnmanagedType.LPArray)] FilterSpec[] types);

            void SetFileTypeIndex(uint index);

            void GetFileTypeIndex(out uint index);

            void Advise(IntPtr events, out uint cookie);

            void Unadvise(uint cookie);

            void SetOptions(uint options);

            void GetOptions(out uint options);

            void SetDefaultFolder(IShellItem folder);

            void SetFolder(IShellItem folder);

            void GetFolder(out IShellItem folder);

            void GetCurrentSelection(out IShellItem item);

            void SetFileName([MarshalAs(UnmanagedType.LPWStr)] string name);

            void GetFileName(out IntPtr name);

            void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string title);

            void SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string text);

            void SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string label);

            void GetResult(out IShellItem item);
        }

        [ComImport]
        [Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe")]
        [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        interface IShellItem
        {
            void BindToHandler(IntPtr context, ref Guid handler, ref Guid iid, out IntPtr result);

            void GetParent(out IShellItem parent);

            void GetDisplayName(uint kind, out IntPtr name);
        }
    }
}
