// The desktop host (DESIGN 7.9) without a window: what it gives the parts
// (sound through SoundFlow, the clock of its frame loop, the system's file
// dialogs), a sound device that does not open leaving the program silent,
// and the dialog each system shows. No test plays a sound or opens a dialog.

using System.Runtime.InteropServices;
using Clarity.Runtime;
using Clarity.Ui.Desktop;

namespace Clarity.Ui.Tests;

[Collection("Ticks")]
public class DesktopHostTests
{
    [Fact]
    public void The_desktop_plays_through_soundflow_and_ticks_from_its_loop()
    {
        var (output, source) = (SoundPart.Output, Ticks.Source);
        try
        {
            var ticks = DesktopWindow.Prepare();
            // Made, not opened: no device is asked for until a sound plays.
            var sound = Assert.IsType<SoundFlowOutput>(SoundPart.Output);
            Assert.False(sound.Failed);
            Assert.Same(ticks, Ticks.Source);
            Assert.IsType<SystemFilePicker>(DesktopWindow.Files);
        }
        finally
        {
            (SoundPart.Output, Ticks.Source) = (output, source);
        }
    }

    [Fact]
    public void Without_a_sound_device_it_plays_nothing_and_runs_on()
    {
        var asked = 0;
        var output = new SoundFlowOutput(() =>
        {
            asked++;
            throw new InvalidOperationException("no playback device");
        });
        var playing = output.Play(Melodies.Wav([new Note(440, 0.1)]), 1f, repeat: false);
        playing.Dispose();
        output.Play(new byte[10], 0.5f, repeat: true).Dispose();
        Assert.True(output.Failed);
        // Asked once: a device that did not open is not asked for again.
        Assert.Equal(1, asked);
    }

    [Fact]
    public void When_miniaudio_itself_finds_no_device_the_program_runs_on_silent()
    {
        // The real native miniaudio, asked for a backend its build has not (null): it opens nothing.
        using var output = new SoundFlowOutput(() => SoundFlowOutput.Open([SoundFlow.Backends.MiniAudio.Enums.MiniAudioBackend.Null]));
        output.Play(Melodies.Wav(Melodies.Read("C4 E4 G4", 240).Plays), 0.5f, repeat: true).Dispose();
        Assert.True(output.Failed);
        output.Play([1, 2, 3], 1f, repeat: false).Dispose();
    }

    static readonly SystemFilePicker.Ask Open = new(SystemFilePicker.Kind.Open, "Choose a file", Filter: new FileFilter("Pictures", ["*.png", "*.jpg"]));
    static readonly SystemFilePicker.Ask Save = new(SystemFilePicker.Kind.Save, "Save a copy", Name: "notes.txt");
    static readonly SystemFilePicker.Ask Folder = new(SystemFilePicker.Kind.Folder, "Choose a folder");

    [Fact]
    public void Each_system_shows_its_own_dialogs()
    {
        Assert.Equal([SystemFilePicker.Way.Portal, SystemFilePicker.Way.Commands], SystemFilePicker.Ways(OSPlatform.Linux));
        Assert.Equal([SystemFilePicker.Way.Commands], SystemFilePicker.Ways(OSPlatform.OSX));
        Assert.Equal([SystemFilePicker.Way.CommonItemDialog], SystemFilePicker.Ways(OSPlatform.Windows));
        Assert.Empty(SystemFilePicker.Commands(OSPlatform.Windows, Open));
    }

