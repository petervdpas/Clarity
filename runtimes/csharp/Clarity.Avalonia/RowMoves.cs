// Moving rows by hand (DESIGN 5.30), the same as on the web: the rows of a
// list, table, tree or tree table that reorders are dragged with the pointer,
// and the chosen row moves with Alt and an arrow key. A line shows where a
// dragged row lands, or the row it goes under in a tree. The list takes its
// new order and each record its number (and parent), then the part tells.
// A list (ListBox) gets it with RowMoves.By; ClarityTable has ReorderBy.

using System.Collections;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.VisualTree;

namespace Clarity.Avalonia;

public static class RowMoves
{
    /// <summary>The record property a list's rows are numbered in: <c>reorder by position</c>.</summary>
    public static readonly AttachedProperty<string?> ByProperty =
        AvaloniaProperty.RegisterAttached<ListBox, string?>("By", typeof(RowMoves));

    public static string? GetBy(ListBox list) => list.GetValue(ByProperty);
    public static void SetBy(ListBox list, string? value) => list.SetValue(ByProperty, value);

    static readonly System.Runtime.CompilerServices.ConditionalWeakTable<ListBox, Action> moved = [];

    static RowMoves()
    {
        ByProperty.Changed.AddClassHandler<ListBox>((list, e) =>
        {
            if (e.OldValue is null && e.NewValue is string)
                Attach(list, () => list.ItemsSource as IList, () => GetBy(list), () => null, v => (v as Visual)?.FindAncestorOfType<ListBoxItem>(includeSelf: true),
                    () => list.SelectedItem, row => list.SelectedItem = row, () => { if (moved.TryGetValue(list, out var run)) run(); });
        });
    }

    /// <summary>Runs <paramref name="run"/> when the person moved a row of a list.</summary>
    public static void OnReorder(ListBox list, Action run) => moved.AddOrUpdate(list, run);

    static object? Read(object row, string property) => row.GetType().GetProperty(property)?.GetValue(row);

    static void Write(object row, string property, object? value)
    {
        if (row.GetType().GetProperty(property) is not { CanWrite: true } p)
            return;
        var type = Nullable.GetUnderlyingType(p.PropertyType) ?? p.PropertyType;
        p.SetValue(row, value is null ? null : Convert.ChangeType(value, type));
    }

    /// <summary>
    /// Applies a move to <paramref name="rows"/>: each record gets its number in
    /// <paramref name="by"/> (and its parent in <paramref name="parent"/>), and
    /// the list its new order. The moved record, or null when nothing changed.
    /// </summary>
    public static object? Apply(IList rows, string by, string? parent, Func<IReadOnlyList<TreeRow>, Reordered?> move, int row)
    {
        var list = rows.Cast<object>().ToList();
        TreeRow[] tree = [.. list.Select(r => new TreeRow(Read(r, "Id"), parent is null ? null : Read(r, parent)))];
        if (move(tree) is not { } result)
            return null;
        for (var i = 0; i < result.Order.Length; i++)
        {
            var record = list[result.Order[i]];
            Write(record, by, result.Numbers[i]);
            if (parent is not null)
                Write(record, parent, result.Parents[i]);
        }
        // Moved in place, so the list keeps its records and the screen its rows.
        var shift = rows.GetType().GetMethod("Move", [typeof(int), typeof(int)]);
        for (var i = 0; i < result.Order.Length; i++)
        {
            var record = list[result.Order[i]];
            var now = rows.IndexOf(record);
            if (now == i)
                continue;
            if (shift is not null)
                shift.Invoke(rows, [now, i]);
            else
            {
                rows.RemoveAt(now);
                rows.Insert(i, record);
            }
        }
        return list[row];
    }

    /// <summary>How far the pointer goes before a press is a drag, in pixels.</summary>
    const double Start = 4;

    /// <summary>Where a drop at <paramref name="y"/> on a row of <paramref name="height"/> lands.</summary>
    public static DropAt Where(double y, double height, bool tree)
    {
        var part = y / Math.Max(height, 1);
        if (!tree)
            return part < 0.5 ? DropAt.Before : DropAt.After;
        return part < 0.25 ? DropAt.Before : part > 0.75 ? DropAt.After : DropAt.Into;
    }

    /// <summary>The move Alt and an arrow key ask for, or none.</summary>
    public static MoveKey? KeyOf(KeyEventArgs e)
    {
        if (!e.KeyModifiers.HasFlag(KeyModifiers.Alt) || e.KeyModifiers.HasFlag(KeyModifiers.Control) || e.KeyModifiers.HasFlag(KeyModifiers.Meta))
            return null;
        return e.Key switch
        {
            Key.Up => MoveKey.Up,
            Key.Down => MoveKey.Down,
            Key.Left => MoveKey.Left,
            Key.Right => MoveKey.Right,
            _ => null,
        };
    }

