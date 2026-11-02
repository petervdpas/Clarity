// How the person reaches every cell of a table, beside the pointer, and what
// the table keeps and says: the check or choice the keys work on (Tab walks
// every kind of edited cell, Space ticks a check, a choice's list opens with
// Enter, Space or Alt+Down and is worked with the keys), a date's calendar
// and a color's palette, the widths the person gave the columns kept for
// the next start per window and table, and what the page bar's buttons and
// a tree's toggles say where the pointer rests.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed partial class TablePart
{
    (int Place, string Field)? focusedCell;

    /// <summary>The cell the keys work on now: the one typed into, or a check or choice reached with Tab or a press; none for none.</summary>
    public (int Place, string Field)? FocusedCell => editing is { } e ? (e.Place, e.Field) : focusedCell;

    /// <summary>While a cell has the keys, Tab goes on to the next cell, and from the last out of the table.</summary>
    public override bool WantsTab => focusedCell is not null || OpenChoices is not null;

    /// <summary>A check or choice becomes the cell the keys work on: the table has the focus and the cell is in sight.</summary>
    void FocusCell(int place, string field)
    {
        focusedCell = (place, field);
        RevealCell(place, field);
        Screen?.Focus(this);
        Invalidate();
    }

    /// <summary>Scrolls as little as it takes, down and across, to show the cell of <paramref name="field"/> in the row of <paramref name="place"/>.</summary>
    void RevealCell(int place, string field)
    {
        var at = ShownPlaces.ToList().IndexOf(place);
        var c = columns.FindIndex(c => c.Field == field);
        if (at < 0 || c < 0)
            return;
        var height = RowHeight(LookNow);
        PlaceRows();
        var (x, width) = Lefts(default)[c];
        Scroller.ScrollInto(new Box(x, at * height, width, height));
    }

    /// <summary>
    /// A key for an open list or calendar, or for the check or choice the keys
    /// work on; true when it took it. Another key leaves the cell and works on
    /// the rows.
    /// </summary>
    bool CellKeyed(KeyInput key)
    {
        if (!Enabled)
            return false;
        if (OpenChoices is { } list)
        {
            switch (key.Key)
            {
                case Key.Down when key.Modifiers == Modifiers.None:
                    list.Move(1);
                    return true;
                case Key.Up when key.Modifiers == Modifiers.Alt:
                    CloseChoices();
                    return true;
                case Key.Up:
                    list.Move(-1);
                    return true;
                case Key.PageDown:
                    list.Move(CellChoices.MostShown);
                    return true;
                case Key.PageUp:
                    list.Move(-CellChoices.MostShown);
                    return true;
                case Key.Home:
                    list.Move(-list.Choices.Count);
                    return true;
                case Key.End:
                    list.Move(list.Choices.Count);
                    return true;
                case Key.Enter or Key.Space:
                    list.PickMarked();
                    return true;
                case Key.Escape:
                    CloseChoices();
                    return true;
                case Key.Tab:
                    CloseChoices();
                    break;
                default:
                    return true;
            }
        }
        if (OpenCalendar is { } calendar && calendar.Keyed(key))
            return true;
        if (focusedCell is not { } cell)
            return false;
        if (columns.FirstOrDefault(c => c.Field == cell.Field) is not { } column || cell.Place >= Records.Count)
        {
            focusedCell = null;
            return false;
        }
        if (key.Key == Key.Tab && key.Modifiers is Modifiers.None or Modifiers.Shift)
        {
            // Past the last cell the table lets Tab move the focus on.
            if (!TypeOn(cell.Place, cell.Field, key.Shift))
                Screen?.KeyPressed(key);
            return true;
        }
        if (column.Kind == CellKind.Bool && key.Key == Key.Space && key.Modifiers == Modifiers.None)
        {
            Edit(cell.Place, column.Field, RowFields.Read(Records[cell.Place], column.Field) is not true);
            return true;
        }
        if (column.Kind == CellKind.Choice
            && ((key.Key is Key.Enter or Key.Space && key.Modifiers == Modifiers.None) || (key.Key == Key.Down && key.Modifiers == Modifiers.Alt)))
        {
            OpenChoicesOf(cell.Place, column);
            return true;
        }
        focusedCell = null;
        Invalidate();
        return key.Key == Key.Escape && key.Modifiers == Modifiers.None;
    }

    void CloseChoices()
    {
        if (choicesOverlay is { } open)
            Screen?.Close(open);
        Invalidate();
    }

    /// <summary>Letters typed while a choice cell's list is open go to the choice that starts with them.</summary>
    public override void OnText(string text)
    {
        if (OpenChoices is { } list)
            list.Seek(text, Screen?.Now() ?? Clock.Now());
    }

    // -- A date's calendar, a color's palette ---------------------------------

    const double CalendarButtonWidth = 24;

    /// <summary>The button at the right of a date cell's input, which opens its calendar.</summary>
    static Box CalendarButton(Box cell) => new(cell.Right - 3 - CalendarButtonWidth, cell.Y + 3, CalendarButtonWidth, Math.Max(0, cell.Height - 6));

    /// <summary>The button of an edited date cell that opens its calendar, while it is in sight.</summary>
    public Box? CalendarButtonBox(int place, string field) =>
        columns.FirstOrDefault(c => c.Field == field) is { Kind: CellKind.Date, Edit: true } && CellBox(place, field) is { } cell ? CalendarButton(cell) : null;

    /// <summary>The swatch at the left of an edited color cell's input, with a little room around it: a press there opens the palette.</summary>
    Box PaletteButton(Box cell)
    {
        var look = LookNow;
        return new Box(cell.X + 3, cell.Y + 3, look.Padding * 2.5 + SwatchSide, Math.Max(0, cell.Height - 6));
    }

    /// <summary>
    /// Opens the palette the color part has under an edited color cell: the
    /// color picked is its edit. The table keeps the focus.
    /// </summary>
    void OpenPalette(int place, TableColumn column)
    {
        RevealCell(place, column.Field);
        if (Screen is not { } screen || Locked || CellBox(place, column.Field) is not { } cell)
            return;
        var field = column.Field;
        Overlay? overlay = null;
        var grid = new PaletteGrid(RowFields.Read(Records[place], field) as string, color =>
        {
            if (overlay is not null)
                screen.Close(overlay);
            marked.Remove((place, field));
            Edit(place, field, color);
        })
        { Holder = this };
        var x = Math.Max(0, Math.Min(cell.X, screen.Width - PaletteGrid.Width));
        var below = cell.Bottom + 2;
        var y = below + PaletteGrid.Height > screen.Height && cell.Y - 2 - PaletteGrid.Height >= 0 ? cell.Y - 2 - PaletteGrid.Height : below;
        overlay = new Overlay(grid, new Box(x, y, PaletteGrid.Width, PaletteGrid.Height));
        screen.Open(overlay);
        screen.Focus(this);
    }

    // -- The person's widths, kept per window and table -------------------------

    bool widthsRead;

    /// <summary>
    /// Where the person's widths are kept: <c>columns.Main.grid</c> for the
    /// resizable table grid of the window Main; none for a table without a
    /// name or outside a named window. Window and table names are the
    /// project's own, an open set, so the key is made from them.
    /// </summary>
    string? RememberKey()
    {
        if (!Resizable || Spec.Name is not { Length: > 0 } name)
            return null;
        for (var e = Parent; e is not null; e = e.Parent)
            if (e is WindowView { Spec.Name: { Length: > 0 } window })
                return $"columns.{window}.{name}";
        return null;
    }

    /// <summary>The widths the person left before, read once the table knows its window.</summary>
    void ReadRememberedWidths()
    {
        if (widthsRead)
            return;
        if (RememberKey() is not { } key)
        {
            // On a screen and still no window: there is nothing to read.
            widthsRead = Screen is not null;
            return;
        }
        widthsRead = true;
        if (Remembered.Get(key) is not { Length: > 0 } text)
            return;
        try
        {
            foreach (var (field, width) in JsonSerializer.Deserialize<Dictionary<string, double>>(text) ?? [])
                if (width > 0)
                    personWidths[field] = Math.Clamp(width, 32, 2000);
        }
        catch (JsonException)
        {
            // Damaged: the designed widths.
        }
    }

    /// <summary>Each column's width as it shows, by field, as they are kept.</summary>
    string WidthsWritten()
    {
        var lefts = Lefts(default);
        var widths = new Dictionary<string, double>();
        for (var c = 0; c < columns.Count; c++)
            widths[columns[c].Field] = Math.Round(lefts[c].Width);
        return JsonSerializer.Serialize(widths);
    }

    /// <summary>The person let go of a header edge: the widths they left are kept, when they changed.</summary>
    void KeepWidths(string before)
    {
        var now = WidthsWritten();
        if (now != before && RememberKey() is { } key)
            Remembered.Set(key, now);
    }

    /// <summary>Back to the designed widths, forgetting the person's.</summary>
    public void DesignedWidths()
    {
        widthsRead = true;
        personWidths.Clear();
        if (RememberKey() is { } key)
            Remembered.Set(key, "");
        PlaceRows();
        Invalidate();
    }

    // -- What it says and the pointer's shape ----------------------------------

    PageButton? PagerButtonAt(double x, double y)
    {
        if (Paged is null)
            return null;
        foreach (var button in Enum.GetValues<PageButton>())
            if (PagerBox(button) is { } box && box.Contains(x, y))
                return button;
        return null;
    }

    int? ToggleAt(double x, double y) =>
        IsTree && Scroller.Bounds.Contains(x, y) && RowAt(y) is { } row && ToggleBox(row.Place) is { } toggle && toggle.Contains(x, y) ? row.Place : null;

    bool OnHeaderEdge(double x, double y) => EdgeColumnAt(x, y) is not null;

    /// <summary>The column whose right edge a point of the header is on, a few pixels either side of the line; none elsewhere or when the person may not resize.</summary>
    string? EdgeColumnAt(double x, double y) =>
        Resizable && HasHeader && Header(LookNow).Contains(x, y)
            ? columns.Select(c => (c.Field, Box: HeaderBox(c.Field))).FirstOrDefault(c => c.Box is { } box && y >= box.Y && y <= box.Bottom && Math.Abs(x - box.Right) <= Edge).Field
            : null;

    /// <summary>The page bar's buttons and a tree row's toggle say what a press does, in Clarity's words.</summary>
    public override string? TooltipAt(double x, double y) =>
        PagerButtonAt(x, y) is { } button ? PageWord(button)
        : ToggleAt(x, y) is { } place ? ToggleWord(place)
        : base.TooltipAt(x, y);

    /// <summary>A right press on a row chooses it first, as a click does, so its menu is about that row.</summary>
    public override IReadOnlyList<MenuEntry>? ContextMenuAt(double x, double y)
    {
        if (Enabled && Scroller.Bounds.Contains(x, y) && RowAt(y) is { } row)
        {
            Take();
            Choose(row.Place);
        }
        return base.ContextMenuAt(x, y);
    }

    /// <summary>A header edge resizes; a page button that works and a toggle are pressed.</summary>
    public override Cursor? CursorAt(double x, double y)
    {
        if (press is OnEdge)
            return Cursor.ResizeLeftRight;
        if (!Enabled)
            return null;
        if (PagerButtonAt(x, y) is { } button)
            return PageButtonWorks(button) ? Cursor.Hand : null;
        if (OnHeaderEdge(x, y))
            return Cursor.ResizeLeftRight;
        return ToggleAt(x, y) is not null ? Cursor.Hand : base.CursorAt(x, y);
    }
}
