// What Clarity's keys window shows on Avalonia (DESIGN 5.28), where the project file
// says `keys editable`: every command per place, Everywhere (the app's
// frame) first, then each window's own, with its keys. Change, then press
// the new key; a key another command has in the same place offers a swap;
// Reset puts the design's key back. In Clarity's words, as on the web.

using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Clarity.Runtime;

namespace Clarity.Avalonia;

/// <summary>A command a key can run: its id among the person's keys, its caption, and the keys its design gives it.</summary>
public sealed record KeyCommandInfo(string Id, Text Caption, IReadOnlyList<string> Keys);

/// <summary>The commands of one place: the app's frame (<c>Id</c> empty, no title) or a window.</summary>
public sealed record KeyLayerInfo(string Id, Text? Title, IReadOnlyList<KeyCommandInfo> Commands);

/// <summary>What the keys window shows: its places, its commands and their keys.</summary>
public sealed class KeysView : UserControl
{
    readonly IReadOnlyList<KeyLayerInfo> layers;
    readonly KeyRules.Command[] commands;
    readonly Dictionary<string, string> names;
    string? capturing;
    string? note;
    (string Id, string Key, string Other)? swap;
    TopLevel? listening;

    const double MessageWidth = 460;
    const double MessageHeight = 72;

    /// <summary>The person closed the window.</summary>
    public event Action? Closed;

    public KeysView(IReadOnlyList<KeyLayerInfo> layers)
    {
        this.layers = layers;
        commands = [.. layers.SelectMany(l => l.Commands.Select(c => new KeyRules.Command(c.Id, [l.Id], c.Keys)))];
        names = layers.SelectMany(l => l.Commands).ToDictionary(c => c.Id, c => Locale.Show(c.Caption));
        // While it shows: the window's keys, and every change of the person's keys.
        AttachedToVisualTree += (_, _) =>
        {
            listening = TopLevel.GetTopLevel(this);
            listening?.AddHandler(KeyDownEvent, Pressed, RoutingStrategies.Tunnel, handledEventsToo: true);
            PersonKeys.Changed += Render;
        };
        DetachedFromVisualTree += (_, _) =>
        {
            listening?.RemoveHandler(KeyDownEvent, Pressed);
            PersonKeys.Changed -= Render;
        };
        Render();
    }

    /// <summary>One of Clarity's words: the project's value, or its English.</summary>
    public static string Word(string key, string english) => Locale.Translate(key, english);

    static string Said(string key, string english, params (string Slot, string Value)[] slots)
    {
        var text = new Text(english) { Key = key };
        foreach (var (slot, value) in slots)
            text.With(slot, Slot.OfText(value));
        return Locale.Show(text);
    }

    static readonly Dictionary<string, (string Key, string English)> Refusals = new()
    {
        ["typing"] = ("keysTyping", "{key} cannot be used: a key without Ctrl or Alt is typing"),
        ["tab"] = ("keysTab", "{key} cannot be used: Tab moves between fields"),
        ["enter"] = ("keysEnter", "{key} cannot be used: Enter confirms"),
        ["escape"] = ("keysEscape", "{key} cannot be used: Escape cancels"),
    };

    string NameOf(string id) => names.TryGetValue(id, out var name) ? name : id;

    void Pressed(object? sender, KeyEventArgs e)
    {
        if (capturing is null) return;
        e.Handled = true;
        if (e.Key == Key.Escape && e.KeyModifiers == KeyModifiers.None)
        {
            capturing = null;
            Render();
            return;
        }
        if (KeyRules.Written(e.Key, e.KeyModifiers) is not { } key) return;
        if (KeyRules.Refused(key) is { } why)
        {
            var (word, english) = Refusals[why];
            note = Said(word, english, ("key", key));
            Render();
            return;
        }
        var id = capturing;
        capturing = null;
        var (_, other) = KeyRules.Assign(commands, PersonKeys.Choices, id, key);
        if (other is not null)
        {
            swap = (id, key, other);
            Render();
        }
        else
            Take(id, key);
    }

    void Take(string id, string key)
    {
        var also = KeyRules.AlsoHas(commands, PersonKeys.Choices, id, key);
        note = also.Count > 0
            ? Said("keysAlso", "{key} is also the key of {command}, elsewhere; here {mine} goes first", ("key", key), ("command", string.Join(", ", also.Select(NameOf))), ("mine", NameOf(id)))
            : null;
        swap = null;
        PersonKeys.Set(KeyRules.Assign(commands, PersonKeys.Choices, id, key).Choices);
    }

