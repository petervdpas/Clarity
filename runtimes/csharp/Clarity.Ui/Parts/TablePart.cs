// Clarity's table part (U1c), and a tree and a tree table, which are a table
// in tree mode. It owns how a table behaves, the same on every stack
// (conformance/tables.json): which input a cell holds, when a row counts as
// chosen and when as edited, and how text that is no value is marked. Rows
// show in the table's order (`order by`, a click on a `sortable` header) but
// keep their place in the data, which every event names
// (conformance/table-order.json); a page at a time with `page`
// (conformance/table-pages.json); a tree's rows under their parents
// (conformance/trees.json). Only the rows in sight are drawn, and the rows
// scroll under the header in the shared scroll view (RowsPart.Scroller). A
// cell is typed into with Clarity.UI's TextEditor, placed over the cell.

using System.Globalization;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>What a cell holds: the kind of value of its column.</summary>
public enum CellKind
{
    Text,
    Number,
    Bool,
    Choice,
    Date,
    Moment,
    /// <summary>A color, "#rrggbb": a swatch of it beside its text, typed as text.</summary>
    Color,
}

/// <summary>A column of a table: the field it shows, how, and whether its cells are edited.</summary>
public sealed record TableColumn(string Field, string Label, CellKind Kind, bool Edit, bool Switch, double? Width, IReadOnlyList<Choice> Choices, FieldFormat? Number);

/// <summary>The buttons of a table's page bar.</summary>
public enum PageButton
{
    First,
    Previous,
    Next,
    Last,
}

[PartKind("table", "tree", "treeTable")]
public sealed class TablePart : RowsPart, ITakesTyped
{
    List<TableColumn> columns = [];
    (string Field, bool Desc)? order;
    bool orderRead;
    int page = 1;
    readonly HashSet<object?> opened = [];
    Dictionary<int, TreeShown> treePlaces = [];
    readonly Dictionary<string, double> personWidths = [];
    readonly Dictionary<(int, string), string> marked = [];
    IReadOnlyList<object> markedFor = [];
    Editing? editing;
    double? pointerY;
    Press? press;

    /// <summary>A cell being typed into: its row's place, its field, and the text editor over it.</summary>
    sealed record Editing(int Place, string Field, TextEditor Editor);

    public TablePart() => Scroller.Scrolled += PlaceEditor;

    /// <summary>The text editor of the cell typed into now; none while no cell is.</summary>
    public TextEditor? CellEditor => editing?.Editor;

    /// <summary>What a press went down on.</summary>
    abstract record Press;
    sealed record OnRow(int Place) : Press;
    sealed record OnHeader(string Field) : Press;
    sealed record OnEdge(string Field, double X, double Width) : Press;
    sealed record OnPager(PageButton Button) : Press;

    /// <summary>The choices of a choice cell, while they show.</summary>
    public CellChoices? OpenChoices { get; private set; }

    public override bool IsTree => Spec.Kind is "tree" or "treeTable";

    bool HasHeader => Spec.Kind != "tree";

    bool Sortable => Spec.Flag("sortable") && !IsTree;

    bool Resizable => Spec.Flag("resizable");

    int? PageSize => Spec.Number("page") is { } size && size >= 1 && !IsTree ? (int)size : null;

    /// <summary>Whether its cells take no edits: read-only, or not enabled.</summary>
    bool Locked => ReadOnly || !Enabled;

    public IReadOnlyList<TableColumn> Columns => columns;

    /// <summary>The order the rows show in now: a field and its way, or none for the data's order.</summary>
    public (string Field, bool Desc)? Order => order;

    // -- Columns --------------------------------------------------------------

    /// <summary>The fields it shows: a tree the fields of <c>show</c>, else its record's first text; a table its <c>columns</c>.</summary>
    IEnumerable<(string Field, double? Width)> Fields()
    {
        var widths = Spec.Json("widths") as System.Text.Json.Nodes.JsonObject;
        IEnumerable<string> names = Spec.Kind == "tree" ? Spec.Names("show") is { Count: > 0 } show ? show : FirstText() : Spec.Names("columns");
        foreach (var written in names)
        {
            // `title 240`: a field and its designed width.
            var words = written.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (words.Length == 0)
                continue;
            double? width = words.Length > 1 && double.TryParse(words[1], NumberStyles.Float, CultureInfo.InvariantCulture, out var w) ? w : null;
            if (width is null && widths?[words[0]] is System.Text.Json.Nodes.JsonValue v && v.TryGetValue<double>(out var designed))
                width = designed;
            yield return (words[0], width);
        }
    }

    IEnumerable<string> FirstText()
    {
        var first = Records.FirstOrDefault();
        var field = first?.GetType().GetProperties().FirstOrDefault(p => p.PropertyType == typeof(string))?.Name
            ?? (first as IDictionary<string, object?>)?.FirstOrDefault(e => e.Value is string).Key
            ?? "name";
        return [field];
    }

    TableColumn ColumnOf(string field, double? width)
    {
        var edit = Spec.Names("edit").Contains(field);
        var choices = ChoicesOf(field);
        var type = Records.Select(r => RowFields.TypeOf(r, field)).FirstOrDefault(t => t is not null)
            ?? (Bound is { } b ? Host.TypeOf(RowFields.ColumnPath(b, field)) is { } t ? Nullable.GetUnderlyingType(t) ?? t : null : null);
        // The field's facts as the application gives them: its label, its format, whether it holds a color.
        var facts = Bound is { } list ? Host.Field(RowFields.ColumnPath(list, field)) : null;
        var kind = facts?.Color == true ? CellKind.Color
            : choices.Count > 0 || type is { IsEnum: true } ? CellKind.Choice
            : type == typeof(bool) ? CellKind.Bool
            : type == typeof(long) || type == typeof(int) || type == typeof(short) ? CellKind.Number
            : type == typeof(decimal) || type == typeof(double) || type == typeof(float) ? CellKind.Number
            : type == typeof(DateTime) ? CellKind.Moment
            : type == typeof(DateOnly) ? CellKind.Date
            : CellKind.Text;
        FieldFormat? number = kind != CellKind.Number ? null
            : type == typeof(long) || type == typeof(int) || type == typeof(short) ? new FieldFormat(FormatKind.Integer) : new FieldFormat(FormatKind.Decimal);
        // Its header: the field's label as the application gives it, else its name.
        if (kind == CellKind.Number && facts?.Format is { } format)
            number = format;
        return new TableColumn(field, Host.Text(facts?.Label ?? field), kind, edit, Spec.Names("switches").Contains(field), width, choices, number);
    }

