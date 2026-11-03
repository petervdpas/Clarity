// What the program reads of a window's markup (Windows/<Name>.cxaml, which
// the library draws): the data each named part binds, so the chosen row of
// a list goes between the library's place and the view model's record. The
// library itself reads the markup for everything else.

using System.Xml;
using System.Xml.Linq;

namespace Clarity.Ui;

internal sealed class WindowMarkup
{
    readonly Dictionary<string, string> binds = [];
    readonly List<(string Part, string Event, string Handler)> pointers = [];

    /// <summary>A canvas's pointer events, which the generator writes a command for each (ClickAt_, DragAt_, DropAt_, BendAt_).</summary>
    static readonly string[] PointerEvents = ["click", "drag", "drop", "bend"];

    public static readonly WindowMarkup None = new(null);

    public WindowMarkup(string? markup)
    {
        if (string.IsNullOrEmpty(markup))
            return;
        XDocument document;
        try
        {
            document = XDocument.Parse(markup);
        }
        catch (XmlException e)
        {
            // Fail safe: markup that does not read binds nothing here; the library says why it shows nothing.
            Console.Error.WriteLine($"the window's markup does not read: {e.Message}");
            return;
        }
        foreach (var element in document.Descendants())
        {
            if (element.Attribute("name")?.Value is not { Length: > 0 } name)
                continue;
            if (element.Attribute("bind")?.Value is { Length: > 0 } bind)
                binds.TryAdd(name, bind);
            if (element.Name.LocalName == "Canvas")
                foreach (var pointer in PointerEvents)
                    if (element.Attribute("on" + pointer)?.Value is { Length: > 0 } handler)
                        pointers.Add((name, pointer, handler));
        }
    }

    /// <summary>The data the part named <paramref name="part"/> binds; none when it binds none.</summary>
    public string? Bind(string part) => binds.GetValueOrDefault(part);

    /// <summary>The canvas and its pointer event whose handler is <paramref name="handler"/> (of <paramref name="part"/> when it is given); none when no canvas wires it.</summary>
    public (string Part, string Event)? PointerOf(string handler, string? part)
    {
        foreach (var (canvas, pointer, wired) in pointers)
            if (wired == handler && (part is null || part == canvas))
                return (canvas, pointer);
        return null;
    }
}
