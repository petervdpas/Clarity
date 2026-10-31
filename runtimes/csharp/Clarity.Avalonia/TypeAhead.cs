// A box to type in with a list of what fits below it, the same as on the web
// (conformance/dropdown-options.json): the dropdown's `typed` (any text, the
// values its field has offered) and `search` (a record picked among many,
// filtered as it is typed). The arrow keys move through the list, Enter or
// a click picks, Escape closes it and puts the text back. Typed text that
// is not picked counts only for `typed`; for `search`, empty text is no
// record.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Controls.Templates;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;

namespace Clarity.Avalonia;

/// <summary>An offered row: what picking it gives, and its texts.</summary>
public sealed record Offered(object? Key, IReadOnlyList<string> Cells);

public abstract class TypeAhead : UserControl, IInput
{
    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<TypeAhead, bool>(nameof(IsReadOnly));

    /// <summary>Shown, but not changed by the person.</summary>
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    public event EventHandler? ValueChanged;

    /// <summary>The box typed in.</summary>
    public TextBox Box { get; } = new() { HorizontalAlignment = HorizontalAlignment.Stretch };

    /// <summary>What fits the typed text.</summary>
    public ListBox Offered { get; } = new() { MaxHeight = 240, Classes = { "offered" } };

    // A click outside closes the list and reaches what it was on, as on the
    // web: the box is left, so what was typed is taken.
    readonly Popup popup = new() { Placement = PlacementMode.BottomEdgeAlignedLeft, IsLightDismissEnabled = true, OverlayDismissEventPassThrough = true };
    bool showing;

    protected TypeAhead()
    {
        Classes.Add("type-ahead");
        Offered.ItemTemplate = new FuncDataTemplate<Offered>((row, _) =>
        {
            var cells = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 12 };
            foreach (var (cell, i) in (row?.Cells ?? []).Select((c, i) => (c, i)))
                cells.Children.Add(new TextBlock { Text = cell, Opacity = i == 0 ? 1 : 0.7 });
            return cells;
        });
        Offered.AddHandler(PointerReleasedEvent, (_, _) =>
        {
            if (Offered.SelectedItem is Offered row)
                Pick(row);
        }, RoutingStrategies.Bubble, handledEventsToo: true);
        popup.Child = new Border { Classes = { "offered" }, Child = Offered };
        popup.PlacementTarget = Box;
        // The text as it changes, at once: TextChanged comes later.
        Box.PropertyChanged += (_, e) =>
        {
            if (e.Property != TextBox.TextProperty || showing)
                return;
            Fill(Box.Text ?? "");
            if (Box.IsFocused)
                popup.IsOpen = Offered.ItemCount > 0;
        };
        Box.GotFocus += (_, _) =>
        {
            Fill(Box.Text ?? "");
            popup.IsOpen = !IsReadOnly && Offered.ItemCount > 0;
        };
        Box.LostFocus += (_, _) =>
        {
            if (!Offered.IsPointerOver)
                Take();
        };
        Box.AddHandler(KeyDownEvent, Key, RoutingStrategies.Tunnel, handledEventsToo: true);
        Content = new Panel { Children = { Box, popup } };
    }

    /// <summary>The rows that fit the typed text.</summary>
    protected abstract IEnumerable<Offered> Offer(string typed);

    /// <summary>The text the value shows in the box.</summary>
    protected abstract string Shown();

    /// <summary>A row was picked: true when that changed the value.</summary>
    protected abstract bool Picked(Offered row);

    /// <summary>The box was left or Enter pressed with typed text no row was picked for: true when that changed the value.</summary>
    protected abstract bool Typed(string typed);

    void Fill(string typed) => Offered.ItemsSource = Offer(typed).ToList();

    /// <summary>Picks a row.</summary>
    public void Pick(Offered row)
    {
        popup.IsOpen = false;
        if (!IsReadOnly && Picked(row))
            ValueChanged?.Invoke(this, EventArgs.Empty);
        Show();
    }

    /// <summary>Takes what is typed, as when the box is left.</summary>
    public void Take()
    {
        popup.IsOpen = false;
        var typed = Box.Text ?? "";
        if (!IsReadOnly && typed != Shown() && Typed(typed))
            ValueChanged?.Invoke(this, EventArgs.Empty);
        Show();
    }

    void Key(object? sender, KeyEventArgs e)
    {
        var count = Offered.ItemCount;
        switch (e.Key)
        {
            case global::Avalonia.Input.Key.Down or global::Avalonia.Input.Key.Up when count > 0:
                popup.IsOpen = true;
                var at = Offered.SelectedIndex;
                Offered.SelectedIndex = e.Key == global::Avalonia.Input.Key.Down ? (at + 1) % count : (at - 1 + count) % count;
                e.Handled = true;
                break;
            case global::Avalonia.Input.Key.Enter:
                if (Offered.SelectedItem is Offered row && popup.IsOpen)
                    Pick(row);
                else
                    Take();
                e.Handled = true;
                break;
            case global::Avalonia.Input.Key.Escape when popup.IsOpen:
                popup.IsOpen = false;
                Show();
                e.Handled = true;
                break;
        }
    }

    /// <summary>Shows the value in the box.</summary>
    protected void Show()
    {
        showing = true;
        Box.Text = Shown();
        showing = false;
        Fill(Box.Text ?? "");
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == IsReadOnlyProperty)
            Box.IsReadOnly = IsReadOnly;
    }
}

