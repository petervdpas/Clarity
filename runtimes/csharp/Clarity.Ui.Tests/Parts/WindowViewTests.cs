// A whole window: its parts in the window's grid inside the content
// padding, scrolling when the window is smaller than its rows, its info
// messages as notes that go by themselves and its warnings and errors in
// Clarity's own dialog; and the windows of an application opened in place,
// one on top of the other, and closed back to the one below.

using Clarity.Runtime;
using Clarity.Ui.Tests.Generated;

namespace Clarity.Ui.Tests;

public class WindowViewTests
{
    const string Markup = """
        <Window name="Orders">
          <Label name="noteLabel" at="1,1" span="6" caption="Note" />
          <Button name="approveButton" at="7,1" span="6" caption="Approve" onclick="approveClicked" />
          <Table name="grid" at="1,2" span="12" rows="4" bind="orders" />
        </Window>
        """;

    static (Screen Screen, WindowView View, OrdersViewModel Model) Shown(string markup = Markup, double width = 800, double height = 400)
    {
        var model = new OrdersViewModel();
        var spec = WindowMarkup.Read(markup);
        var view = new WindowView(spec, new ViewModelHost(model, spec), model, WindowView.ColumnsOf(markup));
        var screen = new Screen(view, ThemeLook.Clarity(dark: false));
        screen.Resize(width, height, 1);
        return (screen, view, model);
    }

    [Fact]
    public void Its_parts_are_in_a_grid_of_twelve_inside_the_content_padding()
    {
        var (_, view, _) = Shown();
        Assert.Equal(3, view.Parts.Count);
        var column = (800 - 2 * WindowView.ContentPadding - 11 * GridLayout.Gap) / 12;
        Assert.Equal(new Box(12, 12, 6 * column + 5 * GridLayout.Gap, GridLayout.RowHeight), view.Parts[0].Bounds);
        Assert.Equal(12 + 6 * (column + GridLayout.Gap), view.Parts[1].Bounds.X, 6);
        Assert.Equal(12 + GridLayout.RowHeight + GridLayout.Gap, view.Parts[2].Bounds.Y);
    }

    [Fact]
    public void A_window_that_names_its_columns_has_a_grid_of_them()
    {
        Assert.Equal(12, WindowView.ColumnsOf("<Window name=\"A\" />"));
        Assert.Equal(4, WindowView.ColumnsOf("<Window name=\"A\" span=\"4\" />"));
        var (_, view, _) = Shown("<Window name=\"A\" span=\"4\"><Button name=\"b\" at=\"1,1\" span=\"2\" /></Window>", width: 424);
        Assert.Equal((400 - 3 * GridLayout.Gap) / 2 + GridLayout.Gap, view.Parts[0].Bounds.Width, 6);
    }

    [Fact]
    public void A_window_smaller_than_its_rows_scrolls_with_the_wheel()
    {
        var (screen, view, _) = Shown("<Window name=\"A\"><Button name=\"low\" at=\"1,20\" span=\"2\" /></Window>", height: 300);
        var content = 20 * GridLayout.RowHeight + 19 * GridLayout.Gap;
        Assert.Equal(content - (300 - 2 * WindowView.ContentPadding), view.MaxScrollY);
        var top = view.Parts[0].Bounds.Y;
        screen.Wheel(100, 100, 0, 40);
        Shot.Of(screen, 800, 300);
        Assert.Equal(40, view.ScrollY);
        Assert.Equal(top - 40, view.Parts[0].Bounds.Y);
        screen.Wheel(100, 100, 0, -5000);
        Assert.Equal(0, view.ScrollY);
        Assert.False(view.OnWheel(0, -40));
    }

    [Fact]
    public void A_window_that_fits_does_not_scroll()
    {
        var (_, view, _) = Shown();
        Assert.Equal(0, view.MaxScrollY);
        Assert.False(view.OnWheel(0, 40));
    }

    [Fact]
    public void An_info_message_is_a_note_that_goes_by_itself()
    {
        var (screen, view, model) = Shown();
        var now = new DateTime(2026, 11, 2, 12, 0, 0, DateTimeKind.Utc);
        view.Now = () => now;
        model.Say(Level.Info, Text.Plain("Saved"));
        Assert.Equal(["Saved"], view.Notes.Select(n => n.Shown));
        Assert.Empty(screen.Overlays);
        Shot.Of(screen, 800, 400);
        now += TimeSpan.FromSeconds(4);
        view.Tick();
        Assert.Single(view.Notes);
        now += TimeSpan.FromSeconds(1);
        view.Tick();
        Assert.Empty(view.Notes);
        Assert.Empty(model.Messages);
    }

    [Fact]
    public void A_note_closes_with_its_x()
    {
        var (screen, view, model) = Shown();
        model.Say(Level.Info, Text.Plain("Saved"));
        Shot.Of(screen, 800, 400);
        // The x is at the top right of the note, at the top right of the window.
        var x = 800 - WindowView.ContentPadding - 18;
        var y = WindowView.ContentPadding + 18;
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
        Assert.Empty(model.Messages);
    }

