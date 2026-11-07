// Text functions alike in every runtime (conformance/texts.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

public class TextsTests
{
    [Fact]
    public void TextFunctionsReadAsEveryRuntimeReadsThem()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "../../../../../../conformance/texts.json");
        using var cases = JsonDocument.Parse(File.ReadAllText(path));
        foreach (var c in cases.RootElement.GetProperty("cases").EnumerateArray())
        {
            var a = c.GetProperty("args").EnumerateArray().ToList();
            string S(int i) => a[i].GetString()!;
            long N(int i) => a[i].GetInt64();
            object got = c.GetProperty("function").GetString() switch
            {
                "words" => Texts.Words(S(0)),
                "split" => Texts.Split(S(0), S(1)),
                "trim" => Texts.Trim(S(0)),
                "lower" => Texts.Lower(S(0)),
                "upper" => Texts.Upper(S(0)),
                "replace" => Texts.Replace(S(0), S(1), S(2)),
                "startsWith" => Texts.StartsWith(S(0), S(1)),
                "endsWith" => Texts.EndsWith(S(0), S(1)),
                "slice" => Texts.Slice(S(0), N(1), N(2)),
                "indexOf" => Texts.IndexOf(S(0), S(1)),
                var other => throw new InvalidOperationException(other),
            };
            var options = new JsonSerializerOptions { Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping };
            Assert.Equal(JsonSerializer.Serialize(c.GetProperty("result"), options), JsonSerializer.Serialize(got, options));
        }
    }
}