    protected override void RowsRead()
    {
        columns = [.. Fields().Select(f => ColumnOf(f.Field, f.Width))];
        if (!orderRead)
        {
            orderRead = true;
            if (Spec.Json("order") is System.Text.Json.Nodes.JsonObject o && o["field"]?.GetValue<string>() is { } field)
                order = (field, o["desc"] is System.Text.Json.Nodes.JsonValue d && d.TryGetValue<bool>(out var desc) && desc);
        }
        // Rows read again are other records: what was typed into them is gone.
        if (!ReferenceEquals(markedFor, Records))
        {
            if (!(markedFor.Count == Records.Count && markedFor.Zip(Records).All(p => ReferenceEquals(p.First, p.Second))))
            {
                marked.Clear();
                Stop();
            }
            markedFor = Records;
        }
    }

    // -- What shows -----------------------------------------------------------

    /// <summary>
    /// The places of the rows in the order they show: by the table's order, a
    /// tree under their parents; all of them, before a page is cut out.
    /// </summary>
    List<int> Ordered()
    {
        if (IsTree)
        {
            var shown = Trees.Shown(TreeRows(), opened);
            treePlaces = shown.ToDictionary(s => s.Place);
            return [.. shown.Select(s => s.Place)];
        }
        var places = Enumerable.Range(0, Records.Count).ToList();
        if (order is not { } o || columns.FirstOrDefault(c => c.Field == o.Field) is not { } column)
            return places;
        var keyed = places.Select(p => (place: p, key: SortKey(column, RowFields.Read(Records[p], column.Field)))).ToList();
        keyed.Sort((a, b) =>
        {
            if (a.key is null || b.key is null)
                return a.key is null && b.key is null ? a.place - b.place : a.key is null ? 1 : -1;
            var by = a.key is string sa && b.key is string sb
                ? string.Compare(sa, sb, CultureInfo.CurrentCulture, CompareOptions.IgnoreCase)
                : a.key is IComparable ca && a.key.GetType() == b.key.GetType() ? ca.CompareTo(b.key) : 0;
            return (o.Desc ? -by : by) is var d && d != 0 ? d : a.place - b.place;
        });
        return [.. keyed.Select(k => k.place)];
    }

    /// <summary>Text ignoring case, numbers as numbers, a choice by the label it shows; an empty value is none and comes last.</summary>
    static object? SortKey(TableColumn column, object? value)
    {
        if (value is null || value is string { Length: 0 })
            return null;
        switch (column.Kind)
        {
            case CellKind.Choice:
                var label = column.Choices.FirstOrDefault(c => Same(c.Value, value))?.Label ?? value.ToString();
                return string.IsNullOrEmpty(label) ? null : label;
            case CellKind.Number:
                try
                {
                    return Convert.ToDecimal(value, CultureInfo.InvariantCulture);
                }
                catch (Exception e) when (e is FormatException or InvalidCastException or OverflowException)
                {
                    return value.ToString();
                }
            case CellKind.Bool:
                return value is true ? 1m : 0m;
            default:
                return value is IComparable ? value : value.ToString();
        }
    }

    /// <summary>The page shown; none without <c>page</c>.</summary>
    public Paged? Paged => PageSize is { } size ? TablePages.Of(Records.Count, size, page) : null;

    /// <summary>The places of the rows shown, top to bottom: the page shown of the ordered rows.</summary>
    public IReadOnlyList<int> ShownPlaces
    {
        get
        {
            var ordered = Ordered();
            if (Paged is not { } paged)
                return ordered;
            page = paged.Page;
            var from = Math.Max(paged.From - 1, 0);
            return ordered.GetRange(from, paged.To - from);
        }
    }

    /// <summary>Where a row of a tree is: how deep, and whether rows are under it.</summary>
    public TreeShown? TreePlace(int place)
    {
        Ordered();
        return treePlaces.TryGetValue(place, out var at) ? at : null;
    }

    /// <summary>Whether a row of a tree shows the rows under it.</summary>
    public bool IsOpen(int place) => place >= 0 && place < Records.Count && opened.Contains(RowFields.Read(Records[place], "id"));

    /// <summary>Shows or hides the rows under a row of a tree.</summary>
    public void Toggle(int place)
    {
        if (place < 0 || place >= Records.Count)
            return;
        var id = RowFields.Read(Records[place], "id");
        if (!opened.Remove(id))
            opened.Add(id);
        PlaceRows();
        Invalidate();
    }

    /// <summary>Clarity's own word on a tree row's toggle: what a press does.</summary>
    public string ToggleWord(int place) => IsOpen(place) ? Locale.Translate("collapseRow", "Collapse") : Locale.Translate("expandRow", "Expand");

    /// <summary>Clarity's own word on a button of the page bar.</summary>
    public static string PageWord(PageButton button) => button switch
    {
        PageButton.First => Locale.Translate("firstPage", "First page"),
        PageButton.Previous => Locale.Translate("previousPage", "Previous page"),
        PageButton.Next => Locale.Translate("nextPage", "Next page"),
        _ => Locale.Translate("lastPage", "Last page"),
    };

    /// <summary>Whether a button of the page bar works now: first and previous past the first page, next and last before the last.</summary>
    public bool PageButtonWorks(PageButton button) => Paged is { } p && Enabled && (button is PageButton.First or PageButton.Previous ? p.Page > 1 : p.Page < p.Pages);

    /// <summary>Shows page <paramref name="number"/>, kept within the pages there are.</summary>
    public void TurnTo(int number)
    {
        page = Paged is { } p ? Math.Clamp(number, 1, p.Pages) : 1;
        ScrollToTop();
        PlaceRows();
        Invalidate();
    }

    /// <summary>Sorts by the column of <paramref name="field"/>, or the other way when it already is; the first page shows.</summary>
    public void SortBy(string field)
    {
        if (!Sortable)
            return;
        order = order is { } o && o.Field == field ? (field, !o.Desc) : (field, false);
        page = 1;
        ScrollToTop();
        PlaceRows();
        Invalidate();
    }

    /// <summary>A row the part chose or code chose shows: in a tree the rows above it open, in pages its page, and it scrolls into sight.</summary>
    protected override void Shown(object record)
    {
        var place = PlaceOf(record);
        if (place < 0)
            return;
        if (IsTree)
            foreach (var id in Trees.Above(TreeRows(), place))
                opened.Add(id);
        if (PageSize is { } size && Ordered().IndexOf(place) is var at and >= 0)
        {
            var wanted = TablePages.With(size, at);
            if (wanted != page)
            {
                page = wanted;
                ScrollToTop();
            }
        }
        if (ShownPlaces.ToList().IndexOf(place) is var row and >= 0)
            ShowIndex(row, RowHeight(LookNow), RowHeight(LookNow));
    }

