// How a sound reads a melody (DESIGN 5.22), as every stack reads it
// (conformance/melody.json), and how tones are made into WAV data, which
// every sound output plays as it plays a file.

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class MelodyTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "melody.json"))).RootElement;

    [Fact]
    public void Melodies_read_as_the_conformance_file_says()
    {
        foreach (var c in File.GetProperty("cases").EnumerateArray())
        {
            var (plays, skips) = Melodies.Read(c.GetProperty("notes").GetString()!, c.GetProperty("tempo").GetInt64());
            var expected = c.GetProperty("plays").EnumerateArray().ToList();
            Assert.Equal(expected.Count, plays.Count);
            for (var i = 0; i < plays.Count; i++)
            {
                var hertz = expected[i].GetProperty("hertz");
                if (hertz.ValueKind == JsonValueKind.Null)
                    Assert.Null(plays[i].Hertz);
                else
                    Assert.Equal(hertz.GetDouble(), Math.Round(plays[i].Hertz!.Value, 2));
                Assert.Equal(expected[i].GetProperty("seconds").GetDouble(), Math.Round(plays[i].Seconds, 4));
            }
            Assert.Equal(c.GetProperty("skips").EnumerateArray().Select(s => s.GetString()), skips);
        }
    }

    [Fact]
    public void A_tune_is_made_into_wav_data()
    {
        var (notes, _) = Melodies.Read("A4 - A4", 120);
        var wav = Melodies.Wav(notes);
        Assert.Equal("RIFF", System.Text.Encoding.ASCII.GetString(wav, 0, 4));
        Assert.Equal("WAVE", System.Text.Encoding.ASCII.GetString(wav, 8, 4));
        // 1.5 seconds of 16-bit mono at 44100 a second, after the 44-byte header.
        Assert.Equal(44 + 2 * (int)(1.5 * Melodies.SampleRate), wav.Length);
        // The rest in the middle is silent; the notes are not.
        Assert.Equal(0, BitConverter.ToInt16(wav, 44 + 2 * (int)(0.75 * Melodies.SampleRate)));
        Assert.Contains(Enumerable.Range(0, 2000).Select(i => BitConverter.ToInt16(wav, 44 + 2 * (2000 + i))), s => Math.Abs((int)s) > 1000);
    }
}
