// The frame of a window as its markup writes it (DESIGN 7.8): the menu bar,
// toolbar, ribbon, sidebars and footer of the window's own `.cxaml` and of
// its app's (`Apps/Shop.cxaml`). A window shows its own region where it has
// one and the app's where it has none, as every screen does
// (clarity_spec::frame shown, runtimes/web/src/screen/frame.ts).

using System.Globalization;
using System.Text.Json.Nodes;
using System.Xml.Linq;

namespace Clarity.Ui;

/// <summary>A toolbar: its name, by which code hides and disables it, and its buttons.</summary>
public sealed record ToolbarSpec(string? Name, IReadOnlyList<CommandSpec> Items);

/// <summary>A group of a ribbon's tab: its name, its title under it and its buttons.</summary>
public sealed record RibbonGroupSpec(string? Name, string Title, IReadOnlyList<CommandSpec> Items);

/// <summary>A tab of a ribbon: its name, its title and its groups.</summary>
public sealed record RibbonTabSpec(string? Name, string Title, IReadOnlyList<RibbonGroupSpec> Groups);

/// <summary>A ribbon: its name, whether the person may fold it and whether it starts folded, and its tabs.</summary>
public sealed record RibbonSpec(string? Name, bool Collapsible, bool Collapsed, IReadOnlyList<RibbonTabSpec> Tabs);

/// <summary>
/// A sidebar on the left or right of the window: its width and the limits a
/// drag keeps it in, what the person may do with it (resize, fold, let it
/// fill the window), whether it starts folded, and its parts on a grid of its columns.
/// </summary>
public sealed record SidebarSpec(
    string Side,
    string? Name,
    double Width,
    double? Min,
    double? Max,
    bool Resizable,
    bool Collapsible,
    bool Collapsed,
    bool Maximizable,
    int Columns,
    IReadOnlyList<PartSpec> Parts)
{
    public bool Left => Side != "right";
}

/// <summary>The footer: its status text at the left and its parts on one row of its columns.</summary>
public sealed record FooterSpec(string? Status, int Columns, IReadOnlyList<PartSpec> Parts)
{
    /// <summary>The columns of a footer's grid (clarity_spec::model::FOOTER_COLUMNS).</summary>
    public const int DefaultColumns = 12;
}

/// <summary>The regions of one frame, a window's own or its app's; each is none where the frame has none.</summary>
public sealed record FrameSpec(MenuSpec? Menu, ToolbarSpec? Toolbar, RibbonSpec? Ribbon, SidebarSpec? Left, SidebarSpec? Right, FooterSpec? Footer)
{
    public static readonly FrameSpec Empty = new(null, null, null, null, null, null);

    /// <summary>A frame of the regions its markup writes: <c>Menu</c>, <c>Toolbar</c>, <c>Ribbon</c>, <c>Sidebar</c> and <c>Footer</c>; other elements are no region.</summary>
    public static FrameSpec Read(IEnumerable<XElement> regions)
    {
        MenuSpec? menu = null;
        ToolbarSpec? toolbar = null;
        RibbonSpec? ribbon = null;
        SidebarSpec? left = null, right = null;
        FooterSpec? footer = null;
        foreach (var region in regions)
            switch (region.Name.LocalName)
            {
                case "Menu":
                    menu = WindowMarkup.MenuOf(region);
                    break;
                case "Toolbar":
                    toolbar = new ToolbarSpec((string?)region.Attribute("name"), [.. region.Elements().Select(WindowMarkup.CommandOf)]);
                    break;
                case "Ribbon":
                    ribbon = RibbonOf(region);
                    break;
                case "Sidebar":
                    var sidebar = SidebarOf(region);
                    if (sidebar.Left)
                        left = sidebar;
                    else
                        right = sidebar;
                    break;
                case "Footer":
                    footer = new FooterSpec((string?)region.Attribute("status"), Whole(region, "columns") ?? FooterSpec.DefaultColumns, [.. region.Elements().Select(WindowMarkup.PartOf)]);
                    break;
            }
        return new FrameSpec(menu, toolbar, ribbon, left, right, footer);
    }

    static RibbonSpec RibbonOf(XElement ribbon) => new(
        (string?)ribbon.Attribute("name"),
        Flag(ribbon, "collapsible"),
        Flag(ribbon, "collapsed"),
        [.. ribbon.Elements("Tab").Select(tab => new RibbonTabSpec(
            (string?)tab.Attribute("name"),
            (string?)tab.Attribute("title") ?? "",
            [.. tab.Elements("Group").Select(group => new RibbonGroupSpec(
                (string?)group.Attribute("name"),
                (string?)group.Attribute("title") ?? "",
                [.. group.Elements().Select(WindowMarkup.CommandOf)]))]))]);

    /// <summary>A sidebar: what its element writes over the catalog's fields for its side (sidebar-defaults.json).</summary>
    static SidebarSpec SidebarOf(XElement sidebar)
    {
        var props = WindowMarkup.SidebarOf(sidebar);
        double? Number(string name) => props[name] is JsonValue v && v.TryGetValue<double>(out var n) ? n : null;
        bool Flag(string name) => props[name] is JsonValue v && v.TryGetValue<bool>(out var b) && b;
        return new(
            props["side"] is JsonValue side && side.TryGetValue<string>(out var s) ? s : "left",
            props["name"] is JsonValue name && name.TryGetValue<string>(out var n) ? n : null,
            Number("width") ?? 0,
            Number("min"),
            Number("max"),
            Flag("resizable"),
            Flag("collapsible"),
            Flag("collapsed"),
            Flag("maximizable"),
            Math.Max(1, (int)(Number("columns") ?? 1)),
            [.. sidebar.Elements().Select(WindowMarkup.PartOf)]);
    }

    static bool Flag(XElement element, string name) => (string?)element.Attribute(name) == "true";

    static double? Number(XElement element, string name) =>
        double.TryParse((string?)element.Attribute(name), NumberStyles.Float, CultureInfo.InvariantCulture, out var n) ? n : null;

    static int? Whole(XElement element, string name) => Number(element, name) is { } n && n >= 1 ? (int)n : null;
}

/// <summary>A region as a window shows it: its own, or the app's (<paramref name="Inherited"/>).</summary>
public sealed record Shown<T>(T Value, bool Inherited);

/// <summary>The regions a window shows: each of its own replaces the app's.</summary>
public sealed record ShownFrame(
    Shown<MenuSpec>? Menu,
    Shown<ToolbarSpec>? Toolbar,
    Shown<RibbonSpec>? Ribbon,
    Shown<SidebarSpec>? Left,
    Shown<SidebarSpec>? Right,
    Shown<FooterSpec>? Footer)
{
    public static readonly ShownFrame None = new(null, null, null, null, null, null);

    /// <summary>The frame of a window with its own regions <paramref name="own"/> in the app whose frame is <paramref name="app"/> (none for a dialog, or a window without an app).</summary>
    public static ShownFrame Of(FrameSpec? app, FrameSpec own) => new(
        Pick(own.Menu, app?.Menu),
        Pick(own.Toolbar, app?.Toolbar),
        Pick(own.Ribbon, app?.Ribbon),
        Pick(own.Left, app?.Left),
        Pick(own.Right, app?.Right),
        Pick(own.Footer, app?.Footer));

    static Shown<T>? Pick<T>(T? own, T? app) where T : class =>
        own is not null ? new Shown<T>(own, false) : app is not null ? new Shown<T>(app, true) : null;

    /// <summary>Whether it shows no region at all.</summary>
    public bool IsEmpty => Menu is null && Toolbar is null && Ribbon is null && Left is null && Right is null && Footer is null;
}
