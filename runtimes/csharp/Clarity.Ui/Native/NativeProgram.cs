// The program handed to the library (ClarityProgram): one callback by
// command, which the library calls from its screen's thread and from the
// threads that run engine frames. The C# side behind it is held by a GC
// handle until the library lets it go (release). Nothing thrown crosses
// into native code: a failure answers as an error the screen shows.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;

namespace Clarity.Ui.Native;

/// <summary>What answers the library's commands: <c>{"value": ...}</c>, <c>{"error": text}</c>, or none for the program's default.</summary>
public delegate JsonObject? ProgramAnswers(string command, string args);

internal sealed unsafe class NativeProgram : IDisposable
{
    nint handle;

    public NativeProgram(ProgramAnswers answers)
    {
        var held = GCHandle.Alloc(answers);
        var callbacks = new ProgramCallbacks
        {
            User = GCHandle.ToIntPtr(held),
            Call = &Call,
            Release = &Release,
        };
        var status = Lib.ProgramNew(&callbacks, out handle);
        if (status != Status.Ok)
        {
            held.Free();
            Interop.Check(status, "clarity_program_new");
        }
    }

    public nint Handle => handle;

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Call(nint user, byte* command, nuint commandLen, byte* args, nuint argsLen, nint answer)
    {
        try
        {
            var answers = (ProgramAnswers)GCHandle.FromIntPtr(user).Target!;
            var name = Interop.Read(command, commandLen);
            if (answers(name, Interop.Read(args, argsLen)) is { } answered)
                Interop.Answer(answer, answered.ToJsonString());
        }
        catch (Exception e)
        {
            // Fail safe: the library shows the error, and the program goes on.
            try
            {
                Interop.Answer(answer, new JsonObject { ["error"] = e.Message }.ToJsonString());
            }
            catch (Exception)
            {
                // Nothing more can be said through this answer.
            }
        }
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    static void Release(nint user)
    {
        try
        {
            GCHandle.FromIntPtr(user).Free();
        }
        catch (Exception)
        {
            // A handle freed twice is gone already.
        }
    }

    public void Dispose()
    {
        if (handle == 0)
            return;
        Lib.ProgramFree(handle);
        handle = 0;
    }
}
