// Clarity.Avalonia splits (DESIGN 7.6): what only a desktop program has
// (native windows, sound through SoundFlow, the desktop shell, quitting and
// the keys window) is Clarity.Avalonia.Desktop's, so a program in a browser
// and its server carry nothing of it.

using System.Xml.Linq;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class SplitTests
{
    static readonly string[] NativeOnly = ["Avalonia.Desktop", "Avalonia.Win32", "Avalonia.X11", "Avalonia.Native", "SoundFlow"];

    /// <summary>The runtime's folder, the one with Clarity.slnx, above the tests' output.</summary>
    static string Runtime()
    {
        var at = new DirectoryInfo(AppContext.BaseDirectory);
        while (!File.Exists(Path.Combine(at.FullName, "Clarity.slnx")))
            at = at.Parent!;
        return at.FullName;
    }

    [Fact]
    public void The_core_references_nothing_native()
    {
        var project = XDocument.Load(Path.Combine(Runtime(), "Clarity.Avalonia", "Clarity.Avalonia.csproj"));
        var packages = project.Descendants("PackageReference").Select(p => (string)p.Attribute("Include")!).ToList();
        var projects = project.Descendants("ProjectReference").Select(p => (string)p.Attribute("Include")!).ToList();
        Assert.Empty(packages.Intersect(NativeOnly));
        Assert.DoesNotContain(projects, p => p.Contains("Clarity.Avalonia.Desktop"));
        var used = typeof(WindowViewModel).Assembly.GetReferencedAssemblies().Select(a => a.Name!).ToList();
        Assert.Empty(used.Intersect(NativeOnly));
    }

    [Fact]
    public void The_desktop_shell_is_the_desktop_packages()
    {
        Assert.Equal("Clarity.Avalonia.Desktop", typeof(Shell).Assembly.GetName().Name);
        Assert.Equal("Clarity.Avalonia.Desktop", typeof(KeysWindow).Assembly.GetName().Name);
        Assert.Equal("Clarity.Avalonia.Desktop", typeof(Unconnected).Assembly.GetName().Name);
        Assert.Equal("Clarity.Avalonia.Desktop", typeof(SoundFlowOutput).Assembly.GetName().Name);
    }

    [AvaloniaFact]
    public void Without_the_desktop_quit_and_keys_do_nothing_and_with_it_they_reach_it()
    {
        var before = DesktopProgram.Current;
        try
        {
            DesktopProgram.Current = null;
            var window = new QuitAndKeysViewModel();
            window.Quit();
            window.Keys();
            var desktop = new Recorded();
            DesktopProgram.Current = desktop;
            window.Quit();
            window.Keys();
            Assert.Equal(["quit", "keys"], desktop.Did);
        }
        finally
        {
            DesktopProgram.Current = before;
        }
    }

    sealed class Recorded : IDesktop
    {
        public List<string> Did { get; } = [];

        public void Quit() => Did.Add("quit");

        public void ShowKeys() => Did.Add("keys");
    }

    sealed class QuitAndKeysViewModel() : WindowViewModel(null)
    {
        public void Quit() => QuitApplication();

        public void Keys() => ShowKeys();
    }
}
