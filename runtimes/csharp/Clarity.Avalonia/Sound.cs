// The sound part (DESIGN 5.22): heard, not seen. Code tells it what to play
// through its PartState, as a custom part is told: play its source (a sound
// of the program's resources), stop, a tone, a melody, a volume. Files and
// made tones play through SoundOutput, SoundFlow (miniaudio) unless replaced.

using System.Text;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Platform;
using SoundFlow.Abstracts.Devices;
using SoundFlow.Backends.MiniAudio;
using SoundFlow.Components;
using SoundFlow.Enums;
using SoundFlow.Providers;
using SoundFlow.Structs;

namespace Clarity.Avalonia;

/// <summary>A note of a melody: its frequency (null for a rest) and how long it lasts.</summary>
public readonly record struct Note(double? Hertz, double Seconds);

/// <summary>How a melody's notes read, alike on every stack (conformance/melody.json), and tones made into WAV data.</summary>
public static class Melodies
{
    /// <summary>Samples a second of the WAV data tones are made into.</summary>
    public const int SampleRate = 44100;

    static readonly Dictionary<char, int> Semitones = new() { ['C'] = 0, ['D'] = 2, ['E'] = 4, ['F'] = 5, ['G'] = 7, ['A'] = 9, ['B'] = 11 };

    /// <summary>The notes of <paramref name="notes"/> at <paramref name="tempo"/> beats a minute, and the words that are not notes.</summary>
    public static (List<Note> Plays, List<string> Skips) Read(string notes, long tempo)
    {
        var beat = 60.0 / Math.Max(1, tempo);
        var plays = new List<Note>();
        var skips = new List<string>();
        foreach (var word in notes.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
        {
            if (Parse(word) is var (hertz, beats))
                plays.Add(new Note(hertz, beats * beat));
            else
                skips.Add(word);
        }
        return (plays, skips);
    }

    static (double? Hertz, double Beats)? Parse(string word)
    {
        var pitch = word;
        var beats = 1.0;
        var at = word.IndexOfAny(['*', '/']);
        if (at >= 0)
        {
            var count = word[(at + 1)..];
            if (count.Length == 0 || count[0] == '0' || !count.All(char.IsAsciiDigit) || !int.TryParse(count, out var n))
                return null;
            beats = word[at] == '*' ? n : 1.0 / n;
            pitch = word[..at];
        }
        if (pitch == "-")
            return (null, beats);
        if (pitch.Length == 0 || !Semitones.TryGetValue(pitch[0], out var semitone))
            return null;
        var rest = pitch[1..];
        var shift = 0;
        if (rest.StartsWith('#')) { shift = 1; rest = rest[1..]; }
        else if (rest.StartsWith('b')) { shift = -1; rest = rest[1..]; }
        var octave = 4;
        if (rest.Length == 1 && rest[0] is >= '0' and <= '8')
            octave = rest[0] - '0';
        else if (rest.Length != 0)
            return null;
        var midi = 12 * (octave + 1) + semitone + shift;
        return (440.0 * Math.Pow(2, (midi - 69) / 12.0), beats);
    }

    /// <summary>The notes as 16-bit mono WAV data, each tone swelling and fading so it starts and ends without a click.</summary>
    public static byte[] Wav(IEnumerable<Note> notes)
    {
        var samples = new List<short>();
        foreach (var note in notes)
        {
            var count = (int)(note.Seconds * SampleRate);
            var edge = Math.Min((int)(0.01 * SampleRate), count / 4);
            for (var i = 0; i < count; i++)
            {
                if (note.Hertz is not { } hertz)
                {
                    samples.Add(0);
                    continue;
                }
                var level = Math.Min(1.0, Math.Min(i, count - 1 - i) / (double)Math.Max(1, edge));
                samples.Add((short)(Math.Sin(2 * Math.PI * hertz * i / SampleRate) * level * 0.8 * short.MaxValue));
            }
        }
        using var data = new MemoryStream();
        using var w = new BinaryWriter(data);
        w.Write(Encoding.ASCII.GetBytes("RIFF"));
        w.Write(36 + samples.Count * 2);
        w.Write(Encoding.ASCII.GetBytes("WAVEfmt "));
        w.Write(16);
        w.Write((short)1);
        w.Write((short)1);
        w.Write(SampleRate);
        w.Write(SampleRate * 2);
        w.Write((short)2);
        w.Write((short)16);
        w.Write(Encoding.ASCII.GetBytes("data"));
        w.Write(samples.Count * 2);
        foreach (var s in samples)
            w.Write(s);
        w.Flush();
        return data.ToArray();
    }
}

/// <summary>Where sounds play: the computer's speakers through SoundFlow, unless replaced (as tests do).</summary>
public abstract class SoundOutput
{
    /// <summary>Plays <paramref name="file"/> (WAV, MP3 or FLAC data) at <paramref name="volume"/> (0 to 1), over and
    /// over when <paramref name="repeat"/>; the playing can be stopped.</summary>
    public abstract IDisposable Play(byte[] file, float volume, bool repeat);
}

/// <summary>SoundFlow's output: one engine and one playback device for the program, made when a sound first plays.</summary>
public sealed class SoundFlowOutput : SoundOutput
{
    static readonly AudioFormat Format = new() { Format = SampleFormat.F32, Channels = 2, SampleRate = 48000 };
    MiniAudioEngine? engine;
    AudioPlaybackDevice? device;
    bool failed;