    // -- Layout ---------------------------------------------------------------

    static double RowHeight(ThemeLook look) => look.Size("text") + 18;

    Box Inner(ThemeLook look) => Framed ? Bounds.Inset(look.Border) : Bounds;

    Box Header(ThemeLook look)
    {
        var inner = Inner(look);
        return inner with { Height = HasHeader ? RowHeight(look) : 0 };
    }

    Box Pager(ThemeLook look)
    {
        var inner = Inner(look);
        var height = PageSize is null ? 0 : RowHeight(look);
        return new Box(inner.X, inner.Bottom - height, inner.Width, height);
    }

    Box Body(ThemeLook look)
    {
        var header = Header(look);
        var pager = Pager(look);
        return new Box(header.X, header.Bottom, header.Width, Math.Max(0, pager.Y - header.Bottom));
    }

    double WidthOf(TableColumn column) => personWidths.TryGetValue(column.Field, out var w) ? w : column.Width ?? 0;

    /// <summary>Each column's left edge and width: a designed width as it is, a check its own, the others sharing what is left.</summary>
    List<(double X, double Width)> Lefts(Box row)
    {
        const double Least = 48;
        var look = LookNow;
        var header = look.Part("header", null);
        // A check's column is as wide as its header.
        double CheckWidth(TableColumn c) =>
            Math.Max(56, Math.Ceiling(Measure(c.Label, header.Font ?? "text", header.Size ?? look.Size("text"), header.Bold)) + 4 * look.Padding + 16);
        var fixedWidth = columns.Sum(c => WidthOf(c) > 0 ? WidthOf(c) : c.Kind == CellKind.Bool ? CheckWidth(c) : 0);
        var sharing = columns.Count(c => WidthOf(c) <= 0 && c.Kind != CellKind.Bool);
        var share = sharing == 0 ? 0 : Math.Max(Least, (row.Width - fixedWidth) / sharing);
        var x = row.X;
        var lefts = new List<(double, double)>();
        foreach (var column in columns)
        {
            var width = WidthOf(column) > 0 ? WidthOf(column) : column.Kind == CellKind.Bool ? CheckWidth(column) : share;
            lefts.Add((x, width));
            x += width;
        }
        return lefts;
    }

    /// <summary>The box of the row of the record at <paramref name="place"/> on the screen; none while it is not in sight.</summary>
    public Box? RowBox(int place)
    {
        var at = ShownPlaces.ToList().IndexOf(place);
        var height = RowHeight(LookNow);
        return at < 0 ? null : RowInSight(at, height, height);
    }

    /// <summary>The box of the cell of <paramref name="field"/> in the row of <paramref name="place"/>, while it is in sight.</summary>
    public Box? CellBox(int place, string field)
    {
        var c = columns.FindIndex(c => c.Field == field);
        if (RowBox(place) is not { } row || c < 0)
            return null;
        var (x, width) = Lefts(row)[c];
        return new Box(x, row.Y, width, row.Height);
    }

    /// <summary>The swatch of the color in a cell of a color column, while it is in sight; none for a cell without a color.</summary>
    public Box? SwatchBox(int place, string field)
    {
        if (columns.FirstOrDefault(c => c.Field == field) is not { Kind: CellKind.Color } column || place < 0 || place >= Records.Count
            || ColorOf(RowFields.Read(Records[place], field)) is null || CellBox(place, field) is not { } cell)
            return null;
        var look = LookNow;
        var left = column.Edit ? cell.X + 3 + look.Padding * 1.5 : cell.X + look.Padding * 2;
        return new Box(left, cell.Y + (cell.Height - SwatchSide) / 2, SwatchSide, SwatchSide);
    }

    const double SwatchSide = 12;

    /// <summary>The color a cell holds ("#rrggbb", or a name the palette knows), or none.</summary>
    static SKColor? ColorOf(object? value) =>
        value is string text && Clarity.Runtime.Palette.Read(text) is { Length: 7 } color && SKColor.TryParse(color, out var parsed) ? parsed : null;

    /// <summary>Draws the swatch of a color cell, and gives the room its text keeps beside it.</summary>
    double DrawSwatch(Painter painter, int place, TableColumn column)
    {
        if (column.Kind != CellKind.Color || SwatchBox(place, column.Field) is not { } swatch)
            return 0;
        var look = painter.Look;
        var radius = Math.Min(2, look.Radius("input"));
        painter.Fill(swatch, ColorOf(RowFields.Read(Records[place], column.Field))!.Value, radius);
        painter.Stroke(swatch, look.Color("strong_border"), look.Border, radius);
        return SwatchSide + look.Padding * 1.5;
    }

    /// <summary>The box of the header of <paramref name="field"/>; none without a header.</summary>
    public Box? HeaderBox(string field)
    {
        var header = Header(LookNow);
        var c = columns.FindIndex(c => c.Field == field);
        if (!HasHeader || c < 0)
            return null;
        var (x, width) = Lefts(header)[c];
        return new Box(x, header.Y, width, header.Height);
    }

    /// <summary>The box of a tree row's toggle, when rows are under it.</summary>
    public Box? ToggleBox(int place)
    {
        if (!IsTree || columns.Count == 0 || CellBox(place, columns[0].Field) is not { } cell || TreePlace(place) is not { Children: true } at)
            return null;
        return new Box(cell.X + LookNow.Padding + at.Depth * 16, cell.Y, 20, cell.Height);
    }

    /// <summary>The box of a button of the page bar; none without pages.</summary>
    public Box? PagerBox(PageButton button)
    {
        var look = LookNow;
        if (Paged is not { } paged)
            return null;
        var bar = Pager(look);
        const double Button = 26;
        var text = Math.Ceiling(Measure(paged.Text, "text", look.Size("text"), false)) + 16;
        var right = bar.Right - look.Padding;
        var lefts = new Dictionary<PageButton, double>
        {
            [PageButton.Last] = right - Button,
            [PageButton.Next] = right - 2 * Button,
            [PageButton.Previous] = right - 3 * Button - text,
            [PageButton.First] = right - 4 * Button - text,
        };
        return new Box(lefts[button], bar.Y, Button, bar.Height);
    }

