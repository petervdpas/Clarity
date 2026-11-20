// A PDF of a report of the running program (`pdf of Invoice with order = current`,
// docs/ledger/reports-pdf.md): Clarity.UI lays it out with the report's data, by
// the names of its Data, and the program's texts and resources.

using System.Text;
using Clarity.Runtime;
using Clarity.Ui.Native;

namespace Clarity.Ui;

/// <summary>The program's reports as PDFs.</summary>
public static unsafe class Reports
{
    /// <summary>A PDF of the report <paramref name="report"/> with its data as JSON by its Data's names.</summary>
    public static byte[] Pdf(string report, string data)
    {
        var (r, d) = (Encoding.UTF8.GetBytes(report), Encoding.UTF8.GetBytes(data));
        byte* made;
        nuint length;
        Status status;
        fixed (byte* rp = r, dp = d)
            status = Lib.ReportPdf(rp, (nuint)r.Length, dp, (nuint)d.Length, out made, out length);
        if (status != Status.Ok)
            throw new InvalidValueException(Interop.Take(Lib.LastError()) ?? "the report's PDF could not be made");
        try
        {
            return new ReadOnlySpan<byte>(made, checked((int)length)).ToArray();
        }
        finally
        {
            Lib.BytesFree(made, length);
        }
    }
}
