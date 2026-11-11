// The data formats of code (docs/FORMATS-PLAN.md) through the native
// Clarity.Formats library (clarity_formats.h), the one implementation every
// stack calls: `read Order from json text` reads the format and fits it to
// the record, which Clarity's JSON then decodes; `json of value` writes the
// value as Clarity's JSON and the library writes it in the format, in
// Clarity's one form, the same bytes as every other stack.

using System.Runtime.InteropServices;
using System.Text;
using Clarity.Runtime;

namespace Clarity.Formats;

/// <summary>The data formats of code.</summary>
public static unsafe partial class FormatFunctions
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Api
    {
        public uint Interface;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte*, nuint, byte*, nuint, byte**, nuint*, int> Read;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte*, nuint, byte*, nuint, byte**, nuint*, int> Write;
        public delegate* unmanaged<byte**, nuint*, int> Catalog;
        public delegate* unmanaged<byte*, nuint, void> Free;
        public delegate* unmanaged<byte*, nuint, int> Problem;
        // Interface 2: a file of a file data source as a table.
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte*, nuint, byte*, nuint, byte**, nuint*, int> ReadTable;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, byte*, nuint, byte*, nuint, byte*, nuint, byte**, nuint*, int> WriteTable;
    }

    /// <summary>The interface whose table holds the table calls.</summary>
    private const uint TableInterface = 2;

    [LibraryImport("clarity_formats", EntryPoint = "clarity_formats_api")]
    private static partial Api* TableOf();

    private static readonly Lazy<nint> Table = new(() => (nint)TableOf());

    private static Api* Of => (Api*)Table.Value;

    /// <summary>Why the last call failed, word for word as every stack says it.</summary>
    private static string Problem()
    {
        var into = stackalloc byte[1024];
        Of->Problem(into, 1024);
        return Marshal.PtrToStringUTF8((nint)into) ?? "the format could not be read";
    }

    private static string Call(bool read, string format, string input, string options, string kind)
    {
        byte[] f = Encoding.UTF8.GetBytes(format), i = Encoding.UTF8.GetBytes(input), o = Encoding.UTF8.GetBytes(options), k = Encoding.UTF8.GetBytes(kind);
        byte* made = null;
        nuint length = 0;
        int code;
        fixed (byte* fp = f, ip = i, op = o, kp = k)
        {
            code = read
                ? Of->Read(fp, (nuint)f.Length, ip, (nuint)i.Length, op, (nuint)o.Length, kp, (nuint)k.Length, &made, &length)
                : Of->Write(fp, (nuint)f.Length, ip, (nuint)i.Length, op, (nuint)o.Length, kp, (nuint)k.Length, &made, &length);
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

    /// <summary>The library's table when it has the table calls; an older library is refused, never called past its end.</summary>
    private static Api* WithTables()
    {
        var api = Of;
        if (api->Interface < TableInterface)
            throw new ValueException($"the Clarity.Formats library is older than this program (interface {api->Interface}, needs {TableInterface})");
        return api;
    }

    /// <summary>What a call made, given back to the library; a failing call's problem thrown.</summary>
    private static byte[] Made(int code, byte* made, nuint length)
    {
        if (code != 0)
            throw new ValueException(Problem());
        try
        {
            return new ReadOnlySpan<byte>(made, (int)length).ToArray();
        }
        finally
        {
            Of->Free(made, length);
        }
    }

    /// <summary>
    /// Reads <paramref name="input"/>, a file in <paramref name="format"/> with its <paramref name="options"/>,
    /// as one table, JSON <c>{"columns":[...],"rows":[[...],...]}</c>: the file's columns in its order, the
    /// cells of the columns an entity maps (<paramref name="fields"/>, JSON <c>[{"name":"site","kind":"text"},...]</c>)
    /// fitted as SQLite keeps them.
    /// </summary>
    public static string ReadTable(string format, byte[] input, string options, string fields)
    {
        var api = WithTables();
        byte[] f = Encoding.UTF8.GetBytes(format), o = Encoding.UTF8.GetBytes(options), k = Encoding.UTF8.GetBytes(fields);
        byte* made = null;
        nuint length = 0;
        int code;
        fixed (byte* fp = f, ip = input, op = o, kp = k)
            code = api->ReadTable(fp, (nuint)f.Length, ip, (nuint)input.Length, op, (nuint)o.Length, kp, (nuint)k.Length, &made, &length);
        return Encoding.UTF8.GetString(Made(code, made, length));
    }

    /// <summary>
    /// <paramref name="table"/> (JSON, as <see cref="ReadTable"/> gives it) written back in <paramref name="format"/>:
    /// the whole file, or for JSON the list at <c>at</c> inside <paramref name="original"/>, the rest kept.
    /// </summary>
    public static byte[] WriteTable(string format, byte[] original, string table, string options, string fields)
    {
        var api = WithTables();
        byte[] f = Encoding.UTF8.GetBytes(format), t = Encoding.UTF8.GetBytes(table), o = Encoding.UTF8.GetBytes(options), k = Encoding.UTF8.GetBytes(fields);
        byte* made = null;
        nuint length = 0;
        int code;
        fixed (byte* fp = f, gp = original, tp = t, op = o, kp = k)
            code = api->WriteTable(fp, (nuint)f.Length, gp, (nuint)original.Length, tp, (nuint)t.Length, op, (nuint)o.Length, kp, (nuint)k.Length, &made, &length);
        return Made(code, made, length);
    }

    /// <summary>Reads <paramref name="input"/> in <paramref name="format"/> with its <paramref name="options"/> (a JSON
    /// object, empty for none), fitted to <paramref name="kind"/>, and decodes what was fitted.</summary>
    public static T Read<T>(string format, string input, string options, string kind) =>
        JsonValues.Decode<T>(Call(true, format, input, options, kind));

    /// <summary><paramref name="value"/> written in <paramref name="format"/> with its <paramref name="options"/>, as
    /// <paramref name="kind"/> describes it.</summary>
    public static string Write<T>(string format, T value, string options, string kind)
    {
        var encoded = JsonValues.Of(value);
        try
        {
            return Call(false, format, encoded, options, kind);
        }
        catch (ValueException)
        {
            // A kind the generator wrote always describes the value; should it not, the value as Clarity's JSON wrote it.
            return encoded;
        }
    }
}