    Box PagerText(ThemeLook look)
    {
        var previous = PagerBox(PageButton.Previous)!.Value;
        var next = PagerBox(PageButton.Next)!.Value;
        return new Box(previous.Right, previous.Y, next.X - previous.Right, previous.Height);
    }

    (int Place, TableColumn Column, Box Cell)? CellAt(double x, double y)
    {
        if (RowAt(y) is not { } row)
            return null;
        var lefts = Lefts(row.Row);
        for (var c = 0; c < columns.Count; c++)
            if (x >= lefts[c].X && x < lefts[c].X + lefts[c].Width)
                return (row.Place, columns[c], new Box(lefts[c].X, row.Row.Y, lefts[c].Width, row.Row.Height));
        return null;
    }

    protected override (int Place, Box Row)? RowAt(double screenY)
    {
        var shown = ShownPlaces;
        var height = RowHeight(LookNow);
        return IndexAt(screenY, height, shown.Count) is { } at && RowInSight(at, height, height) is { } box ? (shown[at], box) : null;
    }

    protected override Box RowsBox(ThemeLook look) => Body(look);

    protected override double RowsHeight(ThemeLook look) => ShownPlaces.Count * RowHeight(look);

    // -- What a cell shows and takes --------------------------------------------

    /// <summary>The text a cell shows: what is typed into it, else its value as its column shows it.</summary>
    public string Shown(int place, string field)
    {
        if (editing is { } e && e.Place == place && e.Field == field)
            return e.Editor.Text;
        if (marked.TryGetValue((place, field), out var typed))
            return typed;
        if (columns.FirstOrDefault(c => c.Field == field) is not { } column || place < 0 || place >= Records.Count)
            return "";
        return ShownValue(column, RowFields.Read(Records[place], field));
    }

    static string ShownValue(TableColumn column, object? value)
    {
        if (column.Kind == CellKind.Choice)
            return ShownOf(value, column.Choices);
        if (!column.Edit)
            return ShownText.Of(value, column.Number);
        return column.Kind switch
        {
            CellKind.Number => value is null ? "" : Locale.Formats.EditText(column.Number!, value),
            // An edited moment shows its day beside its time; the time is what is typed.
            CellKind.Moment => value is DateTime t ? t.ToString("HH:mm", CultureInfo.InvariantCulture) : "",
            _ => ShownText.Of(value),
        };
    }

    /// <summary>Whether a cell holds typed text that is no value (it shows as typed and the value stays).</summary>
    public bool Marked(int place, string field) => marked.ContainsKey((place, field));

    /// <summary>
    /// The person changed a cell: only a value that differs is an edit, written
    /// as the person's edit of the record's field; its row is chosen first
    /// (<c>select</c>), then <c>change</c> runs.
    /// </summary>
    void Edit(int place, string field, object? value)
    {
        if (Locked || Bound is not { } bound || place < 0 || place >= Records.Count)
            return;
        var record = Records[place];
        if (Same(RowFields.Read(record, field), value))
            return;
        Host.Write(Spec.Name ?? "", RowFields.Path(bound, place, field), value);
        Choose(PlaceOf(record) is var now and >= 0 ? now : place);
        Tell("change");
    }

    /// <summary>A day picked for a date or moment cell (from a calendar): a moment keeps its time, an empty one starts at midnight.</summary>
    public void PickDay(int place, string field, DateOnly day)
    {
        if (columns.FirstOrDefault(c => c.Field == field) is not { Edit: true } column || place < 0 || place >= Records.Count)
            return;
        if (column.Kind == CellKind.Moment)
        {
            var time = RowFields.Read(Records[place], field) is DateTime t ? t.TimeOfDay : TimeSpan.Zero;
            Edit(place, field, day.ToDateTime(TimeOnly.MinValue) + time);
        }
        else if (column.Kind == CellKind.Date)
            Edit(place, field, day);
    }

    /// <summary>
    /// Starts typing into a text, number or moment cell: a text editor over it
    /// takes the focus with all of its text selected, so what is typed first
    /// replaces it. Enter takes the text and does nothing else; Escape leaves
    /// the cell and its value stays; leaving it (Tab, a press elsewhere) takes it.
    /// </summary>
    void BeginTyping(int place, TableColumn column)
    {
        Take();
        if (Locked)
            return;
        var field = column.Field;
        var editor = new TextEditor
        {
            Text = Shown(place, field),
            Align = column.Kind switch
            {
                CellKind.Number => TextAlign.Right,
                CellKind.Moment => TextAlign.Center,
                _ => TextAlign.Left,
            },
            Marked = () => Marked(place, field),
            TakesTab = true,
            Entered = () =>
            {
                Take();
                return true;
            },
            KeyFirst = key =>
            {
                if (key.Key == Key.Tab && key.Modifiers is Modifiers.None or Modifiers.Shift)
                {
                    // Past the last cell the table, which has the focus again, lets Tab move it on.
                    if (!TypeOn(place, field, key.Shift))
                        Screen?.KeyPressed(key);
                    return true;
                }
                if (key.Key != Key.Escape)
                    return false;
                Stop();
                return true;
            },
        };
        editor.Edited += Invalidate;
        editor.FocusMoved += focused =>
        {
            if (!focused)
                Take();
        };
        editing = new Editing(place, field, editor);
        AddChild(editor);
        PlaceEditor();
        editor.SelectAll();
        Screen?.Focus(editor);
        Invalidate();
    }

    /// <summary>
    /// Tab in a cell's editor: takes its text and types on in the next typed
    /// cell of the shown rows (the one before with Shift). False past the last,
    /// so the focus leaves the table as Tab moves it.
    /// </summary>
    bool TypeOn(int place, string field, bool back)
    {
        var typed = columns.Where(c => c.Edit && c.Kind is CellKind.Text or CellKind.Number or CellKind.Color or CellKind.Moment).ToList();
        var cells = ShownPlaces.SelectMany(p => typed.Select(c => (Place: p, Column: c))).ToList();
        var at = cells.FindIndex(c => c.Place == place && c.Column.Field == field);
        var next = at < 0 ? -1 : back ? at - 1 : at + 1;
        Take();
        if (next < 0 || next >= cells.Count || Locked)
            return false;
        var (to, column) = cells[next];
        BeginTyping(to, column);
        return true;
    }

