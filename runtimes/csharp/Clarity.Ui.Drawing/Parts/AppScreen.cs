// An application's windows on one screen, as every host shows them (DESIGN
// 7.9): the stack of its windows, each drawn from the markup the application
// carries (Windows/Main.cxaml), in the theme shown as the system's light or
// dark resolves it, drawn again when the person picks another theme or the
// system changes. The desktop has one per desktop window, a browser page
// one. Keys go through Clarity's dialogs first.

using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class AppScreen : IDisposable
{
    readonly Func<string, string?> carried;
    bool systemDark;
    Overlay? keys;

    /// <summary>
    /// The windows of an application whose resources <paramref name="carried"/>
    /// gives by name; addresses open through <paramref name="openAddress"/>,
    /// windows of their own through <paramref name="apart"/> where the host has
    /// them, and the system's file dialogs through <paramref name="files"/>.
    /// </summary>
    public AppScreen(Func<string, string?> carried, bool systemDark, Action<string>? openAddress = null, Action<WindowViewModel, WindowViewModel>? apart = null, WindowViewModel? opener = null, IReadOnlyList<System.Xml.Linq.XElement>? appFrame = null, IFilePicker? files = null)
    {
        this.carried = carried;
        this.systemDark = systemDark;
        Stack = new WindowStack(model => carried(MarkupName(model))) { OpenAddress = openAddress, Apart = apart, Opener = opener, AppFrame = appFrame, Files = files };
        // The window's own keys (shortcuts, default and cancel buttons, undo) come before the part with the focus.
        Screen = new Screen(Stack, Look) { Keys = Stack.KeyPressed, KeysUp = Stack.KeyReleased };
        Themes.Changed += Relook;
    }

    public WindowStack Stack { get; }
    public Screen Screen { get; }

    /// <summary>The look of the theme shown, in the system's light or dark.</summary>
    public ThemeLook Look => AppLook.Of(carried, Themes.Current, systemDark);

    /// <summary>Whether the system shows dark: changing it shows the theme's other mode.</summary>
    public bool SystemDark
    {
        get => systemDark;
        set
        {
            if (value == systemDark)
                return;
            systemDark = value;
            Relook();
        }
    }

    /// <summary>The markup of a window, by its view model: <c>MainViewModel</c> is <c>Windows/Main.cxaml</c>.</summary>
    public static string MarkupName(WindowViewModel model) => $"Windows/{WindowStack.NameOf(model)}.cxaml";

    void Relook() => Screen.Show(Look);

    /// <summary>The texts an application carries as resources, by their logical names (<c>Windows/Main.cxaml</c>); none for one it does not carry.</summary>
    public static Func<string, string?> CarriedBy(System.Reflection.Assembly app) => name =>
    {
        using var stream = app.GetManifestResourceStream(name);
        if (stream is null)
            return null;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    };

    /// <summary>
    /// The regions of the app's frame its assembly carries (<c>Apps/Shop.cxaml</c>);
    /// none when it carries none, or one that cannot be read (it says why).
    /// </summary>
    public static IReadOnlyList<System.Xml.Linq.XElement>? AppFrameOf(System.Reflection.Assembly app)
    {
        var name = app.GetManifestResourceNames().Where(n => n.StartsWith("Apps/", StringComparison.Ordinal) && n.EndsWith(".cxaml", StringComparison.Ordinal)).Order(StringComparer.Ordinal).FirstOrDefault();
        if (name is null || CarriedBy(app)(name) is not { } markup)
            return null;
        try
        {
            return WindowMarkup.ReadApp(markup);
        }
        catch (Exception e) when (e is FormatException or System.Xml.XmlException)
        {
            // Fail safe: the windows show without the app's frame, and say why on the console.
            Console.Error.WriteLine($"the app's frame {name} cannot be read: {e.Message}");
            return null;
        }
    }

    /// <summary>A key the person pressed: a dialog on top hears it first, then the window's keys, then the part with the focus.</summary>
    public void KeyPressed(KeyInput key) => Screen.KeyPressed(key);

    /// <summary>The keys window over the window the person works in, where the host opens no window of its own for it (a browser page).</summary>
    public void ShowKeys()
    {
        if (keys is not null)
            return;
        var window = new KeysWindow(KeysWindow.Catalog);
        keys = Dialogs.Open(Screen, window, window, () => keys = null);
        window.Done = () =>
        {
            if (keys is { } shown)
                Screen.Close(shown);
        };
    }

    /// <summary>The screen is no longer shown: it hears the themes no more.</summary>
    public void Dispose()
    {
        Themes.Changed -= Relook;
        Stack.Dispose();
    }
}
