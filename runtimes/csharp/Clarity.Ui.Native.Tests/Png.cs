// Reads the PNG a driven screen's `shot` writes (8-bit RGBA or RGB, not
// interlaced, as Skia writes it), so a test reads the library's pixels with
// .NET alone: the chunks, the zlib data inflated, each row unfiltered.

using System.Buffers.Binary;
using System.IO.Compression;

namespace Clarity.Ui.Native.Tests;

public sealed class Png
{
    public int Width { get; }
    public int Height { get; }
    readonly byte[] rgba;

    Png(int width, int height, byte[] rgba)
    {
        Width = width;
        Height = height;
        this.rgba = rgba;
    }

    /// <summary>The pixel at <paramref name="x"/>, <paramref name="y"/> as red, green, blue and alpha; a pixel past the edge fails the test.</summary>
    public (byte R, byte G, byte B, byte A) At(int x, int y)
    {
        Assert.InRange(x, 0, Width - 1);
        Assert.InRange(y, 0, Height - 1);
        var i = (y * Width + x) * 4;
        return (rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]);
    }

    public static Png Read(string path)
    {
        var bytes = File.ReadAllBytes(path);
        Assert.True(bytes.AsSpan(0, 8).SequenceEqual(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }), "the file is a PNG");
        int width = 0, height = 0, channels = 0;
        using var idat = new MemoryStream();
        for (var at = 8; at < bytes.Length;)
        {
            var length = BinaryPrimitives.ReadInt32BigEndian(bytes.AsSpan(at));
            var kind = System.Text.Encoding.ASCII.GetString(bytes, at + 4, 4);
            var data = bytes.AsSpan(at + 8, length);
            if (kind == "IHDR")
            {
                width = BinaryPrimitives.ReadInt32BigEndian(data);
                height = BinaryPrimitives.ReadInt32BigEndian(data[4..]);
                Assert.Equal(8, data[8]);
                channels = data[9] switch
                {
                    6 => 4,
                    2 => 3,
                    var other => throw new InvalidDataException($"color type {other} is not read here"),
                };
                Assert.Equal(0, data[12]);
            }
            else if (kind == "IDAT")
                idat.Write(data);
            else if (kind == "IEND")
                break;
            at += 12 + length;
        }
        idat.Position = 0;
        using var inflated = new MemoryStream();
        using (var z = new ZLibStream(idat, CompressionMode.Decompress))
            z.CopyTo(inflated);
        var raw = inflated.ToArray();
        var stride = width * channels;
        var rows = new byte[height * stride];
        for (var y = 0; y < height; y++)
        {
            var filter = raw[y * (stride + 1)];
            var line = raw.AsSpan(y * (stride + 1) + 1, stride);
            for (var x = 0; x < stride; x++)
            {
                int a = x >= channels ? rows[y * stride + x - channels] : 0;
                int b = y > 0 ? rows[(y - 1) * stride + x] : 0;
                int c = x >= channels && y > 0 ? rows[(y - 1) * stride + x - channels] : 0;
                var add = filter switch
                {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => (a + b) / 2,
                    4 => Paeth(a, b, c),
                    _ => throw new InvalidDataException($"filter {filter}"),
                };
                rows[y * stride + x] = (byte)(line[x] + add);
            }
        }
        var rgba = new byte[width * height * 4];
        for (var p = 0; p < width * height; p++)
        {
            rgba[p * 4] = rows[p * channels];
            rgba[p * 4 + 1] = rows[p * channels + 1];
            rgba[p * 4 + 2] = rows[p * channels + 2];
            rgba[p * 4 + 3] = channels == 4 ? rows[p * channels + 3] : (byte)255;
        }
        return new Png(width, height, rgba);
    }

    static int Paeth(int a, int b, int c)
    {
        var p = a + b - c;
        var (pa, pb, pc) = (Math.Abs(p - a), Math.Abs(p - b), Math.Abs(p - c));
        return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
    }
}
