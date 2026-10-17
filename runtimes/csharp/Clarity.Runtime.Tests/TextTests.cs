// Text with slots: the text as written and a typed value per slot, so the
// screen fills each slot in the person's format (clarity-runtime/src/ui.rs).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class TextTests
{
    [Fact]
    public void PlainTextShowsItsBraces()
    {
        Assert.Equal("Brace { yourself }", Text.Plain("Brace { yourself }").Show());
        Assert.Equal("Brace {{ yourself }}", Text.Plain("Brace { yourself }").Template);
    }

    [Fact]
    public void WithoutAScreenSlotsAreFilledPlainly()
    {
        var saved = new Text("Saved {customer.name} on {when}, {{done}} {paid}")
            .With("customer.name", Slot.OfText("Ada"))
            .With("when", Slot.OfDate(new DateOnly(2026, 10, 17), "long"))
            .With("paid", Slot.OfBoolean(true));
        Assert.Equal("Saved Ada on 2026-10-17, {done} yes", saved.Show());
        Assert.Equal("long", saved.Values["when"].Format);
        Assert.Equal(SlotKind.Date, saved.Values["when"].Kind);
    }

    [Fact]
    public void SlotsKeepWhatTheScreenNeeds()
    {
        Assert.Equal("1250.50", Slot.OfMoney(1250.50m, null).Value);
        Assert.Equal(2, Slot.OfDecimal(1.5m, 2, null).Scale);
        Assert.Equal("CustomerKind", Slot.OfName(CustomerKind.Business, "CustomerKind").EnumName);
        Assert.Equal("Business", Slot.OfName(CustomerKind.Business, "CustomerKind").Value);
        Assert.Equal("2026-10-17T09:30:00", Slot.OfDateTime(new DateTime(2026, 10, 17, 9, 30, 0), null).Value);
        Assert.Null(Slot.OfDate(null, null).Value);
    }
}
