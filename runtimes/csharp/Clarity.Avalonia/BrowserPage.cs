// What belongs to the browser a page runs in (DESIGN 7.6): the tab's title,
// a file to save as a download or show in a tab, and what the person keeps
// in the browser's storage. The browser program sets it, through .NET's
// JavaScript interop; on the desktop there is none, and each place does as
// the desktop does.

namespace Clarity.Avalonia;

public interface IBrowserPage
{
    void SetTitle(string title);

    /// <summary>Gives the person the file to save, as the browser's download.</summary>
    void Download(string name, byte[] data);

    /// <summary>Shows the file in a tab of its own, as the system would open it.</summary>
    void Show(string name, byte[] data);

    /// <summary>What the browser keeps under <paramref name="key"/>; null for nothing.</summary>
    string? Read(string key);

    void Write(string key, string value);

    /// <summary>Plays a sound file (WAV, MP3 or FLAC) at <paramref name="volume"/> (0 to 1); its number, to stop it by.</summary>
    int Play(byte[] file, double volume, bool repeat);

    void Stop(int sound);
}

/// <summary>Sounds through the browser's own audio, where SoundFlow cannot play (DESIGN 7.6).</summary>
public sealed class PageSoundOutput(IBrowserPage page) : SoundOutput
{
    sealed class Playing(IBrowserPage page, int sound) : IDisposable
    {
        public void Dispose() => page.Stop(sound);
    }

    public override IDisposable Play(byte[] file, float volume, bool repeat) => new Playing(page, page.Play(file, volume, repeat));
}

public static class BrowserPage
{
    /// <summary>The page the application runs in; none on the desktop.</summary>
    public static IBrowserPage? Current { get; set; }
}