    sealed class Playing(Mixer mixer, SoundPlayer player, AssetDataProvider data) : IDisposable
    {
        public void Dispose()
        {
            player.Stop();
            mixer.RemoveComponent(player);
            player.Dispose();
            data.Dispose();
        }
    }

    sealed class Nothing : IDisposable
    {
        public void Dispose() { }
    }

    /// <inheritdoc />
    public override IDisposable Play(byte[] file, float volume, bool repeat)
    {
        if (!Ready())
            return new Nothing();
        try
        {
            var data = new AssetDataProvider(engine!, file);
            var player = new SoundPlayer(engine!, Format, data) { IsLooping = repeat, Volume = volume };
            var mixer = device!.MasterMixer;
            mixer.AddComponent(player);
            var playing = new Playing(mixer, player, data);
            player.PlaybackEnded += (_, _) => { if (!player.IsLooping) Dispatch(playing.Dispose); };
            player.Play();
            return playing;
        }
        catch (Exception e)
        {
            Console.Error.WriteLine($"clarity: cannot play a sound: {e.Message}");
            return new Nothing();
        }
    }

    static void Dispatch(Action action) => global::Avalonia.Threading.Dispatcher.UIThread.Post(action);

    bool Ready()
    {
        if (device is not null)
            return true;
        if (failed)
            return false;
        try
        {
            engine = new MiniAudioEngine();
            device = engine.InitializePlaybackDevice(null, Format);
            device.Start();
            return true;
        }
        catch (Exception e)
        {
            // Without a sound device the program runs on, quietly.
            failed = true;
            Console.Error.WriteLine($"clarity: no sound: {e.Message}");
            return false;
        }
    }
}

/// <summary>A sound of a window: heard, not seen. Code tells it what to play through <see cref="State"/>.</summary>
public class Sound : Control
{
    /// <summary>Where every sound plays.</summary>
    public static SoundOutput Output { get; set; } = new SoundFlowOutput();

    public static readonly StyledProperty<PartState?> StateProperty = AvaloniaProperty.Register<Sound, PartState?>(nameof(State));
    public static readonly StyledProperty<string?> SourceProperty = AvaloniaProperty.Register<Sound, string?>(nameof(Source));
    public static readonly StyledProperty<int> VolumeProperty = AvaloniaProperty.Register<Sound, int>(nameof(Volume), 100);
    public static readonly StyledProperty<bool> RepeatProperty = AvaloniaProperty.Register<Sound, bool>(nameof(Repeat));

    /// <summary>What code sets and tells it.</summary>
    public PartState? State { get => GetValue(StateProperty); set => SetValue(StateProperty, value); }
    /// <summary>A sound of the program's resources, which <c>play</c> plays.</summary>
    public string? Source { get => GetValue(SourceProperty); set => SetValue(SourceProperty, value); }
    /// <summary>How loud it plays, 0 to 100.</summary>
    public int Volume { get => GetValue(VolumeProperty); set => SetValue(VolumeProperty, value); }
    /// <summary>Whether <c>play</c> plays its source over and over until <c>stop</c>.</summary>
    public bool Repeat { get => GetValue(RepeatProperty); set => SetValue(RepeatProperty, value); }

    /// <summary>Each thing code tells it, as it hears it.</summary>
    public event Action<string>? Heard;

    readonly List<IDisposable> playing = [];

    /// <summary>Takes no room: it is heard, not seen.</summary>
    public Sound()
    {
        IsHitTestVisible = false;
        Focusable = false;
        Width = 0;
        Height = 0;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == StateProperty)
        {
            (change.OldValue as PartState)?.StopListening();
            (change.NewValue as PartState)?.Listen((_, _) => { }, Hear);
        }
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnDetachedFromVisualTree(e);
        Stop();
    }

    void Hear(string hear, object?[] given)
    {
        Heard?.Invoke(hear);
        switch (hear)
        {
            case "play":
                if (Source is { } source && Read(source) is { } file)
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

    static double Number(object?[] given, int index) =>
        given.ElementAtOrDefault(index) switch
        {
            long l => l,
            int i => i,
            decimal d => (double)d,
            double d => d,
            var other => double.TryParse(other?.ToString(), System.Globalization.CultureInfo.InvariantCulture, out var n) ? n : 0,
        };

    void Start(byte[] file, bool repeat) => playing.Add(Output.Play(file, Math.Clamp(Volume, 0, 100) / 100f, repeat));

    /// <summary>Stops everything it plays.</summary>
    public void Stop()
    {
        foreach (var p in playing)
            p.Dispose();
        playing.Clear();
    }

    /// <summary>A sound of the program's resources, embedded as markdown files and pictures are.</summary>
    static byte[]? Read(string name)
    {
        var uri = MarkdownView.ResourceUri(System.Reflection.Assembly.GetEntryAssembly()?.GetName().Name, name);
        if (!AssetLoader.Exists(uri))
            return null;
        using var stream = AssetLoader.Open(uri);
        using var copy = new MemoryStream();
        stream.CopyTo(copy);
        return copy.ToArray();
    }
}
