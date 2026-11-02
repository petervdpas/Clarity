// A scrolling place: content taller or wider than its place scrolls with the
// wheel and a thin bar the person drags, and shows only inside its place.

namespace Clarity.Ui.Tests.Containers;

public class ScrollViewTests
{
    static (Screen Screen, ScrollView View, Plate Content) Tall(double width = 300, double height = 1000, double placeWidth = 200, double placeHeight = 200)
    {
        var content = new Plate("accent");
        var view = new ScrollView(content) { Measure = (_, _) => (width, height) };
        var grid = new GridPanel(columns: 1);
        grid.Add(view, new GridItem(1, 1, 1, 1, true));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(placeWidth, placeHeight, 1);
        return (screen, view, content);
    }

    [Fact]
    public void Content_taller_than_its_place_scrolls_with_the_wheel_and_stops_at_its_ends()
    {
        var (screen, view, content) = Tall();
        screen.Wheel(50, 50, 0, 40);
        Assert.Equal(40, view.ScrollY);
        Assert.Equal(-40, content.Bounds.Y);
        Assert.Equal(1000, content.Bounds.Height);
        screen.Wheel(50, 50, 0, 5000);
        Assert.Equal(800, view.ScrollY);
        Assert.Equal(800, view.MaxScrollY);
        // At its end it takes no more, so what is around it may scroll.
        Assert.False(view.OnWheel(0, 40));
        screen.Wheel(50, 50, 0, -5000);
        Assert.Equal(0, view.ScrollY);
        Assert.False(view.OnWheel(0, -40));
    }

    [Fact]
    public void Content_that_fits_does_not_scroll_and_shows_no_bar()
    {
        var (screen, view, _) = Tall(width: 200, height: 100);
        Assert.False(view.OnWheel(0, 40));
        Assert.Equal(0, view.ScrollY);
        Assert.All(view.Children.Skip(1), bar => Assert.False(bar.IsShown));
        Assert.Same(view.Content, screen.At(195, 50));
    }

    [Fact]
    public void Content_wider_than_its_place_scrolls_across()
    {
        var (screen, view, content) = Tall(width: 500, height: 100);
        screen.Wheel(50, 50, 30, 0);
        Assert.Equal(30, view.ScrollX);
        Assert.Equal(-30, content.Bounds.X);
        Assert.Equal(300, view.MaxScrollX);
    }

    [Fact]
    public void Dragging_the_bar_scrolls_as_far_as_the_bar_went()
    {
        var (screen, view, _) = Tall(width: 200);
        // The bar is at the right edge; its thumb is a fifth of it (200 of 1000) at the top.
        var bar = screen.At(196, 10);
        Assert.NotNull(bar);
        Assert.NotSame(view.Content, bar);
        screen.PointerPressed(196, 10, PointerButton.Left);
        screen.PointerMoved(196, 90);
        screen.PointerReleased(196, 90, PointerButton.Left);
        // 80 of the 160 the thumb can move is half of the 800 the content can.
        Assert.Equal(400, view.ScrollY, 3);
        // A press on the track below the thumb pages down.
        screen.PointerPressed(196, 190, PointerButton.Left);
        screen.PointerReleased(196, 190, PointerButton.Left);
        Assert.Equal(600, view.ScrollY, 3);
    }

    [Fact]
    public void It_shows_its_content_only_inside_its_place()
    {
        var content = new Plate("accent");
        var view = new ScrollView(content) { Measure = (w, _) => (w, 1000) };
        var grid = new GridPanel(columns: 1);
        grid.Add(view, new GridItem(1, 1, 1, 1, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        using var pixels = Shot.Of(screen, 200, 200);
        Assert.Equal(screen.Look.Color("accent"), pixels.GetPixel(20, 20));
        // Its place is one row high: below it the window shows.
        Assert.Equal(screen.Look.Color("background"), pixels.GetPixel(20, 100));
    }

    [Fact]
    public void A_box_of_the_content_scrolls_into_view()
    {
        var (_, view, _) = Tall();
        view.ScrollInto(new Box(0, 500, 10, 50));
        Assert.Equal(350, view.ScrollY);
        view.ScrollInto(new Box(0, 100, 10, 50));
        Assert.Equal(100, view.ScrollY);
        // Already in view, it stays.
        view.ScrollInto(new Box(0, 150, 10, 50));
        Assert.Equal(100, view.ScrollY);
    }

    [Fact]
    public void A_grid_scrolls_by_its_own_height()
    {
        var inner = new GridPanel(columns: 1);
        for (var row = 1; row <= 10; row++)
            inner.Add(new Plate("panel"), new GridItem(1, row, 1, 1, false));
        var view = new ScrollView(inner);
        var grid = new GridPanel(columns: 1);
        grid.Add(view, new GridItem(1, 1, 1, 3, true));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(300, 124, 1);
        // Ten rows of 36 with 8 between them.
        Assert.Equal(10 * 36 + 9 * 8 - 124, view.MaxScrollY);
    }
}
