// What the parts that show the records of a list share (a list, a table, a
// tree and a tree table): the rows read from the window's data, the row the
// person chose, their own border unless a container frames them (DESIGN
// 7.8), and moving rows by hand (DESIGN 5.30, conformance/reorder.json): a
// drag of a row onto another and Alt with an arrow key, written back as one
// edit after which `reorder` runs. Their rows scroll in a ScrollView, and
// evenly spaced rows are placed, hit and kept in sight here once for all.

using System.Globalization;
using System.Reflection;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>A field of a record as the data parts read and write it: a record's property (in any case), or a dictionary's entry.</summary>
public static class RowFields
{
    static PropertyInfo? Property(object row, string field) =>
        row.GetType().GetProperty(field, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase);

    static string? Key(IDictionary<string, object?> row, string field) =>
        row.ContainsKey(field) ? field : row.Keys.FirstOrDefault(k => string.Equals(k, field, StringComparison.OrdinalIgnoreCase));

    /// <summary>The value of <paramref name="field"/> of <paramref name="row"/>; null when it has none.</summary>
    public static object? Read(object? row, string field) => row switch
    {
        null => null,
        IDictionary<string, object?> d => Key(d, field) is { } key ? d[key] : null,
        _ => Property(row, field)?.GetValue(row),
    };

    /// <summary>The type <paramref name="field"/> holds, without its nullable; null when it is not known.</summary>
    public static Type? TypeOf(object? row, string field)
    {
        var type = row is null or IDictionary<string, object?> ? Read(row, field)?.GetType() : Property(row, field)?.PropertyType;
        return type is null ? null : Nullable.GetUnderlyingType(type) ?? type;
    }

    /// <summary>Sets <paramref name="field"/> of <paramref name="row"/>, the value made the type the field holds.</summary>
    public static void Set(object row, string field, object? value)
    {
        if (row is IDictionary<string, object?> d)
        {
            d[Key(d, field) ?? field] = value;
            return;
        }
        if (Property(row, field) is not { CanWrite: true } p)
            return;
        p.SetValue(row, As(value, p.PropertyType));
    }

    /// <summary><paramref name="value"/> as a value of <paramref name="type"/>, when it is not one already.</summary>
    public static object? As(object? value, Type type)
    {
        if (value is null)
            return null;
        var plain = Nullable.GetUnderlyingType(type) ?? type;
        if (plain.IsInstanceOfType(value))
            return value;
        if (plain.IsEnum)
            return value is string s ? Enum.Parse(plain, s, ignoreCase: true) : Enum.ToObject(plain, value);
        return Convert.ChangeType(value, plain, CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// The path of a field of the record at <paramref name="place"/> of the list
    /// at <paramref name="list"/>, as a part writes it: <c>tasks[0].done</c>.
    /// </summary>
    public static string Path(string list, int place, string field) => $"{list}[{place}].{field}";

    /// <summary>The path of a field of every record of a list, whose choices a column offers: <c>tasks.priority</c>.</summary>
    public static string ColumnPath(string list, string field) => $"{list}.{field}";
}

/// <summary>A part that shows the records of a list: a list, a table, a tree or a tree table.</summary>
public abstract class RowsPart : Part
{
    IReadOnlyList<object> rows = [];
    object? own;

    public override bool Grows => true;

    public override bool Focusable => Enabled;

    /// <summary>
    /// Whether it draws its own border: with <c>frame</c>, or when no container
    /// frames it (DESIGN 7.8). Read as it draws, from the parts around it.
    /// </summary>
    public bool Framed => Spec.Flag("frame") || !ContainerPart.IsInside(this);

    /// <summary>The records it shows, in the order of the data.</summary>
    public IReadOnlyList<object> Records => rows;

    /// <summary>The chosen record: its part state's, for a named part; none when the records hold it no more.</summary>
    public object? Chosen
    {
        get
        {
            var chosen = State is { } state ? state.SelectedItem : own;
            return chosen is not null && PlaceOf(chosen) >= 0 ? chosen : null;
        }
    }

