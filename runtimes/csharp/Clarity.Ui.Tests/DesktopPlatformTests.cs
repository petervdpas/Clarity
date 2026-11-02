// What the desktop host does with the system beyond the screen: whatever
// fails on its loop is said once and the program goes on; a window draws at
// the system's real content scale; OpenGL 3.3 core first, then older
// contexts and drawing in software; file dialogs belong to the window, which
// takes no input meanwhile; what a dialog says as it closes shows below.

using System.Runtime.InteropServices;
using Clarity.Runtime;
using Clarity.Ui.Desktop;
using Silk.NET.Windowing;

namespace Clarity.Ui.Tests;

public class DesktopPlatformTests
{
    [Fact]
    public void A_failure_on_the_loop_is_said_once_while_it_repeats_and_the_program_goes_on()
    {
        var said = new List<string>();
        var safe = new FailSafe(e => said.Add(e.Message));
        Assert.False(safe.Run(() => throw new InvalidOperationException("the chart could not draw"), 1000));
        // The same failure every frame: said once.
        Assert.False(safe.Run(() => throw new InvalidOperationException("the chart could not draw"), 1016));
        Assert.False(safe.Run(() => throw new InvalidOperationException("the chart could not draw"), 1032));
        // Another one is said; the first again once its quiet time is over.
        Assert.False(safe.Run(() => throw new ArgumentException("a layout failed"), 1048));
        Assert.False(safe.Run(() => throw new InvalidOperationException("the chart could not draw"), 1048 + FailSafe.Quiet));
        Assert.Equal(["the chart could not draw", "a layout failed", "the chart could not draw"], said);
        Assert.True(safe.Run(() => { }, 9000));
        // Out of memory is no failure to go on from.
        Assert.Throws<OutOfMemoryException>(() => safe.Run(() => throw new OutOfMemoryException(), 9000));
    }

    [Theory]
    // Windows and X11 at 100 percent: the window is in pixels, the scale 1.
    [InlineData(1100, 760, 1100, 760, 1.0f, 1.0f, 1.0)]
    // X11 or Windows at 150 percent: the framebuffer is the window, the content scale says how large a unit is.
    [InlineData(1650, 1140, 1650, 1140, 1.5f, 1.5f, 1.5)]
    [InlineData(2200, 1520, 2200, 1520, 2.0f, 2.0f, 2.0)]
    // macOS Retina and Wayland: the system scales the window, the framebuffer is larger.
    [InlineData(1100, 760, 2200, 1520, 2.0f, 2.0f, 1.0)]
    // A system that gives no content scale: the framebuffer's ratio.
    [InlineData(1100, 760, 2200, 1520, 0.0f, 2.0f, 1.0)]
    [InlineData(1100, 760, 1100, 760, 0.0f, 1.0f, 1.0)]
    public void A_window_draws_at_the_system_s_content_scale(int width, int height, int pixelsX, int pixelsY, float content, float scale, double perUnit)
    {
        var (gotScale, gotPerUnit) = WindowSystem.ScaleOf((width, height), (pixelsX, pixelsY), content);
        Assert.Equal(scale, gotScale, 3);
        Assert.Equal(perUnit, gotPerUnit, 3);
        // The screen's units: the same window on every system.
        Assert.Equal(1100, width / gotPerUnit, 3);
    }

    [Fact]
    public void A_window_asks_for_OpenGL_3_3_core_then_older_contexts_and_can_draw_in_software()
    {
        Assert.Equal(ContextProfile.Core, WindowSystem.Contexts[0].Profile);
        Assert.Equal((3, 3), (WindowSystem.Contexts[0].Version.MajorVersion, WindowSystem.Contexts[0].Version.MinorVersion));
        // Down to Windows' own OpenGL 1.1, as over Remote Desktop.
        Assert.Equal((1, 1), (WindowSystem.Contexts[^1].Version.MajorVersion, WindowSystem.Contexts[^1].Version.MinorVersion));
        Assert.All(WindowSystem.Contexts.Skip(1), c => Assert.Equal(ContextProfile.Compatability, c.Profile));
        Assert.True(WindowSystem.SoftwareAsked(n => n == "CLARITY_RENDER" ? "software" : null));
        Assert.True(WindowSystem.SoftwareAsked(n => n == "CLARITY_RENDER" ? " Software " : null));
        Assert.False(WindowSystem.SoftwareAsked(_ => null));
        Assert.False(WindowSystem.SoftwareAsked(n => n == "CLARITY_RENDER" ? "gpu" : null));
    }

