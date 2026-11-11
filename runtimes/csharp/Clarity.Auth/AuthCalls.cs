// The sign-in of the programs Clarity generates (docs/APP-CAPABILITIES-PLAN.md,
// step 4) through the native Clarity.Auth library (clarity_auth.h), the one
// implementation every stack calls: one operation by name, its arguments and
// result as JSON.

using System.Runtime.InteropServices;
using System.Text;
using Clarity.Runtime;

namespace Clarity.Auth;

/// <summary>The operations of Clarity.Auth.</summary>
public static unsafe partial class AuthCalls
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Api
    {
        public uint Interface;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte**, nuint*, int> Call;
        public delegate* unmanaged<byte*, nuint, void> Free;
        public delegate* unmanaged<byte*, nuint, int> Problem;
    }

    [LibraryImport("clarity_auth", EntryPoint = "clarity_auth_api")]
    private static partial Api* TableOf();

    private static readonly Lazy<nint> Table = new(() => (nint)TableOf());

    private static Api* Of => (Api*)Table.Value;

    private static string Problem()
    {
        var into = stackalloc byte[1024];
        Of->Problem(into, 1024);
        return Marshal.PtrToStringUTF8((nint)into) ?? "Clarity.Auth could not do it";
    }

    /// <summary>The operation <paramref name="op"/> with its arguments as a JSON object; the result as JSON.</summary>
    public static string Call(string op, string args)
    {
        byte[] o = Encoding.UTF8.GetBytes(op), a = Encoding.UTF8.GetBytes(args);
        byte* made = null;
        nuint length = 0;
        int code;
        fixed (byte* op_ = o, ap = a)
        {
            code = Of->Call(op_, (nuint)o.Length, ap, (nuint)a.Length, &made, &length);
        }
        if (code != 0)
            throw new ValueException(Problem());
        try
        {
            return Encoding.UTF8.GetString(made, (int)length);
        }
        finally
        {
            Of->Free(made, length);
        }
    }
}
