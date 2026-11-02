// A value as the person reads it, outside any part: in a search dropdown's
// cells, as the screen shows a value of a field (conformance/formats.json,
// list-fields.json): numbers and dates in the person's format, a field's own
// format where it has one, yes and no in the person's language, a list as its
// items.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ShownTextTests
{
    [Fact]
    public void A_value_shows_in_the_persons_format()
    {
        var formats = Locale.Formats;
        Assert.Equal("", ShownText.Of(null));
        Assert.Equal("Ann", ShownText.Of("Ann"));
        Assert.Equal(formats.Show(new FieldFormat(FormatKind.Integer), 1200L), ShownText.Of(1200L));
        Assert.Equal(formats.Show(new FieldFormat(FormatKind.Decimal), 2.5m), ShownText.Of(2.5m));
        Assert.Equal(formats.Show(new FieldFormat(FormatKind.Date), new DateOnly(2026, 11, 2)), ShownText.Of(new DateOnly(2026, 11, 2)));
        Assert.Equal(Locale.Translate("yes", "Yes"), ShownText.Of(true));
        Assert.Equal(Locale.Translate("no", "No"), ShownText.Of(false));
    }

    [Fact]
    public void A_field_with_a_format_shows_in_it()
    {
        var money = new FieldFormat(FormatKind.Money);
        Assert.Equal(Locale.Formats.Show(money, 9.95m), ShownText.Of(9.95m, money));
    }

    [Fact]
    public void A_list_shows_its_items_each_as_it_is_shown()
    {
        Assert.Equal($"red, {ShownText.Of(2L)}", ShownText.Of(new List<object?> { "red", 2L }));
    }
}
