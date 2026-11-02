// Clarity's keys window on Clarity.UI (DESIGN 5.28), where the project file
// says `keys editable`: every command per place, Everywhere (the app's
// frame) first, then each window's own, with its keys. Change, then press
// the new key; a key another command has in the same place offers a swap;
// Reset puts the design's key back, Reset all every one; a key the inputs
// need is refused, saying why. In Clarity's words, as on the web.
//
// It works in two places: as the root of a screen of its own (the host hands
// it the keys first through Screen.Keys) and as the content of a modal
// overlay over the window the person works in, where it takes the focus and
// hears the keys through it. It scrolls with the wheel when its commands are
// taller than it is.

using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>A command a key can run: its id among the person's keys, its caption, and the keys its design gives it.</summary>
public sealed record KeyCommandInfo(string Id, Text Caption, IReadOnlyList<string> Keys);

/// <summary>The commands of one place: the app's frame (<c>Id</c> empty, no title) or a window.</summary>
public sealed record KeyLayerInfo(string Id, Text? Title, IReadOnlyList<KeyCommandInfo> Commands);

/// <summary>The keys window: its places, its commands and their keys, and the buttons that change them.</summary>
public sealed class KeysWindow : Element
{
    /// <summary>How wide its content is, so a long message wraps instead of widening it.</summary>
    const double ContentWidth = 460;
    const double Pad = 16;
    const double Gap = 8;
    const double HeadingHeight = 28;
    const double MessageHeight = 76;
    const double Line = 20;
    const double TitleHeight = 22;
    const double RowHeight = 28;
    const double RowGap = 4;
    const double GroupPad = 8;
    const double KeysWidth = 120;
    const double ColumnGap = 6;
    const double MostHigh = 640;

    /// <summary>Why a key cannot be a command's, as Clarity's words say it.</summary>
    static readonly Dictionary<string, (string Key, string English)> Refusals = new()
    {
        ["typing"] = ("keysTyping", "{key} cannot be used: a key without Ctrl or Alt is typing"),
        ["tab"] = ("keysTab", "{key} cannot be used: Tab moves between fields"),
        ["enter"] = ("keysEnter", "{key} cannot be used: Enter confirms"),
        ["escape"] = ("keysEscape", "{key} cannot be used: Escape cancels"),
    };

    /// <summary>How Clarity writes the keys whose name in Clarity.UI differs, or that are symbols.</summary>
    static readonly Dictionary<Key, string> Names = new()
    {
        [Key.Plus] = "+", [Key.Minus] = "-", [Key.Multiply] = "*", [Key.Slash] = "/", [Key.Period] = ".",
        [Key.Comma] = ",", [Key.Equal] = "=", [Key.Semicolon] = ";", [Key.Quote] = "'", [Key.Backslash] = "\\",
        [Key.BracketLeft] = "[", [Key.BracketRight] = "]", [Key.Backquote] = "`",
    };

    readonly IReadOnlyList<KeyLayerInfo> layers;
    readonly KeyRules.Command[] commands;
    readonly Dictionary<string, Text> captions;
    readonly List<(KeyCommandInfo Command, Push Change, Push Reset)> rows = [];
    readonly Push swapButton;
    readonly Push cancelButton;
    readonly Push resetAll;
    readonly Push close;
    string? capturing;
    string? note;
    (string Id, string Key, string Other)? swap;
    double scroll;
    bool focusedOnce;

    /// <summary>The application's commands, which the generated application gives at its start.</summary>
    public static IReadOnlyList<KeyLayerInfo> Catalog { get; set; } = [];

    /// <summary>Its title, for its window's title bar too.</summary>
    public static string Title => Locale.Translate("keysTitle", "Keys");

    /// <summary>The person closed it: Close, or Escape while it waits for no key.</summary>
    public Action? Done { get; set; }

