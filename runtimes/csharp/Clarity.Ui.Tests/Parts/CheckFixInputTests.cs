// The inputs as the person meets them: a read-only checkbox or switch is
// greyed as a disabled one; the pointer is a hand over a link it can follow;
// a spin box's arrows are drawn in the theme's text color (muted while
// disabled) and are no Tab stops, the Down arrow too; a read-only dropdown
// takes no Up, Down or letter; a read-only radio group's choices are no Tab
// stops; a required text cleared is empty, as a required day cleared is no
// day; and an unbound tags part keeps its list when its window is made again.

using System.Globalization;
using Clarity.Runtime;
using CommunityToolkit.Mvvm.Input;

namespace Clarity.Ui.Tests;

/// <summary>A window with an unbound tags part, as the generator writes it: a state for the named part.</summary>
public sealed class MoodsViewModel() : WindowViewModel(null)
{
    public override string Title => "Moods";

    public PartState Moods { get; } = new("moods");

    public int Changes;

    RelayCommand? moodsChanged;
    public IRelayCommand MoodsChangedCommand => moodsChanged ??= new RelayCommand(MoodsChanged);
    void MoodsChanged() => Handle(OnMoodsChanged);
    void OnMoodsChanged() => Changes++;
}

public class CheckFixInputTests
{
    static bool Near(SKColor a, SKColor b) => Math.Abs(a.Red - b.Red) + Math.Abs(a.Green - b.Green) + Math.Abs(a.Blue - b.Blue) < 24;

    static void Press(Screen screen, Key key) => screen.KeyPressed(new KeyInput(key, Modifiers.None));

    [Fact]
    public void A_read_only_checkbox_or_switch_is_greyed_as_a_disabled_one()
    {
        var host = new TestHost();
        foreach (var path in new[] { "a", "b", "c", "d", "e", "f" })
            host.Values[path] = true;
        var (screen, _, parts) = Made.Window("""
            <Checkbox at="1,1" span="3" name="plain" bind="a" />
            <Checkbox at="1,2" span="3" name="reading" bind="b" readonly="true" />
            <Checkbox at="1,3" span="3" name="off" bind="c" />
            <Switch at="1,4" span="3" name="plainSwitch" bind="d" />
            <Switch at="1,5" span="3" name="readingSwitch" bind="e" readonly="true" />
            <Switch at="1,6" span="3" name="offSwitch" bind="f" />
            """, host);
        host.State("off")!.IsEnabled = false;
        host.State("offSwitch")!.IsEnabled = false;
        using var shot = Shot.Of(screen, 800, 400);
        var accent = screen.Look.Color("accent");
        // In the ticked box, away from the tick; in the switch's track, left of the knob.
        SKColor BoxAt(Part p) => shot.PixelAt((int)(p.Bounds.X + 1 + 4), (int)(p.Bounds.Y + (p.Bounds.Height - CheckPart.BoxSize) / 2 + 3));
        SKColor TrackAt(Part p) => shot.PixelAt((int)(p.Bounds.X + 6), (int)(p.Bounds.Y + p.Bounds.Height / 2));
        Assert.True(Near(accent, BoxAt(parts[0])));
        Assert.False(Near(accent, BoxAt(parts[1])), "a read-only box is greyed");
        Assert.Equal(BoxAt(parts[2]), BoxAt(parts[1]));
        Assert.True(Near(accent, TrackAt(parts[3])));
        Assert.False(Near(accent, TrackAt(parts[4])), "a read-only switch is greyed");
        Assert.Equal(TrackAt(parts[5]), TrackAt(parts[4]));
    }

