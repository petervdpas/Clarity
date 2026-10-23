// Clarity's table part on Avalonia. It owns how a table behaves, the same
// as on the web (conformance/tables.json, table-order.json): which control a
// cell holds, when a row counts as chosen and when it counts as edited, how
// text that is no value is marked, and the order rows show in. Generated
// windows only describe the columns. Rows show in the table's order (its
// first order, then a click on a sortable header) but stay the same records,
// so the chosen row and every event name the record the person sees. With
// Resizable the person drags the header edges; the widths are remembered
// under Remember, and a double-click on an edge gives the designed widths
// back.

using System.Collections;
using System.Collections.Specialized;
using System.Globalization;
using System.Text.Json;
using Avalonia;
using Avalonia.Collections;
using Avalonia.Controls;
using Avalonia.Controls.Templates;
using Avalonia.Data;
using Avalonia.Data.Converters;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.VisualTree;
using Clarity.Runtime;

namespace Clarity.Avalonia;

/// <summary>What a cell holds: the kind of value of its column.</summary>
public enum CellKind
{
    Text,
    Number,
    Bool,
    Choice,
    Date,
    Moment,
    /// <summary>A color, "#rrggbb": a swatch beside it, edited with the color input.</summary>
    Color,
}

/// <summary>A column of a table: the record's property it shows, and whether and how it is edited.</summary>
public sealed class TableColumn
{
    /// <summary>The record's property, like <c>Done</c>.</summary>
    public string Field { get; set; } = "";
    public string Header { get; set; } = "";
    public CellKind Kind { get; set; }
    /// <summary>Whether its cells are inputs (<c>edit</c>).</summary>
    public bool Edit { get; set; }
    /// <summary>A bool column shown as a switch (<c>as switch</c>).</summary>
    public bool Switch { get; set; }
    /// <summary>The choices of a choice column: an enum's values, or the records a reference can point at.</summary>
    public IEnumerable<Choice>? Choices { get; set; }
    /// <summary>Where the table's window keeps the choices instead, by name, like <c>PersonChoices</c>: the records it read.</summary>
    public string? ChoicesFrom { get; set; }
    /// <summary>A value as the column shows it, when it is not edited.</summary>
    public IValueConverter? Shown { get; set; }
    /// <summary>How a number cell shows its value and reads what is typed.</summary>
    public FieldFormat? Number { get; set; }
    /// <summary>The designed width in pixels (<c>columns title 240</c>); 0 shares what is left.</summary>
    public double Width { get; set; }
}

public sealed class ClarityTable : UserControl
{
    public static readonly StyledProperty<IEnumerable?> ItemsSourceProperty =
        AvaloniaProperty.Register<ClarityTable, IEnumerable?>(nameof(ItemsSource));

