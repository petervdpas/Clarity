// A list of records (U1c): each row shows the fields of `show`, the values
// in the person's format, in the theme's list look (hover, selected); or with
// `cards` each row is a card in the card look, the first field its title
// and the others lines below it in the card_line look. The person chooses a
// row with the pointer or Up and Down; a double click opens it. Its rows
// scroll in the shared scroll view (RowsPart.Scroller), and it moves rows by
// hand with `reorder by` (DESIGN 5.30). A text longer than its row ends in
// dots, and the rows leave room for the bar while they scroll.

using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("list")]
public sealed class ListPart : RowsPart
{
    Dictionary<string, IReadOnlyList<Choice>> choices = [];
    (int Place, bool Moved)? pressed;
    double? pointerY;

    bool Cards => Spec.Flag("cards");

    IReadOnlyList<string> Show => Spec.Names("show");

    protected override void RowsRead() =>
        choices = Show.Distinct().ToDictionary(f => f, ChoicesOf);

    /// <summary>What a row shows: the fields of <c>show</c> in the person's format, between bars.</summary>
    public string TextOf(object record) => string.Join(" | ", Lines(record));

    IEnumerable<string> Lines(object record) => Show.Select(f => ShownOf(RowFields.Read(record, f), choices.GetValueOrDefault(f)));

    double LineHeight(ThemeLook look, string size) => Math.Ceiling(look.Size(size) * 1.3);

    /// <summary>A row's height, and the space below it.</summary>
    (double Height, double Gap) RowSize(ThemeLook look)
    {
        var pad = look.Padding;
        if (!Cards)
            return (LineHeight(look, "text") + 2 * (pad - 1) + look.Border, 0);
        var lines = Math.Max(0, Show.Count - 1);
        return (4 * pad + LineHeight(look, "text") + lines * LineHeight(look, "small") + 2 * look.Border, Math.Round(pad * 1.5));
    }

    /// <summary>Where the rows show: inside its border and padding; a list of cards has neither.</summary>
    Box View(ThemeLook look) => Cards ? Bounds : Bounds.Inset((Framed ? look.Border : 0) + look.Padding);

    protected override Box RowsBox(ThemeLook look) => View(look);

    protected override double RowsHeight(ThemeLook look)
    {
        var (height, gap) = RowSize(look);
        return Records.Count * (height + gap);
    }

    /// <summary>The box of the row of the record at <paramref name="place"/> on the screen; none while it is scrolled out of sight.</summary>
    public Box? RowBox(int place)
    {
        var (height, gap) = RowSize(LookNow);
        return place < 0 || place >= Records.Count ? null : RowInSight(place, height + gap, height) is { } row ? Beside(row) : null;
    }

    /// <summary>A row narrowed by the bar while the rows scroll, so the bar covers none of it.</summary>
    Box Beside(Box row) => Scroller.MaxScrollY > 0 ? row with { Width = Math.Max(0, row.Width - ScrollView.BarThickness) } : row;

    protected override (int Place, Box Row)? RowAt(double screenY)
    {
        var (height, gap) = RowSize(LookNow);
        return IndexAt(screenY, height + gap, Records.Count) is { } place && RowBox(place) is { } box ? (place, box) : null;
    }

    void ShowRow(int place)
    {
        var (height, gap) = RowSize(LookNow);
        ShowIndex(place, height + gap, height);
    }

    protected override void Shown(object record)
    {
        if (PlaceOf(record) is var place and >= 0)
            ShowRow(place);
    }

    public override void OnPointerPressed(PointerInput at)
    {
        if (!Enabled || RowAt(at.ScreenY) is not { } row)
        {
            pressed = null;
            return;
        }
        pressed = (row.Place, false);
        PressRow(row.Place, at);
        if (at.Clicks == 2)
        {
            Choose(row.Place);
            Open();
            pressed = null;
        }
    }

    public override void OnPointerMoved(PointerInput at)
    {
        pointerY = at.ScreenY;
        if (Holding)
            DragTo(at);
        Invalidate();
    }

    public override void OnPointerReleased(PointerInput at)
    {
        var was = pressed;
        pressed = null;
        if (DropRow() || was is not { } press)
            return;
        if (RowAt(at.ScreenY) is { } row && row.Place == press.Place)
            Choose(row.Place);
    }

