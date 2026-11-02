// How windows open and close, against the cases every stack shares
// (conformance/windows.json), run through WindowStack as a desktop host runs
// it: a dialog or a target marked `in new window` gets a desktop window of
// its own (Apart), a stack of its own whose Opener is told when it ends. A
// dialog's desktop window is listed on top of the desktop window it was
// opened from, as the cases expect.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>A window of a case: it logs its events, and its handler does the step's actions.</summary>
sealed class CaseViewModel(string name, HashSet<string> dialogs, List<string> log) : WindowViewModel(null)
{
    public override string Title => name;
    public override bool IsDialog => dialogs.Contains(name);

    protected override void OnOpen() => log.Add($"{name}:open");
    protected override void OnClose() => log.Add($"{name}:close");
    protected override void OnReturn() => log.Add($"{name}:return");

    public void Run(IEnumerable<string> actions) => Handle(() =>
    {
        foreach (var action in actions)
        {
            if (action == "close")
                Close();
            else if (action.EndsWith(" in new window", StringComparison.Ordinal))
                Open(new CaseViewModel(action[5..^" in new window".Length], dialogs, log), newWindow: true);
            else if (action.StartsWith("open ", StringComparison.Ordinal))
                Open(new CaseViewModel(action[5..], dialogs, log));
            else
                throw new ArgumentException($"unknown action {action}");
        }
    });
}

public class WindowCaseTests
{
    static string[] Strings(JsonElement array) => [.. array.EnumerateArray().Select(e => e.GetString()!)];

    /// <summary>The desktop windows of a case's application, as a desktop host keeps them.</summary>
    sealed class Desktops
    {
        public readonly List<(WindowStack Stack, bool Dialog, WindowViewModel? Opener)> Open = [];
        public bool Ended;

        public WindowStack Start(WindowViewModel first, WindowViewModel? opener = null, bool dialog = false)
        {
            WindowStack? stack = null;
            stack = new WindowStack(_ => "<Window name=\"Case\" />") { Apart = (window, from) => Start(window, from, window.IsDialog), Opener = opener };
            var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
            screen.Resize(800, 600, 1);
            Open.Add((stack, dialog, opener));
            stack.Ended += () =>
            {
                if (Open.FindIndex(d => d.Stack == stack) == 0)
                    Ended = true;
                Open.RemoveAll(d => d.Stack == stack);
            };
            stack.Push(first);
            return stack;
        }

        /// <summary>Each desktop window that is no dialog with its windows, bottom to top, the dialogs opened from it on top.</summary>
        public List<string> Shown() =>
        [
            .. Open.Where(d => !d.Dialog).Select(d => string.Join(", ",
                d.Stack.Windows.Concat(Open.Where(o => o.Dialog && d.Stack.Windows.Contains(o.Opener)).SelectMany(o => o.Stack.Windows)).Select(w => w.Title))),
        ];

        /// <summary>The window on top of the newest desktop window, which runs the next step.</summary>
        public CaseViewModel Top => (CaseViewModel)Open[^1].Stack.Windows[^1];
    }

    [Fact]
    public void EveryCaseOpensAndClosesItsWindowsTheSameWay()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "windows.json");
        var cases = JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("cases").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var @case in cases)
        {
            var name = @case.GetProperty("name").GetString();
            var dialogs = Strings(@case.GetProperty("dialogs")).ToHashSet();
            var log = new List<string>();
            var desktops = new Desktops();
            desktops.Start(new CaseViewModel(@case.GetProperty("start").GetString()!, dialogs, log));
            foreach (var step in @case.GetProperty("steps").EnumerateArray())
                desktops.Top.Run(Strings(step));

            var expect = @case.GetProperty("expect");
            var wanted = expect.GetProperty("desktops").EnumerateArray().Select(d => string.Join(", ", Strings(d))).ToList();
            var shown = desktops.Shown();
            Assert.True(wanted.SequenceEqual(shown), $"{name}: desktop windows [{string.Join("] [", shown)}]");
            Assert.True(Strings(expect.GetProperty("events")).SequenceEqual(log), $"{name}: events {string.Join(", ", log)}");
            Assert.Equal(expect.GetProperty("ended").GetBoolean(), desktops.Ended);
        }
    }
}
