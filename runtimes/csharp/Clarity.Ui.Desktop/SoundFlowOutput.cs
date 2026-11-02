// Sound on the desktop (DESIGN 5.22): SoundFlow, which plays through
// miniaudio on Windows, macOS and Linux. One engine and one playback device
// for the program, opened when a sound first plays. A computer without a
// sound device (a server, a virtual display) plays nothing and runs on: the
// device that does not open is said once and never asked for again.

using Clarity.Runtime;
using SoundFlow.Abstracts;
using SoundFlow.Abstracts.Devices;
using SoundFlow.Backends.MiniAudio;
using SoundFlow.Backends.MiniAudio.Enums;
using SoundFlow.Components;
using SoundFlow.Enums;
using SoundFlow.Providers;
using SoundFlow.Structs;

namespace Clarity.Ui.Desktop;

/// <summary>SoundFlow's output: WAV, MP3 and FLAC data, and the melodies and tones made into WAV.</summary>
public sealed class SoundFlowOutput : ISoundOutput, IDisposable
{
    internal static readonly AudioFormat Format = new() { Format = SampleFormat.F32, Channels = 2, SampleRate = 48000 };

    readonly Func<(AudioEngine Engine, AudioPlaybackDevice Device)> open;
    AudioEngine? engine;
    AudioPlaybackDevice? device;
    bool failed;

    /// <summary>The output on the system's default playback device.</summary>
    public SoundFlowOutput()
        : this(() => Open())
    {
    }

    /// <summary>The output on the device <paramref name="open"/> opens: a test gives one that fails.</summary>
    internal SoundFlowOutput(Func<(AudioEngine Engine, AudioPlaybackDevice Device)> open) => this.open = open;

    /// <summary>Whether a device was asked for and did not open: it plays nothing from then on.</summary>
    public bool Failed => failed;

    /// <summary>The playback device of the first of <paramref name="backends"/> that has one; the system's usual ones when none are named.</summary>
    internal static (AudioEngine, AudioPlaybackDevice) Open(MiniAudioBackend[]? backends = null)
    {
        var engine = backends is null ? new MiniAudioEngine() : new MiniAudioEngine(backends);
        try
        {
            var device = engine.InitializePlaybackDevice(null, Format);
            device.Start();
            return (engine, device);
        }
        catch
        {
            engine.Dispose();
            throw;
        }
    }

    sealed class Playing(Mixer mixer, SoundPlayer player, AssetDataProvider data) : IDisposable
    {
        bool stopped;

        public void Dispose()
        {
            if (stopped)
                return;
            stopped = true;
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

    public IDisposable Play(byte[] file, float volume, bool repeat)
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
            // A sound that played to its end lets go of what it held, on the screen's thread.
            player.PlaybackEnded += (_, _) =>
            {
                if (!player.IsLooping)
                    Later.Post(playing.Dispose);
            };
            player.Play();
            return playing;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"a sound did not play: {e.Message}");
            return new Nothing();
        }
    }

    bool Ready()
    {
        if (device is not null)
            return true;
        if (failed)
            return false;
        try
        {
            (engine, device) = open();
            return true;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            // Without a sound device the program runs on, silent.
            failed = true;
            Console.Error.WriteLine($"no sound device: {e.Message}");
            return false;
        }
    }

    /// <summary>Lets go of the device and the engine: the program ends.</summary>
    public void Dispose()
    {
        try
        {
            device?.Stop();
            device?.Dispose();
            engine?.Dispose();
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"the sound device did not close cleanly: {e.Message}");
        }
        device = null;
        engine = null;
        failed = true;
    }
}
