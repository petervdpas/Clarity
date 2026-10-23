// Clarity's tags input on Avalonia, the same as on the web
// (conformance/list-fields.json): a list field's items as chips, each with
// a button that removes it, and a place to add one: a box to type text or a
// number in (taken on Enter or when the box is left), a date to pick, or a
// dropdown of the values of an enum the list has not. A typed item is
// trimmed; empty text and an item the list has add nothing, and text that
// is no value of the list's kind stays as typed and is marked. Every change
// gives the record a new list, so its property tells it changed.

using System.Collections;
using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;

namespace Clarity.Avalonia;

/// <summary>What the items of a tags input are.</summary>
public enum TagItem
{
    Text,
    Integer,
    Decimal,
    Date,
    /// <summary>The values of an enum, picked from <see cref="TagsInput.Choices"/>.</summary>
    Choice,
}

/// <summary>An item shown as a chip, with the button that removes it.</summary>
public sealed class Chip(object? item, TextBlock label, Button remove)
{
    public object? Item { get; } = item;
    public TextBlock Label { get; } = label;
    public Button Remove { get; } = remove;
}

public class TagsInput : UserControl, IInput
{
    public static readonly StyledProperty<IList?> ValueProperty =
        AvaloniaProperty.Register<TagsInput, IList?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<IEnumerable?> ChoicesProperty = AvaloniaProperty.Register<TagsInput, IEnumerable?>(nameof(Choices));

    public static readonly StyledProperty<TagItem> ItemProperty = AvaloniaProperty.Register<TagsInput, TagItem>(nameof(Item));

    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<TagsInput, bool>(nameof(IsReadOnly));

    public static readonly StyledProperty<string?> AddTextProperty = AvaloniaProperty.Register<TagsInput, string?>(nameof(AddText));

    public static readonly StyledProperty<string?> RemoveTextProperty = AvaloniaProperty.Register<TagsInput, string?>(nameof(RemoveText));

    /// <summary>The record's list: a <c>List&lt;T&gt;</c> of the field's item type.</summary>
    public IList? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>An enum's values with their labels (<see cref="Choice"/>), for <see cref="TagItem.Choice"/>.</summary>
    public IEnumerable? Choices { get => GetValue(ChoicesProperty); set => SetValue(ChoicesProperty, value); }

    public TagItem Item { get => GetValue(ItemProperty); set => SetValue(ItemProperty, value); }

    /// <summary>Shown, but not changed by the person.</summary>
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    /// <summary>Clarity's own word in the place a new item goes (<c>addTag</c>).</summary>
    public string? AddText { get => GetValue(AddTextProperty); set => SetValue(AddTextProperty, value); }

    /// <summary>Clarity's own word on the button that removes an item (<c>removeTag</c>).</summary>
    public string? RemoveText { get => GetValue(RemoveTextProperty); set => SetValue(RemoveTextProperty, value); }

    public event EventHandler? ValueChanged;

    /// <summary>The box a text or a number is typed in.</summary>
    public TextBox Box { get; } = new() { MinWidth = 80, Classes = { "tag" } };

    /// <summary>The values of an enum the list has not, to add.</summary>
    public ComboBox Offered { get; } = new() { MinWidth = 100, DisplayMemberBinding = new Binding(nameof(Choice.Label)) };

    /// <summary>The date to add.</summary>
    public DateInput Day { get; } = new();

    /// <summary>The chips, in the order of the items.</summary>
    public IReadOnlyList<Chip> Chips => chips;

    readonly List<Chip> chips = [];
    readonly WrapPanel panel = new() { Orientation = Orientation.Horizontal, ItemSpacing = 4, LineSpacing = 4 };
    bool showing;