    /// <summary>Places the cell's editor over its input (a moment's time at its right); out of sight it has no place.</summary>
    void PlaceEditor()
    {
        if (editing is not { } e)
            return;
        var box = default(Box);
        if (CellBox(e.Place, e.Field) is { } cell && columns.FirstOrDefault(c => c.Field == e.Field) is { } column)
        {
            cell = cell with { Height = cell.Height - LookNow.Border };
            box = (column.Kind == CellKind.Moment ? TimeBox(cell) : cell).Inset(3);
        }
        e.Editor.Arrange(box);
    }

    public override void Arrange(Box box)
    {
        base.Arrange(box);
        PlaceEditor();
    }

    /// <summary>Ends typing into the cell without taking the text: the editor goes, and the table has the focus again when the editor had it.</summary>
    Editing? Stop()
    {
        if (editing is not { } e)
            return null;
        editing = null;
        if (Screen is { } screen && screen.Focused == e.Editor)
            screen.Focus(this);
        RemoveChild(e.Editor);
        Invalidate();
        return e;
    }

    /// <summary>A key that runs a command has the cell being typed into take its text first.</summary>
    public void TakeTyped() => Take();

    /// <summary>Takes what was typed into the cell being typed into: a value is an edit, text that is no value is marked.</summary>
    void Take()
    {
        if (Stop() is not { } e)
            return;
        if (columns.FirstOrDefault(c => c.Field == e.Field) is not { } column || e.Place >= Records.Count)
            return;
        var record = Records[e.Place];
        var typed = e.Editor.Text;
        object? value;
        switch (column.Kind)
        {
            case CellKind.Number:
                value = typed.Trim().Length == 0 ? null : Locale.Formats.Read(column.Number!, typed);
                if (value is null && typed.Trim().Length > 0)
                {
                    marked[(e.Place, e.Field)] = typed;
                    return;
                }
                break;
            case CellKind.Moment:
                if (!TimeSpan.TryParseExact(typed.Trim(), ["h\\:mm", "hh\\:mm"], CultureInfo.InvariantCulture, out var time) || time.TotalHours >= 24)
                {
                    if (typed.Trim().Length > 0)
                        marked[(e.Place, e.Field)] = typed;
                    return;
                }
                // Without a day, a time waits for one.
                if (RowFields.Read(record, e.Field) is not DateTime day)
                    return;
                value = day.Date + time;
                break;
            default:
                value = typed;
                if (Same(RowFields.Read(record, e.Field) ?? "", typed))
                {
                    marked.Remove((e.Place, e.Field));
                    return;
                }
                break;
        }
        marked.Remove((e.Place, e.Field));
        Edit(e.Place, e.Field, value);
    }

    /// <summary>Opens the choices of a choice cell under it.</summary>
    void OpenChoicesOf(int place, TableColumn column, Box cell)
    {
        if (Screen is not { } screen || Locked)
            return;
        var height = RowHeight(screen.Look) - 4;
        var list = new CellChoices(column.Choices, RowFields.Read(Records[place], column.Field), height);
        var place_ = place;
        Overlay? overlay = null;
        list.Picked = value =>
        {
            if (overlay is not null)
                screen.Close(overlay);
            Edit(place_, column.Field, value);
        };
        var below = new Box(cell.X, cell.Bottom, Math.Max(cell.Width, 120), CellChoices.HeightFor(column.Choices.Count, height));
        if (below.Bottom > screen.Height && cell.Y - below.Height >= 0)
            below = below with { Y = cell.Y - below.Height };
        overlay = new Overlay(list, below) { Closed = () => OpenChoices = null };
        OpenChoices = list;
        screen.Open(overlay);
    }

    /// <summary>The calendar of a date or moment cell, while it is open.</summary>
    public DayCalendar? OpenCalendar { get; private set; }

    /// <summary>Opens the calendar under a date or moment cell: the day picked is its edit (PickDay).</summary>
    void OpenCalendarOf(int place, TableColumn column, Box cell)
    {
        if (Screen is not { } screen || Locked)
            return;
        var day = RowFields.Read(Records[place], column.Field) switch
        {
            DateOnly d => d,
            DateTime t => DateOnly.FromDateTime(t),
            _ => (DateOnly?)null,
        };
        var field = column.Field;
        var overlay = DayCalendar.Open(screen, cell, day, picked => PickDay(place, field, picked), closed: () => OpenCalendar = null);
        OpenCalendar = (DayCalendar)overlay.Content;
    }

    // -- What the person does -------------------------------------------------

    const double Edge = 5;

    public override void OnFocusChanged(bool focused)
    {
        // The focus going to the cell's editor is no leaving.
        if (!focused && (editing is null || Screen?.Focused != editing.Editor))
            Take();
    }

    public override void OnPointerPressed(PointerInput at)
    {
        press = null;
        if (!Enabled)
            return;
        var look = LookNow;
        var (x, y) = (at.ScreenX, at.ScreenY);
        if (Pager(look).Contains(x, y))
        {
            foreach (var button in Enum.GetValues<PageButton>())
                if (PagerBox(button) is { } box && box.Contains(x, y))
                    press = new OnPager(button);
            return;
        }
        if (HasHeader && Header(look).Contains(x, y))
        {
            Take();
            foreach (var heading in columns)
                if (HeaderBox(heading.Field) is { } box && box.Contains(x, y))
                {
                    if (Resizable && x >= box.Right - Edge)
                    {
                        if (at.Clicks == 2)
                        {
                            // The designed widths again.
                            personWidths.Clear();
                            Invalidate();
                            return;
                        }
                        press = new OnEdge(heading.Field, x, box.Width);
                    }
                    else
                        press = new OnHeader(heading.Field);
                }
            return;
        }
        if (CellAt(x, y) is not { } hit)
        {
            Take();
            return;
        }
        if (editing is { } e && (e.Place != hit.Place || e.Field != hit.Column.Field))
            Take();
        if (IsTree && ToggleBox(hit.Place) is { } toggle && toggle.Contains(x, y))
        {
            Toggle(hit.Place);
            return;
        }
        var column = hit.Column;
        // A press on an edited cell's input chooses no row: a taken edit chooses it.
        // A press beside the input, in the cell's margin, chooses the row.
        if (column.Edit && !Locked && InputOf(column, hit.Cell).Contains(x, y))
        {
            switch (column.Kind)
            {
                case CellKind.Bool:
                    Edit(hit.Place, column.Field, RowFields.Read(Records[hit.Place], column.Field) is not true);
                    return;
                case CellKind.Choice:
                    OpenChoicesOf(hit.Place, column, hit.Cell);
                    return;
                case CellKind.Text or CellKind.Number or CellKind.Color:
                    if (editing is null)
                        BeginTyping(hit.Place, column);
                    return;
                case CellKind.Moment when x >= TimeBox(hit.Cell).X:
                    if (editing is null)
                        BeginTyping(hit.Place, column);
                    return;
                case CellKind.Moment or CellKind.Date:
                    OpenCalendarOf(hit.Place, column, hit.Cell);
                    return;
            }
        }
        press = new OnRow(hit.Place);
        PressRow(hit.Place, at);
        if (at.Clicks == 2)
        {
            Choose(hit.Place);
            Open();
            press = null;
        }
    }

