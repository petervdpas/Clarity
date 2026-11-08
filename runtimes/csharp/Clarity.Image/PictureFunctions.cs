// The picture functions of code (docs/IMAGE-PLAN.md) through the native
// Clarity.Image library (clarity_image.h): each takes a picture in hand, or
// bytes read from a file, and gives a new one as PNG, leaving its arguments
// as they were. Every stack gives the same pixels (conformance/picture-functions.json).

using System.Runtime.InteropServices;
using Clarity.Runtime;

namespace Clarity.Image;

/// <summary>The picture functions of code.</summary>
public static unsafe partial class PictureFunctions
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Api
    {
        public uint Interface;
        public delegate* unmanaged<byte*, nuint, long, long, byte**, nuint*, int> Resize;
        public delegate* unmanaged<byte*, nuint, long, long, long, long, byte**, nuint*, int> Crop;
        public delegate* unmanaged<byte*, nuint, double, byte**, nuint*, int> Rotate;
        public delegate* unmanaged<byte*, nuint, int, byte**, nuint*, int> Flip;
        public delegate* unmanaged<byte*, nuint, byte*, nuint, long, long, long, byte**, nuint*, int> Blend;
        public delegate* unmanaged<byte*, nuint, double, double, double, double, byte**, nuint*, int> Adjust;
        public delegate* unmanaged<byte*, nuint, uint*, uint*, int> Size;
        public delegate* unmanaged<byte*, nuint, void> Free;
        public delegate* unmanaged<byte*, nuint, int> Problem;
    }

    [LibraryImport("clarity_image", EntryPoint = "clarity_image_api")]
    private static partial Api* TableOf();

    private static readonly Lazy<nint> Table = new(() => (nint)TableOf());

    private static Api* Of => (Api*)Table.Value;

    /// <summary>The bytes of a picture in hand; none, or one not read from its record, is refused.</summary>
    private static byte[] InHand(Picture? picture) =>
        picture is null ? throw new InvalidValueException("there is no picture")
        : picture.Bytes ?? throw new InvalidValueException("the picture is not read from its record");

    /// <summary>Why the last call failed, in Clarity.Image's words.</summary>
    private static InvalidValueException Failed()
    {
        var into = stackalloc byte[512];
        Of->Problem(into, 512);
        return new InvalidValueException(Marshal.PtrToStringUTF8((nint)into) ?? "the picture could not be made");
    }

    private delegate int Call(byte* picture, nuint length, byte** made, nuint* madeLength);

    private static Picture Made(byte[] picture, Call call)
    {
        byte* made = null;
        nuint length = 0;
        fixed (byte* bytes = picture)
        {
            if (call(bytes, (nuint)picture.Length, &made, &length) != 0) throw Failed();
        }
        try
        {
            return Picture.New(new ReadOnlySpan<byte>(made, checked((int)length)).ToArray());
        }
        finally
        {
            Of->Free(made, length);
        }
    }

    /// <summary><c>resize(p, width, height)</c>.</summary>
    public static Picture Resize(Picture? p, long width, long height) => Resize(InHand(p), width, height);

    /// <summary><c>resize(p, width, height)</c> of read bytes.</summary>
    public static Picture Resize(byte[] p, long width, long height) => Made(p, (b, n, o, l) => Of->Resize(b, n, width, height, o, l));

    /// <summary><c>crop(p, x, y, width, height)</c>.</summary>
    public static Picture Crop(Picture? p, long x, long y, long width, long height) => Crop(InHand(p), x, y, width, height);

    /// <summary><c>crop(p, x, y, width, height)</c> of read bytes.</summary>
    public static Picture Crop(byte[] p, long x, long y, long width, long height) => Made(p, (b, n, o, l) => Of->Crop(b, n, x, y, width, height, o, l));

    /// <summary><c>rotate(p, degrees)</c>.</summary>
    public static Picture Rotate(Picture? p, decimal degrees) => Rotate(InHand(p), degrees);

    /// <summary><c>rotate(p, degrees)</c> of read bytes.</summary>
    public static Picture Rotate(byte[] p, decimal degrees) => Made(p, (b, n, o, l) => Of->Rotate(b, n, (double)degrees, o, l));

    /// <summary><c>flip(p, horizontal)</c>.</summary>
    public static Picture Flip(Picture? p, bool horizontal) => Flip(InHand(p), horizontal);

    /// <summary><c>flip(p, horizontal)</c> of read bytes.</summary>
    public static Picture Flip(byte[] p, bool horizontal) => Made(p, (b, n, o, l) => Of->Flip(b, n, horizontal ? 1 : 0, o, l));

    /// <summary><c>blend(base, over, x, y, opacity)</c>.</summary>
    public static Picture Blend(Picture? under, Picture? over, long x, long y, long opacity) => Blend(InHand(under), InHand(over), x, y, opacity);

    /// <summary><c>blend(base, over, x, y, opacity)</c> of read bytes.</summary>
    public static Picture Blend(byte[] under, byte[] over, long x, long y, long opacity) =>
        Made(under, (b, n, o, l) =>
        {
            fixed (byte* top = over)
            {
                return Of->Blend(b, n, top, (nuint)over.Length, x, y, opacity, o, l);
            }
        });

    /// <summary><c>adjust(p, hue, saturation, brightness, contrast)</c>.</summary>
    public static Picture Adjust(Picture? p, decimal hue, decimal saturation, decimal brightness, decimal contrast) => Adjust(InHand(p), hue, saturation, brightness, contrast);

    /// <summary><c>adjust(p, hue, saturation, brightness, contrast)</c> of read bytes.</summary>
    public static Picture Adjust(byte[] p, decimal hue, decimal saturation, decimal brightness, decimal contrast) =>
        Made(p, (b, n, o, l) => Of->Adjust(b, n, (double)hue, (double)saturation, (double)brightness, (double)contrast, o, l));

    /// <summary><c>pictureWidth(p)</c>.</summary>
    public static long Width(Picture? p) => SizeOf(InHand(p)).Width;

    /// <summary><c>pictureWidth(p)</c> of read bytes.</summary>
    public static long Width(byte[] p) => SizeOf(p).Width;

    /// <summary><c>pictureHeight(p)</c>.</summary>
    public static long Height(Picture? p) => SizeOf(InHand(p)).Height;

    /// <summary><c>pictureHeight(p)</c> of read bytes.</summary>
    public static long Height(byte[] p) => SizeOf(p).Height;

    private static (long Width, long Height) SizeOf(byte[] picture)
    {
        uint width = 0, height = 0;
        fixed (byte* bytes = picture)
        {
            if (Of->Size(bytes, (nuint)picture.Length, &width, &height) != 0) throw Failed();
        }
        return (width, height);
    }
}
