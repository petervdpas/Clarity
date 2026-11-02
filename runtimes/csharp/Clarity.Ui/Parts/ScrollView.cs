// A place that scrolls: content taller or wider than the place moves with
// the wheel and with a thin bar the person drags or presses, and shows only
// inside the place. Parts reuse it for whatever needs to scroll: a scroll
// box's grid, a list's rows, a long text. The wheel's turn is in the
// screen's units, down and right positive, as a browser's wheel event gives
// it; at an end it takes no more, so what is around it may scroll on.

namespace Clarity.Ui;

public sealed class ScrollView : Element
{
    /// <summary>How thick a bar is, from the edge of the place.</summary>
    public const double BarThickness = 8;

    const double LeastThumb = 24;

    readonly ScrollBar down;
    readonly ScrollBar across;
    double scrollX;
    double scrollY;

    public ScrollView(Element content)
    {
        Content = content;
        down = new ScrollBar(this, vertical: true);
        across = new ScrollBar(this, vertical: false);
        AddChild(content);
        AddChild(down);
        AddChild(across);
    }

    public Element Content { get; }

    /// <summary>
    /// How wide and high the content is for a place of a width and height.
    /// Without one, a grid is as high as its rows and anything else is as big as the place.
    /// </summary>
    public Func<double, double, (double Width, double Height)>? Measure { get; init; }

    /// <summary>Whether the content is made narrower by the bar while the bar down shows, so the bar covers none of it.</summary>
    public bool KeepsBarRoom { get; init; }

    /// <summary>How wide and high the content was when it was last placed.</summary>
    public double ContentWidth { get; private set; }
    public double ContentHeight { get; private set; }

    public double MaxScrollX => Math.Max(0, ContentWidth - Bounds.Width);
    public double MaxScrollY => Math.Max(0, ContentHeight - Bounds.Height);

    /// <summary>How far the content is scrolled across, from 0 to <see cref="MaxScrollX"/>.</summary>
    public double ScrollX
    {
        get => scrollX;
        set => ScrollTo(value, scrollY);
    }

    /// <summary>How far the content is scrolled down, from 0 to <see cref="MaxScrollY"/>.</summary>
    public double ScrollY
    {
        get => scrollY;
        set => ScrollTo(scrollX, value);
    }

    /// <summary>Scrolled somewhere else: what scrolls hears it, as a list that shows only the rows in view.</summary>
    public event Action? Scrolled;

    void ScrollTo(double x, double y)
    {
        x = Math.Clamp(x, 0, MaxScrollX);
        y = Math.Clamp(y, 0, MaxScrollY);
        if (x == scrollX && y == scrollY)
            return;
        scrollX = x;
        scrollY = y;
        Place();
        Scrolled?.Invoke();
        Invalidate();
    }

    /// <summary>Scrolls as little as it takes to show <paramref name="box"/>, a place in the content from its top left.</summary>
    public void ScrollInto(Box box)
    {
        var y = scrollY;
        if (box.Y < y)
            y = box.Y;
        else if (box.Bottom > y + Bounds.Height)
            y = box.Bottom - Bounds.Height;
        var x = scrollX;
        if (box.X < x)
            x = box.X;
        else if (box.Right > x + Bounds.Width)
            x = box.Right - Bounds.Width;
        ScrollTo(x, y);
    }

    (double Width, double Height) Measured(double width, double height)
    {
        if (Measure is { } measure)
            return measure(width, height);
        if (Content is GridPanel grid)
        {
            grid.Arrange(new Box(0, 0, width, height));
            return (width, Math.Max(height, grid.Content));
        }
        return (width, height);
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var (width, height) = Measured(box.Width, box.Height);
        var room = box.Width;
        if (KeepsBarRoom && height > box.Height)
        {
            room = Math.Max(0, box.Width - BarThickness);
            (width, height) = Measured(room, box.Height);
        }
        ContentWidth = Math.Max(width, room);
        ContentHeight = Math.Max(height, box.Height);
        scrollX = Math.Clamp(scrollX, 0, MaxScrollX);
        scrollY = Math.Clamp(scrollY, 0, MaxScrollY);
        Place();
    }

    void Place()
    {
        var box = Bounds;
        Content.Arrange(new Box(box.X - scrollX, box.Y - scrollY, ContentWidth, ContentHeight));
        down.Arrange(new Box(box.Right - BarThickness, box.Y, BarThickness, box.Height - (across.IsShown ? BarThickness : 0)));
        across.Arrange(new Box(box.X, box.Bottom - BarThickness, box.Width - (down.IsShown ? BarThickness : 0), BarThickness));
    }

    public override bool OnWheel(double dx, double dy)
    {
        var (x, y) = (scrollX, scrollY);
        ScrollTo(scrollX + dx, scrollY + dy);
        return x != scrollX || y != scrollY;
    }

    public override void Draw(Painter painter)
    {
        using (painter.Clip(Bounds))
            base.Draw(painter);
    }

    /// <summary>A bar at an edge: its thumb shows how much of the content is in view and where, and moves it.</summary>
    sealed class ScrollBar(ScrollView view, bool vertical) : Element
    {
        double grab;

        public override bool IsShown => vertical ? view.MaxScrollY > 0 : view.MaxScrollX > 0;

        double Track => vertical ? Bounds.Height : Bounds.Width;
        double Content => vertical ? view.ContentHeight : view.ContentWidth;
        double Max => vertical ? view.MaxScrollY : view.MaxScrollX;
        double Scroll => vertical ? view.scrollY : view.scrollX;

        double ThumbLength => Content <= 0 ? Track : Math.Min(Track, Math.Max(LeastThumb, Track * Track / Content));

        double ThumbAt => Max <= 0 ? 0 : (Track - ThumbLength) * Scroll / Max;

        void MoveThumbTo(double at)
        {
            var travel = Track - ThumbLength;
            var scroll = travel <= 0 ? 0 : at / travel * Max;
            if (vertical)
                view.ScrollY = scroll;
            else
                view.ScrollX = scroll;
        }

        public override void OnPointerPressed(PointerInput at)
        {
            var along = vertical ? at.Y : at.X;
            if (along >= ThumbAt && along < ThumbAt + ThumbLength)
            {
                grab = along - ThumbAt;
                return;
            }
            // A press beside the thumb moves a place's length that way.
            var page = vertical ? view.Bounds.Height : view.Bounds.Width;
            var to = Scroll + (along < ThumbAt ? -page : page);
            if (vertical)
                view.ScrollY = to;
            else
                view.ScrollX = to;
            grab = double.NaN;
        }

        public override void OnPointerMoved(PointerInput at)
        {
            if (!Pressed || double.IsNaN(grab))
                return;
            MoveThumbTo((vertical ? at.Y : at.X) - grab);
        }

        public override void Draw(Painter painter)
        {
            if (!IsShown)
                return;
            var thumb = vertical
                ? new Box(Bounds.X + 2, Bounds.Y + ThumbAt, Bounds.Width - 4, ThumbLength)
                : new Box(Bounds.X + ThumbAt, Bounds.Y + 2, ThumbLength, Bounds.Height - 4);
            var color = Pressed || Hovered ? "muted" : "strong_border";
            painter.Fill(thumb, painter.Look.Color(color), (vertical ? thumb.Width : thumb.Height) / 2);
        }
    }
}
