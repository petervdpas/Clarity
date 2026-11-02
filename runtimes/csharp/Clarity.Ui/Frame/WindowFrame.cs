// The frame around a window (DESIGN 7.9, runtimes/web/src/frame/FrameView.svelte):
// the menu bar, toolbar and ribbon at the top, the sidebars at the sides,
// the footer at the bottom, and the window's own parts in between. Each
// region is the window's own where its markup has one, else the app's.
// Clicking an item runs its handler or opens the window it targets; the
// `language` and `theme` targets pick in place from a menu of choices.

namespace Clarity.Ui;

public sealed class WindowFrame
{
    readonly List<FrameRegion> regions = [];

    /// <summary>The frame of <paramref name="shown"/> over <paramref name="host"/>.</summary>
    public WindowFrame(IPartHost host, ShownFrame shown)
    {
        Host = host;
        Shown = shown;
        if (shown.Menu is { } menu)
            regions.Add(MenuBar = new FrameMenuBar(this, menu.Value));
        if (shown.Toolbar is { } toolbar)
            regions.Add(Toolbar = new FrameToolbar(this, toolbar.Value));
        if (shown.Ribbon is { } ribbon)
            regions.Add(Ribbon = new FrameRibbon(this, ribbon.Value));
        if (shown.Left is { } left)
            regions.Add(Left = new FrameSidebar(this, left.Value));
        if (shown.Right is { } right)
            regions.Add(Right = new FrameSidebar(this, right.Value));
        if (shown.Footer is { } footer)
            regions.Add(Footer = new FrameFooter(this, footer.Value));
    }

    public IPartHost Host { get; }
    public ShownFrame Shown { get; }

    /// <summary>Whether the menu bar's access letters show, and how.</summary>
    public FrameAccess Access { get; set; }

    /// <summary>The name of the window it frames, by which the person's keys know its own commands.</summary>
    public string? WindowName { get; init; }

    /// <summary>The keys a command of its region answers; <paramref name="app"/>: the region is the app's.</summary>
    public IReadOnlyList<string> KeysOf(CommandSpec item, bool app) => FrameRules.KeysOf(Host, WindowName, item.Text("name"), app, item.Text("shortcut"));

    public FrameMenuBar? MenuBar { get; }
    public FrameToolbar? Toolbar { get; }
    public FrameRibbon? Ribbon { get; }
    public FrameSidebar? Left { get; }
    public FrameSidebar? Right { get; }
    public FrameFooter? Footer { get; }

    /// <summary>Its regions, in the order they are drawn.</summary>
    public IReadOnlyList<FrameRegion> Regions => regions;

    /// <summary>The menus open over the window: a menu bar's, a picker's.</summary>
    public FrameMenus Menus { get; } = new();

    /// <summary>What the <c>language</c> target picks from; a test gives its own.</summary>
    public FramePicker Language { get; set; } = FramePicker.Language;

    /// <summary>What the <c>theme</c> target picks from.</summary>
    public FramePicker Theme { get; set; } = FramePicker.Theme;

    public FramePicker PickerOf(string target) => target == "theme" ? Theme : Language;

    /// <summary>Where the window's own parts are, inside the regions, as last placed.</summary>
    public Box Body { get; private set; }

    /// <summary>Whether a sidebar fills the window: the window's own parts and the other sidebar make room.</summary>
    public FrameSidebar? Filling => Left is { Shows: true, Maximized: true } ? Left : Right is { Shows: true, Maximized: true } ? Right : null;

    /// <summary>The parts of the sidebars and the footer.</summary>
    public IEnumerable<Part> Parts => regions.SelectMany(r => r.InOrder()).OfType<Part>();

