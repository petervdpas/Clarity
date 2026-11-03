// A program driven without a window (DESIGN 1.6, clarity_driven_*): the
// library shows the program's windows on a screen with no display and takes
// steps as JSON (`{"do":"click","part":"save"}`), answering each with what
// shows. Tests use it to drive a C# window model through the library as a
// person would.

using System.Text.Json.Nodes;

namespace Clarity.Ui.Native;

internal sealed unsafe class Driven : IDisposable
{
    readonly NativeProgram program;
    readonly NativeFiles files;
    nint handle;

    public Driven(ProgramAnswers answers, NativeFiles files)
    {
        program = new NativeProgram(answers);
        this.files = files;
        var status = Lib.DrivenStart(program.Handle, files.Handle, out handle);
        if (status != Status.Ok)
        {
            program.Dispose();
            Interop.Check(status, "clarity_driven_start");
        }
    }

    /// <summary>Does one step and answers what the library said of it.</summary>
    public JsonObject Step(JsonObject step)
    {
        var s = Interop.Utf8(step.ToJsonString());
        byte* answer;
        fixed (byte* sp = s)
            Interop.Check(Lib.DrivenStep(handle, sp, (nuint)s.Length, out answer), "clarity_driven_step");
        return JsonNode.Parse(Interop.Take(answer) ?? "{}") as JsonObject ?? [];
    }

    public void Dispose()
    {
        if (handle != 0)
        {
            Lib.DrivenFree(handle);
            handle = 0;
        }
        program.Dispose();
        files.Dispose();
    }
}
