// An accordion against the cases every renderer shares
// (conformance/accordions.json), run through the real AccordionView: its
// headers pressed, the open section shown, the part's state from 1.

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Interactivity;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class AccordionTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "accordions.json"))).RootElement;

    static AccordionView Accordion(int sections, long open)
    {
        var view = new AccordionView();
        for (var i = 1; i <= sections; i++)
            view.Sections.Add(new SectionItem { Header = $"Section {i}", Content = new TextBlock() });
        view.Open = open;
        return view;
    }

    [AvaloniaFact]
    public void The_open_section_shows_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("shown").EnumerateArray())
        {
            var view = Accordion(c.GetProperty("sections").GetInt32(), c.GetProperty("open").GetInt64());
            var shown = c.GetProperty("shown").GetInt64();
            Assert.Equal(shown, AccordionView.Shown(view.Open, view.Sections.Count));
            for (var i = 0; i < view.Sections.Count; i++)
                Assert.Equal(i + 1 == shown, view.Headers[i].Classes.Contains("open"));
        }
    }

    [AvaloniaFact]
    public void A_press_on_a_header_opens_or_folds_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("press").EnumerateArray())
        {
            var view = Accordion(c.GetProperty("sections").GetInt32(), c.GetProperty("open").GetInt64());
            var changed = 0;
            view.Changed += () => changed++;
            view.Headers[c.GetProperty("press").GetInt32() - 1].RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.Equal(c.GetProperty("open after").GetInt64(), view.Open);
            Assert.Equal(1, changed);
        }
    }

    [AvaloniaFact]
    public void An_accordions_part_state_opens_the_first_section()
    {
        var part = new PartState("faq");
        Assert.Equal(1, part.OpenSection);
        foreach (var c in File.GetProperty("set").EnumerateArray())
        {
            part.OpenSection = c.GetProperty("open").GetInt64();
            Assert.Equal(c.GetProperty("part").GetProperty("open").GetInt64(), part.OpenSection);
        }
    }
}
