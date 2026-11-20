// The PDFs of code (docs/ledger/reports-pdf.md) through the native Clarity.Pdf
// library (clarity_pdf.h): `pdf of text stylesheet css theme true`, a Markdown
// text laid out by Typst in a stylesheet's look, the same file on every stack.

using System.Runtime.InteropServices;
using System.Text;
using Clarity.Runtime;

namespace Clarity.Pdf;

/// <summary>The PDFs of code.</summary>
public static unsafe partial class PdfFunctions
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Api
    {
        public uint Interface;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte*, nuint, byte**, nuint*, int> Pdf;
        public delegate* unmanaged<byte*, nuint, void> Free;
        public delegate* unmanaged<byte*, nuint, int> Problem;
    }

    [LibraryImport("clarity_pdf", EntryPoint = "clarity_pdf_api")]
    private static partial Api* TableOf();

    private static readonly Lazy<nint> Table = new(() => (nint)TableOf());

    private static Api* Native => (Api*)Table.Value;

    /// <summary>Why the last call failed, in Clarity.Pdf's words.</summary>
    private static InvalidValueException Failed()
    {
        var into = stackalloc byte[2048];
        Native->Problem(into, 2048);
        return new InvalidValueException(Marshal.PtrToStringUTF8((nint)into) ?? "the PDF could not be made");
    }

    /// <summary><c>pdf of text stylesheet css theme true</c>: a PDF of a Markdown text in a
    /// stylesheet's look (empty for Clarity's own), reading a theme file's JSON (empty for none).</summary>
    public static byte[] Of(string text, string stylesheet, string theme)
    {
        var (t, s, th) = (Encoding.UTF8.GetBytes(text), Encoding.UTF8.GetBytes(stylesheet), Encoding.UTF8.GetBytes(theme));
        byte* made = null;
        nuint length = 0;
        fixed (byte* tp = t, sp = s, thp = th)
        {
            if (Native->Pdf(tp, (nuint)t.Length, sp, (nuint)s.Length, thp, (nuint)th.Length, &made, &length) != 0) throw Failed();
        }
        try
        {
            return new ReadOnlySpan<byte>(made, checked((int)length)).ToArray();
        }
        finally
        {
            Native->Free(made, length);
        }
    }
}
