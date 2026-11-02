// How a sound reads a melody (DESIGN 5.22), alike on every stack
// (conformance/melody.json), and how tones are made into WAV data, which
// every sound output plays as it plays a file.

using System.Text;

namespace Clarity.Runtime;

/// <summary>A note of a melody: its frequency (null for a rest) and how long it lasts.</summary>
public readonly record struct Note(double? Hertz, double Seconds);

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

    /// <summary>A word as a note or a rest and its beats; none when it is neither.</summary>
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
        if (rest.StartsWith('#'))
        {
            shift = 1;
            rest = rest[1..];
        }
        else if (rest.StartsWith('b'))
        {
            shift = -1;
            rest = rest[1..];
        }
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
