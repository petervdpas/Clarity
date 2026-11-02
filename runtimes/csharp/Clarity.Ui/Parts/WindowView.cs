// A whole window of the application (DESIGN 7.9): its parts in the window's
// grid, with the content padding every stack keeps around it
// (clarity_spec::layout CONTENT_PADDING), scrolling when the window is
// smaller than its rows. What its handlers say shows here too: an info
// message as a note at the top right that goes away by itself, a warning
// or an error in Clarity's own dialog over the window, read and closed with
// OK, never the system's. Around the parts is the window's frame (its menu
// bar, toolbar, ribbon, sidebars and footer, its own or its app's). The
// window's own keys are here too: a part's or a frame command's shortcut,
// Enter for its default button and Escape for its cancel button.

using System.Collections.Specialized;
using System.Xml.Linq;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class WindowView : Element, IDisposable
{
    /// <summary>The space around the grid of a running window (clarity_spec::layout CONTENT_PADDING).</summary>
    public const double ContentPadding = 12;

    /// <summary>How long an info message stays (clarity_spec::layout MESSAGE_MS).</summary>
    public static readonly TimeSpan InfoDuration = TimeSpan.FromMilliseconds(5000);

    /// <summary>The columns of a window that says none.</summary>
    public const int StandardColumns = 12;

    readonly GridPanel grid;
    readonly Showing body;
    readonly NoteList notes;
    WindowFrame frame;
    IReadOnlyList<XElement>? appFrame;
    readonly WindowViewModel? model;
    readonly Dictionary<Message, DateTime> since = new(ReferenceEqualityComparer.Instance);
    readonly List<Part> parts = [];
    Overlay? said;
    bool disposed;

    /// <summary>The window of <paramref name="spec"/> over <paramref name="host"/>; with <paramref name="model"/>, what its handlers say shows.</summary>
    public WindowView(WindowSpec spec, IPartHost host, WindowViewModel? model = null, int columns = StandardColumns)
    {
        Spec = spec;
        Host = host;
        this.model = model;
        grid = new GridPanel(Math.Max(1, columns));
        foreach (var part in spec.Parts)
        {
            var made = PartKinds.Make(part, host);
            parts.Add(made);
            grid.Add(made, part.Place(made.Grows));
        }
        frame = new WindowFrame(host, ShownFrame.Of(null, FrameSpec.Read(spec.Frame))) { WindowName = spec.Name };
        // While a sidebar fills the window, its own parts make room.
        body = new Showing(() => frame.Filling is null);
        body.Add(grid);
        // The frame comes first in Tab order, as it is docked around the grid.
        foreach (var region in frame.Regions)
            AddChild(region);
        AddChild(body);
        notes = new NoteList(this);
        AddChild(notes);
        host.Changed += Invalidate;
        if (model is not null)
            model.Messages.CollectionChanged += MessagesChanged;
    }

    public WindowSpec Spec { get; }
    public IPartHost Host { get; }

    /// <summary>The window's own parts, in the order of its markup.</summary>
    public IReadOnlyList<Part> Parts => parts;

    /// <summary>
    /// The regions of the app's frame (<c>WindowMarkup.ReadApp</c> of
    /// <c>Apps/Shop.cxaml</c>), which the window shows where it has none of
    /// its own; none for a dialog or a window without an app.
    /// </summary>
    public IReadOnlyList<XElement>? AppFrame
    {
        get => appFrame;
        set
        {
            appFrame = value;
            frame.Menus.CloseAll();
            foreach (var region in frame.Regions)
                RemoveChild(region);
            RemoveChild(body);
            RemoveChild(notes);
            frame = new WindowFrame(Host, ShownFrame.Of(value is null ? null : FrameSpec.Read(value), FrameSpec.Read(Spec.Frame))) { WindowName = Spec.Name };
            foreach (var region in frame.Regions)
                AddChild(region);
            AddChild(body);
            AddChild(notes);
            Invalidate();
        }
    }

    /// <summary>The frame around the window's parts: its regions and the menus open over it.</summary>
    public WindowFrame Frame => frame;

    /// <summary>How wide its sidebars are now: each shown one's width, a folded one its strip; one that fills the window counts its own width, so giving the window back grows nothing.</summary>
    public double SidebarsWidth => new[] { frame.Left, frame.Right }.Sum(s => s is { Shows: true } ? s.Wanted : 0);

    /// <summary>Why the window shows nothing: its markup could not be read.</summary>
    public string? Problem { get; init; }

    /// <summary>The clock the info messages go by; a test sets its own.</summary>
    public Func<DateTime> Now { get; set; } = () => DateTime.UtcNow;

    /// <summary>How far the window's content is scrolled down, and how far it can be.</summary>
    public double ScrollY { get; private set; }
    public double MaxScrollY { get; private set; }

    /// <summary>The place of the grid: inside the frame, inside the padding.</summary>
    Box Inside => frame.Body.Inset(ContentPadding);

    /// <summary>The columns a window's markup gives its grid (<c>span</c> on the window), else the standard 12.</summary>
    public static int ColumnsOf(string markup)
    {
        var root = XDocument.Parse(markup).Root;
        var written = (string?)root?.Attribute("span") ?? (string?)root?.Attribute("columns");
        return int.TryParse(written, out var columns) && columns > 0 ? columns : StandardColumns;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        frame.Arrange(box);
        var inside = Inside;
        body.Arrange(inside);
        MaxScrollY = Math.Max(0, grid.Content - inside.Height);
        ScrollY = Math.Clamp(ScrollY, 0, MaxScrollY);
        if (MaxScrollY > 0)
            grid.Arrange(new Box(inside.X, inside.Y - ScrollY, inside.Width, grid.Content));
        notes.Arrange(box);
        if (said is not null)
            said.Place = box;
        Say();
    }

    public override void Draw(Painter painter)
    {
        var inside = Inside;
        if (body.IsShown)
            using (painter.Clip(inside))
                grid.Draw(painter);
        if (MaxScrollY > 0 && body.IsShown)
        {
            // A bar at the right says how much of the content shows, and where; it is wider while the pointer is on it or drags it.
            var thumb = Thumb;
            var wide = dragFrom is not null || (Hovered && hoverX >= Track.X);
            var bar = wide ? thumb with { X = Track.X + 2, Width = Track.Width - 4 } : thumb;
            painter.Fill(bar, painter.Look.Color(wide ? "muted" : "strong_border"), bar.Width / 2);
        }
        if (Problem is not null)
            painter.Text(Problem, new Box(inside.X, inside.Y, inside.Width, GridLayout.RowHeight), painter.Look.Color("error"), painter.Look.Size("text"));
        foreach (var region in frame.Regions)
            if (region.IsShown)
                region.Draw(painter);
        notes.Draw(painter);
    }

    public override bool OnWheel(double dx, double dy) => ScrollTo(ScrollY + dy);

    /// <summary>Scrolls the window's content to <paramref name="y"/>, within what it can; true when it moved.</summary>
    public bool ScrollTo(double y)
    {
        var to = Math.Clamp(y, 0, MaxScrollY);
        if (to == ScrollY)
            return false;
        ScrollY = to;
        if (Bounds.Width > 0)
            Arrange(Bounds);
        Invalidate();
        return true;
    }

    /// <summary>Where the scroll bar runs: the padding at the window's right, beside the content.</summary>
    Box Track => new(Bounds.Right - ContentPadding, Inside.Y, ContentPadding, Inside.Height);

    /// <summary>The part of the bar that says what shows: as long as the share shown, where it shows.</summary>
    public Box Thumb
    {
        get
        {
            var inside = Inside;
            var content = Math.Max(grid.Content, 1);
            var height = Math.Max(16, inside.Height * inside.Height / content);
            var y = inside.Y + (inside.Height - height) * (MaxScrollY <= 0 ? 0 : ScrollY / MaxScrollY);
            return new Box(Bounds.Right - ContentPadding / 2 - 2, y, 4, height);
        }
    }

    double hoverX;
    (double PointerY, double ScrollY)? dragFrom;

    public override void OnPointerMoved(PointerInput at)
    {
        hoverX = at.ScreenX;
        if (dragFrom is { } from && Pressed)
        {
            var inside = Inside;
            var room = Math.Max(1, inside.Height - Thumb.Height);
            ScrollTo(from.ScrollY + (at.ScreenY - from.PointerY) * MaxScrollY / room);
        }
        else if (MaxScrollY > 0)
            Invalidate();
    }

    /// <summary>A press on the bar: on what shows it drags it, above or below it pages there, as a scroll bar does.</summary>
    public override void OnPointerPressed(PointerInput at)
    {
        dragFrom = null;
        if (MaxScrollY <= 0 || !body.IsShown || !Track.Contains(at.ScreenX, at.ScreenY))
            return;
        var thumb = Thumb;
        if (at.ScreenY < thumb.Y)
            Page(-1);
        else if (at.ScreenY > thumb.Bottom)
            Page(1);
        else
            dragFrom = (at.ScreenY, ScrollY);
    }

    public override void OnPointerReleased(PointerInput at)
    {
        dragFrom = null;
        Invalidate();
    }

    /// <summary>One page down (1) or up (-1): the height shown, less a row, as PageDown does.</summary>
    public bool Page(int by) => ScrollTo(ScrollY + by * Math.Max(GridLayout.RowHeight, Inside.Height - GridLayout.RowHeight));

    /// <summary>The focus moved: a part of the window's grid that is out of sight scrolls into view, as Avalonia's ScrollViewer brought it.</summary>
    void FocusMoved(Element? focused)
    {
        if (focused is null || MaxScrollY <= 0 || !Within(focused, grid) || heard?.Pointing == true)
            return;
        var inside = Inside;
        var place = focused.Bounds;
        // A part in a part (a text box's editor) shows with the part it is in, up to the window's grid.
        for (var e = focused.Parent; e is not null && e != grid; e = e.Parent)
            if (e is Part)
                place = e.Bounds;
        if (place.Y < inside.Y || place.Height > inside.Height)
            ScrollTo(ScrollY - (inside.Y - place.Y));
        else if (place.Bottom > inside.Bottom)
            ScrollTo(ScrollY + (place.Bottom - inside.Bottom));
    }

    static bool Within(Element element, Element root)
    {
        for (Element? e = element; e is not null; e = e.Parent)
            if (e == root)
                return true;
        return false;
    }

    Screen? heard;

    protected override void OnAttached()
    {
        heard = Screen;
        if (heard is not null)
            heard.FocusChanged += FocusMoved;
    }

    protected override void OnDetached()
    {
        if (heard is not null)
            heard.FocusChanged -= FocusMoved;
        heard = null;
    }

    // -- Keys --------------------------------------------------------------------

    /// <summary>The parts' own menus, as their items' shortcuts reach them.</summary>
    IEnumerable<(Part Part, CommandSpec Item)> PartMenuItems()
    {
        static IEnumerable<CommandSpec> All(IEnumerable<CommandSpec> items) =>
            items.SelectMany(i => All(i.Items).Prepend(i));
        foreach (var part in ShownParts())
            if (part.Spec.Menu is { } own)
                foreach (var item in All(own.Items))
                    yield return (part, item);
    }

    /// <summary>The parts that show and take clicks: the window's own, and those of its sidebars and footer.</summary>
    IEnumerable<Part> ShownParts() => grid.InOrder().Concat(frame.Parts).OfType<Part>().Where(p => p.ShowsInTree && p.Enabled);

    /// <summary>A part pressed by a key: a button clicks, any other part as a click on it does.</summary>
    static void Press(Part part)
    {
        if (part is ButtonPart button)
            button.Click();
        else
            part.OnClick();
    }

    /// <summary>
    /// A key, before the part with the focus has it (Screen.Keys): a shortcut
    /// of a shown, enabled part runs it, then one of the frame's commands
    /// (its menu, toolbar and ribbon, while they show and take clicks), then
    /// one of a part's menu items; Enter presses the
    /// default button and Escape the cancel button, once the part with the
    /// focus did not take them (a memo's new line, a list that closes). True
    /// when the window took the key, or asked the part with the focus for it;
    /// false for Escape in a window without a cancel button.
    /// </summary>
    public bool KeyPressed(KeyInput key)
    {
        if (Screen is not { } screen)
            return false;
        if (key.Typed is null && AccessKey(key) is { } took)
            return took;
        var focused = screen.Focused;
        var typing = focused is TextEditor;
        foreach (var part in ShownParts())
            if (Answers(KeysOf(part.Spec.Name, app: false, part.Spec.Text("shortcut")), key, typing))
            {
                // What is typed in the input with the focus is taken first, as a click on the button does.
                TakesTyped.From(focused);
                Press(part);
                return true;
            }
        // While a menu of the frame is open, its commands wait for it to close, as a desktop menu's do.
        foreach (var (item, app) in frame.Menus.IsOpen || screen.ContextMenus.IsOpen ? [] : FrameRules.Keyed(frame.Shown, Host))
            if (Answers(KeysOf(item.Text("name"), app, item.Text("shortcut")), key, typing))
            {
                TakesTyped.From(focused);
                frame.RunByKey(item, screen);
                return true;
            }
        foreach (var (part, item) in PartMenuItems())
            if (item.Text("shortcut") is { Length: > 0 } shortcut && Shortcuts.Matches(shortcut, key, typing))
            {
                TakesTyped.From(focused);
                PartKinds.Pick(Host, part.Spec.Name, item);
                return true;
            }
        // F10 puts the focus on the menu bar, once no shortcut took it.
        if (key.Typed is null && key.Key == Key.F10 && key.Modifiers == Modifiers.None && frame.MenuBar?.FocusFirst() == true)
            return true;
        // Alt with a top item's access letter opens its menu, once no shortcut took the key.
        if (key.Modifiers == Modifiers.Alt && FrameRules.AccessChar(key.Key) is { } letter && frame.MenuBar is { } bar && bar.OpenByLetter(letter))
            return true;
        // PageUp and PageDown that the part with the focus left scroll the window a page.
        if (key.Typed is null && key.Modifiers == Modifiers.None && key.Key is Key.PageUp or Key.PageDown && MaxScrollY > 0)
        {
            Page(key.Key == Key.PageDown ? 1 : -1);
            return true;
        }
        if (key.Typed is not null || key.Modifiers != Modifiers.None || key.Key is not (Key.Enter or Key.Escape))
            return false;
        // The screen asked the part with the focus first; a memo keeps its Enter.
        if (key.Key == Key.Enter && focused is TextEditor { Multiline: true })
            return true;
        // Enter anywhere in a form (a checkbox, a dropdown, a radio) runs its submit, as Avalonia's Events.OnEnter did.
        if (key.Key == Key.Enter && focused is not null && PartTexts.Submit(focused))
            return true;
        var flag = key.Key == Key.Enter ? "default" : "cancel";
        if (ShownParts().FirstOrDefault(p => p is ButtonPart && p.Spec.Flag(flag)) is { } pressed)
        {
            TakesTyped.From(focused);
            Press(pressed);
        }
        // A wizard's Next or Finish is the default button of its step.
        else if (key.Key == Key.Enter && ShownParts().OfType<IDefaultButton>().FirstOrDefault(d => d.TakesEnter) is { } wizard)
        {
            TakesTyped.From(focused);
            wizard.PressDefault();
        }
        // Escape without a cancel button is left to the dialog the window is in, which closes as its close does.
        else if (key.Key == Key.Escape)
            return false;
        return true;
    }

    /// <summary>
    /// The menu bar's access letters before any other use of a key: Alt
    /// shows them; after Alt went alone the letter alone opens its menu, and
    /// any other key ends them (Escape doing nothing more). None when the key
    /// goes on as any key.
    /// </summary>
    bool? AccessKey(KeyInput key)
    {
        if (key.Key == Key.Alt)
        {
            if (frame.MenuBar is not { IsShown: true })
                return null;
            frame.Access = frame.Access == FrameAccess.HeldUsed ? FrameAccess.HeldUsed : FrameAccess.Held;
            Invalidate();
            return true;
        }
        switch (frame.Access)
        {
            case FrameAccess.Kept:
                frame.Access = FrameAccess.Off;
                Invalidate();
                if (key.Modifiers == Modifiers.None && FrameRules.AccessChar(key.Key) is { } letter && frame.MenuBar is { } bar && bar.OpenByLetter(letter))
                    return true;
                return key.Key == Key.Escape ? true : null;
            case FrameAccess.Held:
                frame.Access = FrameAccess.HeldUsed;
                return null;
            default:
                return null;
        }
    }

    /// <summary>A key let go: Alt let go alone keeps the access letters for a plain letter and puts the focus on the menu bar; after another key it hides them. True when it took the key.</summary>
    public bool KeyReleased(KeyInput key)
    {
        if (key.Key != Key.Alt || frame.Access is not (FrameAccess.Held or FrameAccess.HeldUsed))
            return false;
        frame.Access = frame.Access == FrameAccess.Held ? FrameAccess.Kept : FrameAccess.Off;
        // Alt alone puts the focus on the menu bar, its first menu marked, as a desktop menu bar does.
        if (frame.Access == FrameAccess.Kept)
            frame.MenuBar?.FocusFirst();
        Invalidate();
        return true;
    }

    /// <summary>
    /// The keys a command answers: the person's where they set their own
    /// (a command of the app's frame by its name, the window's own by
    /// <c>Window.name</c>), else its designed one.
    /// </summary>
    IReadOnlyList<string> KeysOf(string? name, bool app, string? designed) => FrameRules.KeysOf(Host, Spec.Name, name, app, designed);

    static bool Answers(IReadOnlyList<string> keys, KeyInput key, bool typing) => keys.Any(k => Shortcuts.Matches(k, key, typing));

    // -- Messages ----------------------------------------------------------------

    /// <summary>The info messages on screen, oldest first.</summary>
    public IReadOnlyList<Message> Notes => model is null ? [] : [.. model.Messages.Where(m => m.Level == Level.Info)];

    /// <summary>The warning or error its dialog shows now; none when no dialog is open.</summary>
    public Message? Saying { get; private set; }

    void MessagesChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        Tick();
        Invalidate();
    }

    /// <summary>Time went on: an info message that has stayed long enough goes, and a dialog opens for the next warning or error.</summary>
    public void Tick()
    {
        if (model is null || disposed)
            return;
        var now = Now();
        foreach (var message in model.Messages.Where(m => m.Level == Level.Info).ToList())
        {
            if (!since.TryGetValue(message, out var from))
                since[message] = now;
            else if (now - from >= InfoDuration)
                Dismiss(message);
        }
        Say();
    }

    /// <summary>Takes an info message away: its time is up, or the person closed it.</summary>
    public void Dismiss(Message message)
    {
        since.Remove(message);
        model?.Messages.Remove(message);
        Invalidate();
    }

    /// <summary>The next warning or error, in Clarity's dialog over the window, one at a time, once the window is on a screen.</summary>
    void Say()
    {
        if (model is null || disposed || said is not null || Screen is not { } screen)
            return;
        if (model.Messages.FirstOrDefault(m => m.Level != Level.Info) is not { } message)
            return;
        var dialog = new SaidDialog(message);
        Overlay? overlay = null;
        overlay = new Overlay(dialog, Bounds)
        {
            Modal = true,
            Closed = () =>
            {
                said = null;
                Saying = null;
                if (disposed)
                    return;
                model.Messages.Remove(message);
                Invalidate();
            },
        };
        dialog.Done = () => screen.Close(overlay);
        said = overlay;
        Saying = message;
        screen.Open(overlay);
        screen.Focus(dialog.Ok);
    }

    /// <summary>The window is no longer shown: it hears its host and messages no more, and its dialog closes (the message stays for the window below).</summary>
    public void Dispose()
    {
        if (disposed)
            return;
        disposed = true;
        frame.Menus.CloseAll();
        Host.Changed -= Invalidate;
        if (model is not null)
            model.Messages.CollectionChanged -= MessagesChanged;
        if (said is { } open)
            Screen?.Close(open);
        said = null;
    }

    /// <summary>The info messages, as notes at the top right of the window, each with a small x that closes it.</summary>
    sealed class NoteList(WindowView view) : Element
    {
        const double Width = 420;
        const double Line = 20;
        readonly List<(Message Message, Box Note, Box Close)> placed = [];

        public override void Arrange(Box window)
        {
            placed.Clear();
            var width = Math.Min(Width, Math.Max(0, window.Width - 2 * ContentPadding));
            var x = window.Right - ContentPadding - width;
            var y = window.Y + ContentPadding;
            var fonts = view.Screen?.Fonts;
            var size = view.Screen?.Look.Size("text") ?? 14;
            foreach (var message in view.Notes)
            {
                var lines = fonts is null ? 1 : SaidDialog.Wrap(message.Shown, width - 48, fonts, size).Count;
                var note = new Box(x, y, width, lines * Line + 20);
                placed.Add((message, note, new Box(note.Right - 30, note.Y + 6, 24, 24)));
                y = note.Bottom + 8;
            }
            Bounds = placed.Count == 0 ? new Box(window.Right, window.Y, 0, 0) : new Box(x, window.Y + ContentPadding, width, y - 8 - window.Y - ContentPadding);
        }

        public override void Draw(Painter painter)
        {
            var look = painter.Look;
            foreach (var (message, note, close) in placed)
            {
                var radius = look.Radius("panel");
                painter.Fill(note, look.Color("raised"), radius);
                painter.Stroke(note, look.Color("border"), look.Border, radius);
                painter.Fill(new Box(note.X, note.Y, 4, note.Height), look.Color("accent"), radius);
                var size = look.Size("text");
                var lines = SaidDialog.Wrap(message.Shown, note.Width - 48, painter.Fonts, size);
                for (var i = 0; i < lines.Count; i++)
                    painter.Text(lines[i], new Box(note.X + 14, note.Y + 10 + i * Line, note.Width - 48, Line), look.Color("text"), size);
                painter.Text("\u00d7", close, look.Color("muted"), size, align: TextAlign.Center);
            }
        }

        public override void OnClick()
        {
        }

        /// <summary>A note's x says what it does.</summary>
        public override string? TooltipAt(double x, double y) =>
            placed.Any(p => p.Close.Contains(x, y)) ? Locale.Translate("dismiss", "Dismiss") : null;

        public override Cursor? CursorAt(double x, double y) => placed.Any(p => p.Close.Contains(x, y)) ? Cursor.Hand : null;

        public override void OnPointerPressed(PointerInput at)
        {
            foreach (var (message, _, close) in placed)
                if (close.Contains(at.ScreenX, at.ScreenY))
                {
                    view.Dismiss(message);
                    return;
                }
        }
    }
}