    public override void OnPointerMoved(PointerInput at)
    {
        pointerY = at.ScreenY;
        if (Holding && press is OnEdge edge)
        {
            personWidths[edge.Field] = Math.Clamp(edge.Width + at.ScreenX - edge.X, 32, 2000);
            // The other columns keep the widths they show.
            foreach (var other in columns)
                if (!personWidths.ContainsKey(other.Field) && HeaderBox(other.Field) is { } box)
                    personWidths[other.Field] = box.Width;
        }
        else if (Holding && press is OnRow)
            DragTo(at);
        Invalidate();
    }

    public override void OnPointerReleased(PointerInput at)
    {
        var was = press;
        press = null;
        switch (was)
        {
            case OnRow row:
                if (!DropRow() && RowAt(at.ScreenY) is { } over && over.Place == row.Place)
                    Choose(row.Place);
                break;
            case OnHeader header when HeaderBox(header.Field) is { } box && box.Contains(at.ScreenX, at.ScreenY):
                SortBy(header.Field);
                break;
            case OnPager pager when PagerBox(pager.Button) is { } box && box.Contains(at.ScreenX, at.ScreenY) && PageButtonWorks(pager.Button):
                var paged = Paged!;
                TurnTo(pager.Button switch
                {
                    PageButton.First => 1,
                    PageButton.Previous => paged.Page - 1,
                    PageButton.Next => paged.Page + 1,
                    _ => paged.Pages,
                });
                break;
        }
    }

    public override bool OnKey(KeyInput key)
    {
        if (MoveKeyed(key))
            return true;
        if (!Enabled || key.Modifiers != Modifiers.None)
            return false;
        var shown = ShownPlaces.ToList();
        if (shown.Count == 0)
            return false;
        var at = shown.IndexOf(ChosenPlace);
        var page = Math.Max(1, (int)(Body(LookNow).Height / RowHeight(LookNow)));
        if (IsTree && at >= 0 && key.Key is Key.Right or Key.Left)
        {
            var open = IsOpen(shown[at]);
            if (TreePlace(shown[at]) is { Children: true } && open == (key.Key == Key.Left))
                Toggle(shown[at]);
            return true;
        }
        if (key.Key == Key.Enter && at >= 0)
        {
            Open();
            return true;
        }
        if (MovedTo(key.Key, at, shown.Count, page) is not { } to)
            return false;
        Choose(shown[to]);
        Invalidate();
        return true;
    }

    // -- Drawing --------------------------------------------------------------