    /// <summary>The place of the chosen record in the data; -1 for none.</summary>
    public int ChosenPlace => Chosen is { } c ? PlaceOf(c) : -1;

    protected int PlaceOf(object record)
    {
        for (var i = 0; i < rows.Count; i++)
            if (ReferenceEquals(rows[i], record))
                return i;
        return -1;
    }

    protected internal override void Made()
    {
        Read();
        Host.Changed += () =>
        {
            Read();
            Invalidate();
        };
        if (State is { } state)
            state.PropertyChanged += (_, e) =>
            {
                if (e.PropertyName == nameof(PartState.SelectedItem) && Chosen is { } chosen)
                    Shown(chosen);
                Invalidate();
            };
    }

    /// <summary>Reads the records again; ones that are other records have no row chosen (conformance/tables.json).</summary>
    void Read()
    {
        rows = Bound is { } bound ? Host.Rows(bound) : [];
        var chosen = State is { } state ? state.SelectedItem : own;
        if (chosen is not null && PlaceOf(chosen) < 0)
            SetChosen(null);
        RowsRead();
        PlaceRows();
    }

    /// <summary>The records were read again: what the part keeps of them follows.</summary>
    protected virtual void RowsRead() { }

    /// <summary>A record became the chosen one: the part shows it (opens its tree, turns to its page, scrolls to it).</summary>
    protected virtual void Shown(object record) { }

    void SetChosen(object? record)
    {
        if (State is { } state)
            state.SelectedItem = record;
        else
            own = record;
    }

    /// <summary>The person chose the record at <paramref name="place"/>: <c>select</c> runs when it is another one.</summary>
    protected void Choose(int place)
    {
        if (place < 0 || place >= rows.Count || ReferenceEquals(Chosen, rows[place]))
            return;
        var record = rows[place];
        SetChosen(record);
        Shown(record);
        Invalidate();
        Fire("select");
    }

    /// <summary>The person opened a row (a double click): <c>open</c>.</summary>
    protected void Open() => Fire("open");

    /// <summary>Runs a handler of the part: <c>change</c> after the person's edit.</summary>
    protected void Tell(string @event) => Fire(@event);

    // -- Moving rows by hand (DESIGN 5.30) -----------------------------------

    /// <summary>The field the rows are numbered in when the person moves them (<c>reorder by position</c>); none, and they stay put.</summary>
    public string? ReorderBy => Spec.Text("reorder");

    /// <summary>Whether the person moves rows now.</summary>
    protected bool Moves => ReorderBy is not null && Bound is not null && Enabled && !ReadOnly;

    /// <summary>Whether the rows are a tree, under their parents.</summary>
    public virtual bool IsTree => false;

    /// <summary>The field of a record that holds its parent's id, in a tree: as the application's facts of its entity say, else <c>parent</c>.</summary>
    public virtual string ParentField =>
        (Bound is { } bound ? new[] { "id", "parent" }.Select(f => Host.Field(RowFields.ColumnPath(bound, f))?.Parent).FirstOrDefault(p => p is not null) : null) ?? "parent";

    /// <summary>The rows as a tree places them: their ids and parents' ids.</summary>
    protected IReadOnlyList<TreeRow> TreeRows() =>
        [.. rows.Select(r => new TreeRow(RowFields.Read(r, "id"), IsTree ? RowFields.Read(r, ParentField) : null))];

    /// <summary>The row under the pointer at <paramref name="screenY"/>: its place in the data and its box on the screen.</summary>
    protected abstract (int Place, Box Row)? RowAt(double screenY);

    /// <summary>How far the pointer goes before a press is a drag.</summary>
    const double DragStart = 4;

    (int Place, double X, double Y)? pressedRow;
    bool dragging;

    /// <summary>Where a dragged row lands now: on the row at a place, before, after or into it.</summary>
    public (int Target, DropAt Where)? Drop { get; private set; }

    /// <summary>Whether a press became a drag.</summary>
    protected bool Dragging => dragging;

