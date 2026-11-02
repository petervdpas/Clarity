// The radio group: one button per choice, side by side or one under the
// other; picking another choice is a change, the one it has is not, and
// what code sets is not; the arrow keys pick the next or previous choice.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class RadioPartTests
{
    static TestHost Sizes()
    {
        var host = new TestHost();
        host.Values["task.size"] = "Small";
        host.Offered["task.size"] = [new("Small", "Small"), new("Medium", "Medium"), new("Large", "Large")];
        return host;
    }

    static void Click(Screen screen, Box place)
    {
        screen.PointerPressed(place.X + 4, place.Y + place.Height / 2, PointerButton.Left);
        screen.PointerReleased(place.X + 4, place.Y + place.Height / 2, PointerButton.Left);
    }

    [Fact]
    public void Its_choices_stand_side_by_side_or_one_under_the_other()
    {
        var host = Sizes();
        var (screen, _, parts) = Made.Window("""
            <Radio at="1,1" span="12" name="across" bind="task.size" />
            <Radio at="1,2" span="12" rows="3" name="down" bind="task.size" orientation="vertical" />
            """, host);
        var across = ((RadioPart)parts[0]).Places(screen.Fonts);
        Assert.Equal(3, across.Count);
        Assert.True(across[1].X > across[0].Right && across[1].Y == across[0].Y);
        var down = ((RadioPart)parts[1]).Places(screen.Fonts);
        Assert.True(down[1].Y >= down[0].Bottom && down[1].X == down[0].X);
    }

    [Fact]
    public void Picking_another_choice_changes_it_and_the_one_it_has_does_not()
    {
        var host = Sizes();
        var (screen, _, parts) = Made.Window("""<Radio at="1,1" span="12" name="size" bind="task.size" onchange="sized" />""", host);
        var radio = (RadioPart)parts[0];
        var places = radio.Places(screen.Fonts);
        Click(screen, places[0]);
        Assert.Empty(host.Log);
        Click(screen, places[2]);
        Assert.Equal("Large", host.Values["task.size"]);
        Assert.Equal(["write size task.size Large", "run size change sized"], host.Log);
        Assert.Equal(2, radio.Chosen);
    }

    [Fact]
    public void What_code_sets_is_no_change()
    {
        var host = Sizes();
        var (_, _, parts) = Made.Window("""<Radio at="1,1" span="12" name="size" bind="task.size" onchange="sized" />""", host);
        host.Values["task.size"] = "Medium";
        host.Change();
        Assert.Equal(1, ((RadioPart)parts[0]).Chosen);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void The_arrow_keys_pick_the_next_and_the_previous_choice()
    {
        var host = Sizes();
        var (screen, _, _) = Made.Window("""<Radio at="1,1" span="12" name="size" bind="task.size" />""", host);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        Assert.Equal("Medium", host.Values["task.size"]);
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        Assert.Equal("Large", host.Values["task.size"]);
    }

    [Fact]
    public void A_disabled_or_read_only_group_cannot_be_picked()
    {
        var host = Sizes();
        var (screen, _, parts) = Made.Window("""
            <Radio at="1,1" span="12" name="first" bind="task.size" readonly="true" />
            <Radio at="1,2" span="12" name="second" bind="task.size" />
            """, host);
        host.State("second")!.IsEnabled = false;
        Click(screen, ((RadioPart)parts[0]).Places(screen.Fonts)[1]);
        Click(screen, ((RadioPart)parts[1]).Places(screen.Fonts)[1]);
        Assert.Empty(host.Log);
    }
}