    static Button Button(string caption, string name, Action click, bool enabled = true)
    {
        var button = new Button { Content = caption, IsEnabled = enabled };
        AutomationProperties.SetName(button, name);
        button.Click += (_, _) => click();
        return button;
    }

    void Render()
    {
        // A width of its own, so a long message wraps instead of widening the window.
        var panel = new StackPanel { Spacing = 8, Margin = new Thickness(16), Width = MessageWidth };
        panel.Children.Add(new TextBlock { Text = Word("keysTitle", "Keys"), Classes = { "heading" } });
        // The message line is always there, so nothing moves when it says something.
        var message = new Panel { MinHeight = MessageHeight };
        if (swap is { } s)
        {
            var row = new WrapPanel { Orientation = Orientation.Horizontal };
            row.Children.Add(new TextBlock { Text = Said("keysTaken", "{key} is the key of {command}. Swap them?", ("key", s.Key), ("command", NameOf(s.Other))), VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(0, 0, 8, 0), TextWrapping = global::Avalonia.Media.TextWrapping.Wrap });
            var swapWord = Word("keysSwap", "Swap");
            row.Children.Add(Button(swapWord, "Swap", () => Take(s.Id, s.Key)));
            row.Children.Add(Button(Word("cancel", "Cancel"), "Cancel", () => { swap = null; Render(); }));
            message.Children.Add(row);
        }
        else if ((note ?? (capturing is null ? null : Word("keysPress", "Press the new key; Escape stops"))) is { } said)
            message.Children.Add(new TextBlock { Text = said, TextWrapping = global::Avalonia.Media.TextWrapping.Wrap });
        panel.Children.Add(message);
        foreach (var layer in layers)
        {
            var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto,Auto,Auto"), RowSpacing = 4, ColumnSpacing = 6 };
            var row = 0;
            foreach (var c in layer.Commands)
            {
                grid.RowDefinitions.Add(new RowDefinition(GridLength.Auto));
                var name = NameOf(c.Id);
                var keys = PersonKeys.Choices.TryGetValue(c.Id, out var mine) ? mine : c.Keys;
                var shown = keys.Count > 0 ? string.Join(", ", keys) : Word("keysNone", "None");
                // The command that waits for its new key stands out; the message line says what to do.
                var weight = capturing == c.Id ? global::Avalonia.Media.FontWeight.Bold : global::Avalonia.Media.FontWeight.Normal;
                var cells = new Control[]
                {
                    new TextBlock { Text = name, FontWeight = weight, VerticalAlignment = VerticalAlignment.Center },
                    new TextBlock { Text = shown, VerticalAlignment = VerticalAlignment.Center },
                    Button(Word("keysChange", "Change"), $"Change {name}", () => { capturing = c.Id; swap = null; note = null; Render(); }),
                    Button(Word("keysReset", "Reset"), $"Reset {name}", () => { note = null; PersonKeys.Set(KeyRules.Reset(commands, PersonKeys.Choices, c.Id)); }, PersonKeys.Choices.ContainsKey(c.Id)),
                };
                for (var column = 0; column < cells.Length; column++)
                {
                    Grid.SetRow(cells[column], row);
                    Grid.SetColumn(cells[column], column);
                    grid.Children.Add(cells[column]);
                }
                row++;
            }
            var title = layer.Title is { } t ? Locale.Show(t) : Word("keysEverywhere", "Everywhere");
            panel.Children.Add(new TextBlock { Text = title, FontWeight = global::Avalonia.Media.FontWeight.SemiBold, HorizontalAlignment = HorizontalAlignment.Left });
            panel.Children.Add(new Border { Child = grid, Classes = { "keys-group" }, Padding = new Thickness(8, 4) });
        }
        var actions = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6, HorizontalAlignment = HorizontalAlignment.Right };
        actions.Children.Add(Button(Word("keysResetAll", "Reset all"), "Reset all", () => { note = null; swap = null; PersonKeys.Set(new Dictionary<string, IReadOnlyList<string>>()); }, PersonKeys.Choices.Count > 0));
        actions.Children.Add(Button(Word("keysClose", "Close"), "Close", () => Closed?.Invoke()));
        panel.Children.Add(actions);
        Content = new ScrollViewer { Content = panel };
    }
}
