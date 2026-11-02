// The sound part (DESIGN 5.22): heard, not seen, with no place in the
// window. Code tells it what to play through its PartState, as it tells a
// custom part: play its source (a sound of the program's resources), stop,
// a tone, a melody, a volume. Everything plays through SoundPart.Output: the
// page's audio in a browser (BrowserPage), the desktop's once its host sets
// one, nothing until then. It plays only while its window is on the screen:
// what it plays stops as its window leaves (closed, or under a window opened
// in place), and a sound told to play while it is off the screen is not heard.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>Where sounds play: the computer's speakers on the desktop, the page's audio in a browser.</summary>
public interface ISoundOutput
{
    /// <summary>
    /// Plays <paramref name="file"/> (WAV, MP3 or FLAC data) at
    /// <paramref name="volume"/> (0 to 1), over and over when
    /// <paramref name="repeat"/>; disposing it stops it.
    /// </summary>
    IDisposable Play(byte[] file, float volume, bool repeat);
}

/// <summary>No output: what a program plays before it has one, and on a server.</summary>
public sealed class SilentOutput : ISoundOutput
{
    sealed class Nothing : IDisposable
    {
        public void Dispose() { }
    }

    public IDisposable Play(byte[] file, float volume, bool repeat) => new Nothing();
}

/// <summary>Sounds through the browser page the program runs in, by the page's own audio.</summary>
public sealed class PageSoundOutput(IBrowserPage page) : ISoundOutput
{
    sealed class Playing(IBrowserPage page, int sound) : IDisposable
    {
        bool stopped;

        public void Dispose()
        {
            if (stopped)
                return;
            stopped = true;
            page.Stop(sound);
        }
    }

    public IDisposable Play(byte[] file, float volume, bool repeat) => new Playing(page, page.Play(file, volume, repeat));
}

[PartKind("sound")]
public sealed class SoundPart : Part
{
    static ISoundOutput? output;

    /// <summary>Where every sound plays: the one the host sets, else the browser page's, else none.</summary>
    public static ISoundOutput Output
    {
        get => output ?? (BrowserPage.Current is { } page ? new PageSoundOutput(page) : new SilentOutput());
        set => output = value;
    }

    readonly List<IDisposable> playing = [];

    /// <summary>Never seen: it takes no pointer, no focus and draws nothing.</summary>
    public override bool IsShown => false;

    /// <summary>How loud it plays, 0 to 100: its <c>volume</c>, until code sets another.</summary>
    public int Volume { get; private set; } = 100;

    /// <summary>A sound of the program's resources, which <c>play</c> plays.</summary>
    public string? Source => Spec.Text("source");

    /// <summary>Whether <c>play</c> plays its source over and over until <c>stop</c>.</summary>
    public bool Repeat => Spec.Flag("repeat");

    /// <summary>How many sounds it plays now.</summary>
    public int Playing => playing.Count;

    /// <summary>Each thing code tells it, as it hears it.</summary>
    public event Action<string>? Heard;

    protected internal override void Made()
    {
        Volume = Math.Clamp((int)(Spec.Number("volume") ?? 100), 0, 100);
        // What code told it before the window showed comes first.
        State?.Listen((_, _) => { }, Hear);
    }

    /// <summary>Laid out with its window: it takes no room.</summary>
    public override void Arrange(Box box) => Bounds = new Box(box.X, box.Y, 0, 0);

    void Hear(string hear, object?[] given)
    {
        Heard?.Invoke(hear);
        switch (hear)
        {
            case "play":
                if (Source is { Length: > 0 } source && Host.Resource(source) is { } file)
                    Start(file, Repeat);
                break;
            case "stop":
                Stop();
                break;
            case "tone":
                Start(Melodies.Wav([new Note(Number(given, 0), Number(given, 1))]), false);
                break;
            case "melody":
                Start(Melodies.Wav(Melodies.Read(given.ElementAtOrDefault(0)?.ToString() ?? "", (long)Number(given, 1)).Plays), false);
                break;
            case "volume":
                Volume = Math.Clamp((int)Number(given, 0), 0, 100);
                break;
        }
    }

    /// <summary>A told value as a number, however code gave it.</summary>
    static double Number(object?[] given, int index) =>
        given.ElementAtOrDefault(index) switch
        {
            long l => l,
            int i => i,
            decimal d => (double)d,
            double d => d,
            var other => double.TryParse(other?.ToString(), NumberStyles.Float, CultureInfo.InvariantCulture, out var n) ? n : 0,
        };

    void Start(byte[] file, bool repeat)
    {
        if (Attached)
            playing.Add(Output.Play(file, Volume / 100f, repeat));
    }

    /// <summary>Its window left the screen: what it plays stops.</summary>
    protected override void OnDetached() => Stop();

    /// <summary>Stops everything it plays.</summary>
    public void Stop()
    {
        foreach (var p in playing)
            p.Dispose();
        playing.Clear();
    }
}
