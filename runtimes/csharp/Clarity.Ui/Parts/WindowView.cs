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
        AddChild(body);
        foreach (var region in frame.Regions)
            AddChild(region);
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
            RemoveChild(notes);
            frame = new WindowFrame(Host, ShownFrame.Of(value is null ? null : FrameSpec.Read(value), FrameSpec.Read(Spec.Frame))) { WindowName = Spec.Name };
            foreach (var region in frame.Regions)
                AddChild(region);
            AddChild(notes);
            Invalidate();
        }
    }

    /// <summary>The frame around the window's parts: its regions and the menus open over it.</summary>
    public WindowFrame Frame => frame;

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
            // A thin bar at the right says how much of the content shows, and where.
            var shown = inside.Height / grid.Content;
            var bar = new Box(Bounds.Right - ContentPadding / 2 - 2, inside.Y + inside.Height * (ScrollY / grid.Content), 4, Math.Max(16, inside.Height * shown));
            painter.Fill(bar, painter.Look.Color("strong_border"), 2);
        }
        if (Problem is not null)
            painter.Text(Problem, new Box(inside.X, inside.Y, inside.Width, GridLayout.RowHeight), painter.Look.Color("error"), painter.Look.Size("text"));
        foreach (var region in frame.Regions)
            if (region.IsShown)
                region.Draw(painter);
        notes.Draw(painter);
    }

    public override bool OnWheel(double dx, double dy)
    {
        var to = Math.Clamp(ScrollY + dy, 0, MaxScrollY);
        if (to == ScrollY)
            return false;
        ScrollY = to;
        Invalidate();
        return true;
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
    /// when the window took the key, or asked the part with the focus for it.
    /// </summary>
    public bool KeyPressed(KeyInput key)
    {
        if (Screen is not { } screen)
            return false;
        var focused = screen.Focused;
        var typing = focused is TextEditor;
        foreach (var part in ShownParts())
            if (Answers(KeysOf(part.Spec.Name, app: false, part.Spec.Text("shortcut")), key, typing))
            {
                Press(part);
                return true;
            }
        foreach (var (item, app) in FrameRules.Keyed(frame.Shown, Host))
            if (Answers(KeysOf(item.Text("name"), app, item.Text("shortcut")), key, typing))
            {
                frame.RunByKey(item, screen);
                return true;
            }
        foreach (var (part, item) in PartMenuItems())
            if (item.Text("shortcut") is { Length: > 0 } shortcut && Shortcuts.Matches(shortcut, key, typing))
            {
                PartKinds.Pick(Host, part.Spec.Name, item);
                return true;
            }
        if (key.Modifiers != Modifiers.None || key.Key is not (Key.Enter or Key.Escape))
            return false;
        // The part with the focus has been asked here, so the screen asks it no more.
        if (focused is not null && focused.OnKey(key))
            return true;
        if (key.Key == Key.Enter && focused is TextEditor { Multiline: true })
            return true;
        var flag = key.Key == Key.Enter ? "default" : "cancel";
        if (ShownParts().FirstOrDefault(p => p is ButtonPart && p.Spec.Flag(flag)) is { } pressed)
            Press(pressed);
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
        const double Width = 360;
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
                painter.Text("x", close, look.Color("muted"), size, align: TextAlign.Center);
            }
        }

        public override void OnClick()
        {
        }

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

/// <summary>A warning or an error in Clarity's own dialog: its kind, its text and OK, over a dimmed window.</summary>
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

    string Heading => Message.Level == Level.Error ? Locale.Translate("error", "Error") : Locale.Translate("warning", "Warning");

    public override void Arrange(Box box)
    {
        Bounds = box;
        var width = Math.Min(Width, Math.Max(160, box.Width - 32));
        var lines = Screen is { } screen ? Wrap(Message.Shown, width - 40, screen.Fonts, screen.Look.Size("text")).Count : 1;
        var height = 20 + 28 + lines * Line + 20 + GridLayout.RowHeight + 20;
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
        painter.Text(Heading, new Box(Panel.X + 20, Panel.Y + 20, inner, 28), tone, look.Size("heading"), bold: true);
        var size = look.Size("text");
        var lines = Wrap(Message.Shown, inner, painter.Fonts, size);
        for (var i = 0; i < lines.Count; i++)
            painter.Text(lines[i], new Box(Panel.X + 20, Panel.Y + 48 + i * Line, inner, Line), look.Color("text"), size);
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