    public TagsInput()
    {
        Classes.Add("tags");
        Box.LostFocus += (_, _) => Take();
        Box.AddHandler(KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                Take();
        }, RoutingStrategies.Tunnel, handledEventsToo: true);
        Offered.SelectionChanged += (_, _) =>
        {
            if (!showing && Offered.SelectedItem is Choice choice)
                Add(choice.Value);
        };
        Day.ValueChanged += (_, _) =>
        {
            if (!showing && Day.Value is { } day)
                Add((DateOnly?)day);
        };
        Content = panel;
        Show();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty || change.Property == ChoicesProperty || change.Property == ItemProperty
            || change.Property == IsReadOnlyProperty || change.Property == AddTextProperty || change.Property == RemoveTextProperty)
            Show();
    }

    void Show()
    {
        showing = true;
        panel.Children.Clear();
        chips.Clear();
        var items = Value?.Cast<object?>().ToList() ?? [];
        foreach (var item in items)
        {
            var label = new TextBlock { Text = Label(item), VerticalAlignment = VerticalAlignment.Center };
            var remove = new Button
            {
                Classes = { "remove" },
                Padding = new Thickness(3),
                IsVisible = !IsReadOnly,
                Content = new global::Avalonia.Controls.Shapes.Path { Data = Cross, Stroke = Brushes.Gray, StrokeThickness = 1.5, Width = 8, Height = 8 },
                [ToolTip.TipProperty] = RemoveText,
            };
            var at = chips.Count;
            remove.Click += (_, _) => RemoveAt(at);
            var chip = new Border
            {
                Classes = { "chip" },
                CornerRadius = new CornerRadius(10),
                Padding = new Thickness(8, 2, 2, 2),
                Child = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 2, Children = { label, remove } },
            };
            chips.Add(new Chip(item, label, remove));
            panel.Children.Add(chip);
        }
        if (!IsReadOnly)
        {
            switch (Item)
            {
                case TagItem.Choice:
                    Offered.ItemsSource = Choices?.Cast<Choice>().Where(c => !items.Any(i => Equals(i, c.Value))).ToList() ?? [];
                    Offered.SelectedItem = null;
                    Offered.PlaceholderText = AddText;
                    panel.Children.Add(Offered);
                    break;
                case TagItem.Date:
                    Day.Value = null;
                    Day.PlaceholderText = AddText;
                    panel.Children.Add(Day);
                    break;
                default:
                    Box.PlaceholderText = AddText;
                    panel.Children.Add(Box);
                    break;
            }
        }
        showing = false;
    }

    static readonly Geometry Cross = Geometry.Parse("M0,0 L8,8 M8,0 L0,8");

    /// <summary>An item as people read it: an enum's value by its label.</summary>
    string Label(object? item) =>
        Choices?.Cast<Choice>().FirstOrDefault(c => Equals(c.Value, item))?.Label
        ?? new ShownConverter(_ => null).Convert(item, typeof(string), null, CultureInfo.CurrentCulture) as string
        ?? "";

    /// <summary>Takes what is typed: an item the list has not, of its kind.</summary>
    public void Take()
    {
        var typed = (Box.Text ?? "").Trim();
        if (typed == "")
        {
            Box.Text = "";
            Box.Classes.Remove("error");
            return;
        }
        object? item = Item switch
        {
            TagItem.Integer => long.TryParse(typed, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? n : null,
            TagItem.Decimal => decimal.TryParse(typed, NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var d) ? d : null,
            _ => typed,
        };
        if (item is null)
        {
            Box.Classes.Add("error");
            return;
        }
        Box.Classes.Remove("error");
        Box.Text = "";
        Add(item);
    }

    void Add(object? item)
    {
        if (IsReadOnly || Value is { } list && list.Cast<object?>().Any(i => Equals(i, item)))
        {
            Show();
            return;
        }
        var next = Copy();
        next.Add(item);
        Offer(next);
    }

    void RemoveAt(int at)
    {
        if (IsReadOnly)
            return;
        var next = Copy();
        next.RemoveAt(at);
        Offer(next);
    }

    /// <summary>A new list of the record's own type, with its items.</summary>
    IList Copy()
    {
        var next = Value is { } list ? (IList)Activator.CreateInstance(list.GetType())! : new List<object?>();
        foreach (var item in Value ?? Array.Empty<object?>())
            next.Add(item);
        return next;
    }

    void Offer(IList next)
    {
        Value = next;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}