    static Box TimeBox(Box cell) => new(cell.Right - Math.Min(72, cell.Width / 2), cell.Y, Math.Min(72, cell.Width / 2), cell.Height);

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var radius = look.Radius("panel");
        var plain = look.Part("list", null);
        painter.Fill(Bounds, plain.Background ?? look.Color("input"), Framed ? radius : 0);
        var header = Header(look);
        using (painter.Clip(Inner(look)))
        {
            if (HasHeader)
                DrawHeader(painter, header);
            PlaceRows();
            Scroller.Draw(painter);
            if (Paged is { } paged)
                DrawPager(painter, paged);
        }
        if (Framed)
            painter.Stroke(Bounds, plain.Border ?? look.Color("strong_border"), look.Border, radius);
    }

    /// <summary>Only the rows in sight are drawn.</summary>
    protected override void DrawRows(Painter painter)
    {
        var shown = ShownPlaces;
        var height = RowHeight(painter.Look);
        var hover = PointerOver && pointerY is { } y && !Dragging ? RowAt(y)?.Place : null;
        var chosen = ChosenPlace;
        var (first, end) = InSight(height, shown.Count);
        for (var at = first; at < end; at++)
        {
            var row = RowOnScreen(at, height, height);
            DrawRow(painter, shown[at], row, shown[at] == chosen ? "selected" : shown[at] == hover ? "hover" : null);
            if (Drop is { } drop && drop.Target == shown[at])
                DrawDrop(painter, row, drop.Where);
        }
    }

    void DrawHeader(Painter painter, Box header)
    {
        var look = painter.Look;
        var shown = look.Part("header", null);
        painter.Fill(header, shown.Background ?? look.Color("raised"));
        painter.Fill(header with { Y = header.Bottom - look.Border, Height = look.Border }, look.Color("border"));
        var lefts = Lefts(header);
        var pad = look.Padding * 2;
        for (var c = 0; c < columns.Count; c++)
        {
            var column = columns[c];
            var cell = new Box(lefts[c].X, header.Y, lefts[c].Width, header.Height);
            var color = shown.Text ?? look.Color("muted");
            var text = cell with { X = cell.X + pad, Width = Math.Max(0, cell.Width - 2 * pad - 12) };
            painter.Text(column.Label, text, color, shown.Size ?? look.Size("text"), shown.Font ?? "text", shown.Bold);
            if (order is { } o && o.Field == column.Field)
            {
                var width = painter.Fonts.Measure(column.Label, shown.Font ?? "text", shown.Size ?? look.Size("text"), shown.Bold);
                var x = Math.Min(text.X + width + 6, cell.Right - 14);
                var middle = cell.Y + cell.Height / 2;
                Chevron(painter, x + 4, middle, o.Desc ? "down" : "up", color);
            }
            if (c > 0)
                painter.Fill(new Box(cell.X, cell.Y + 6, look.Border, cell.Height - 12), look.Color("border"));
        }
    }

    void DrawRow(Painter painter, int place, Box row, string? state)
    {
        var look = painter.Look;
        var text = DrawRowBack(painter, row, state).Text ?? look.Color("text");
        var lefts = Lefts(row);
        for (var c = 0; c < columns.Count; c++)
        {
            var cell = new Box(lefts[c].X, row.Y, lefts[c].Width, row.Height - look.Border);
            using (painter.Clip(cell))
                DrawCell(painter, place, columns[c], cell, text, c == 0);
        }
    }

    void DrawCell(Painter painter, int place, TableColumn column, Box cell, SKColor text, bool first)
    {
        var look = painter.Look;
        var pad = look.Padding * 2;
        var size = look.Size("text");
        if (first && IsTree && TreePlace(place) is { } at)
        {
            var indent = look.Padding + at.Depth * 16;
            if (at.Children)
                Chevron(painter, cell.X + indent + 10, cell.Y + cell.Height / 2, IsOpen(place) ? "down" : "right", look.Color("muted"));
            cell = cell with { X = cell.X + indent + 20 - pad + look.Padding, Width = Math.Max(0, cell.Width - indent - 20 + pad - look.Padding) };
        }
        var value = RowFields.Read(Records[place], column.Field);
        var inert = Locked;
        if (!column.Edit)
        {
            var align = column.Kind == CellKind.Number ? TextAlign.Right : TextAlign.Left;
            var lead = DrawSwatch(painter, place, column);
            painter.Text(Shown(place, column.Field), cell with { X = cell.X + pad + lead, Width = Math.Max(0, cell.Width - 2 * pad - lead) }, text, size, "text", false, align);
            return;
        }
        switch (column.Kind)
        {
            case CellKind.Bool when column.Switch:
                DrawSwitch(painter, cell, value is true, inert);
                return;
            case CellKind.Bool:
                DrawCheck(painter, cell, value is true, inert);
                return;
            case CellKind.Moment:
                var day = value is DateTime t ? Locale.Formats.Show(new FieldFormat(FormatKind.Date), DateOnly.FromDateTime(t)) : "";
                var time = TimeBox(cell);
                DrawInput(painter, new Box(cell.X, cell.Y, time.X - cell.X, cell.Height), day, place, column.Field, false, TextAlign.Left);
                DrawInput(painter, time, Shown(place, column.Field), place, column.Field, true, TextAlign.Center);
                return;
            case CellKind.Choice:
                DrawInput(painter, cell, Shown(place, column.Field), place, column.Field, false, TextAlign.Left);
                Chevron(painter, cell.Right - 14, cell.Y + cell.Height / 2, "down", look.Color("muted"));
                return;
            case CellKind.Color:
                DrawInput(painter, cell, Shown(place, column.Field), place, column.Field, true, TextAlign.Left, () => DrawSwatch(painter, place, column));
                return;
            default:
                DrawInput(painter, cell, Shown(place, column.Field), place, column.Field, true, column.Kind == CellKind.Number ? TextAlign.Right : TextAlign.Left);
                return;
        }
    }

    /// <summary>
    /// An input in a cell, in the theme's input look, marked when it holds text
    /// that is no value; the cell typed into shows its text editor there instead.
    /// </summary>
    void DrawInput(Painter painter, Box cell, string text, int place, string field, bool typed, TextAlign align, Func<double>? leading = null)
    {
        var look = painter.Look;
        var box = cell.Inset(3);
        if (typed && editing is { } e && e.Place == place && e.Field == field)
        {
            e.Editor.Arrange(box);
            e.Editor.Draw(painter);
            return;
        }
        var input = look.Part("input", null);
        var radius = input.Radius ?? look.Radius("input");
        painter.Fill(box, input.Background ?? look.Color("input"), radius);
        var border = typed && Marked(place, field) ? look.Color("error") : input.Border ?? look.Color("strong_border");
        painter.Stroke(box, border, look.Border, radius);
        // What goes before the text (a color's swatch) and the room it takes.
        var lead = leading?.Invoke() ?? 0;
        var inside = box with { X = box.X + look.Padding * 1.5 + lead, Width = Math.Max(0, box.Width - look.Padding * 3 - lead - (align == TextAlign.Left ? 12 : 0)) };
        painter.Text(text, inside, input.Text ?? look.Color("text"), look.Size("text"), "text", false, align);
    }

    /// <summary>Where an edited cell's input takes a press: a switch or a check with a little room around it, else the input inside the cell's margin.</summary>
    static Box InputOf(TableColumn column, Box cell)
    {
        if (column.Kind != CellKind.Bool)
            return cell.Inset(3);
        var (width, height) = column.Switch ? (32.0, 18.0) : (16.0, 16.0);
        const double room = 4;
        return new Box(cell.X + (cell.Width - width) / 2 - room, cell.Y + (cell.Height - height) / 2 - room, width + 2 * room, height + 2 * room);
    }

    void DrawCheck(Painter painter, Box cell, bool on, bool inert)
    {
        var look = painter.Look;
        var side = 16.0;
        var box = new Box(cell.X + (cell.Width - side) / 2, cell.Y + (cell.Height - side) / 2, side, side);
        var radius = Math.Min(4, look.Radius("input"));
        painter.Fill(box, on ? look.Color("accent") : look.Color("input"), radius);
        painter.Stroke(box, on ? look.Color("accent") : look.Color("strong_border"), look.Border, radius);
        if (on)
        {
            using var paint = new SKPaint { Color = look.Color("on_accent"), IsAntialias = true, StrokeWidth = 2, Style = SKPaintStyle.Stroke, StrokeCap = SKStrokeCap.Round };
            painter.Canvas.DrawLine((float)(box.X + 4), (float)(box.Y + 8.5), (float)(box.X + 7), (float)(box.Y + 11.5), paint);
            painter.Canvas.DrawLine((float)(box.X + 7), (float)(box.Y + 11.5), (float)(box.X + 12.5), (float)(box.Y + 5), paint);
        }
        if (inert)
            painter.Fill(box, look.Color("panel").WithAlpha(110), radius);
    }

    void DrawSwitch(Painter painter, Box cell, bool on, bool inert)
    {
        var look = painter.Look;
        var track = new Box(cell.X + (cell.Width - 32) / 2, cell.Y + (cell.Height - 18) / 2, 32, 18);
        painter.Fill(track, on ? look.Color("accent") : look.Color("strong_border"), 9);
        var knob = new Box(on ? track.Right - 16 : track.X + 2, track.Y + 2, 14, 14);
        painter.Fill(knob, on ? look.Color("on_accent") : look.Color("input"), 7);
        if (inert)
            painter.Fill(track, look.Color("panel").WithAlpha(110), 9);
    }

    void DrawPager(Painter painter, Paged paged)
    {
        var look = painter.Look;
        var bar = Pager(look);
        painter.Fill(bar with { Height = look.Border }, look.Color("border"));
        foreach (var button in Enum.GetValues<PageButton>())
        {
            var box = PagerBox(button)!.Value;
            var color = PageButtonWorks(button) ? look.Color("text") : look.Color("muted").WithAlpha(110);
            var (x, y) = (box.X + box.Width / 2, box.Y + box.Height / 2);
            switch (button)
            {
                case PageButton.First:
                    Chevron(painter, x + 2, y, "left", color);
                    Line(painter, x - 4, y - 5, x - 4, y + 5, color);
                    break;
                case PageButton.Previous:
                    Chevron(painter, x, y, "left", color);
                    break;
                case PageButton.Next:
                    Chevron(painter, x, y, "right", color);
                    break;
                default:
                    Chevron(painter, x - 2, y, "right", color);
                    Line(painter, x + 4, y - 5, x + 4, y + 5, color);
                    break;
            }
        }
        painter.Text(paged.Text, PagerText(look), look.Color("text"), look.Size("text"), "text", false, TextAlign.Center);
    }

    static void Line(Painter painter, double x0, double y0, double x1, double y1, SKColor color)
    {
        using var paint = new SKPaint { Color = color, IsAntialias = true, StrokeWidth = 2, Style = SKPaintStyle.Stroke, StrokeCap = SKStrokeCap.Round };
        painter.Canvas.DrawLine((float)x0, (float)y0, (float)x1, (float)y1, paint);
    }

    /// <summary>A small chevron pointing up, down, left or right, around a point.</summary>
    static void Chevron(Painter painter, double x, double y, string way, SKColor color)
    {
        const double S = 3.5;
        switch (way)
        {
            case "down":
                Line(painter, x - S, y - S / 2, x, y + S / 2, color);
                Line(painter, x, y + S / 2, x + S, y - S / 2, color);
                break;
            case "up":
                Line(painter, x - S, y + S / 2, x, y - S / 2, color);
                Line(painter, x, y - S / 2, x + S, y + S / 2, color);
                break;
            case "left":
                Line(painter, x + S / 2, y - S, x - S / 2, y, color);
                Line(painter, x - S / 2, y, x + S / 2, y + S, color);
                break;
            default:
                Line(painter, x - S / 2, y - S, x + S / 2, y, color);
                Line(painter, x + S / 2, y, x - S / 2, y + S, color);
                break;
        }
    }
}

