// The data formats of code end to end on the C# runtime
// (conformance/formats-runtime.json): read through Clarity.Formats, decoded
// by Clarity's JSON into records as the generator writes them, written back.

using System.Text.Json;
using Clarity.Formats;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Ui.Native.Tests;

public sealed class FormatFunctionsTests
{
    public sealed class Line
    {
        public long Id { get; set; }
        public string Product { get; set; } = "";
        public long Quantity { get; set; }
    }

    public sealed class Order
    {
        public long Id { get; set; }
        public string Number { get; set; } = "";
        public decimal Total { get; set; }
        public DateOnly? Placed { get; set; }
        public List<Line> Lines { get; set; } = [];
    }

    static string Conformance(string name)
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
        {
            var file = Path.Combine(dir.FullName, "conformance", name);
            if (File.Exists(file))
                return File.ReadAllText(file);
        }
        throw new FileNotFoundException(name);
    }

    [Fact]
    public void FormatsReadDecodeAndWriteAsEveryStackDoes()
    {
        using var cases = JsonDocument.Parse(Conformance("formats-runtime.json"));
        foreach (var c in cases.RootElement.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var kind = c.GetProperty("kind").GetRawText();
            var options = c.TryGetProperty("options", out var o) ? o.GetRawText() : "";
            var input = c.GetProperty("input").GetString()!;
            var many = c.GetProperty("kind").GetProperty("kind").GetString() == "records";
            Func<string> run = many
                ? () => FormatFunctions.Write("json", FormatFunctions.Read<List<Order>>("json", input, options, kind), "", kind)
                : () => FormatFunctions.Write("json", FormatFunctions.Read<Order>("json", input, options, kind), "", kind);
            if (c.TryGetProperty("written", out var written))
                Assert.True(written.GetString() == run(), name);
            else
                Assert.Equal(c.GetProperty("error").GetString(), Assert.Throws<ValueException>(() => run()).Message);
        }
    }
}