    [Fact]
    public void A_file_dialog_belongs_to_the_window_it_was_asked_from()
    {
        var owner = new SystemFilePicker.SystemWindow(X11: 0x3a00007);
        var ask = new SystemFilePicker.Ask(SystemFilePicker.Kind.Open, "Choose a file", Owner: owner);
        var commands = SystemFilePicker.Commands(OSPlatform.Linux, ask);
        Assert.Equal(["--file-selection", "--title=Choose a file", "--modal", "--attach=60817415"], commands[0].Arguments);
        Assert.Equal(["--title", "Choose a file", "--attach", "60817415"], commands[1].Arguments.Take(4));
        // The portal names its parent window as x11: and the number in hex.
        Assert.Equal("x11:3a00007", owner.Portal);
        Assert.Equal("", new SystemFilePicker.SystemWindow().Portal);
        // Without a window the dialog belongs to none.
        Assert.DoesNotContain("--modal", SystemFilePicker.Commands(OSPlatform.Linux, ask with { Owner = null })[0].Arguments);
    }

    [Fact]
    public async Task While_a_file_dialog_is_open_the_windows_take_no_input()
    {
        using var open = new SemaphoreSlim(0);
        SystemFilePicker.Ask? asked = null;
        var picker = new SystemFilePicker(ask =>
        {
            asked = ask;
            open.Wait();
            return null;
        }, _ => { }, action => action())
        {
            Owner = () => new SystemFilePicker.SystemWindow(X11: 7),
        };
        Assert.False(picker.Showing);
        var choosing = picker.ChooseFolder("Choose a folder");
        Assert.True(picker.Showing);
        open.Release();
        Assert.Null(await choosing);
        Assert.False(picker.Showing);
        Assert.Equal(7ul, asked!.Owner!.X11);
    }

    [Fact]
    public void What_a_dialog_says_as_it_closes_shows_in_the_window_below()
    {
        var dialog = new List<Message> { new(Level.Info, Text.Plain("Kept the note")), new(Level.Warning, Text.Plain("Check the date")) };
        var below = new List<Message> { new(Level.Info, Text.Plain("Earlier")) };
        DesktopWindow.HandDown(dialog, below);
        Assert.Equal(["Earlier", "Kept the note", "Check the date"], below.Select(m => Locale.Show(m.Text)));
        Assert.Empty(dialog);
    }

    [Fact]
    public void The_pointer_takes_the_system_s_shape_of_what_is_under_it()
    {
        Assert.Equal(Silk.NET.Input.StandardCursor.Hand, DesktopWindow.StandardOf(Cursor.Hand));
        Assert.Equal(Silk.NET.Input.StandardCursor.IBeam, DesktopWindow.StandardOf(Cursor.Text));
        Assert.Equal(Silk.NET.Input.StandardCursor.HResize, DesktopWindow.StandardOf(Cursor.ResizeLeftRight));
        Assert.Equal(Silk.NET.Input.StandardCursor.VResize, DesktopWindow.StandardOf(Cursor.ResizeUpDown));
        Assert.Equal(Silk.NET.Input.StandardCursor.ResizeAll, DesktopWindow.StandardOf(Cursor.Move));
        Assert.Equal(Silk.NET.Input.StandardCursor.Crosshair, DesktopWindow.StandardOf(Cursor.Crosshair));
        Assert.Equal(Silk.NET.Input.StandardCursor.NotAllowed, DesktopWindow.StandardOf(Cursor.NotAllowed));
        Assert.Equal(Silk.NET.Input.StandardCursor.Default, DesktopWindow.StandardOf(Cursor.Default));
    }

    [Fact]
    public void A_notch_of_the_wheel_is_fifty()
    {
        Assert.Equal(50, DesktopWindow.WheelNotch);
    }
}
