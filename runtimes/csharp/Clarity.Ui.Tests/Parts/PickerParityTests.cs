// The language picker of the frame as the Avalonia LocaleTests had it,
// through the real picker and Locale rather than a fake: each language by
// its own name, the one shown checked, and a choice that becomes the
// language shown, which Locale tells so the application's settings keep it.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

[Collection("Language")]
public class PickerParityTests
{
    [Fact]
    public void The_language_picker_offers_each_language_by_its_own_name_and_a_choice_is_told()
    {
        Locale.Start("en", ["en", "nl", "de"], null, (_, _) => null, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
        try
        {
            var choices = FramePicker.Language.Choices();
            Assert.Equal([("de", Languages.Name("de"), false), ("en", Languages.Name("en"), true), ("nl", Languages.Name("nl"), false)], choices.OrderBy(c => c.Value));
            Assert.Equal("Nederlands", Languages.Name("nl"), ignoreCase: true);
            var told = 0;
            void Heard() => told++;
            Locale.Changed += Heard;
            FramePicker.Language.Choose("nl");
            Locale.Changed -= Heard;
            Assert.Equal("nl", Locale.Current);
            Assert.Equal(1, told);
            Assert.Contains(("nl", Languages.Name("nl"), true), FramePicker.Language.Choices());
        }
        finally
        {
            Locale.Reset();
        }
    }
}