    /// <summary>A press on the row at <paramref name="place"/>, which may become a drag.</summary>
    protected void PressRow(int place, PointerInput at)
    {
        pressedRow = Moves ? (place, at.ScreenX, at.ScreenY) : null;
        dragging = false;
        Drop = null;
    }

    /// <summary>The pointer moved while pressed: a drag past a few pixels, marking where the row lands.</summary>
    protected void DragTo(PointerInput at)
    {
        if (pressedRow is not { } from)
            return;
        if (!dragging && Math.Abs(at.ScreenX - from.X) + Math.Abs(at.ScreenY - from.Y) < DragStart)
            return;
        dragging = true;
        Drop = RowAt(at.ScreenY) is { } over && over.Place != from.Place
            ? (over.Place, RowOrder.Where(at.ScreenY - over.Row.Y, over.Row.Height, IsTree))
            : null;
        Invalidate();
    }

    /// <summary>The press ended: a drag drops its row; true when it was a drag.</summary>
    protected bool DropRow()
    {
        var was = dragging;
        var row = pressedRow;
        var drop = Drop;
        pressedRow = null;
        dragging = false;
        Drop = null;
        if (was)
            Invalidate();
        if (was && row is { } r && drop is { } d)
            Move(tree => Reorder.Drag(tree, r.Place, d.Where, d.Target, IsTree), r.Place);
        return was;
    }

    /// <summary>The move Alt and an arrow key ask for, or none.</summary>
    static MoveKey? KeyOf(KeyInput key)
    {
        if (!key.Modifiers.HasFlag(Modifiers.Alt) || key.Control || key.Modifiers.HasFlag(Modifiers.Meta))
            return null;
        return key.Key switch
        {
            Key.Up => MoveKey.Up,
            Key.Down => MoveKey.Down,
            Key.Left => MoveKey.Left,
            Key.Right => MoveKey.Right,
            _ => null,
        };
    }

    /// <summary>Alt and an arrow key move the chosen row; true when the key was one of those.</summary>
    protected bool MoveKeyed(KeyInput key)
    {
        if (!Moves || KeyOf(key) is not { } move)
            return false;
        if (ChosenPlace is var place and >= 0)
            Move(tree => Reorder.Key(tree, place, move, IsTree), place);
        return true;
    }

    /// <summary>
    /// A move of the row at <paramref name="place"/>: each record gets its number
    /// (and in a tree its parent), the list its new order, written as one edit;
    /// the moved row is chosen and <c>reorder</c> runs. A move that changes
    /// nothing does nothing.
    /// </summary>
    void Move(Func<IReadOnlyList<TreeRow>, Reordered?> move, int place)
    {
        if (ReorderBy is not { } by || Bound is not { } bound || move(TreeRows()) is not { } result)
            return;
        var moved = rows[place];
        var order = new List<object>();
        for (var i = 0; i < result.Order.Length; i++)
        {
            var record = rows[result.Order[i]];
            RowFields.Set(record, by, (long)result.Numbers[i]);
            if (IsTree)
                RowFields.Set(record, ParentField, result.Parents[i]);
            order.Add(record);
        }
        Host.Write(Spec.Name ?? "", bound, order);
        Read();
        SetChosen(moved);
        Shown(moved);
        Invalidate();
        Tell("reorder");
    }

    /// <summary>The line where a dragged row lands, or the row it goes under in a tree.</summary>
    protected void DrawDrop(Painter painter, Box row, DropAt where)
    {
        var accent = painter.Look.Color("accent");
        switch (where)
        {
            case DropAt.Before:
                painter.Fill(row with { Height = 2 }, accent);
                break;
            case DropAt.After:
                painter.Fill(row with { Y = row.Bottom - 2, Height = 2 }, accent);
                break;
            default:
                painter.Fill(row, accent.WithAlpha(64));
                break;
        }
    }

