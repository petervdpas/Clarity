// What the splash screen says is loading (splash.status = @loadingData):
// code sets it while the app's on start runs, and the screen asks for it a
// few times a second until the main window shows. It is a text as a message
// is one, so the screen shows a key in the person's language.

namespace Clarity.Runtime;

public static class Splash
{
    static readonly object Gate = new();
    static Text? status;

    /// <summary>Code says what loads now.</summary>
    public static void SetStatus(Text text)
    {
        lock (Gate)
            status = text;
    }

    /// <summary>What code last said loads; null while it said nothing.</summary>
    public static Text? Status
    {
        get
        {
            lock (Gate)
                return status;
        }
    }
}
