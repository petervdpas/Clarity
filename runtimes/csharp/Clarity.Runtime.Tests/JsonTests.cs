// Clarity's JSON on the C# runtime: records written and decoded as every
// stack writes values (the data formats themselves are Clarity.Formats',
// tested in Clarity.Ui.Native.Tests against conformance/formats-runtime.json).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class JsonTests
{
    sealed class Order
    {
        public long Id { get; set; }
        public string Number { get; set; } = "new";
        public decimal CreditLimit { get; set; }
        public DateOnly? Since { get; set; }
    }

    /// <summary>Records written and read as Clarity's JSON: names as the project wrote them, decimals as text.</summary>
    [Fact]
    public void RecordsTravelAsClaritysJson()
    {
        var order = new Order { Id = 4, Number = "A-1", CreditLimit = 12.50m, Since = new DateOnly(2026, 10, 25) };
        Assert.Equal("{\"id\":4,\"number\":\"A-1\",\"creditLimit\":\"12.50\",\"since\":\"2026-10-25\"}", JsonValues.Of(order));
        var read = JsonValues.Decode<List<Order>>("[{\"id\":0,\"number\":\"new\",\"creditLimit\":\"99.95\",\"since\":null}]");
        Assert.Equal(("new", 99.95m, (DateOnly?)null, 0L), (read[0].Number, read[0].CreditLimit, read[0].Since, read[0].Id));
    }
}