    /// <summary>A value of a field as the person reads it: a choice by its label, else in the person's format.</summary>
    protected static string ShownOf(object? value, IReadOnlyList<Choice>? choices)
    {
        if (choices is { Count: > 0 })
            return choices.FirstOrDefault(c => Same(c.Value, value))?.Label ?? (value is null ? "" : ShownText.Of(value));
        return ShownText.Of(value);
    }

    /// <summary>Whether a choice's value is the value a field holds: an id as a long or an int, an enum by its name too.</summary>
    public static bool Same(object? a, object? b)
    {
        if (a is null || b is null)
            return a is null && b is null;
        if (Equals(a, b))
            return true;
        if (a is IConvertible && b is IConvertible && !(a is string) && !(b is string) && !a.GetType().IsEnum && !b.GetType().IsEnum)
            try
            {
                return Convert.ToDecimal(a, CultureInfo.InvariantCulture) == Convert.ToDecimal(b, CultureInfo.InvariantCulture);
            }
            catch (Exception e) when (e is FormatException or InvalidCastException or OverflowException)
            {
                return false;
            }
        return string.Equals(a.ToString(), b.ToString(), StringComparison.Ordinal);
    }

    /// <summary>The choices a field of the records offers: the host's, else an enum's values by their names.</summary>
    protected IReadOnlyList<Choice> ChoicesOf(string field)
    {
        if (Bound is not { } bound)
            return [];
        var offered = Host.Choices(RowFields.ColumnPath(bound, field));
        if (offered.Count > 0)
            return offered;
        var type = rows.Count > 0 ? RowFields.TypeOf(rows[0], field) : null;
        return type is { IsEnum: true } ? [.. Enum.GetValues(type).Cast<object>().Select(v => new Choice(v, v.ToString() ?? ""))] : [];
    }

    // -- Scrolling ------------------------------------------------------------

    readonly RowsSurface surface;

    protected RowsPart()
    {
        surface = new RowsSurface(this);
        Scroller = new ScrollView(surface) { Measure = (width, _) => (width, RowsHeight(LookNow)) };
        Scroller.Scrolled += Invalidate;
        AddChild(Scroller);
    }

    /// <summary>
    /// The place the rows scroll in: a scroll view over a surface as high as
    /// all the rows, which draws only the rows in sight and hands the pointer
    /// to the part. Its bar and the wheel are the scroll view's.
    /// </summary>
    public ScrollView Scroller { get; }

    /// <summary>How far the rows are scrolled, in pixels from the top.</summary>
    public double Scrolled => Scroller.ScrollY;

    /// <summary>Where the rows show on the screen.</summary>
    protected abstract Box RowsBox(ThemeLook look);

    /// <summary>How high all the rows are together.</summary>
    protected abstract double RowsHeight(ThemeLook look);

    /// <summary>Draws the rows in sight; the scroll view has clipped to <see cref="RowsBox"/>.</summary>
    protected abstract void DrawRows(Painter painter);

    /// <summary>Places the rows' scroll view again: the part moved, or the rows changed in number.</summary>
    protected void PlaceRows() => Scroller.Arrange(RowsBox(LookNow));

    public override void Arrange(Box box)
    {
        Bounds = box;
        PlaceRows();
    }

    /// <summary>The wheel anywhere over the part (a table's header too) scrolls the rows.</summary>
    public override bool OnWheel(double dx, double dy) => Scroller.OnWheel(dx, dy);

    /// <summary>Scrolls back to the first row (another page, another order).</summary>
    protected void ScrollToTop() => Scroller.ScrollY = 0;

    /// <summary>Whether the pointer is over the rows or the part.</summary>
    protected bool PointerOver => Hovered || surface.Hovered;

    /// <summary>Whether a press on the rows or the part holds the pointer.</summary>
    protected bool Holding => Pressed || surface.Pressed;

    // Rows that are each <c>pitch</c> apart from the top of the next, the
    // first at the top of the content: where one is on the screen, which one
    // is under the pointer, which are in sight.

