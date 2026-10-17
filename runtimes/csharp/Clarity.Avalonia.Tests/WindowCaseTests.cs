// How windows open and close, against the cases every stack shares
// (conformance/windows.json), run through the real Shell.

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

/// A window of a case: it logs its events, and its handler does the step's actions.
sealed class CaseViewModel(Db db, string name, HashSet<string> dialogs, List<string> log) : WindowViewModel(db)
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
                Open(new CaseViewModel(Db, action[5..^" in new window".Length], dialogs, log), newWindow: true);
            else if (action.StartsWith("open ", StringComparison.Ordinal))
                Open(new CaseViewModel(Db, action[5..], dialogs, log));
            else
                throw new ArgumentException($"unknown action {action}");
        }
    });
}

public class WindowCaseTests
{
    static JsonElement Cases()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "windows.json");
        return JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("cases");
    }

    static string[] Strings(JsonElement array) => array.EnumerateArray().Select(e => e.GetString()!).ToArray();

    [AvaloniaFact]
    public void EveryCaseOpensAndClosesItsWindowsTheSameWay()
    {
        var cases = Cases().EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var @case in cases)
        {
            var name = @case.GetProperty("name").GetString();
            var dialogs = Strings(@case.GetProperty("dialogs")).ToHashSet();
            var log = new List<string>();
            using var db = Db.Memory();
            var shell = Shell.Start(new CaseViewModel(db, @case.GetProperty("start").GetString()!, dialogs, log),
                vm => new ContentControl { DataContext = vm });
            foreach (var step in @case.GetProperty("steps").EnumerateArray())
                ((CaseViewModel)Shell.Desktops[^1].Windows[^1]).Run(Strings(step));

            var expect = @case.GetProperty("expect");
            var desktops = expect.GetProperty("desktops").EnumerateArray().Select(d => string.Join(", ", Strings(d))).ToList();
            var shown = Shell.Desktops.Select(s => string.Join(", ", s.Windows.Select(w => w.Title))).ToList();
            Assert.True(desktops.SequenceEqual(shown), $"{name}: desktop windows [{string.Join("] [", shown)}]");
            Assert.True(Strings(expect.GetProperty("events")).SequenceEqual(log), $"{name}: events {string.Join(", ", log)}");
            Assert.Equal(expect.GetProperty("ended").GetBoolean(), !shell.IsVisible);
            // No desktop window besides the shells, except one per open dialog.
            foreach (var s in Shell.Desktops)
                Assert.Equal(s.Windows.Count(w => w.IsDialog), s.OwnedWindows.Count);
            foreach (var s in Shell.Desktops.ToList())
                s.Close();
        }
    }
}
