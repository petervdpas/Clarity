// A pixel read past the edge of a shot fails its test instead of crashing
// the run, and no test reads a pixel any other way.

using System.Text.RegularExpressions;
using SkiaSharp;

namespace Clarity.Ui.Tests;

public class PixelsTests
{
    [Fact]
    public void A_pixel_past_the_edge_fails_the_test_instead_of_crashing_the_run()
    {
        using var shot = new SKBitmap(new SKImageInfo(800, 400, SKColorType.Rgba8888, SKAlphaType.Premul));
        Assert.Equal(new SKColor(0, 0, 0, 0), shot.PixelAt(799, 399));
        // The SVG test once read row 405 of a 400 row shot: unmapped memory, now and then.
        Assert.Throws<ArgumentOutOfRangeException>(() => shot.PixelAt(30, 405));
        Assert.Throws<ArgumentOutOfRangeException>(() => shot.PixelAt(800, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => shot.PixelAt(-1, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => shot.PixelAt(0, -1));
    }

    [Fact]
    public void Tests_read_pixels_only_through_PixelAt()
    {
        var folder = SourceFolder();
        var unchecked_ = Directory.EnumerateFiles(folder, "*.cs", SearchOption.AllDirectories)
            .Where(f => !f.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}") && !f.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}"))
            .Where(f => Path.GetFileName(f) != "Pixels.cs")
            .SelectMany(f => File.ReadLines(f).Select((line, i) => (f, i, line)))
            .Where(l => Regex.IsMatch(l.line, @"\.GetPixel\("))
            .Select(l => $"{Path.GetRelativePath(folder, l.f)}:{l.i + 1}")
            .ToList();
        Assert.True(unchecked_.Count == 0, "read pixels with PixelAt, not GetPixel: " + string.Join(", ", unchecked_));
    }

    // The test project's own source folder, found from where the tests run.
    static string SourceFolder()
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
            if (File.Exists(Path.Combine(dir.FullName, "Clarity.Ui.Tests.csproj")))
                return dir.FullName;
        throw new InvalidOperationException("the Clarity.Ui.Tests sources are not above " + AppContext.BaseDirectory);
    }
}
