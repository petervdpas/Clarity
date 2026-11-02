// Questions as Avalonia's Ask.cs asked them: a day or a moment gets a date
// input that answers a DateOnly or a DateTime, and an empty text answers "".

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class AskParityTests
{
    [Fact]
    public void A_date_question_gets_a_date_input_and_answers_a_day()
    {
        var day = new DateOnly(2026, 11, 2);
        var ask = new AskDialog(new Question(Text.Plain("When?"), new ValueForm(new FieldFormat(FormatKind.Date), null, day), _ => { }));
        var input = Assert.IsType<DatePart>(ask.Input);
        Assert.False(input.IsMoment);
        Assert.Equal(day, ask.Value());
    }

    [Fact]
    public void A_moment_question_gets_a_date_and_time_input_and_answers_a_moment()
    {
        var moment = new DateTime(2026, 11, 2, 14, 30, 0);
        var ask = new AskDialog(new Question(Text.Plain("When exactly?"), new ValueForm(new FieldFormat(FormatKind.DateTime), null, moment), _ => { }));
        var input = Assert.IsType<DatePart>(ask.Input);
        Assert.True(input.IsMoment);
        Assert.Equal(moment, ask.Value());
    }

    [Fact]
    public void An_empty_text_answers_an_empty_text()
    {
        var ask = new AskDialog(new Question(Text.Plain("Name?"), new ValueForm(null, null, null), _ => { }));
        Assert.Equal("", ask.Value());
    }
}

public class AskSizeTests
{
    [Fact]
    public void An_ask_dialog_is_as_wide_as_its_question_between_320_and_520()
    {
        static double Width(string question)
        {
            var ask = new AskDialog(new Question(Clarity.Runtime.Text.Plain(question), new ConfirmForm(null, null), _ => { }));
            var screen = new Screen(ask, ThemeLook.Clarity(dark: false));
            screen.Resize(1200, 800, 1);
            return ask.Panel.Width;
        }
        Assert.Equal(320, Width("Sure?"));
        Assert.Equal(520, Width(string.Join(" ", Enumerable.Repeat("a long question", 20))));
    }
}
