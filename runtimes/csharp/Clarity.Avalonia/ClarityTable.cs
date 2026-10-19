// Clarity's table part on Avalonia. It owns how a table behaves, the same
// as on the web (conformance/tables.json): which control a cell holds, when
// a row counts as chosen and when it counts as edited, and how text that is
// no value is marked. Generated windows only describe the columns.

using System.Collections;
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
        Grid.DoubleTapped += (_, _) => Opened?.Invoke(this, EventArgs.Empty);
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
            Grid.ItemsSource = ItemsSource;
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

    void Build()
    {
        Grid.Columns.Clear();
        foreach (var column in Columns)
        {
            var at = column;
            Grid.Columns.Add(new DataGridTemplateColumn
            {
                Header = column.Header,
                // The columns fill the table; a checkbox or switch needs only its own width.
                Width = column.Kind == CellKind.Bool ? DataGridLength.Auto : new DataGridLength(1, DataGridLengthUnitType.Star),
                CellTemplate = new FuncDataTemplate<object>((_, _) =>
                {
                    var cell = Cell(at, ChoicesOf(at));
                    if (cell is IInput input)
                        input.ValueChanged += (_, _) => CellEdited(cell.DataContext);
                    return new Border { Tag = at.Field, Child = cell };
                }),
            });
        }
    }

    static Binding Bound(TableColumn column, IValueConverter? converter = null) => new(column.Field) { Mode = BindingMode.TwoWay, Converter = converter };

    /// <summary>A choice column's choices: its own, or those its window keeps.</summary>
    IEnumerable<Choice>? ChoicesOf(TableColumn column) =>
        column.Choices ?? (column.ChoicesFrom is { } from && DataContext is { } window ? window.GetType().GetProperty(from)?.GetValue(window) as IEnumerable<Choice> : null);

    static Control Cell(TableColumn column, IEnumerable<Choice>? choices)
    {
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
            CellKind.Moment => new DateTimeInput { Classes = { "cell" }, [!DateTimeInput.ValueProperty] = Bound(column) },
            CellKind.Number => new NumberInput { Classes = { "cell" }, Format = column.Number ?? new FieldFormat(FormatKind.Decimal), [!NumberInput.ValueProperty] = Bound(column) },
            _ => new Clarity.Avalonia.TextInput { Classes = { "cell" }, [!Clarity.Avalonia.TextInput.ValueProperty] = Bound(column) },
        };
    }
}
