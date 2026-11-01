// Sounds through the browser page a program runs in (DESIGN 7.6), where
// SoundFlow cannot play.

using Clarity.Ui;

namespace Clarity.Avalonia;

/// <summary>Sounds through the browser's own audio, where SoundFlow cannot play (DESIGN 7.6).</summary>
public sealed class PageSoundOutput(IBrowserPage page) : SoundOutput
{
    sealed class Playing(IBrowserPage page, int sound) : IDisposable
    {
        public void Dispose() => page.Stop(sound);
    }

    public override IDisposable Play(byte[] file, float volume, bool repeat) => new Playing(page, page.Play(file, volume, repeat));
}
