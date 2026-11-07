// zip(paths, texts) alike in every runtime (conformance/zip.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

public class ZipTests
{
    [Fact]
    public void AZipIsWrittenAsEveryRuntimeWritesIt()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "../../../../../../conformance/zip.json");
        using var cases = JsonDocument.Parse(File.ReadAllText(path));
        foreach (var c in cases.RootElement.GetProperty("cases").EnumerateArray())
        {
            List<string> List(string k) => c.GetProperty(k).EnumerateArray().Select(e => e.GetString()!).ToList();
            var got = Convert.ToHexString(Zip.Of(List("paths"), List("texts"))).ToLowerInvariant();
            Assert.Equal(c.GetProperty("hex").GetString(), got);
        }
    }
}