/// <summary>A dropdown of text any text may be typed into (<c>typed</c>), the values its field has offered.</summary>
public class TypedInput : TypeAhead
{
    public static readonly StyledProperty<string?> ValueProperty =
        AvaloniaProperty.Register<TypedInput, string?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<IEnumerable<string>?> ValuesProperty = AvaloniaProperty.Register<TypedInput, IEnumerable<string>?>(nameof(Values));

    public string? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>The values the field already has, with repeats and empty ones.</summary>
    public IEnumerable<string>? Values { get => GetValue(ValuesProperty); set => SetValue(ValuesProperty, value); }

    public TypedInput() => Show();

    protected override IEnumerable<Offered> Offer(string typed) =>
        DropdownOptions.Suggestions(Values ?? [], typed).Select(v => new Offered(v, [v]));

    protected override string Shown() => Value ?? "";

    protected override bool Picked(Offered row) => Typed((string)row.Key!);

    protected override bool Typed(string typed)
    {
        if (typed == (Value ?? ""))
            return false;
        Value = typed;
        return true;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty || change.Property == ValuesProperty)
            Show();
    }
}

/// <summary>A dropdown of a reference whose records filter as it is typed (<c>search</c>), each with the fields it shows.</summary>
public class LookupInput : TypeAhead
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<LookupInput, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<IEnumerable<Choice>?> ChoicesProperty = AvaloniaProperty.Register<LookupInput, IEnumerable<Choice>?>(nameof(Choices));

    /// <summary>The id of the record picked, or none.</summary>
    public object? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>The records it can point at, each with the texts of the fields it shows.</summary>
    public IEnumerable<Choice>? Choices { get => GetValue(ChoicesProperty); set => SetValue(ChoicesProperty, value); }

    protected override IEnumerable<Offered> Offer(string typed) =>
        DropdownOptions.Searched(Choices ?? [], typed).Select(c => new Offered(c.Value, [c.Label, .. c.Cells ?? []]));

    protected override string Shown() => Choices?.FirstOrDefault(c => Equals(c.Value, Value))?.Label ?? "";

    protected override bool Picked(Offered row)
    {
        if (Equals(row.Key, Value))
            return false;
        Value = row.Key;
        return true;
    }

    // Typed text picks nothing; emptied, it is no record.
    protected override bool Typed(string typed)
    {
        if (typed.Trim() != "" || Value is null)
            return false;
        Value = null;
        return true;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty || change.Property == ChoicesProperty)
            Show();
    }
}
