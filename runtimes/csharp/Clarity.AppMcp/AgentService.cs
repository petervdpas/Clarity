// A program's commands for AI agents (docs/ledger/app-mcp.md) through the
// native Clarity.AppMcp library (clarity_app_mcp.h), the one service every
// stack calls: `<program> mcp` serves the tool list over standard input and
// output; each call runs as the command line it becomes (Agents.RunCall),
// and while the app is closed the person's settings allow, else nobody is
// asked and the call is refused.

using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.AppMcp;

/// <summary>Where the messages come from and go: <paramref name="Read"/> gives the next line, or null at the end.</summary>
public sealed record AgentLines(Func<string?> Read, Action<string> Write);

/// <summary>The MCP service of a command-line program.</summary>
public static unsafe partial class AgentService
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Api
    {
        public uint Interface;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, Host*, int> Serve;
        public delegate* unmanaged<nint, byte*, nuint, void> Printed;
        public delegate* unmanaged<nint, byte*, nuint, void> Refused;
        public delegate* unmanaged<nint, byte*, nuint, byte*, nuint, byte**, nuint*, int> Ask;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte*, nuint, byte**, nuint*, int> RunningWrite;
        public delegate* unmanaged<byte*, nuint, byte**, nuint*, int> RunningRead;
        public delegate* unmanaged<byte*, nuint, void> RunningRemove;
        public delegate* unmanaged<byte*, nuint, void> Free;
        public delegate* unmanaged<byte*, nuint, int> Problem;
        public delegate* unmanaged<nint, byte**, nuint*, int> CommandLine;
        public delegate* unmanaged<nint, byte*, nuint, byte*, nuint, byte**, nuint*, int> AskCli;
        public delegate* unmanaged<byte*, nuint, int> Forward;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Host
    {
        public nint User;
        public delegate* unmanaged<nint, byte**, nuint*, int> Read;
        public delegate* unmanaged<nint, byte*, nuint, void> Write;
        public delegate* unmanaged<nint, nint, byte*, nuint, byte*, nuint, void> Run;
        public delegate* unmanaged<nint, byte*, nuint, int> Grant;
    }

    private const int Ok = 0;
    private const int Refuse = 2;

    [LibraryImport("clarity_app_mcp", EntryPoint = "clarity_app_mcp_api")]
    private static partial Api* TableOf();

    private static readonly Lazy<nint> Table = new(() => (nint)TableOf());

    private static Api* Of => (Api*)Table.Value;

    /// <summary>What a serve runs with.</summary>
    private sealed class Serving(CliProgram program, AgentLines? lines, Func<CliSession, int, IReadOnlyList<string?>, int> run)
    {
        public readonly CliProgram Program = program;
        public readonly AgentLines? Lines = lines;
        public readonly Func<CliSession, int, IReadOnlyList<string?>, int> Run = run;
        /// <summary>The line read last, kept until the next read.</summary>
        public nint Line;
    }

    private static Serving Of_(nint user) => (Serving)GCHandle.FromIntPtr(user).Target!;

    /// <summary>What the library made, given back to it.</summary>
    private static string Taken(byte* made, nuint length)
    {
        var text = Encoding.UTF8.GetString(made, (int)length);
        Of->Free(made, length);
        return text;
    }

    private static void Answer(nint call, string text, bool refused)
    {
        var bytes = Encoding.UTF8.GetBytes(text.TrimEnd('\n', ' '));
        fixed (byte* at = bytes)
        {
            if (refused)
                Of->Refused(call, at, (nuint)bytes.Length);
            else
                Of->Printed(call, at, (nuint)bytes.Length);
        }
    }

    private static string? AskOf(nint call, string question, string form)
    {
        byte[] q = Encoding.UTF8.GetBytes(question), f = Encoding.UTF8.GetBytes(form);
        byte* made = null;
        nuint length = 0;
        fixed (byte* qp = q, fp = f)
        {
            if (Of->AskCli(call, qp, (nuint)q.Length, fp, (nuint)f.Length, &made, &length) != Ok)
                return null;
        }
        return Taken(made, length);
    }

    [UnmanagedCallersOnly]
    private static void RunCall(nint user, nint call, byte* tool, nuint toolLength, byte* arguments, nuint argumentsLength)
    {
        try
        {
            var serving = Of_(user);
            byte* made = null;
            nuint length = 0;
            if (Of->CommandLine(call, &made, &length) != Ok)
            {
                Answer(call, "", true);
                return;
            }
            var line = JsonSerializer.Deserialize<List<string>>(Taken(made, length)) ?? [];
            var (text, refused) = Agents.RunCall(serving.Program, line, (question, form) => AskOf(call, question, form), serving.Run);
            Answer(call, text, refused);
        }
        catch (Exception e)
        {
            // No exception crosses into the library.
            Answer(call, e.Message, true);
        }
    }

    /// <summary>While the app is closed nobody is there to ask.</summary>
    [UnmanagedCallersOnly]
    private static int Grant(nint user, byte* grant, nuint length) => Refuse;

    [UnmanagedCallersOnly]
    private static int ReadLine(nint user, byte** line, nuint* length)
    {
        var serving = Of_(user);
        if (serving.Line != 0)
        {
            Marshal.FreeHGlobal(serving.Line);
            serving.Line = 0;
        }
        if (serving.Lines!.Read() is not { } next)
            return 1;
        var bytes = Encoding.UTF8.GetBytes(next);
        serving.Line = Marshal.AllocHGlobal(Math.Max(bytes.Length, 1));
        Marshal.Copy(bytes, 0, serving.Line, bytes.Length);
        *line = (byte*)serving.Line;
        *length = (nuint)bytes.Length;
        return 0;
    }

    [UnmanagedCallersOnly]
    private static void WriteLine(nint user, byte* line, nuint length) =>
        Of_(user).Lines!.Write(Encoding.UTF8.GetString(line, (int)length));

    /// <summary>Forwards standard input and output to the running app that running.json at <paramref name="file"/>
    /// names, until either ends; false when none answers.</summary>
    public static bool Forward(string file)
    {
        var bytes = Encoding.UTF8.GetBytes(file);
        fixed (byte* at = bytes)
            return Of->Forward(at, (nuint)bytes.Length) == 1;
    }

    /// <summary>Serves <paramref name="program"/>'s commands for agents until the lines end (standard input
    /// and output when <paramref name="lines"/> is null), with the person's settings <paramref name="kept"/>.</summary>
    public static void Serve(CliProgram program, Agents agents, JsonObject kept, AgentLines? lines, Func<CliSession, int, IReadOnlyList<string?>, int> run)
    {
        var serving = new Serving(program, lines, run);
        var handle = GCHandle.Alloc(serving);
        try
        {
            var host = new Host
            {
                User = GCHandle.ToIntPtr(handle),
                Read = lines is null ? null : &ReadLine,
                Write = lines is null ? null : &WriteLine,
                Run = &RunCall,
                Grant = &Grant,
            };
            byte[] tools = Encoding.UTF8.GetBytes(agents.Tools), options = Encoding.UTF8.GetBytes(agents.Options(kept));
            int code;
            fixed (byte* tp = tools, op = options)
                code = Of->Serve(tp, (nuint)tools.Length, op, (nuint)options.Length, &host);
            if (code != Ok)
            {
                var into = stackalloc byte[1024];
                Of->Problem(into, 1024);
                throw new InvalidOperationException(Marshal.PtrToStringUTF8((nint)into));
            }
        }
        finally
        {
            if (serving.Line != 0)
                Marshal.FreeHGlobal(serving.Line);
            handle.Free();
        }
    }

    /// <summary>Runs a command-line program whose commands agents may call: <c>mcp</c> as the only word
    /// serves them over standard input and output; anything else is the command line as
    /// <see cref="Cli.RunProgram"/> reads it.</summary>
    public static int RunProgram(CliProgram program, Agents agents, string[] args, Func<CliSession, int, IReadOnlyList<string?>, int> run)
    {
        if (args is not ["mcp"])
            return Cli.RunProgram(program, args, run);
        // The running app answers agents itself, granting each change.
        if (KeptSettings.File(agents.Program) is { } settings && Forward(Path.Combine(Path.GetDirectoryName(settings)!, "running.json")))
            return 0;
        try
        {
            Serve(program, agents, KeptSettings.Read(agents.Program), null, run);
            return 0;
        }
        catch (InvalidOperationException e)
        {
            Console.Error.WriteLine(e.Message);
            return 1;
        }
    }
}
