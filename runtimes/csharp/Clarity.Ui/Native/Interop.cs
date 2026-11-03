// What every call of the library shares: text in and out as UTF-8, a failed
// call as a ClarityException with the library's own message, and answers
// given back to a callback. No exception crosses into native code: a
// callback catches what it throws and answers as the library expects.

using System.Runtime.InteropServices;
using System.Text;

namespace Clarity.Ui.Native;

/// <summary>A call of the native Clarity.UI library failed: its status and the library's message.</summary>
public sealed class ClarityException(Status status, string message) : Exception(message)
{
    /// <summary>What the call answered.</summary>
    public Status Status { get; } = status;
}

internal static unsafe class Interop
{
    /// <summary>Throws the library's message when <paramref name="status"/> is no success; <paramref name="what"/> names the call.</summary>
    public static void Check(Status status, string what)
    {
        if (status == Status.Ok)
            return;
        var said = Take(Lib.LastError());
        throw new ClarityException(status, string.IsNullOrEmpty(said) ? $"{what} answered {status}" : said);
    }

    /// <summary>A string the library gave, read and freed; none for NULL.</summary>
    public static string? Take(byte* text)
    {
        if (text is null)
            return null;
        try
        {
            return Marshal.PtrToStringUTF8((nint)text);
        }
        finally
        {
            Lib.StringFree(text);
        }
    }

    /// <summary>Text the library gave as (pointer, length); empty for NULL.</summary>
    public static string Read(byte* text, nuint len) =>
        text is null || len == 0 ? "" : Encoding.UTF8.GetString(text, checked((int)len));

    /// <summary>Text the library gave as (pointer, length), none for NULL.</summary>
    public static string? ReadOrNone(byte* text, nuint len) =>
        text is null ? null : Read(text, len);

    /// <summary>Text as UTF-8 bytes for a call.</summary>
    public static byte[] Utf8(string? text) => text is null ? [] : Encoding.UTF8.GetBytes(text);

    /// <summary>Sets what a callback answers: text as UTF-8; nothing for none.</summary>
    public static void Answer(nint answer, string? text)
    {
        if (text is null || answer == 0)
            return;
        Answer(answer, Encoding.UTF8.GetBytes(text));
    }

    /// <summary>Sets what a callback answers: bytes.</summary>
    public static void Answer(nint answer, byte[] bytes)
    {
        if (answer == 0)
            return;
        fixed (byte* data = bytes)
            Lib.AnswerSet(answer, data, (nuint)bytes.Length);
    }
}