    public static readonly StyledProperty<object?> SelectedItemProperty =
        AvaloniaProperty.Register<ClarityTable, object?>(nameof(SelectedItem), defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The rows: records that tell when a field changes.</summary>
    public IEnumerable? ItemsSource
    {
        get => GetValue(ItemsSourceProperty);
        set => SetValue(ItemsSourceProperty, value);
    }

    /// <summary>The chosen row.</summary>
    public object? SelectedItem
    {
        get => GetValue(SelectedItemProperty);
        set => SetValue(SelectedItemProperty, value);
    }

    public AvaloniaList<TableColumn> Columns { get; } = [];

    /// <summary>The person drags the header edges (<c>resizable</c>).</summary>
    public bool Resizable
    {
        get => resizable;
        set
        {
            resizable = value;
            Grid.CanUserResizeColumns = value;
        }
    }

    /// <summary>A click on a header sorts by its column (<c>sortable</c>).</summary>
    public bool Sortable { get; set; }

    /// <summary>The column the rows show ordered by first (<c>order by</c>), by its field; none keeps the data's order.</summary>
    public string? OrderBy
    {
        get => order?.Field;
        set
        {
            order = value is null ? null : new Order(value, order?.Descending ?? false);
            ShowRows();
        }
    }

    /// <summary>Whether the first order is the other way (<c>desc</c>).</summary>
    public bool OrderDescending
    {
        get => order?.Descending ?? false;
        set
        {
            if (order is { } o)
                order = o with { Descending = value };
            ShowRows();
        }
    }

    /// <summary>Where the person's widths are kept (<c>Window.part</c>); none keeps them for the run.</summary>
    public string? Remember { get; set; }

    /// <summary>The order the rows show in now: a field and its way, or none for the data's order.</summary>
    public (string Field, bool Descending)? Ordered => order is { } o ? (o.Field, o.Descending) : null;

    sealed record Order(string Field, bool Descending);

    bool resizable;
    Order? order;
    INotifyCollectionChanged? watched;

    /// <summary>Another row was chosen: Clarity's <c>select</c>.</summary>
    public event EventHandler? Selected;

    /// <summary>A row was opened (double-click): Clarity's <c>open</c>.</summary>
    public event EventHandler? Opened;

    /// <summary>The person edited the row, which is chosen first: Clarity's <c>change</c>.</summary>
    public event Action<object>? Edited;

    public DataGrid Grid { get; } = new()
    {
        AutoGenerateColumns = false,
        IsReadOnly = true,
        GridLinesVisibility = DataGridGridLinesVisibility.Horizontal,
        CanUserReorderColumns = false,
    };

    object? chosen;

    public ClarityTable()
    {
        Content = Grid;
        Columns.CollectionChanged += (_, _) => Build();
        Grid.SelectionChanged += (_, _) =>
        {
            SelectedItem = Grid.SelectedItem;
            if (Grid.SelectedItem is { } row && !ReferenceEquals(row, chosen))
            {
                chosen = row;
                Selected?.Invoke(this, EventArgs.Empty);
            }
        };
        Grid.DoubleTapped += (_, e) =>
        {
            if (e.Source is Visual source && source.FindAncestorOfType<DataGridColumnHeader>(includeSelf: true) is null)
                Opened?.Invoke(this, EventArgs.Empty);
        };
        Grid.CanUserResizeColumns = false;
        Grid.CanUserSortColumns = true;
        // A click on a header sorts the shown rows by Clarity's rules, not the grid's.
        Grid.Sorting += (_, e) =>
        {
            e.Handled = true;
            var index = Grid.Columns.IndexOf(e.Column);
            if (Sortable && index >= 0 && index < Columns.Count)
                SortBy(Columns[index].Field);
        };
        // Widths are kept when an edge is let go; a double-click on an edge gives the designed widths back.
        Grid.AddHandler(PointerPressedEvent, (_, _) => pressedWidths = Widths(), RoutingStrategies.Tunnel, handledEventsToo: true);
        Grid.AddHandler(PointerReleasedEvent, (_, _) => KeepWidths(), RoutingStrategies.Bubble, handledEventsToo: true);
        Grid.AddHandler(DoubleTappedEvent, (_, e) =>
        {
            if (Resizable && e.Source is Visual source && source.FindAncestorOfType<DataGridColumnHeader>(includeSelf: true) is { } header
                && e.GetPosition(header).X >= header.Bounds.Width - EdgeWidth)
            {
                e.Handled = true;
                DesignedWidths();
            }
        }, RoutingStrategies.Tunnel, handledEventsToo: true);
        // Enter takes a text cell (the input does that itself), as leaving it
        // does, and does nothing else: the grid would move to the next row,
        // which the web does not.
        Grid.AddHandler(KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter && e.Source is TextBox)
                e.Handled = true;
        }, RoutingStrategies.Tunnel);
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ItemsSourceProperty)
        {
            chosen = null;
            if (watched is not null)
                watched.CollectionChanged -= RowsChanged;
            watched = ItemsSource as INotifyCollectionChanged;
            if (watched is not null)
                watched.CollectionChanged += RowsChanged;
            ShowRows();
        }
        else if (change.Property == SelectedItemProperty && !ReferenceEquals(Grid.SelectedItem, SelectedItem))
            Grid.SelectedItem = SelectedItem;
    }

    /// <summary>The cell of <paramref name="row"/> in the column of <paramref name="field"/>, while it is shown.</summary>
    public Control? CellOf(object row, string field) =>
        Grid.GetVisualDescendants().OfType<Border>().FirstOrDefault(b => b.Tag as string == field && ReferenceEquals(b.DataContext, row));

    /// <summary>The person changed a cell's value: its row is chosen, then edited.</summary>
    void CellEdited(object? row)
    {
        if (row is null)
            return;
        if (!ReferenceEquals(Grid.SelectedItem, row))
            Grid.SelectedItem = row;
        Edited?.Invoke(row);
    }

    /// <summary>Sorts by the column of <paramref name="field"/>, or the other way when it already is.</summary>
    public void SortBy(string field)
    {
        order = order?.Field == field ? order with { Descending = !order.Descending } : new Order(field, false);
        ShowRows();
    }

    void RowsChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (order is not null)
            ShowRows();
    }

    /// <summary>The rows in the order they show: the data's, or sorted by Clarity's rules into a list of the same records.</summary>
    void ShowRows()
    {
        var selected = SelectedItem;
        var column = order is { } o ? Columns.FirstOrDefault(c => c.Field == o.Field) : null;
        Grid.ItemsSource = column is null || ItemsSource is null ? ItemsSource : Sorted(ItemsSource.Cast<object>().ToList(), column, order!.Descending);
        for (var i = 0; i < Columns.Count && i < Grid.Columns.Count; i++)
            Grid.Columns[i].Header = Columns[i].Field == column?.Field ? $"{Columns[i].Header} {(order!.Descending ? "▼" : "▲")}" : Columns[i].Header;
        if (selected is not null && !ReferenceEquals(Grid.SelectedItem, selected))
            Grid.SelectedItem = selected;
    }

    /// <summary>
    /// Text ignoring case, numbers as numbers, a choice by the label it shows,
    /// an empty value last either way; equal rows keep the data's order.
    /// </summary>
    List<object> Sorted(List<object> rows, TableColumn column, bool descending)
    {
        var choices = column.Kind == CellKind.Choice ? ChoicesOf(column)?.ToList() : null;
        object? Key(object row)
        {
            var value = row.GetType().GetProperty(column.Field)?.GetValue(row);
            if (value is null || value is string { Length: 0 })
                return null;
            if (column.Kind == CellKind.Choice)
            {
                var label = choices?.FirstOrDefault(c => Equals(c.Value, value) || Equals(c.Value?.ToString(), value.ToString()))?.Label ?? value.ToString();
                return string.IsNullOrEmpty(label) ? null : label;
            }
            if (column.Kind == CellKind.Number)
                return Convert.ToDecimal(value, CultureInfo.InvariantCulture);
            if (column.Kind == CellKind.Bool)
                return value is true ? 1m : 0m;
            return value;
        }
        var keyed = rows.Select((row, place) => (row, place, key: Key(row))).ToList();
        keyed.Sort((a, b) =>
        {
            if (a.key is null || b.key is null)
                return a.key is null && b.key is null ? a.place - b.place : a.key is null ? 1 : -1;
            var by = a.key is string sa && b.key is string sb
                ? string.Compare(sa, sb, CultureInfo.CurrentCulture, CompareOptions.IgnoreCase)
                : a.key is IComparable ca ? ca.CompareTo(b.key) : 0;
            return (descending ? -by : by) is var d && d != 0 ? d : a.place - b.place;
        });
        return keyed.Select(k => k.row).ToList();
    }

    /// <summary>How near the right edge of a header a double-click counts as on the edge.</summary>
    const double EdgeWidth = 6;

    /// <summary>The widths when the pointer went down in the grid: a release that changed them was a resize.</summary>
    Dictionary<string, double>? pressedWidths;

    static string RememberKey(string remember) => $"columns.{remember}";

    /// <summary>Each column's width as shown, by field.</summary>
    Dictionary<string, double> Widths() =>
        Columns.Select((c, i) => (c.Field, width: i < Grid.Columns.Count ? Math.Round(Grid.Columns[i].ActualWidth) : 0)).ToDictionary(w => w.Field, w => w.width);

    /// <summary>The person let go of a header edge: the widths they left are remembered.</summary>
    void KeepWidths()
    {
        var before = pressedWidths;
        pressedWidths = null;
        if (!Resizable || before is null)
            return;
        var widths = Widths();
        if (widths.All(w => before.GetValueOrDefault(w.Key) == w.Value))
            return;
        if (Remember is { } key)
            Remembered.Set(RememberKey(key), JsonSerializer.Serialize(widths));
    }

    /// <summary>Back to the designed widths, forgetting the person's.</summary>
    public void DesignedWidths()
    {
        if (Remember is { } key)
            Remembered.Set(RememberKey(key), "");
        for (var i = 0; i < Columns.Count && i < Grid.Columns.Count; i++)
            Grid.Columns[i].Width = DesignedLength(Columns[i]);
    }

    /// <summary>A column's designed width: its pixels; else the column shares what is left, a checkbox or switch its own width.</summary>
    static DataGridLength DesignedLength(TableColumn column) =>
        column.Width > 0 ? new DataGridLength(column.Width, DataGridLengthUnitType.Pixel)
        : column.Kind == CellKind.Bool ? DataGridLength.Auto : new DataGridLength(1, DataGridLengthUnitType.Star);

    /// <summary>The widths the person left before, when Remember keeps any.</summary>
    Dictionary<string, double>? RememberedWidths()
    {
        if (Remember is not { } key || Remembered.Get(RememberKey(key)) is not { Length: > 0 } text)
            return null;
        try
        {
            return JsonSerializer.Deserialize<Dictionary<string, double>>(text);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    void Build()
    {
        Grid.Columns.Clear();
        var kept = RememberedWidths();
        foreach (var column in Columns)
        {
            var at = column;
            Grid.Columns.Add(new DataGridTemplateColumn
            {
                Header = column.Header,
                // A header click reaches Sorting only for a column that names what it sorts by; Clarity sorts it itself.
                SortMemberPath = column.Field,
                Width = kept?.GetValueOrDefault(column.Field) is > 0 and var w ? new DataGridLength(w, DataGridLengthUnitType.Pixel) : DesignedLength(column),
                CellTemplate = new FuncDataTemplate<object>((_, _) =>
                {
                    var cell = Cell(at, ChoicesOf(at));
                    if (cell is IInput input)
                        input.ValueChanged += (_, _) => CellEdited(cell.DataContext);
                    return new Border { Tag = at.Field, Child = cell };
                }),
            });
        }
        ShowRows();
    }

    /// <summary>A kept color as the brush of its swatch.</summary>
    static readonly IValueConverter ColorBrush = new global::Avalonia.Data.Converters.FuncValueConverter<string?, global::Avalonia.Media.IBrush?>(Palette.Brush);

    static Binding Bound(TableColumn column, IValueConverter? converter = null) => new(column.Field) { Mode = BindingMode.TwoWay, Converter = converter };

    /// <summary>A choice column's choices: its own, or those its window keeps.</summary>
    IEnumerable<Choice>? ChoicesOf(TableColumn column) =>
        column.Choices ?? (column.ChoicesFrom is { } from && DataContext is { } window ? window.GetType().GetProperty(from)?.GetValue(window) as IEnumerable<Choice> : null);

    static Control Cell(TableColumn column, IEnumerable<Choice>? choices)
    {
        if (!column.Edit && column.Kind == CellKind.Color)
            return new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 6,
                VerticalAlignment = VerticalAlignment.Center,
                Children =
                {
                    new Border
                    {
                        Classes = { "swatch" },
                        Width = 12,
                        Height = 12,
                        CornerRadius = new CornerRadius(2),
                        [!Border.BackgroundProperty] = new Binding(column.Field) { Converter = ColorBrush, Mode = BindingMode.OneWay },
                    },
                    new TextBlock { Classes = { "cell" }, [!TextBlock.TextProperty] = new Binding(column.Field) { Mode = BindingMode.OneWay } },
                },
            };
        if (!column.Edit)
            return new TextBlock
            {
                Classes = { "cell" },
                VerticalAlignment = VerticalAlignment.Center,
                [!TextBlock.TextProperty] = new Binding(column.Field) { Converter = column.Shown, Mode = BindingMode.OneWay },
            };
        // The library's inputs: a cell is edited like the same input in a form.
        return column.Kind switch
        {
            CellKind.Bool when column.Switch => new SwitchInput
            {
                Classes = { "cell" },
                OnContent = null,
                OffContent = null,
                [!ToggleSwitch.IsCheckedProperty] = Bound(column),
            },
            CellKind.Bool => new CheckInput { Classes = { "cell" }, [!CheckBox.IsCheckedProperty] = Bound(column) },
            CellKind.Choice => new ChoiceInput { Classes = { "cell" }, Choices = choices, [!ChoiceInput.ValueProperty] = Bound(column) },
            CellKind.Date => new DateInput { Classes = { "cell" }, [!DateInput.ValueProperty] = Bound(column) },
            CellKind.Color => new ColorInput { Classes = { "cell" }, [!ColorInput.ValueProperty] = Bound(column) },
            CellKind.Moment => new DateTimeInput { Classes = { "cell" }, [!DateTimeInput.ValueProperty] = Bound(column) },
            CellKind.Number => new NumberInput { Classes = { "cell" }, Format = column.Number ?? new FieldFormat(FormatKind.Decimal), [!NumberInput.ValueProperty] = Bound(column) },
            _ => new Clarity.Avalonia.TextInput { Classes = { "cell" }, [!Clarity.Avalonia.TextInput.ValueProperty] = Bound(column) },
        };
    }
}
