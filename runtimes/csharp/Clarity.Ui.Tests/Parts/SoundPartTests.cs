// The sound part (DESIGN 5.22), as Avalonia's SoundTests had it: what code
// tells a sound reaching the part through its PartState and its output; it
// has no place in the window, and stops as its window leaves the screen.
// How melodies are read and made into WAV data is Clarity.Runtime's
// (MelodyTests there).

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

[Collection("Ticks")]
public class SoundPartTests
{
    /// <summary>An output that keeps what it was given to play, and what was stopped.</summary>
    sealed class Recorded : ISoundOutput
    {
        public readonly List<(int Bytes, float Volume, bool Repeat)> Played = [];
        public int Stopped;

        public IDisposable Play(byte[] file, float volume, bool repeat)
        {
            Played.Add((file.Length, volume, repeat));
            return new Playing(this);
        }

        sealed class Playing(Recorded output) : IDisposable
        {
            bool done;

            public void Dispose()
            {
                if (!done)
                    output.Stopped++;
                done = true;
            }
        }
    }

    static (Screen Screen, SoundPart Sound, Recorded Output, HandTicks Ticks) Window(string markup, TestHost host)
    {
        var output = new Recorded();
        SoundPart.Output = output;
        var ticks = new HandTicks();
        Ticks.Source = ticks;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, (SoundPart)parts.Single(p => p is SoundPart), output, ticks);
    }

    [Fact]
    public void A_sound_hears_what_code_tells_it()
    {
        var host = new TestHost();
        // Told before the window shows: it hears it once it does.
        host.State("rollSound")!.Hear("volume", 40L);
        var (_, sound, output, _) = Window("""<Sound name="rollSound" at="1,1" source="roll.wav" volume="80" />""", host);
        var heard = new List<string>();
        sound.Heard += heard.Add;
        Assert.Equal(40, sound.Volume);
        host.State("rollSound")!.Hear("tone", 440L, 0.25m);
        Assert.Equal(["tone"], heard);
        // A quarter of a second of tone, at the volume code set.
        Assert.Equal([(44 + 2 * (int)(0.25 * Melodies.SampleRate), 0.4f, false)], output.Played);
    }

    [Fact]
    public void Play_plays_its_source_and_stop_stops_everything()
    {
        var host = new TestHost();
        host.Resources["roll.wav"] = new byte[100];
        var (_, sound, output, _) = Window("""<Sound name="rollSound" at="1,1" source="roll.wav" repeat="true" />""", host);
        Assert.Equal(100, sound.Volume);
        var state = host.State("rollSound")!;
        state.Hear("play");
        state.Hear("melody", "C4 E4 nope", 120L);
        Assert.Equal([(100, 1f, true), (44 + 2 * (int)(0.5 * Melodies.SampleRate) * 2, 1f, false)], output.Played);
        Assert.Equal(2, sound.Playing);
        state.Hear("stop");
        Assert.Equal(2, output.Stopped);
        Assert.Equal(0, sound.Playing);
    }

    [Fact]
    public void A_source_the_program_does_not_carry_plays_nothing()
    {
        var host = new TestHost();
        var (_, _, output, _) = Window("""<Sound name="rollSound" at="1,1" source="gone.wav" />""", host);
        host.State("rollSound")!.Hear("play");
        Assert.Empty(output.Played);
    }

    [Fact]
    public void It_has_no_place_in_the_window()
    {
        var host = new TestHost();
        var (screen, sound, _, _) = Window("""
            <Button name="go" at="1,1" span="3" caption="Go" onclick="went" />
            <Sound name="rollSound" at="1,1" span="12" />
            """, host);
        Assert.False(sound.IsShown);
        Assert.Equal(0, sound.Bounds.Width);
        var go = screen.Root.Children[0].Bounds;
        Assert.IsType<ButtonPart>(screen.At(go.X + 4, go.Y + 4));
    }

    [Fact]
    public void It_stops_as_its_window_leaves_the_screen()
    {
        var output = new Recorded();
        SoundPart.Output = output;
        var ticks = new HandTicks();
        Ticks.Source = ticks;
        var host = new TestHost();
        var spec = WindowMarkup.Read("""<Window name="Test"><Sound name="tune" at="1,1" /></Window>""").Parts[0];
        var sound = (SoundPart)PartKinds.Make(spec, host);
        var holder = new ScreenPlace();
        var screen = new Screen(holder, ThemeLook.Clarity(dark: false));
        holder.Put(sound);
        screen.Resize(400, 300, 1);
        host.State("tune")!.Hear("melody", "C4 D4 E4", 60L);
        Assert.Equal(1, sound.Playing);
        // Still on the screen: it plays on.
        ticks.TickAll();
        Assert.Equal(1, sound.Playing);
        // Off the screen: it stops at once; what it is told to play then still plays, as Avalonia's did.
        holder.Take(sound);
        Assert.Equal(0, sound.Playing);
        Assert.Equal(1, output.Stopped);
        host.State("tune")!.Hear("tone", 440L, 0.25m);
        Assert.Equal(1, sound.Playing);
        Assert.Equal(2, output.Played.Count);
        // Its sounds need no clock of their own.
        Assert.Empty(ticks.Clocks);
    }

    /// <summary>A browser page that keeps what it was asked to play and stop.</summary>
    sealed class Page : IBrowserPage
    {
        public readonly List<string> Asked = [];
        public void SetTitle(string title) { }
        public void Download(string name, byte[] data) { }
        public void Show(string name, byte[] data) { }
        public string? Read(string key) => null;
        public void Write(string key, string value) { }

        public int Play(byte[] file, double volume, bool repeat)
        {
            Asked.Add($"play {file.Length} {volume} {repeat}");
            return Asked.Count;
        }

        public void Stop(int sound) => Asked.Add($"stop {sound}");
    }

    [Fact]
    public void In_a_browser_sounds_play_through_the_page()
    {
        var page = new Page();
        var output = new PageSoundOutput(page);
        var playing = output.Play(new byte[10], 0.5f, true);
        playing.Dispose();
        Assert.Equal(["play 10 0.5 True", "stop 1"], page.Asked);
    }
}