    static IBrush Accent => Application.Current?.FindResource("ClarityAccent") as IBrush ?? Brushes.SteelBlue;

    /// <summary>The mark on a row where a dragged row lands.</summary>
    static Control Mark(DropAt where) => where switch
    {
        DropAt.Before => new Border { BorderBrush = Accent, BorderThickness = new Thickness(0, 2, 0, 0), IsHitTestVisible = false },
        DropAt.After => new Border { BorderBrush = Accent, BorderThickness = new Thickness(0, 0, 0, 2), IsHitTestVisible = false },
        _ => new Border { Background = Accent, Opacity = 0.25, IsHitTestVisible = false },
    };

    /// <summary>
    /// Lets the person move the rows of <paramref name="host"/>, the records of
    /// <paramref name="items"/> in the order of the data: <paramref name="by"/>
    /// is the property the rows are numbered in (none: they do not move),
    /// <paramref name="parent"/> a tree's parent property, <paramref name="rowOf"/>
    /// finds the row a visual is in, and <paramref name="told"/> runs after a move.
    /// </summary>
    public static void Attach(
        Control host, Func<IList?> items, Func<string?> by, Func<string?> parent, Func<object?, Control?> rowOf,
        Func<object?> chosen, Action<object> choose, Action told)
    {
        Point? pressed = null;
        object? dragged = null;
        var dragging = false;
        Control? over = null;
        DropAt where = DropAt.Before;

        IList? Rows() => items();
        bool Moves() => by() is not null && host.IsEffectivelyEnabled && Rows() is not null;

        void Clear()
        {
            if (over is not null)
                AdornerLayer.SetAdorner(over, null);
            over = null;
        }

        void Move(Func<IReadOnlyList<TreeRow>, int, Reordered?> move, object record)
        {
            if (Rows() is not { } rows || by() is not { } field)
                return;
            var place = rows.IndexOf(record);
            if (place < 0 || Apply(rows, field, parent(), tree => move(tree, place), place) is not { } row)
                return;
            choose(row);
            told();
        }

        host.AddHandler(InputElement.PointerPressedEvent, (_, e) =>
        {
            if (!Moves() || !e.GetCurrentPoint(host).Properties.IsLeftButtonPressed)
                return;
            // A toggle, a cell's input or a button keeps its own press.
            if (e.Source is Visual v && v.FindAncestorOfType<Button>(includeSelf: true) is not null)
                return;
            if (e.Source is Visual input && (input.FindAncestorOfType<TextBox>(includeSelf: true) ?? input.FindAncestorOfType<ToggleButton>(includeSelf: true) as Control ?? input.FindAncestorOfType<ComboBox>(includeSelf: true)) is not null)
                return;
            if (rowOf(e.Source) is not { DataContext: { } record })
                return;
            pressed = e.GetPosition(host);
            dragged = record;
            dragging = false;
        }, RoutingStrategies.Tunnel, handledEventsToo: true);

        host.AddHandler(InputElement.PointerMovedEvent, (_, e) =>
        {
            if (pressed is not { } from || dragged is null)
                return;
            var at = e.GetPosition(host);
            if (!dragging && Math.Abs(at.X - from.X) + Math.Abs(at.Y - from.Y) < Start)
                return;
            if (!dragging)
            {
                dragging = true;
                e.Pointer.Capture(host);
            }
            var row = rowOf(host.InputHitTest(at));
            if (row is null || row.DataContext is null)
            {
                Clear();
                return;
            }
            var next = Where(e.GetPosition(row).Y, row.Bounds.Height, parent() is not null);
            if (row == over && next == where)
                return;
            Clear();
            over = row;
            where = next;
            AdornerLayer.SetAdorner(row, Mark(next));
        }, RoutingStrategies.Tunnel, handledEventsToo: true);

        host.AddHandler(InputElement.PointerReleasedEvent, (_, e) =>
        {
            var record = dragged;
            var target = dragging ? over?.DataContext : null;
            var at = where;
            if (dragging)
                e.Pointer.Capture(null);
            pressed = null;
            dragged = null;
            dragging = false;
            Clear();
            if (record is null || target is null || Rows() is not { } rows)
                return;
            var onto = rows.IndexOf(target);
            if (onto >= 0)
                Move((tree, place) => Reorder.Drag(tree, place, at, onto, parent() is not null), record);
        }, RoutingStrategies.Tunnel, handledEventsToo: true);

        host.AddHandler(InputElement.PointerCaptureLostEvent, (_, _) =>
        {
            pressed = null;
            dragged = null;
            dragging = false;
            Clear();
        });

        host.AddHandler(InputElement.KeyDownEvent, (_, e) =>
        {
            if (!Moves() || KeyOf(e) is not { } key || chosen() is not { } record)
                return;
            e.Handled = true;
            Move((tree, place) => Reorder.Key(tree, place, key, parent() is not null), record);
        }, RoutingStrategies.Tunnel);
    }
}