    [Fact]
    public void A_warning_is_said_in_clarities_own_dialog_until_ok()
    {
        var (screen, view, model) = Shown();
        model.Say(Level.Warning, Text.Plain("Type a task first"));
        Shot.Of(screen, 800, 400);
        var overlay = Assert.Single(screen.Overlays);
        Assert.True(overlay.Modal);
        var dialog = Assert.IsType<SaidDialog>(overlay.Content);
        Assert.Equal("Type a task first", view.Saying?.Shown);
        Assert.Same(dialog.Ok, screen.Focused);
        // What is under the dialog takes nothing; Escape does not close it, OK does.
        Assert.IsType<SaidDialog>(screen.At(view.Parts[1].Bounds.X + 4, view.Parts[1].Bounds.Y + 4));
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.Empty(model.Messages);
        Assert.Empty(view.Notes);
    }

    [Fact]
    public void Errors_are_said_one_at_a_time()
    {
        var (screen, view, model) = Shown();
        model.Say(Level.Error, Text.Plain("one"));
        model.Say(Level.Error, Text.Plain("two"));
        Shot.Of(screen, 800, 400);
        Assert.Single(screen.Overlays);
        Assert.Equal("one", view.Saying?.Shown);
        var ok = ((SaidDialog)screen.Overlays[0].Content).Ok;
        var (x, y) = (ok.Bounds.X + 4, ok.Bounds.Y + 4);
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
        Shot.Of(screen, 800, 400);
        Assert.Equal("two", view.Saying?.Shown);
    }

    [Fact]
    public void Markup_that_cannot_be_read_says_why_in_the_window()
    {
        var stack = new WindowStack(_ => "<Window");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        stack.Push(new OtherViewModel());
        Shot.Of(screen, 400, 200);
        Assert.NotNull(stack.View?.Problem);
        Assert.Empty(stack.View!.Parts);
    }

    [Fact]
    public void A_window_opens_in_place_and_closes_back_to_the_one_below()
    {
        var stack = new WindowStack(model => WindowStack.NameOf(model) == "Orders" ? Markup : "<Window name=\"Other\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        var titles = 0;
        var ended = false;
        stack.TitleChanged += () => titles++;
        stack.Ended += () => ended = true;
        var orders = new OrdersViewModel();
        stack.Push(orders);
        Assert.Equal(1, orders.OpenedTimes);
        Assert.Same(orders.Messages, stack.Messages);
        Shot.Of(screen, 800, 400);
        Assert.Equal(3, stack.View!.Parts.Count);

        // A click that opens a window opens it on top, in place.
        new ViewModelHost(orders).Run("usersButton", new HandlerCall("click", "Other", []));
        var other = Assert.IsType<OtherViewModel>(stack.Current);
        Assert.Equal(1, other.OpenedTimes);
        Assert.Equal([orders, other], stack.Windows);
        Assert.Single(stack.Children);
        Assert.Empty(stack.View!.Parts);
        Assert.Equal(2, titles);

        // Closing it goes back to the window below, which is told.
        other.CloseWindow_Command.Execute(null);
        Assert.Same(orders, stack.Current);
        Assert.Equal(1, orders.ReturnedTimes);
        Assert.Equal(3, stack.View!.Parts.Count);
        Assert.False(ended);

        orders.CloseWindow_Command.Execute(null);
        Assert.True(ended);
        Assert.Empty(stack.Windows);
    }

    [Fact]
    public void A_message_said_as_a_window_closes_shows_in_the_window_below()
    {
        var stack = new WindowStack(_ => "<Window name=\"W\" />");
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        var orders = new OrdersViewModel();
        stack.Push(orders);
        var other = new OtherViewModel();
        stack.Push(other);
        other.Say(Level.Warning, Text.Plain("bye"));
        Shot.Of(screen, 800, 400);
        Assert.Single(screen.Overlays);
        other.CloseWindow_Command.Execute(null);
        Shot.Of(screen, 800, 400);
        Assert.Equal("bye", stack.View?.Saying?.Shown);
    }

    [Fact]
    public void Ctrl_z_takes_back_the_last_edit_of_the_focused_parts_scope()
    {
        const string markup = "<Window name=\"Orders\"><Label name=\"note\" at=\"1,1\" span=\"4\" bind=\"order.note\" undoScope=\"form\" /></Window>";
        var stack = new WindowStack(_ => markup);
        var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
        var orders = new OrdersViewModel();
        stack.Push(orders);
        Shot.Of(screen, 800, 400);
        var host = new ViewModelHost(orders, WindowMarkup.Read(markup));
        host.Write("note", "order.note", "typed");
        var part = stack.View!.Parts[0];
        screen.Focus(part);
        stack.Tick();
        Assert.Equal("form", orders.FocusedScope);
        Assert.False(stack.KeyPressed(new KeyInput(Key.Z, Modifiers.None)));
        Assert.True(stack.KeyPressed(new KeyInput(Key.Z, Modifiers.Control)));
        Assert.Equal("", orders.Order.Note);
        Assert.True(stack.KeyPressed(new KeyInput(Key.Y, Modifiers.Control)));
        Assert.Equal("typed", orders.Order.Note);
    }
}
