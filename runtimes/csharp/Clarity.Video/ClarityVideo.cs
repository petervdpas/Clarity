// Hands the native Clarity.Video library to Clarity.UI (clarity_video.h,
// clarity_ui_use_video): from then on the video parts play through it. The
// generated Program calls Use once, before its first window.

using System.Runtime.InteropServices;

namespace Clarity.Video;

/// <summary>Clarity.Video for the program's video parts.</summary>
public static partial class ClarityVideo
{
    [LibraryImport("clarity_video", EntryPoint = "clarity_video_api")]
    private static partial nint Api();

    [LibraryImport("clarity_ui", EntryPoint = "clarity_ui_use_video")]
    private static partial int UseVideo(nint api);

    /// <summary>
    /// Videos play through Clarity.Video from now on. When its library cannot
    /// load, the reason goes to the error output, video parts say so and the
    /// program goes on; the answer says whether it is in use.
    /// </summary>
    public static bool Use()
    {
        try
        {
            if (UseVideo(Api()) == 0) return true;
            Console.Error.WriteLine("Clarity.Video was not taken by Clarity.UI");
        }
        catch (Exception problem) when (problem is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException)
        {
            Console.Error.WriteLine($"Clarity.Video cannot load: {problem.Message}");
        }
        return false;
    }

    /// <summary>The functions of clarity_video.h's table before add_codec: open, free, size, length_ms, frame_ms, picture_at, ended_at, problem.</summary>
    internal const int FunctionsBeforeAddCodec = 8;

    /// <summary>
    /// Loads the codec library <paramref name="file"/> (clarity_codec.h) where
    /// Clarity carries it and registers it, so it plays the formats it takes
    /// (the table's <c>add_codec</c>, interface 2). The reason goes to the
    /// error output when it cannot; the answer says whether it was added.
    /// </summary>
    public static unsafe bool AddCodec(string file)
    {
        try
        {
            var api = Api();
            if (api == 0 || Marshal.ReadInt32(api) < 2) return false;
            // The table: its interface (4 bytes, padded to a pointer), the functions before it, then add_codec.
            var add = (delegate* unmanaged[Cdecl]<byte*, nuint, int>)Marshal.ReadIntPtr(api, IntPtr.Size + FunctionsBeforeAddCodec * IntPtr.Size);
            var bytes = System.Text.Encoding.UTF8.GetBytes(file);
            fixed (byte* at = bytes)
            {
                if (add(at, (nuint)bytes.Length) == 0) return true;
            }
            Console.Error.WriteLine($"The codec library {file} could not be added");
        }
        catch (Exception problem) when (problem is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException)
        {
            Console.Error.WriteLine($"Clarity.Video cannot load: {problem.Message}");
        }
        return false;
    }
}
