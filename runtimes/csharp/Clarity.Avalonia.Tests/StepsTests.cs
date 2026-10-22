// A wizard against the cases every renderer shares (conformance/steps.json),
// run through the real StepsView: its buttons pressed, the handler of
// `on next` refusing or choosing the step, as a window's handler does.

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

sealed class WizardViewModel(Db db) : WindowViewModel(db)
{
    public PartState Wizard { get; } = new("wizard");
}

public class StepsTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "steps.json"))).RootElement;

    [AvaloniaFact]
    public void EveryWalkThroughAWizardIsAsOnEveryRenderer()
    {
        var cases = File.GetProperty("walk").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var @case in cases)
            Walk(@case);
    }

    static void Press(Button button) => button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));

    static void Walk(JsonElement @case)
    {
        var name = @case.GetProperty("name").GetString();
        using var db = Db.Memory();
        var vm = new WizardViewModel(db);
        var wizard = new StepsView();
        for (var i = 1; i <= @case.GetProperty("steps").GetInt32(); i++)
            wizard.Steps.Add(new StepItem { Header = $"Step {i}", Content = new TextBlock() });
        wizard.Step = @case.TryGetProperty("start", out var start) ? start.GetInt64() : 1;
        var ran = new List<string>();
        var handler = @case.GetProperty("handler");
        if (handler.ValueKind == JsonValueKind.Object)
        {
            // The handler of `on next`, as generated: it runs in Handle and reads and sets the part's step.
            wizard.Next = () => vm.Handle(() =>
            {
                vm.Wizard.Step = wizard.Step;
                ran.Add($"next {vm.Wizard.Step}");
                if (handler.TryGetProperty("refuses", out var refuses) && refuses.EnumerateArray().Any(s => s.GetInt64() == vm.Wizard.Step))
                    throw new RefusedException(Text.Plain("Not yet"));
                if (handler.TryGetProperty("jumps", out var jumps) && jumps.GetProperty("from").GetInt64() == vm.Wizard.Step)
                    vm.Wizard.Step = jumps.GetProperty("to").GetInt64();
                wizard.Step = vm.Wizard.Step;
            });
        }
        wizard.Finished += () => ran.Add($"finish {wizard.Step}");

        foreach (var press in @case.GetProperty("do").EnumerateArray())
        {
            var button = press.GetString() switch
            {
                "next" => wizard.Forward,
                "back" => wizard.Back,
                "finish" => wizard.Finish,
                var other => throw new ArgumentException($"{name}: unknown button {other}"),
            };
            Assert.True(button.IsVisible, $"{name}: {press} is not shown");
            Press(button);
        }

        var expect = @case.GetProperty("expect");
        Assert.True(expect.GetProperty("shown").GetInt64() == StepsView.Shown(wizard.Step, wizard.Steps.Count), $"{name}: shows step {StepsView.Shown(wizard.Step, wizard.Steps.Count)}");
        var buttons = new[] { ("back", wizard.Back), ("next", wizard.Forward), ("finish", wizard.Finish) }.Where(b => b.Item2.IsVisible).Select(b => b.Item1);
        Assert.True(expect.GetProperty("buttons").EnumerateArray().Select(b => b.GetString()!).SequenceEqual(buttons), $"{name}: buttons [{string.Join(", ", buttons)}]");
        Assert.True(expect.GetProperty("ran").EnumerateArray().Select(r => r.GetString()!).SequenceEqual(ran), $"{name}: ran [{string.Join(", ", ran)}]");
    }

    [AvaloniaFact]
    public void A_wizards_step_is_its_part_state_from_one()
    {
        var part = new PartState("wizard");
        Assert.Equal(1, part.Step);
        foreach (var c in File.GetProperty("set").EnumerateArray())
        {
            part.Step = c.GetProperty("step").GetInt64();
            Assert.Equal(c.GetProperty("part").GetProperty("step").GetInt64(), part.Step);
        }
    }
}
