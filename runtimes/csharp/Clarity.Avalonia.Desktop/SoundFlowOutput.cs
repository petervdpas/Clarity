// Sound on the desktop (DESIGN 5.22): SoundFlow, miniaudio for Windows, macOS
// and Linux, native code a browser cannot load.

using SoundFlow.Abstracts.Devices;
using SoundFlow.Backends.MiniAudio;
using SoundFlow.Components;
using SoundFlow.Enums;
using SoundFlow.Providers;
using SoundFlow.Structs;

namespace Clarity.Avalonia;

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
