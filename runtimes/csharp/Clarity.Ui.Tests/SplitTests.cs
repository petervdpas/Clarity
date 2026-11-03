// How Clarity.UI for C# splits since the one native library (DESIGN 7.9,
// R2): the desktop is the library's (its window, sound, loop), reached
// through Clarity.Ui, the thin layer; the browser program and its server
// carry none of it. The C# drawing (the folder Clarity.Ui.Drawing) lives on
// compiled into the browser's assembly only, until the browser build.

using System.Xml.Linq;
using Clarity.Runtime;

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
    public void No_project_carries_a_desktop_shell_of_its_own(string package)
    {
        var project = XDocument.Load(Path.Combine(Runtime(), package, package + ".csproj"));
        var packages = project.Descendants("PackageReference").Select(p => (string)p.Attribute("Include")!).ToList();
        var projects = project.Descendants("ProjectReference").Select(p => (string)p.Attribute("Include")!).ToList();
        Assert.Empty(packages.Intersect(NativeOnly));
        // No Silk.NET and no Avalonia: the desktop window is the native library's.
        Assert.DoesNotContain(packages, p => p.StartsWith("Silk.NET", StringComparison.Ordinal) || p.StartsWith("Avalonia", StringComparison.Ordinal));
        Assert.DoesNotContain(projects, p => p.Contains("Clarity.Ui.Desktop"));
        foreach (var assembly in new[] { typeof(Screen).Assembly, typeof(Clarity.Ui.Browser.BrowserAddress).Assembly, typeof(WindowViewModel).Assembly })
            Assert.Empty(assembly.GetReferencedAssemblies().Select(a => a.Name!).Intersect(NativeOnly));
    }

    [Fact]
    public void The_desktop_package_is_gone_and_the_drawing_is_the_browser_s()
    {
        Assert.False(Directory.Exists(Path.Combine(Runtime(), "Clarity.Ui.Desktop")));
        Assert.Equal("Clarity.Ui.Browser", typeof(Screen).Assembly.GetName().Name);
        Assert.Empty(Directory.GetFiles(Path.Combine(Runtime(), "Clarity.Ui.Drawing"), "*.csproj"));
        var browser = XDocument.Load(Path.Combine(Runtime(), "Clarity.Ui.Browser", "Clarity.Ui.Browser.csproj"));
        Assert.Contains(browser.Descendants("Compile"), c => ((string)c.Attribute("Include")!).StartsWith("../Clarity.Ui.Drawing/", StringComparison.Ordinal));
    }

    [Fact]
    public void The_clarity_ui_package_carries_its_readme_icon_and_licence()
    {
        var project = XDocument.Load(Path.Combine(Runtime(), "Clarity.Ui", "Clarity.Ui.csproj"));
        var packed = project.Descendants("None").Where(n => (string?)n.Attribute("Pack") == "true").Select(n => (string)n.Attribute("Include")!).ToList();
        Assert.Contains("README.md", packed);
        Assert.Contains("../assets/icon.png", packed);
        Assert.Contains("../assets/LICENSE.txt", packed);
        Assert.Contains("native", File.ReadAllText(Path.Combine(Runtime(), "Clarity.Ui", "README.md")));
    }
}
