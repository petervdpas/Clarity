// The sound part on Avalonia (DESIGN 5.22): melodies read as every stack
// reads them (conformance/melody.json), tones made into WAV data, and what
// code tells a sound reaching the Sound control through its PartState.

using System.Text.Json;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class SoundTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "melody.json"))).RootElement;

    [Fact]
    public void MelodiesReadAsTheConformanceFileSays()
    {
        foreach (var c in File.GetProperty("cases").EnumerateArray())
        {
            var notes = c.GetProperty("notes").GetString()!;
            var (plays, skips) = Melodies.Read(notes, c.GetProperty("tempo").GetInt64());
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
    public void ATuneIsMadeIntoWavData()
    {
        var (notes, _) = Melodies.Read("A4 - A4", 120);
        var wav = Melodies.Wav(notes);
        Assert.Equal("RIFF", System.Text.Encoding.ASCII.GetString(wav, 0, 4));
        Assert.Equal("WAVE", System.Text.Encoding.ASCII.GetString(wav, 8, 4));
        // 1.5 seconds of 16-bit mono at 44100 a second, after the 44-byte header.
        Assert.Equal(44 + 2 * (int)(1.5 * Melodies.SampleRate), wav.Length);
        // The rest in the middle is silent; the notes are not.
        var middle = 44 + 2 * (int)(0.75 * Melodies.SampleRate);
        Assert.Equal(0, BitConverter.ToInt16(wav, middle));
        Assert.Contains(Enumerable.Range(0, 2000).Select(i => BitConverter.ToInt16(wav, 44 + 2 * (2000 + i))), s => Math.Abs((int)s) > 1000);
    }

    sealed class Recorded : SoundOutput
    {
        public readonly List<(int Bytes, float Volume, bool Repeat)> Played = [];

        public override IDisposable Play(byte[] file, float volume, bool repeat)
        {
            Played.Add((file.Length, volume, repeat));
            return new Stopped();
        }

        sealed class Stopped : IDisposable
        {
            public void Dispose() { }
        }
    }

    [AvaloniaFact]
    public void ASoundHearsWhatCodeTellsIt()
    {
        var output = new Recorded();
        Sound.Output = output;
        var state = new PartState("rollSound");
        state.Hear("volume", 40L);
        var sound = new Sound { Source = "roll.wav", Volume = 80 };
        var heard = new List<string>();
        sound.Heard += hear => heard.Add(hear);
        sound.State = state;
        state.Hear("tone", 440L, 0.25m);
        Assert.Equal(["volume", "tone"], heard);
        Assert.Equal(40, sound.Volume);
        // A quarter of a second of tone, at the volume code set.
        Assert.Equal([(44 + 2 * (int)(0.25 * Melodies.SampleRate), 0.4f, false)], output.Played);
    }
}
