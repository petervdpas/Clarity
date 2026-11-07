// Hands the native Clarity.Audio library to Clarity.UI (clarity_audio.h,
// clarity_ui_use_audio): from then on the sound parts' songs play through
// it. The generated Program calls Use once, before its first window.

using System.Runtime.InteropServices;

namespace Clarity.Audio;

/// <summary>Clarity.Audio for the program's sound parts.</summary>
public static partial class ClarityAudio
{
    [LibraryImport("clarity_audio", EntryPoint = "clarity_audio_api")]
    private static partial nint Api();

    [LibraryImport("clarity_ui", EntryPoint = "clarity_ui_use_audio")]
    private static partial int UseAudio(nint api);

    /// <summary>
    /// Songs play through Clarity.Audio from now on. When its library cannot
    /// load, the reason goes to the error output, songs play nothing and the
    /// program goes on; the answer says whether it is in use.
    /// </summary>
    public static bool Use()
    {
        try
        {
            if (UseAudio(Api()) == 0) return true;
            Console.Error.WriteLine("Clarity.Audio was not taken by Clarity.UI");
        }
        catch (Exception problem) when (problem is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException)
        {
            Console.Error.WriteLine($"Clarity.Audio cannot load: {problem.Message}");
        }
        return false;
    }
}