    /// <summary>Places the regions in <paramref name="window"/>, and says where the window's own parts go.</summary>
    public Box Arrange(Box window)
    {
        var top = window.Y;
        foreach (var region in new FrameRegion?[] { MenuBar, Toolbar, Ribbon })
            if (region is { IsShown: true })
            {
                var high = region.Wanted;
                region.Arrange(new Box(window.X, top, window.Width, high));
                top += high;
            }
        var bottom = window.Bottom;
        if (Footer is { IsShown: true } footer)
        {
            var high = footer.Wanted;
            bottom = Math.Max(top, bottom - high);
            footer.Arrange(new Box(window.X, bottom, window.Width, high));
        }
        var middle = new Box(window.X, top, window.Width, Math.Max(0, bottom - top));
        var filling = Filling;
        var left = middle.X;
        var right = middle.Right;
        if (Left is { IsShown: true } l)
        {
            var wide = filling == l ? middle.Width : Math.Min(l.Wanted, middle.Width);
            l.Arrange(new Box(left, middle.Y, wide, middle.Height));
            left += wide;
        }
        if (Right is { IsShown: true } r)
        {
            var wide = filling == r ? middle.Width : Math.Min(r.Wanted, Math.Max(0, right - left));
            r.Arrange(new Box(right - wide, middle.Y, wide, middle.Height));
            right -= wide;
        }
        Body = filling is null ? new Box(left, middle.Y, Math.Max(0, right - left), middle.Height) : new Box(middle.X, middle.Y, 0, 0);
        return Body;
    }

    /// <summary>
    /// The person picked a command at <paramref name="at"/>: the <c>language</c>
    /// or <c>theme</c> target opens its choices there; any other runs its
    /// handler or opens the window it targets.
    /// </summary>
    public void Run(CommandSpec item, Box at, Screen? screen)
    {
        if (FrameRules.PickerOf(item) is { } target)
        {
            if (screen is not null)
                Menus.Open(screen, item, MenuEntries.Choices(PickerOf(target)), at);
            return;
        }
        Menus.CloseAll();
        PartKinds.Pick(Host, item.Text("name"), item);
    }

    /// <summary>A command's key was pressed: it runs as a click on it does, a picker's choices opening at the top of the window's parts.</summary>
    public void RunByKey(CommandSpec item, Screen? screen) => Run(item, new Box(Body.X, Body.Y, 0, 0), screen);

    /// <summary>The items of a menu as they show, each run through this frame, each with the key it answers (the person's, else its designed one).</summary>
    public IReadOnlyList<MenuEntry> EntriesOf(IEnumerable<CommandSpec> items) =>
        MenuEntries.Of(Host, items, item => PartKinds.Pick(Host, item.Text("name"), item), PickerOf, item => KeysOf(item, Shown.Menu?.Inherited == true).FirstOrDefault());
}

/// <summary>A region of a window's frame: it draws in the frame's colors and knows the height or width it wants.</summary>
/// <summary>
/// Whether the menu bar's access letters show: off, while Alt is held
/// (<see cref="HeldUsed"/> once a key went with it), or kept after Alt went
/// alone, when the letter alone opens its menu.
/// </summary>
public enum FrameAccess { Off, Held, HeldUsed, Kept }

public abstract class FrameRegion(WindowFrame frame) : Element
{
    public WindowFrame Frame { get; } = frame;

    public IPartHost Host => Frame.Host;

    /// <summary>How high a region at the top or bottom wants to be; how wide a sidebar.</summary>
    public abstract double Wanted { get; }

    /// <summary>Where the pointer is over it, while it is.</summary>
    protected (double X, double Y)? Pointer { get; private set; }

    /// <summary>Whether the pointer is over <paramref name="box"/>.</summary>
    protected bool Over(Box box) => Hovered && Pointer is { } p && box.Contains(p.X, p.Y);

    public override void OnPointerMoved(PointerInput at)
    {
        Pointer = (at.ScreenX, at.ScreenY);
        Invalidate();
    }

    /// <summary>A frame does not scroll the window's parts.</summary>
    public override bool OnWheel(double dx, double dy) => true;

    /// <summary>The frame's own background, with a line on the side toward the window's parts.</summary>
    protected static void Plate(Painter painter, Box box, Box line)
    {
        painter.Fill(box, painter.Look.Color("frame"));
        painter.Fill(line, painter.Look.Color("border"));
    }
}

/// <summary>An element that holds others and shows while <paramref name="shows"/> says so: a folded sidebar's parts, the window's parts while a sidebar fills it.</summary>
public sealed class Showing(Func<bool> shows) : Element
{
    public override bool IsShown => shows();

    public void Add(Element child) => AddChild(child);

    public override void Arrange(Box box)
    {
        Bounds = box;
        foreach (var child in Children)
            child.Arrange(box);
    }
}
