// A wizard against the cases every renderer shares (conformance/steps.json):
// its buttons pressed, the handler of `on next` refusing or choosing the step
// as a window's handler does, through a host that says whether it refused.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Containers;

/// <summary>A window whose wizard's handlers behave as a case says, and say what ran on which step.</summary>
sealed class WizardHost(JsonElement handler) : TestHost
{
    public readonly List<string> Ran = [];

    public override bool Run(string? part, HandlerCall call)
    {
        base.Run(part, call);
        var state = State(part)!;
        Ran.Add($"{call.Event} {state.Step}");
        if (call.Event != "next")
            return true;
        if (handler.TryGetProperty("refuses", out var refuses) && refuses.EnumerateArray().Any(s => s.GetInt64() == state.Step))
            return false;
        if (handler.TryGetProperty("jumps", out var jumps) && jumps.GetProperty("from").GetInt64() == state.Step)
            state.Step = jumps.GetProperty("to").GetInt64();
        return true;
    }
}

public class StepsPartTests
{
    static readonly JsonElement File = Conformance.Read("steps.json");

    static string Wizard(int steps, bool handler) =>
        $"""<Steps name="wizard" at="1,1" span="12" rows="8"{(handler ? " onnext=\"checkStep\"" : "")} onfinish="done">"""
        + string.Concat(Enumerable.Range(1, steps).Select(i => $"""<Step title="Step {i}"><Trend at="1,1" span="3" /></Step>"""))
        + "</Steps>";

    [Fact]
    public void Every_walk_through_a_wizard_is_as_on_every_renderer()
    {
        var cases = File.GetProperty("walk").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var c in cases)
        {
            var name = c.GetProperty("name").GetString();
            var handler = c.GetProperty("handler");
            var host = new WizardHost(handler);
            if (c.TryGetProperty("start", out var start))
                host.State("wizard")!.Step = start.GetInt64();
            var (screen, _, parts) = Made.Window(Wizard(c.GetProperty("steps").GetInt32(), handler.ValueKind == JsonValueKind.Object), host, width: 808);
            var wizard = Assert.IsType<StepsPart>(parts[0]);
            foreach (var press in c.GetProperty("do").EnumerateArray())
            {
                var button = press.GetString() switch
                {
                    "next" => wizard.Next,
                    "back" => wizard.Back,
                    "finish" => wizard.Finish,
                    var other => throw new ArgumentException($"{name}: unknown button {other}"),
                };
                Assert.True(button.IsShown, $"{name}: {press} is not shown");
                // Pressed where the person sees it.
                screen.Resize(808, 400, 1);
                var at = button.Bounds;
                screen.PointerPressed(at.X + at.Width / 2, at.Y + at.Height / 2, PointerButton.Left);
                screen.PointerReleased(at.X + at.Width / 2, at.Y + at.Height / 2, PointerButton.Left);
            }
            var expect = c.GetProperty("expect");
            Assert.Equal(expect.GetProperty("shown").GetInt64(), StepsPart.Shown(wizard.Step, c.GetProperty("steps").GetInt32()));
            var buttons = new[] { ("back", wizard.Back), ("next", wizard.Next), ("finish", wizard.Finish) }.Where(b => b.Item2.IsShown).Select(b => b.Item1);
            Assert.True(expect.GetProperty("buttons").EnumerateArray().Select(b => b.GetString()!).SequenceEqual(buttons), $"{name}: buttons [{string.Join(", ", buttons)}]");
            Assert.True(expect.GetProperty("ran").EnumerateArray().Select(r => r.GetString()!).SequenceEqual(host.Ran), $"{name}: ran [{string.Join(", ", host.Ran)}]");
            // Drawn again, only the shown step's page is there.
            screen.Resize(808, 400, 1);
            var shown = (int)StepsPart.Shown(wizard.Step, wizard.Pages.Count);
            for (var i = 0; i < wizard.Pages.Count; i++)
                Assert.True((i + 1 == shown) == wizard.Children.Contains(wizard.Pages[i]), $"{name}: page {i + 1} of {shown}");
        }
    }

    [Fact]
    public void A_wizards_step_is_its_part_state_from_one()
    {
        foreach (var c in File.GetProperty("step").EnumerateArray())
        {
            var part = new PartState("wizard");
            if (c.GetProperty("part").TryGetProperty("step", out var step))
                part.Step = step.GetInt64();
            Assert.Equal(c.GetProperty("step").GetInt64(), part.Step);
        }
        foreach (var c in File.GetProperty("set").EnumerateArray())
        {
            var part = new PartState("wizard") { Step = c.GetProperty("step").GetInt64() };
            Assert.Equal(c.GetProperty("part").GetProperty("step").GetInt64(), part.Step);
        }
    }

    [Fact]
    public void Its_buttons_are_in_claritys_own_words()
    {
        var (_, _, parts) = Made.Window(Wizard(2, handler: false), new TestHost());
        var wizard = (StepsPart)parts[0];
        Assert.Equal(("Back", "Next", "Finish"), (wizard.Back.Text, wizard.Next.Text, wizard.Finish.Text));
    }

    [Fact]
    public void A_host_that_cannot_answer_runs_the_handler_and_the_wizard_moves_on()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window(Wizard(3, handler: true), host);
        var wizard = (StepsPart)parts[0];
        wizard.Next.Press();
        Assert.Equal(2, host.State("wizard")!.Step);
        Assert.Equal(["run wizard next checkStep"], host.Log);
    }

    [Fact]
    public void A_wizard_without_a_name_keeps_its_own_step()
    {
        var (_, _, parts) = Made.Window(Wizard(2, handler: false).Replace("name=\"wizard\" ", ""), new TestHost());
        var wizard = (StepsPart)parts[0];
        wizard.Next.Press();
        Assert.Equal(2, wizard.Step);
        wizard.Back.Press();
        Assert.Equal(1, wizard.Step);
    }
}
