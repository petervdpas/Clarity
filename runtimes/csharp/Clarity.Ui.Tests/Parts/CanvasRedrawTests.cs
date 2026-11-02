// A canvas draws anew after a change the person makes that its draw handler
// may read, though no handler runs (a name typed in a box bound to a
// setting, as in the walker example), as it does after every handler; a
// handler that runs on the change draws it once, not twice.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>A window whose canvas draws the name the person types, as the generator writes one.</summary>
public sealed class NamedFieldViewModel() : WindowViewModel(null)
{
    public string Name { get => name; set => SetProperty(ref name, value); }
    string name = "";

    public PartState Field { get; } = new("field");
    public PartState NameBox { get; } = new("nameBox");
    public int Draws { get; private set; }

    protected override void Redraw()
    {
        Field.BeginDrawing();
        Draws++;
        Field.Draw(new Shape.Clear("panel"));
        Field.Draw(new Shape.Words(50, 50, Text.Plain(Name), "text", 10));
        Field.EndDrawing();
    }

    public void Renamed() => Handle(() => Name += "!");
}

public class CanvasRedrawTests
{
    static (NamedFieldViewModel Model, ViewModelHost Host) Hosted()
    {
        var model = new NamedFieldViewModel();
        var spec = WindowMarkup.Read("""<Window name="NamedField"><Canvas name="field" at="1,1" span="6" /><Textbox name="nameBox" at="1,2" bind="name" /></Window>""");
        model.Opened();
        return (model, new ViewModelHost(model, spec));
    }

    static string Said(NamedFieldViewModel model) => Locale.Show(model.Field.Drawing.OfType<Shape.Words>().Single().Text);

    [Fact]
    public void A_canvas_draws_anew_after_a_bound_value_the_person_changed()
    {
        var (model, host) = Hosted();
        var drawn = model.Draws;
        host.Write("nameBox", "name", "Ada");
        Assert.Equal("Ada", Said(model));
        Assert.Equal(drawn + 1, model.Draws);
    }

    [Fact]
    public void A_canvas_draws_anew_after_an_unbound_part_the_person_changed()
    {
        var (model, host) = Hosted();
        var drawn = model.Draws;
        model.Name = "Bo";
        host.Edited("nameBox");
        Assert.Equal("Bo", Said(model));
        Assert.Equal(drawn + 1, model.Draws);
    }

    [Fact]
    public void A_handler_draws_anew_once_and_a_window_without_a_canvas_draws_nothing()
    {
        var (model, _) = Hosted();
        var drawn = model.Draws;
        model.Renamed();
        Assert.Equal(drawn + 1, model.Draws);
        // Nothing to draw: told of a change, a window without draw handlers runs nothing.
        var plain = new Generated.OrdersViewModel();
        plain.Changed();
        Assert.Empty(plain.Messages);
    }
}