/// <summary>The choices of a choice cell, as a list over the window: a press on one picks it.</summary>
public sealed class CellChoices : Element
{
    /// <summary>How many choices show at once; more scroll.</summary>
    public const int MostShown = 8;

    readonly object? value;
    readonly double itemHeight;
    readonly ChoiceRows rows;

    public CellChoices(IReadOnlyList<Choice> choices, object? value, double itemHeight)
    {
        Choices = choices;
        this.value = value;
        this.itemHeight = itemHeight;
        rows = new ChoiceRows(this);
        View = new ScrollView(rows) { Measure = (width, _) => (width, Choices.Count * itemHeight) };
        AddChild(View);
    }

    public IReadOnlyList<Choice> Choices { get; }

    /// <summary>Where its choices scroll.</summary>
    public ScrollView View { get; }

    /// <summary>A choice was picked, by its value.</summary>
    public Action<object?>? Picked { get; set; }

    /// <summary>How high it is for <paramref name="count"/> choices of <paramref name="itemHeight"/>: at most <see cref="MostShown"/>, with its border.</summary>
    public static double HeightFor(int count, double itemHeight) => Math.Min(MostShown, Math.Max(1, count)) * itemHeight + 2;

    /// <summary>The box of the choice at <paramref name="index"/> on the screen, scrolled into view first.</summary>
    public Box ItemBox(int index)
    {
        View.ScrollInto(new Box(0, index * itemHeight, 1, itemHeight));
        return rows.Item(index);
    }

    /// <summary>Where its value is among its choices; -1 when it is none of them.</summary>
    public int Marked
    {
        get
        {
            for (var i = 0; i < Choices.Count; i++)
                if (RowsPart.Same(Choices[i].Value, value))
                    return i;
            return -1;
        }
    }

    bool revealed;

    public override void Arrange(Box box)
    {
        Bounds = box;
        View.Arrange(box.Inset(1));
        // It opens with its value's choice in view, as a dropdown's list does.
        if (!revealed && box.Height > 0 && Marked is var marked and >= 0)
        {
            revealed = true;
            View.ScrollInto(new Box(0, marked * itemHeight, 1, itemHeight));
        }
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var menu = look.Part("menu", null);
        var radius = menu.Radius ?? look.Radius("input");
        painter.Fill(Bounds, menu.Background ?? look.Color("panel"), radius);
        base.Draw(painter);
        painter.Stroke(Bounds, menu.Border ?? look.Color("border"), look.Border, radius);
    }

    /// <summary>The choices one under the other, as tall as all of them, which the view scrolls.</summary>
    sealed class ChoiceRows(CellChoices list) : Element
    {
        int? pressedAt;
        double? pointerY;

        public Box Item(int index) => new(Bounds.X, Bounds.Y + index * list.itemHeight, Bounds.Width, list.itemHeight);

        int? At(double y)
        {
            var at = (int)Math.Floor((y - Bounds.Y) / list.itemHeight);
            return at >= 0 && at < list.Choices.Count ? at : null;
        }

        public override void OnPointerPressed(PointerInput at) => pressedAt = At(at.ScreenY);

        public override void OnPointerMoved(PointerInput at)
        {
            pointerY = at.ScreenY;
            Invalidate();
        }

        public override void OnClick()
        {
            if (pressedAt is { } at)
                list.Picked?.Invoke(list.Choices[at].Value);
        }

        public override void Draw(Painter painter)
        {
            var look = painter.Look;
            var menu = look.Part("menu", null);
            var hover = Hovered && pointerY is { } y ? At(y) : null;
            var view = list.View.Bounds;
            var first = Math.Max(0, (int)Math.Floor((view.Y - Bounds.Y) / list.itemHeight));
            var last = Math.Min(list.Choices.Count - 1, (int)Math.Ceiling((view.Bottom - Bounds.Y) / list.itemHeight));
            for (var i = first; i <= last; i++)
            {
                var item = Item(i);
                var state = i == hover ? look.Part("menu", "hover") : null;
                if (RowsPart.Same(list.Choices[i].Value, list.value))
                    painter.Fill(item, look.Color("selection"));
                else if (state is not null)
                    painter.Fill(item, state.Background ?? look.Color("accent_soft"));
                painter.Text(list.Choices[i].Label, item with { X = item.X + look.Padding * 2, Width = Math.Max(0, item.Width - look.Padding * 4) }, (state?.Text ?? menu.Text) ?? look.Color("text"), look.Size("text"));
            }
        }
    }
}
