using System.Text;

namespace Clarity.Runtime;

/// <summary>
/// zip(paths, texts) (conformance/zip.json): a zip archive of one file per path
/// holding the text of the same place, stored as it is, in order, so an EPUB's
/// mimetype can come first. Paths and texts pair up; what the longer list has
/// more is left out. The same files give the same bytes.
/// </summary>
public static class Zip
{
    public static byte[] Of(IReadOnlyList<string> paths, IReadOnlyList<string> texts)
    {
        using var output = new MemoryStream();
        using var central = new MemoryStream();
        var outWriter = new BinaryWriter(output);
        var centralWriter = new BinaryWriter(central);
        var count = 0;
        for (var i = 0; i < paths.Count && i < texts.Count; i++)
        {
            var name = Encoding.UTF8.GetBytes(paths[i]);
            var data = Encoding.UTF8.GetBytes(texts[i]);
            var crc = Crc32(data);
            var offset = (uint)output.Length;
            // Version 20, UTF-8 names, stored, 1980-01-01 00:00.
            outWriter.Write(0x04034b50u);
            foreach (var v in new ushort[] { 20, 0x0800, 0, 0, 0x21 }) outWriter.Write(v);
            outWriter.Write(crc); outWriter.Write((uint)data.Length); outWriter.Write((uint)data.Length);
            outWriter.Write((ushort)name.Length); outWriter.Write((ushort)0);
            outWriter.Write(name); outWriter.Write(data);
            centralWriter.Write(0x02014b50u);
            foreach (var v in new ushort[] { 20, 20, 0x0800, 0, 0, 0x21 }) centralWriter.Write(v);
            centralWriter.Write(crc); centralWriter.Write((uint)data.Length); centralWriter.Write((uint)data.Length);
            foreach (var v in new ushort[] { (ushort)name.Length, 0, 0, 0, 0 }) centralWriter.Write(v);
            centralWriter.Write(0u); centralWriter.Write(offset);
            centralWriter.Write(name);
            count++;
        }
        centralWriter.Flush();
        outWriter.Flush();
        var centralSize = (uint)central.Length;
        var centralAt = (uint)output.Length;
        outWriter.Write(central.ToArray());
        outWriter.Write(0x06054b50u);
        foreach (var v in new ushort[] { 0, 0, (ushort)count, (ushort)count }) outWriter.Write(v);
        outWriter.Write(centralSize); outWriter.Write(centralAt); outWriter.Write((ushort)0);
        outWriter.Flush();
        return output.ToArray();
    }

    static uint Crc32(byte[] data)
    {
        var crc = 0xFFFFFFFFu;
        foreach (var b in data)
        {
            crc ^= b;
            for (var k = 0; k < 8; k++) crc = (crc & 1) == 1 ? (crc >> 1) ^ 0xEDB88320u : crc >> 1;
        }
        return ~crc;
    }
}