    [Fact]
    public void Linux_asks_zenity_then_kdialog()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var open = SystemFilePicker.Commands(OSPlatform.Linux, Open);
        Assert.Equal(["zenity", "kdialog"], open.Select(c => c.Program));
        Assert.Equal(["--file-selection", "--title=Choose a file", "--file-filter=Pictures | *.png *.jpg"], open[0].Arguments);
        Assert.Equal(["--title", "Choose a file", "--getopenfilename", home, "Pictures (*.png *.jpg)"], open[1].Arguments);
        var save = SystemFilePicker.Commands(OSPlatform.Linux, Save);
        Assert.Equal(["--file-selection", "--title=Save a copy", "--save", "--confirm-overwrite", "--filename=notes.txt"], save[0].Arguments);
        Assert.Equal(["--title", "Save a copy", "--getsavefilename", Path.Combine(home, "notes.txt")], save[1].Arguments);
        var folder = SystemFilePicker.Commands(OSPlatform.Linux, Folder);
        Assert.Equal(["--file-selection", "--title=Choose a folder", "--directory"], folder[0].Arguments);
        Assert.Equal(["--title", "Choose a folder", "--getexistingdirectory", home], folder[1].Arguments);
        Assert.All(open.Concat(save).Concat(folder), c => Assert.False(c.Shell));
    }

    [Fact]
    public void MacOS_asks_applescript()
    {
        Assert.Equal(["-e", "POSIX path of (choose file with prompt \"Choose a file\" of type {\"png\", \"jpg\"})"], Assert.Single(SystemFilePicker.Commands(OSPlatform.OSX, Open)).Arguments);
        Assert.Equal(["-e", "POSIX path of (choose file name with prompt \"Save a copy\" default name \"notes.txt\")"], SystemFilePicker.Commands(OSPlatform.OSX, Save)[0].Arguments);
        Assert.Equal(["-e", "POSIX path of (choose folder with prompt \"Choose a folder\")"], SystemFilePicker.Commands(OSPlatform.OSX, Folder)[0].Arguments);
        // A title is quoted as an AppleScript string: it cannot end the script's own.
        var sly = new SystemFilePicker.Ask(SystemFilePicker.Kind.Folder, "a \"b\" \\c");
        Assert.Equal("POSIX path of (choose folder with prompt \"a \\\"b\\\" \\\\c\")", SystemFilePicker.Commands(OSPlatform.OSX, sly)[0].Arguments[1]);
        Assert.Equal("osascript", SystemFilePicker.Commands(OSPlatform.OSX, sly)[0].Program);
    }

    [Fact]
    public void A_file_is_shown_in_the_system_s_program_for_its_kind()
    {
        Assert.Equal(new SystemAddress.Command("xdg-open", ["/tmp/a.pdf"], false).ToString(), SystemFilePicker.ShowCommand(OSPlatform.Linux, "/tmp/a.pdf").ToString());
        Assert.Equal("open", SystemFilePicker.ShowCommand(OSPlatform.OSX, "/tmp/a.pdf").Program);
        Assert.True(SystemFilePicker.ShowCommand(OSPlatform.Windows, @"C:\a.pdf").Shell);
        Assert.Equal("passwd", SystemFilePicker.SafeName("../../etc/passwd"));
        Assert.Equal("file", SystemFilePicker.SafeName(".."));
    }

    [Fact]
    public void A_program_s_answer_is_its_first_line_and_nothing_is_no_choice()
    {
        Assert.Equal("/home/ada/notes.txt", SystemFilePicker.Answer("/home/ada/notes.txt\n"));
        Assert.Equal("/Users/ada/Documents", SystemFilePicker.Answer("/Users/ada/Documents/\n"));
        Assert.Null(SystemFilePicker.Answer("\n"));
        Assert.Equal([], SystemFilePicker.Extensions(new FileFilter("All", ["*"])));
    }

    [Fact]
    public async Task What_the_person_chose_comes_back_on_the_screen_s_thread()
    {
        var file = Path.GetTempFileName();
        File.WriteAllBytes(file, [1, 2, 3]);
        var posted = new System.Collections.Concurrent.BlockingCollection<Action>();
        try
        {
            var picker = new SystemFilePicker(_ => file, _ => { }, posted.Add);
            var chosen = picker.OpenFile("Choose a file");
            // Chosen off the screen's thread; the answer waits for the screen.
            Assert.True(posted.TryTake(out var back, TimeSpan.FromSeconds(10)));
            Assert.False(chosen.IsCompleted);
            back!();
            var got = await chosen;
            Assert.Equal(Path.GetFileName(file), got!.Name);
            Assert.Equal([1, 2, 3], got.Bytes);
            var none = new SystemFilePicker(_ => null, _ => { }, posted.Add).ChooseFolder("Choose a folder");
            Assert.True(posted.TryTake(out back, TimeSpan.FromSeconds(10)));
            back!();
            Assert.Null(await none);
        }
        finally
        {
            File.Delete(file);
        }
    }

    [Fact]
    public async Task Saving_writes_the_copy_where_the_person_chose()
    {
        var place = Path.Combine(Path.GetTempPath(), "clarity-save-" + Guid.NewGuid().ToString("N") + ".txt");
        try
        {
            SystemFilePicker.Ask? asked = null;
            var picker = new SystemFilePicker(ask => { asked = ask; return place; }, _ => { }, action => action());
            Assert.Equal(place, await picker.SaveFile("Save a copy", "notes.txt", [7, 8]));
            Assert.Equal(new SystemFilePicker.Ask(SystemFilePicker.Kind.Save, "Save a copy", Name: "notes.txt"), asked);
            Assert.Equal([7, 8], File.ReadAllBytes(place));
        }
        finally
        {
            File.Delete(place);
        }
    }
}