/// <summary>A warning or an error in Clarity's own dialog: its text and OK over a dimmed window, its level in the color of its top edge, as the web screen's.</summary>
public sealed class SaidDialog : Element
{
    const double Width = 420;
    const double Line = 20;

    public SaidDialog(Message message)
    {
        Message = message;
        Ok = new OkButton(this);
        AddChild(Ok);
    }

    public Message Message { get; }
    public OkButton Ok { get; }

    /// <summary>The person read it: OK, Enter or Escape.</summary>
    public Action? Done { get; set; }

    /// <summary>Where the dialog itself is, in the middle of the window.</summary>
    public Box Panel { get; private set; }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var width = Math.Min(Width, Math.Max(160, box.Width - 32));
        var lines = Screen is { } screen ? Wrap(Message.Shown, width - 40, screen.Fonts, screen.Look.Size("text")).Count : 1;
        var height = 20 + lines * Line + 20 + GridLayout.RowHeight + 20;
        Panel = new Box(box.X + (box.Width - width) / 2, box.Y + Math.Max(16, (box.Height - height) / 2), width, height);
        Ok.Arrange(new Box(Panel.Right - 20 - 96, Panel.Bottom - 20 - GridLayout.RowHeight, 96, GridLayout.RowHeight));
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        // The window under it is dimmed: it takes nothing while the dialog is open.
        painter.Fill(Bounds, look.Color("text").WithAlpha(56));
        var radius = look.Radius("panel");
        painter.Fill(Panel, look.Color("panel"), radius);
        painter.Stroke(Panel, look.Color("strong_border"), look.Border, radius);
        var tone = look.Color(Message.Level == Level.Error ? "error" : "warning");
        painter.Fill(new Box(Panel.X, Panel.Y, Panel.Width, 4), tone, radius);
        var inner = Panel.Width - 40;
        var size = look.Size("text");
        var lines = Wrap(Message.Shown, inner, painter.Fonts, size);
        for (var i = 0; i < lines.Count; i++)
            painter.Text(lines[i], new Box(Panel.X + 20, Panel.Y + 20 + i * Line, inner, Line), look.Color("text"), size);
        base.Draw(painter);
    }

    /// <summary>A text in lines that fit <paramref name="width"/>, broken between words; a word wider than the line keeps a line of its own.</summary>
    public static IReadOnlyList<string> Wrap(string text, double width, Fonts fonts, double size)
    {
        var lines = new List<string>();
        foreach (var paragraph in text.Split('\n'))
        {
            var line = "";
            foreach (var word in paragraph.Split(' ', StringSplitOptions.RemoveEmptyEntries))
            {
                var longer = line.Length == 0 ? word : $"{line} {word}";
                if (line.Length > 0 && fonts.Measure(longer, "text", size, false) > width)
                {
                    lines.Add(line);
                    line = word;
                }
                else
                    line = longer;
            }
            lines.Add(line);
        }
        return lines;
    }

    /// <summary>The dialog's OK: it has the focus, so Enter takes the dialog away.</summary>
    public sealed class OkButton(SaidDialog dialog) : Element
    {
        public override bool Focusable => true;

        public override void OnClick() => dialog.Done?.Invoke();

        public override bool OnKey(KeyInput key)
        {
            if (key.Key is not (Key.Enter or Key.Space or Key.Escape))
                return false;
            dialog.Done?.Invoke();
            return true;
        }

        public override void Draw(Painter painter)
        {
            var look = painter.Look;
            var values = look.Part("button", Pressed ? "pressed" : Hovered ? "hover" : null);
            var radius = values.Radius ?? look.Radius("button");
            painter.Fill(Bounds, values.Background ?? look.Color("accent"), radius);
            painter.Stroke(Bounds, values.Border ?? look.Color("border"), look.Border, radius);
            if (Focused)
                painter.Stroke(Bounds.Inset(-2), look.Color("focus"), 2, radius + 2);
            painter.Text(Locale.Translate("ok", "OK"), Bounds, values.Text ?? look.Color("on_accent"), values.Size ?? look.Size("text"), "text", values.Bold, TextAlign.Center);
        }
    }
}