    public KeysWindow(IReadOnlyList<KeyLayerInfo> layers)
    {
        this.layers = layers;
        commands = [.. layers.SelectMany(l => l.Commands.Select(c => new KeyRules.Command(c.Id, [l.Id], c.Keys)))];
        captions = [];
        foreach (var c in layers.SelectMany(l => l.Commands))
            captions.TryAdd(c.Id, c.Caption);
        foreach (var c in layers.SelectMany(l => l.Commands))
        {
            var id = c.Id;
            var change = new Push(this, "change", id, () => Locale.Translate("keysChange", "Change"), () => StartCapture(id));
            var reset = new Push(this, "reset", id, () => Locale.Translate("keysReset", "Reset"), () => ResetOne(id), enabled: () => PersonKeys.Choices.ContainsKey(id));
            rows.Add((c, change, reset));
            AddChild(change);
            AddChild(reset);
        }
        swapButton = new Push(this, "swap", null, () => Locale.Translate("keysSwap", "Swap"), () =>
        {
            if (swap is { } s)
                Take(s.Id, s.Key);
        }, shown: () => swap is not null);
        cancelButton = new Push(this, "cancel", null, () => Locale.Translate("cancel", "Cancel"), () =>
        {
            swap = null;
            Changed();
        }, shown: () => swap is not null);
        resetAll = new Push(this, "resetAll", null, () => Locale.Translate("keysResetAll", "Reset all"), () =>
        {
            note = null;
            swap = null;
            PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>>());
        }, enabled: () => PersonKeys.Choices.Count > 0);
        close = new Push(this, "close", null, () => Locale.Translate("keysClose", "Close"), Close);
        AddChild(swapButton);
        AddChild(cancelButton);
        AddChild(resetAll);
        AddChild(close);
    }

    /// <summary>While it is on a screen, every change of the person's keys draws it again.</summary>
    protected override void OnAttached() => PersonKeys.Changed += PersonKeysChanged;

    protected override void OnDetached() => PersonKeys.Changed -= PersonKeysChanged;

    /// <summary>Whether it waits for the new key of a command.</summary>
    public bool Capturing => capturing is not null;

    /// <summary>Where its panel is: the whole box as a window of its own, a centered panel as an overlay.</summary>
    public Box Panel { get; private set; }

    /// <summary>The size it wants for a window of its own: its content's width, and its height up to 640.</summary>
    public Extent Wanted => new(ContentWidth + 2 * Pad, Math.Min(MostHigh, ContentHeight + 2 * Pad));

    /// <summary>How high everything it shows is, from the heading to the Close button.</summary>
    double ContentHeight =>
        HeadingHeight + Gap + MessageHeight + Gap + layers.Sum(l => TitleHeight + RowGap + GroupHeight(l) + Gap) + RowHeight;

    static double GroupHeight(KeyLayerInfo layer) =>
        2 * GroupPad + layer.Commands.Count * RowHeight + Math.Max(0, layer.Commands.Count - 1) * RowGap;

    public override bool Focusable => true;

    public override bool WantsTab => Capturing;

    /// <summary>A button of the window: its role (change, reset, swap, cancel, resetAll, close) and, for a row's, its command.</summary>
    public Push Button(string role, string? command = null) =>
        Children.OfType<Push>().First(p => p.Role == role && p.Command == command);

    /// <summary>Every text it shows now, in order: for the tests, and for a reader.</summary>
    public IReadOnlyList<string> Says
    {
        get
        {
            var said = new List<string> { Title };
            if (Message is { } message)
                said.Add(message);
            foreach (var layer in layers)
            {
                said.Add(PlaceName(layer));
                foreach (var c in layer.Commands)
                {
                    said.Add(NameOf(c.Id));
                    said.Add(KeysText(c));
                }
            }
            return said;
        }
    }

    string? Message =>
        swap is { } s ? Said("keysTaken", "{key} is the key of {command}. Swap them?", ("key", s.Key), ("command", NameOf(s.Other)))
        : note ?? (capturing is null ? null : Locale.Translate("keysPress", "Press the new key; Escape stops"));

    static string PlaceName(KeyLayerInfo layer) =>
        layer.Title is { } title ? Locale.Show(title) : Locale.Translate("keysEverywhere", "Everywhere");

    string NameOf(string id) => captions.TryGetValue(id, out var caption) ? Locale.Show(caption) : id;

    IReadOnlyList<string> KeysOf(KeyCommandInfo c) =>
        PersonKeys.Choices.TryGetValue(c.Id, out var mine) ? mine : c.Keys;

    string KeysText(KeyCommandInfo c) =>
        KeysOf(c) is { Count: > 0 } keys ? string.Join(", ", keys) : Locale.Translate("keysNone", "None");

    static string Said(string key, string english, params (string Slot, string Value)[] slots)
    {
        var text = new Text(english) { Key = key };
        foreach (var (slot, value) in slots)
            text.With(slot, Slot.OfText(value));
        return Locale.Show(text);
    }

    /// <summary>A key pressed as Clarity writes keys (<c>Ctrl+Shift+N</c>); null for a key Clarity.UI does not know.</summary>
    public static string? Written(KeyInput key)
    {
        if (key.Key == Key.Other)
            return null;
        var name = key.Key switch
        {
            >= Key.A and <= Key.Z => ((char)('A' + (key.Key - Key.A))).ToString(),
            >= Key.D0 and <= Key.D9 => ((char)('0' + (key.Key - Key.D0))).ToString(),
            _ => Names.TryGetValue(key.Key, out var written) ? written : key.Key.ToString(),
        };
        var held = new List<string>();
        if (key.Modifiers.HasFlag(Modifiers.Control))
            held.Add("Ctrl");
        if (key.Modifiers.HasFlag(Modifiers.Alt))
            held.Add("Alt");
        if (key.Modifiers.HasFlag(Modifiers.Shift))
            held.Add("Shift");
        if (key.Modifiers.HasFlag(Modifiers.Meta))
            held.Add("Meta");
        held.Add(name);
        return string.Join("+", held);
    }

    /// <summary>
    /// A key for the window: while it waits for a command's new key, the key
    /// becomes that (or is refused, saying why); Escape stops waiting, or else
    /// closes it. True when it took the key.
    /// </summary>
    public bool KeyPressed(KeyInput key)
    {
        var escape = key.Key == Key.Escape && key.Modifiers == Modifiers.None;
        if (capturing is not { } id)
        {
            if (!escape)
                return false;
            Close();
            return true;
        }
        if (escape)
        {
            capturing = null;
            Changed();
            return true;
        }
        if (Written(key) is not { } written)
            return true;
        if (KeyRules.Refused(written) is { } why)
        {
            var (word, english) = Refusals[why];
            note = Said(word, english, ("key", written));
            Changed();
            return true;
        }
        capturing = null;
        var (_, other) = KeyRules.Assign(commands, PersonKeys.Choices, id, written);
        if (other is not null)
        {
            swap = (id, written, other);
            Changed();
        }
        else
            Take(id, written);
        return true;
    }

    public override bool OnKey(KeyInput key) => KeyPressed(key);

    void StartCapture(string id)
    {
        capturing = id;
        swap = null;
        note = null;
        Changed();
    }

    void ResetOne(string id)
    {
        note = null;
        PersonKeys.Set(KeyRules.Reset(commands, PersonKeys.Choices, id));
    }

    void Take(string id, string key)
    {
        var also = KeyRules.AlsoHas(commands, PersonKeys.Choices, id, key);
        note = also.Count > 0
            ? Said("keysAlso", "{key} is also the key of {command}, elsewhere; here {mine} goes first", ("key", key), ("command", string.Join(", ", also.Select(NameOf))), ("mine", NameOf(id)))
            : null;
        swap = null;
        PersonKeys.Set(KeyRules.Assign(commands, PersonKeys.Choices, id, key).Choices);
        Changed();
    }

    void Close()
    {
        capturing = null;
        swap = null;
        Done?.Invoke();
    }

    void PersonKeysChanged() => Changed();

    /// <summary>What it shows changed: a button that no longer shows gives the focus back to the window, and it draws again.</summary>
    void Changed()
    {
        if (Screen is { } screen && screen.Focused is Push { } push && Children.Contains(push) && !push.ShowsInTree)
            screen.Focus(this);
        Invalidate();
    }

    bool IsRoot => Screen is { } screen && screen.Root == this;

    /// <summary>Where the content shows: inside the panel's padding.</summary>
    Box View => Panel.Inset(Pad);

    double MaxScroll => Math.Max(0, ContentHeight - View.Height);

    public override void Arrange(Box box)
    {
        Bounds = box;
        if (IsRoot)
            Panel = box;
        else
        {
            var width = Math.Min(ContentWidth + 2 * Pad, Math.Max(0, box.Width - 32));
            var height = Math.Min(ContentHeight + 2 * Pad, Math.Max(0, box.Height - 32));
            Panel = new Box(box.X + (box.Width - width) / 2, box.Y + (box.Height - height) / 2, width, height);
        }
        scroll = Math.Clamp(scroll, 0, MaxScroll);
        var view = View;
        var x = view.X;
        var width_ = Math.Min(ContentWidth, view.Width);
        var y = view.Y - scroll;
        // The swap offer's buttons sit at the bottom of the message line, kept for it.
        var messageBottom = y + HeadingHeight + Gap + MessageHeight;
        var swapWidth = ButtonWidth(swapButton);
        swapButton.Arrange(new Box(x, messageBottom - RowHeight, swapWidth, RowHeight));
        cancelButton.Arrange(new Box(x + swapWidth + ColumnGap, messageBottom - RowHeight, ButtonWidth(cancelButton), RowHeight));
        y = messageBottom + Gap;
        var index = 0;
        foreach (var layer in layers)
        {
            y += TitleHeight + RowGap + GroupPad;
            foreach (var _ in layer.Commands)
            {
                var (_, change, reset) = rows[index++];
                var resetWidth = ButtonWidth(reset);
                var changeWidth = ButtonWidth(change);
                var right = x + width_ - GroupPad;
                reset.Arrange(new Box(right - resetWidth, y, resetWidth, RowHeight));
                change.Arrange(new Box(right - resetWidth - ColumnGap - changeWidth, y, changeWidth, RowHeight));
                y += RowHeight + RowGap;
            }
            if (layer.Commands.Count > 0)
                y -= RowGap;
            y += GroupPad + Gap;
        }
        var closeWidth = ButtonWidth(close);
        var allWidth = ButtonWidth(resetAll);
        close.Arrange(new Box(x + width_ - closeWidth, y, closeWidth, RowHeight));
        resetAll.Arrange(new Box(x + width_ - closeWidth - ColumnGap - allWidth, y, allWidth, RowHeight));
        if (!focusedOnce && Screen is { } screen)
        {
            focusedOnce = true;
            if (screen.Focused is null || !Within(screen.Focused))
                screen.Focus(this);
        }
    }

    bool Within(Element element)
    {
        for (Element? e = element; e is not null; e = e.Parent)
            if (e == this)
                return true;
        return false;
    }

    double ButtonWidth(Push push) =>
        Math.Max(72, FontsNow.Measure(push.Caption, "text", LookNow.Size("text"), false) + 24);

    /// <summary>Whether a button is in the part of the content that shows.</summary>
    internal bool InView(Box box)
    {
        var view = IsRoot ? Panel : Panel.Inset(1);
        return box.Y >= view.Y && box.Bottom <= view.Bottom;
    }

    public override bool OnWheel(double dx, double dy)
    {
        if (MaxScroll <= 0)
            return false;
        scroll = Math.Clamp(scroll + dy, 0, MaxScroll);
        Arrange(Bounds);
        return true;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var radius = look.Radius("panel");
        if (!IsRoot)
        {
            // The window under it is dimmed: it takes nothing while the keys window is open.
            painter.Fill(Bounds, look.Color("text").WithAlpha(56));
            painter.Fill(Panel, look.Color("panel"), radius);
            painter.Stroke(Panel, look.Color("strong_border"), look.Border, radius);
        }
        var view = View;
        var x = view.X;
        var width = Math.Min(ContentWidth, view.Width);
        var size = look.Size("text");
        using (painter.Clip(IsRoot ? Panel : Panel.Inset(look.Border)))
        {
            var y = view.Y - scroll;
            painter.Text(Title, new Box(x, y, width, HeadingHeight), look.Color("text"), look.Size("heading"), bold: true);
            y += HeadingHeight + Gap;
            if (Message is { } message)
            {
                var room = swap is null ? MessageHeight : MessageHeight - RowHeight - 4;
                var lines = SaidDialog.Wrap(message, width, painter.Fonts, size);
                for (var i = 0; i < lines.Count && (i + 1) * Line <= room; i++)
                    painter.Text(lines[i], new Box(x, y + i * Line, width, Line), look.Color("text"), size);
            }
            y += MessageHeight + Gap;
            var index = 0;
            foreach (var layer in layers)
            {
                painter.Text(PlaceName(layer), new Box(x, y, width, TitleHeight), look.Color("text"), size, bold: true);
                y += TitleHeight + RowGap;
                var group = new Box(x, y, width, GroupHeight(layer));
                painter.Stroke(group, look.Color("border"), look.Border, radius);
                var rowY = y + GroupPad;
                foreach (var c in layer.Commands)
                {
                    var (_, change, _) = rows[index++];
                    var keysX = change.Bounds.X - ColumnGap - KeysWidth;
                    var nameX = x + GroupPad;
                    painter.Text(NameOf(c.Id), new Box(nameX, rowY, Math.Max(0, keysX - ColumnGap - nameX), RowHeight), look.Color("text"), size, bold: capturing == c.Id);
                    var keys = KeysOf(c);
                    painter.Text(KeysText(c), new Box(keysX, rowY, KeysWidth, RowHeight), look.Color(keys.Count > 0 ? "text" : "muted"), size);
                    rowY += RowHeight + RowGap;
                }
                y += group.Height + Gap;
            }
            base.Draw(painter);
        }
    }

    /// <summary>A button of the keys window: Enter or Space presses it; any other key goes to the window.</summary>
    public sealed class Push : Element
    {
        readonly KeysWindow window;
        readonly Func<string> caption;
        readonly Action click;
        readonly Func<bool>? enabled;
        readonly Func<bool>? shown;

        internal Push(KeysWindow window, string role, string? command, Func<string> caption, Action click, Func<bool>? enabled = null, Func<bool>? shown = null)
        {
            this.window = window;
            Role = role;
            Command = command;
            this.caption = caption;
            this.click = click;
            this.enabled = enabled;
            this.shown = shown;
        }

        /// <summary>What it does: change, reset, swap, cancel, resetAll or close.</summary>
        public string Role { get; }

        /// <summary>The command of its row; none for the window's own buttons.</summary>
        public string? Command { get; }

        public string Caption => caption();

        public bool Enabled => enabled?.Invoke() ?? true;

        public override bool IsShown => (shown?.Invoke() ?? true) && window.InView(Bounds);

        public override bool Focusable => Enabled;

        public override bool WantsTab => window.Capturing;

        public override void OnClick()
        {
            if (Enabled)
                click();
        }

        public override bool OnKey(KeyInput key)
        {
            if (!window.Capturing && key.Modifiers == Modifiers.None && key.Key is Key.Enter or Key.Space)
            {
                OnClick();
                return true;
            }
            return window.KeyPressed(key);
        }

        public override void Draw(Painter painter)
        {
            var look = painter.Look;
            using var dimmed = PressLooks.Dimmed(painter, Enabled);
            var values = look.Part("button", Enabled && Pressed && Hovered ? "pressed" : Enabled && Hovered ? "hover" : null);
            var radius = values.Radius ?? look.Radius("button");
            painter.Fill(Bounds, values.Background ?? look.Color("raised"), radius);
            painter.Stroke(Bounds, values.Border ?? look.Color("border"), look.Border, radius);
            if (Focused)
                PressLooks.FocusRing(painter, Bounds, radius);
            painter.Text(Caption, Bounds.Inset(look.Padding), values.Text ?? look.Color("text"), values.Size ?? look.Size("text"), values.Font ?? "text", values.Bold, TextAlign.Center);
        }
    }
}