    /// <summary>The box on the screen of the row at <paramref name="index"/> in the content, in sight or not.</summary>
    protected Box RowOnScreen(int index, double pitch, double height)
    {
        var view = Scroller.Bounds;
        return new Box(view.X, surface.Bounds.Y + index * pitch, view.Width, height);
    }

    /// <summary>The box on the screen of the row at <paramref name="index"/>; none while it is scrolled out of sight.</summary>
    protected Box? RowInSight(int index, double pitch, double height)
    {
        var view = Scroller.Bounds;
        var box = RowOnScreen(index, pitch, height);
        return box.Bottom <= view.Y || box.Y >= view.Bottom ? null : box;
    }

    /// <summary>The index of the row at <paramref name="screenY"/>, of <paramref name="count"/>; none outside the rows' place.</summary>
    protected int? IndexAt(double screenY, double pitch, int count)
    {
        var view = Scroller.Bounds;
        if (screenY < view.Y || screenY >= view.Bottom || pitch <= 0)
            return null;
        var at = (int)Math.Floor((screenY - surface.Bounds.Y) / pitch);
        return at >= 0 && at < count ? at : null;
    }

    /// <summary>The indexes of the rows in sight, from <c>First</c> up to but not with <c>End</c>.</summary>
    protected (int First, int End) InSight(double pitch, int count)
    {
        if (pitch <= 0)
            return (0, 0);
        var first = Math.Max(0, (int)(Scrolled / pitch));
        var end = Math.Min(count, (int)Math.Ceiling((Scrolled + Scroller.Bounds.Height) / pitch));
        return (first, Math.Max(first, end));
    }

    /// <summary>Scrolls as little as it takes to show the row at <paramref name="index"/>.</summary>
    protected void ShowIndex(int index, double pitch, double height)
    {
        PlaceRows();
        Scroller.ScrollInto(new Box(0, index * pitch, 0, height));
    }

    /// <summary>The content of the rows' scroll view: it draws through the part and hands it the pointer.</summary>
    sealed class RowsSurface(RowsPart part) : Element
    {
        public override void Draw(Painter painter) => part.DrawRows(painter);

        public override void OnPointerPressed(PointerInput at) => part.OnPointerPressed(at);

        public override void OnPointerMoved(PointerInput at) => part.OnPointerMoved(at);

        public override void OnPointerReleased(PointerInput at) => part.OnPointerReleased(at);
    }

    /// <summary>
    /// The row a key moves the choice to, of <paramref name="count"/> rows with
    /// the row at <paramref name="at"/> chosen (-1 for none): Up and Down one
    /// row (the first when none is chosen), Home and End the first and last,
    /// Page Up and Page Down <paramref name="page"/> rows; none for another key.
    /// </summary>
    protected static int? MovedTo(Key key, int at, int count, int page)
    {
        int? next = key switch
        {
            Key.Down => at < 0 ? 0 : at + 1,
            Key.Up => at < 0 ? 0 : at - 1,
            Key.Home => 0,
            Key.End => count - 1,
            Key.PageDown => at + page,
            Key.PageUp => at - page,
            _ => null,
        };
        return next is { } to && count > 0 ? Math.Clamp(to, 0, count - 1) : null;
    }

    /// <summary>
    /// A row's background in the list look: filled while it is chosen or under
    /// the pointer, a line along its bottom; gives the look its text is drawn in.
    /// </summary>
    protected static LookValues DrawRowBack(Painter painter, Box row, string? state)
    {
        var look = painter.Look;
        var shown = state is null ? look.Part("list", null) : look.Part("list", state);
        if (state is not null)
            painter.Fill(row, shown.Background ?? look.Color(state == "selected" ? "selection" : "accent_soft"));
        painter.Fill(row with { Y = row.Bottom - look.Border, Height = look.Border }, look.Color("border"));
        return shown;
    }

    /// <summary>How wide a text is drawn, also before the part is on a screen.</summary>
    protected double Measure(string text, string role, double size, bool bold) =>
        FontsNow.Measure(text, role, size, bold);

}
