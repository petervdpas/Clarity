// Clarity.UI splits as Clarity.Avalonia did (DESIGN 7.6, 7.9): what only a
// desktop program has (the system's windows, sound through SoundFlow, the
// desktop's loop, quitting) is Clarity.Ui.Desktop's, so a program in a
// browser and its server carry nothing of it. The keys window and the
// unconnected flow are Clarity.Ui's, since the browser shows them too.

using System.Xml.Linq;
using Clarity.Runtime;
using Clarity.Ui.Desktop;

namespace Clarity.Ui.Tests;

[Collection("Remembered")]
public class SplitTests
{
    static readonly string[] NativeOnly = ["Silk.NET.Windowing", "Silk.NET.Input", "Silk.NET.GLFW", "Silk.NET.Windowing.Glfw", "Silk.NET.Input.Glfw", "SoundFlow", "SkiaSharp.NativeAssets.Linux", "SkiaSharp.NativeAssets.Win32", "SkiaSharp.NativeAssets.macOS"];

    /// <summary>The runtime's folder, the one with Clarity.slnx, above the tests' output.</summary>
    static string Runtime()
    {
        var at = new DirectoryInfo(AppContext.BaseDirectory);
        while (!File.Exists(Path.Combine(at.FullName, "Clarity.slnx")))
            at = at.Parent!;
        return at.FullName;
    }

    [Theory]
    [InlineData("Clarity.Ui")]
    [InlineData("Clarity.Ui.Browser")]
    [InlineData("Clarity.Ui.Server")]
    [InlineData("Clarity.Runtime")]
    public void The_core_references_nothing_native(string package)
    {
        var project = XDocument.Load(Path.Combine(Runtime(), package, package + ".csproj"));
        var packages = project.Descendants("PackageReference").Select(p => (string)p.Attribute("Include")!).ToList();
        var projects = project.Descendants("ProjectReference").Select(p => (string)p.Attribute("Include")!).ToList();
        Assert.Empty(packages.Intersect(NativeOnly));
        // No desktop shell of any kind: no Silk.NET, and no Avalonia, which Clarity.UI replaced.
        Assert.DoesNotContain(packages, p => p.StartsWith("Silk.NET", StringComparison.Ordinal) || p.StartsWith("Avalonia", StringComparison.Ordinal));
        Assert.DoesNotContain(projects, p => p.Contains("Clarity.Ui.Desktop"));
        foreach (var assembly in new[] { typeof(Screen).Assembly, typeof(Clarity.Ui.Browser.BrowserAddress).Assembly, typeof(WindowViewModel).Assembly })
            Assert.Empty(assembly.GetReferencedAssemblies().Select(a => a.Name!).Intersect(NativeOnly));
    }

    [Fact]
    public void The_desktop_shell_is_the_desktop_packages()
    {
        var project = XDocument.Load(Path.Combine(Runtime(), "Clarity.Ui.Desktop", "Clarity.Ui.Desktop.csproj"));
        var packages = project.Descendants("PackageReference").Select(p => (string)p.Attribute("Include")!).ToList();
        Assert.Contains(packages, p => p.StartsWith("Silk.NET.Windowing", StringComparison.Ordinal));
        foreach (var system in new[] { "Linux", "Win32", "macOS" })
            Assert.Contains($"SkiaSharp.NativeAssets.{system}", packages);
        Assert.Equal("Clarity.Ui.Desktop", typeof(DesktopWindow).Assembly.GetName().Name);
        Assert.Equal("Clarity.Ui.Desktop", typeof(SoundFlowOutput).Assembly.GetName().Name);
        Assert.Equal("Clarity.Ui.Desktop", typeof(SystemFilePicker).Assembly.GetName().Name);
        // Shown in a browser too: Clarity.Ui's.
        Assert.Equal("Clarity.Ui", typeof(KeysWindow).Assembly.GetName().Name);
        Assert.Equal("Clarity.Ui", typeof(Unconnected).Assembly.GetName().Name);
    }

    [Fact]
    public void The_desktop_package_carries_its_readme_icon_and_licence()
    {
        var project = XDocument.Load(Path.Combine(Runtime(), "Clarity.Ui.Desktop", "Clarity.Ui.Desktop.csproj"));
        var packed = project.Descendants("None").Where(n => (string?)n.Attribute("Pack") == "true").Select(n => (string)n.Attribute("Include")!).ToList();
        Assert.Contains("README.md", packed);
        Assert.Contains("../assets/icon.png", packed);
        Assert.Contains("../assets/LICENSE.txt", packed);
        Assert.Contains("on the desktop", File.ReadAllText(Path.Combine(Runtime(), "Clarity.Ui.Desktop", "README.md")));
    }
}
