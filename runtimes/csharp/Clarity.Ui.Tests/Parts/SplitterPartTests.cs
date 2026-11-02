// A splitter: panes side by side, `pane span n` giving the first of two n of
// the part's columns, and a divider the person drags to share the room anew.
// Also the scroll box, whose grid scrolls in its own place.

namespace Clarity.Ui.Tests.Containers;

public class SplitterPartTests
{
    const string Splitter = """
        <Splitter name="split" at="1,1" span="12" rows="6">
          <Pane span="4"><Label at="1,1" span="4" /></Pane>
          <Pane><Label at="1,1" span="8" /></Pane>
        </Splitter>
        """;

    [Fact]
    public void A_pane_span_gives_the_first_pane_its_columns_and_the_second_the_rest()
    {
        var (_, _, parts) = Made.Window(Splitter, new TestHost(), width: 808);
        var splitter = Assert.IsType<SplitterPart>(parts[0]);
        Assert.Equal([4, 8], splitter.Panes.Select(p => p.Columns));
        Assert.Equal(1 / 3.0, splitter.Shares[0], 6);
        // The first pane is a third of the room beside the divider.
        Assert.Equal((808 - SplitterPart.DividerThickness) / 3, splitter.Dividers[0].Bounds.X, 3);
    }

    [Fact]
    public void Without_a_span_or_one_above_the_other_each_pane_has_all_the_columns()
    {
        Assert.Equal([12, 12], SplitterPart.PaneColumns([new PageSpec("pane", null, null, []), new PageSpec("pane", null, null, [])], 12, vertical: false));
        Assert.Equal([12, 12], SplitterPart.PaneColumns([new PageSpec("pane", null, 4, []), new PageSpec("pane", null, null, [])], 12, vertical: true));
        Assert.Equal([12, 12, 12], SplitterPart.PaneColumns([new PageSpec("pane", null, 4, []), new PageSpec("pane", null, null, []), new PageSpec("pane", null, null, [])], 12, vertical: false));
        var (_, _, parts) = Made.Window(Splitter.Replace("rows=\"6\"", "rows=\"6\" orientation=\"vertical\""), new TestHost(), width: 808);
        var splitter = (SplitterPart)parts[0];
        Assert.True(splitter.Vertical);
        Assert.True(splitter.Panes[1].Bounds.Y > splitter.Panes[0].Bounds.Bottom);
    }

    [Fact]
    public void Dragging_the_divider_shares_the_room_anew()
    {
        var (screen, _, parts) = Made.Window(Splitter, new TestHost(), width: 808);
        var splitter = (SplitterPart)parts[0];
        var divider = splitter.Dividers[0].Bounds;
        Assert.Same(splitter.Dividers[0], screen.At(divider.X + 2, 50));
        screen.PointerPressed(divider.X + 2, 50, PointerButton.Left);
        screen.PointerMoved(404 + 2, 50);
        screen.PointerReleased(404 + 2, 50, PointerButton.Left);
        Assert.Equal(404, splitter.Dividers[0].Bounds.X, 3);
        Assert.Equal(0.5, splitter.Shares[0], 2);
        // No pane gets less than a tenth.
        screen.PointerPressed(406, 50, PointerButton.Left);
        screen.PointerMoved(0, 50);
        screen.PointerReleased(0, 50, PointerButton.Left);
        Assert.Equal(0.1, splitter.Shares[0], 6);
        Assert.True(splitter.Panes[0].Bounds.X < splitter.Dividers[0].Bounds.X);
    }

    [Fact]
    public void A_pane_whose_parts_go_further_than_its_room_scrolls_so_none_is_out_of_reach()
    {
        var rows = string.Concat(Enumerable.Range(1, 8).Select(r => $"""<Label at="1,{r}" span="8" />"""));
        var markup = $"""
            <Splitter name="split" at="1,1" span="12" rows="3">
              <Pane span="4"><Label at="1,1" span="4" /></Pane>
              <Pane>{rows}</Pane>
            </Splitter>
            """;
        var (screen, _, parts) = Made.Window(markup, new TestHost(), width: 808);
        var splitter = (SplitterPart)parts[0];
        // A place lower than the splitter asks for, as in a window too low for it.
        splitter.Arrange(new Box(0, 0, 808, 124));
        var view = splitter.Views[1];
        Assert.True(view.MaxScrollY > 0);
        Assert.Equal(0, splitter.Views[0].MaxScrollY);
        var last = splitter.Panes[1].Children[^1];
        Assert.True(last.Bounds.Bottom > view.Bounds.Bottom);
        screen.Wheel(view.Bounds.X + 20, view.Bounds.Y + 20, 0, 1000);
        Assert.Equal(view.MaxScrollY, view.ScrollY);
        Assert.True(last.Bounds.Bottom <= view.Bounds.Bottom + 0.5);
    }

    [Fact]
    public void A_scroll_boxs_grid_scrolls_when_its_parts_go_further_than_its_rows()
    {
        var rows = string.Concat(Enumerable.Range(1, 10).Select(r => $"""<Label at="1,{r}" span="3" />"""));
        var (screen, _, parts) = Made.Window($"""<Scroll at="1,1" span="6" rows="3">{rows}</Scroll>""", new TestHost(), width: 808);
        var scroll = Assert.IsType<ScrollPart>(parts[0]);
        Assert.Equal(10 * 36 + 9 * 8 - 124, scroll.View.MaxScrollY);
        var first = scroll.Inner.Children[0];
        screen.Wheel(20, 20, 0, 50);
        Assert.Equal(-50, first.Bounds.Y);
        Assert.True(ContainerPart.IsInside(first));
    }

    [Fact]
    public void A_splitter_asks_room_for_its_padding_so_a_pane_of_its_own_rows_needs_no_scrolling()
    {
        var rows = string.Concat(Enumerable.Range(1, 3).Select(r => $"""<Label at="1,{r}" span="8" />"""));
        var (_, grid, parts) = Made.Window($"""<Splitter at="1,1" span="12" rows="3"><Pane span="4"><Label at="1,1" span="4" /></Pane><Pane>{rows}</Pane></Splitter>""", new TestHost(), width: 808);
        var splitter = (SplitterPart)parts[0];
        Assert.True(splitter.Around > 0);
        Assert.Equal(0, splitter.Views[1].MaxScrollY);
        Assert.True(grid.Content >= 3 * 36 + 2 * 8 + splitter.Around);
    }
}