    [Fact]
    public void The_pointer_is_a_hand_over_a_link_it_can_follow()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""
            <Link at="1,1" span="3" name="docs" to="https://example.org/docs" caption="Docs" />
            <Link at="1,2" span="3" name="gone" to="https://example.org/gone" caption="Gone" />
            <Label at="1,3" span="3" caption="Plain" />
            """, host);
        host.State("gone")!.IsEnabled = false;
        Shot.Of(screen, 800, 400);
        void Over(Part p) => screen.PointerMoved(p.Bounds.X + 6, p.Bounds.Y + p.Bounds.Height / 2);
        Over(parts[0]);
        Assert.Equal(Cursor.Hand, screen.Cursor);
        Over(parts[2]);
        Assert.Equal(Cursor.Default, screen.Cursor);
        Over(parts[1]);
        Assert.Equal(Cursor.Default, screen.Cursor);
    }

    [Fact]
    public void A_spin_boxs_arrows_are_drawn_in_the_themes_text_color_and_are_no_tab_stops()
    {
        foreach (var dark in new[] { false, true })
        {
            var host = new TestHost();
            host.Values["count"] = 3L;
            host.Values["other"] = 3L;
            host.Fields["count"] = new FieldFacts("Count", new FieldFormat(FormatKind.Integer));
            host.Fields["other"] = new FieldFacts("Other", new FieldFormat(FormatKind.Integer));
            var (screen, _, parts) = Made.Window("""
                <Spinbox at="1,1" span="3" name="count" bind="count" min="0" max="9" />
                <Spinbox at="1,2" span="3" name="other" bind="other" min="0" max="9" />
                """, host);
            host.State("other")!.IsEnabled = false;
            screen.Show(ThemeLook.Clarity(dark));
            using var shot = Shot.Of(screen, 1600, 800, 2);
            var look = screen.Look;
            bool Shows(InputSideButton arrow, SKColor color)
            {
                var b = arrow.Bounds;
                for (var x = (int)(b.X * 2); x < (int)(b.Right * 2); x++)
                    for (var y = (int)(b.Y * 2); y < (int)(b.Bottom * 2); y++)
                        if (shot.PixelAt(x, y) == color)
                            return true;
                return false;
            }
            var count = (NumberPart)parts[0];
            var other = (NumberPart)parts[1];
            foreach (var arrow in new[] { count.Up!, count.Down! })
                Assert.True(Shows(arrow, look.Color("text")), $"an arrow in {(dark ? "dark" : "light")} is drawn in the text color");
            foreach (var arrow in new[] { other.Up!, other.Down! })
            {
                Assert.True(Shows(arrow, look.Color("muted")));
                Assert.False(Shows(arrow, look.Color("text")));
            }
            // Neither arrow is a Tab stop.
            Assert.False(count.Up!.Focusable);
            Assert.False(count.Down!.Focusable);
            Assert.DoesNotContain(screen.TabOrder(), e => e is InputSideButton);
        }
    }

    [Fact]
    public void A_read_only_dropdown_takes_no_up_down_or_letter()
    {
        var host = new TestHost();
        host.Values["task.owner"] = 1L;
        host.Offered["task.owner"] = [new(null, ""), new(1L, "Ada"), new(2L, "Grace")];
        var (screen, _, parts) = Made.Window("""<Dropdown at="1,1" span="4" name="owner" bind="task.owner" readonly="true" onchange="ownerChanged" />""", host);
        var dropdown = (DropdownPart)parts[0];
        screen.Focus(dropdown);
        Press(screen, Key.Down);
        Press(screen, Key.Up);
        screen.TextTyped("g");
        Assert.False(dropdown.OnKey(new KeyInput(Key.Down, Modifiers.None)));
        dropdown.OnText("g");
        Assert.Null(dropdown.List);
        Assert.Equal(1L, host.Values["task.owner"]);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_read_only_radio_groups_choices_are_no_tab_stops()
    {
        var host = new TestHost();
        host.Values["task.size"] = "Small";
        host.Offered["task.size"] = [new("Small", "Small"), new("Medium", "Medium"), new("Large", "Large")];
        var (screen, _, parts) = Made.Window("""
            <Radio at="1,1" span="12" name="reading" bind="task.size" readonly="true" />
            <Radio at="1,2" span="12" name="picking" bind="task.size" />
            """, host);
        Shot.Of(screen, 800, 400);
        var reading = (RadioPart)parts[0];
        var picking = (RadioPart)parts[1];
        var order = screen.TabOrder();
        Assert.DoesNotContain(order, e => reading.Buttons.Contains(e));
        Assert.Equal(picking.Buttons.Count, order.Count(e => picking.Buttons.Contains(e)));
    }

    [Fact]
    public void A_required_text_cleared_is_empty_as_every_input_takes_an_empty_value()
    {
        var host = new TestHost();
        host.Values["task.name"] = "Ada";
        host.Fields["task.name"] = new FieldFacts("Name", Required: true);
        var (screen, _, parts) = Made.Window("""<Textbox at="1,1" span="4" name="name" bind="task.name" />""", host);
        var editor = parts[0].Find<TextEditor>()!;
        screen.Focus(editor);
        editor.SelectAll();
        Press(screen, Key.Delete);
        Press(screen, Key.Enter);
        Assert.Equal("", host.Values["task.name"]);
    }

    [Collection("Language")]
    public class Remade
    {
        [Fact]
        public void An_unbound_tags_part_keeps_its_list_when_its_window_is_made_again()
        {
            string? Lookup(string key, CultureInfo culture) => null;
            Locale.Start("en", ["en", "nl"], null, Lookup, (_, value) => value, CultureInfo.GetCultureInfo("en-US"));
            try
            {
                var stack = new WindowStack(_ => """<Window name="Moods"><Tags at="1,1" span="8" name="moods" onchange="moodsChanged" /></Window>""");
                var screen = new Screen(stack, ThemeLook.Clarity(dark: false));
                screen.Resize(800, 400, 1);
                var model = new MoodsViewModel();
                stack.Push(model);
                var tags = stack.View!.Find<TagsPart>(_ => true)!;
                screen.Focus(tags.Editor);
                screen.TextTyped("calm");
                Press(screen, Key.Enter);
                Assert.Equal(["calm"], tags.Value!.Cast<string>());
                Assert.Equal(1, model.Changes);

                Locale.Choose("nl");

                var again = stack.View!.Find<TagsPart>(_ => true)!;
                Assert.NotSame(tags, again);
                Assert.Equal(["calm"], again.Value!.Cast<string>());
                Assert.Equal(["calm"], again.Chips.Select(c => c.Label));
            }
            finally
            {
                Locale.Choose("en");
                Locale.Reset();
            }
        }
    }
}