    public override bool OnKey(KeyInput key)
    {
        if (MoveKeyed(key))
            return true;
        if (!Enabled || Records.Count == 0 || key.Modifiers != Modifiers.None)
            return false;
        var look = LookNow;
        var (height, gap) = RowSize(look);
        var page = Math.Max(1, (int)(View(look).Height / (height + gap)));
        var at = ChosenPlace;
        if (key.Key == Key.Enter && at >= 0)
        {
            Open();
            return true;
        }
        if (MovedTo(key.Key, at, Records.Count, page) is not { } place)
            return false;
        Choose(place);
        ShowRow(place);
        Invalidate();
        return true;
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var list = look.Part("list", null);
        var radius = list.Radius ?? look.Radius("panel");
        if (!Cards)
            painter.Fill(Bounds, list.Background ?? look.Color("input"), radius);
        PlaceRows();
        Scroller.Draw(painter);
        if (Framed && !Cards)
            painter.Stroke(Bounds, list.Border ?? look.Color("strong_border"), look.Border, radius);
        if (Focused && !Cards)
            painter.Stroke(Bounds, look.Color("focus"), 2, radius);
    }

    /// <summary>Only the rows in sight are drawn.</summary>
    protected override void DrawRows(Painter painter)
    {
        var look = painter.Look;
        var hover = PointerOver && pointerY is { } y && !Dragging ? RowAt(y)?.Place : null;
        var chosen = ChosenPlace;
        var (height, gap) = RowSize(look);
        var (first, end) = InSight(height + gap, Records.Count);
        for (var place = first; place < end; place++)
        {
            var row = Beside(RowOnScreen(place, height + gap, height));
            var state = place == chosen ? "selected" : place == hover ? "hover" : null;
            if (Cards)
                DrawCard(painter, Records[place], row, state);
            else
                DrawRow(painter, Records[place], row, state);
            if (Drop is { } drop && drop.Target == place)
                DrawDrop(painter, row, drop.Where);
        }
    }

    void DrawRow(Painter painter, object record, Box row, string? state)
    {
        var look = painter.Look;
        var shown = DrawRowBack(painter, row, state);
        var pad = look.Padding;
        var text = row with { X = row.X + pad * 1.5, Width = Math.Max(0, row.Width - pad * 3), Height = row.Height - look.Border };
        painter.Text(PartTexts.Fit(painter, TextOf(record), text.Width, shown.Size ?? look.Size("text"), shown.Font ?? "text", shown.Bold), text, shown.Text ?? look.Color("text"), shown.Size ?? look.Size("text"), shown.Font is null ? "text" : shown.Font, shown.Bold);
    }

    void DrawCard(Painter painter, object record, Box card, string? state)
    {
        var look = painter.Look;
        var plain = look.Part("card", null);
        var shown = state is null ? plain : look.Part("card", state);
        var radius = shown.Radius ?? look.Radius("panel");
        painter.Fill(card, shown.Background ?? look.Color("raised"), radius);
        painter.Stroke(card, shown.Border ?? SkiaSharp.SKColors.Transparent, look.Border, radius);
        var line = state is "selected" ? look.Part("card_line", "selected") : state is "hover" ? look.Part("card_line", "hover") : look.Part("card_line", null);
        var pad = look.Padding;
        var x = card.X + pad * 2.5;
        var width = Math.Max(0, card.Width - pad * 5);
        var y = card.Y + 2 * pad + look.Border;
        var at = 0;
        foreach (var text in Lines(record))
        {
            if (at == 0)
            {
                var height = LineHeight(look, "text");
                var size = shown.Size ?? look.Size("text");
                painter.Text(PartTexts.Fit(painter, text, width, size, "text", shown.Bold), new Box(x, y, width, height), shown.Text ?? look.Color("text"), size, "text", shown.Bold);
                y += height;
            }
            else
            {
                var height = LineHeight(look, "small");
                painter.Text(PartTexts.Fit(painter, text, width, look.Size("small"), "text", line.Bold), new Box(x, y, width, height), line.Text ?? shown.Text ?? look.Color("text"), look.Size("small"), "text", line.Bold);
                y += height;
            }
            at++;
        }
    }
}
