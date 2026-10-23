// An accordion (U1b, conformance/accordions.json): sections that fold, one
// open at a time. Each section's header is a button; the open section's
// parts fill the room under it. Open is the open section, counted from 1, 0
// for none, and binds both ways to the part's state, so code reads and sets
// it. A press on a header opens that section, or folds it when it was the
// open one, and tells Changed.

using Avalonia;
using Avalonia.Collections;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Layout;
using Avalonia.Metadata;

namespace Clarity.Avalonia;

/// <summary>A section of an accordion: its title and its parts.</summary>
public class SectionItem : HeaderedContentControl
{
    protected override Type StyleKeyOverride => typeof(ContentControl);
}

public class AccordionView : UserControl
{
    public static readonly StyledProperty<long> OpenProperty =
        AvaloniaProperty.Register<AccordionView, long>(nameof(Open), 1, defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The open section, counted from 1; 0 when none is open.</summary>
    public long Open { get => GetValue(OpenProperty); set => SetValue(OpenProperty, value); }

    /// <summary>The sections, in order.</summary>
    [Content]
    public AvaloniaList<SectionItem> Sections { get; } = [];

    /// <summary>The person opened or folded a section.</summary>
    public event Action? Changed;

    /// <summary>The headers, one per section, as the person presses them.</summary>
    public IReadOnlyList<Button> Headers => headers;

    readonly List<Button> headers = [];
    readonly Grid grid = new();

    /// <summary>The section shown open when <paramref name="open"/> is asked of <paramref name="count"/>: it, when it is there; else none.</summary>
    public static long Shown(long open, int count) => open >= 1 && open <= count ? open : 0;

    /// <summary>The open section after a press on the header of <paramref name="press"/>: it, or none when it was the open one.</summary>
    public static long Pressed(long open, long press, int count) => Shown(open, count) == press ? 0 : press;

    public AccordionView()
    {
        Classes.Add("accordion");
        Content = grid;
        Sections.CollectionChanged += (_, _) => Show();
        Show();
    }

    /// <summary>A press on the header of section <paramref name="press"/>, from 1.</summary>
    public void Press(long press)
    {
        Open = Pressed(Open, press, Sections.Count);
        Changed?.Invoke();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == OpenProperty)
            Show();
    }

    /// Lays out the headers, the open section's parts under its own header.
    void Show()
    {
        var open = (int)Shown(Open, Sections.Count);
        grid.Children.Clear();
        grid.RowDefinitions.Clear();
        headers.Clear();
        for (var i = 0; i < Sections.Count; i++)
        {
            var section = Sections[i];
            var number = i + 1;
            var title = new TextBlock { VerticalAlignment = VerticalAlignment.Center };
            title.Bind(TextBlock.TextProperty, new Binding(nameof(SectionItem.Header)) { Source = section });
            var chevron = new TextBlock { Text = number == open ? "▾" : "▸", VerticalAlignment = VerticalAlignment.Center };
            var header = new Button
            {
                HorizontalAlignment = HorizontalAlignment.Stretch,
                HorizontalContentAlignment = HorizontalAlignment.Left,
                Content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { chevron, title } },
            };
            header.Classes.Add("section");
            if (number == open)
                header.Classes.Add("open");
            header.Click += (_, _) => Press(number);
            headers.Add(header);
            grid.RowDefinitions.Add(new RowDefinition(GridLength.Auto));
            Grid.SetRow(header, grid.RowDefinitions.Count - 1);
            grid.Children.Add(header);
            if (number == open)
            {
                grid.RowDefinitions.Add(new RowDefinition(1, GridUnitType.Star));
                Grid.SetRow(section, grid.RowDefinitions.Count - 1);
                grid.Children.Add(section);
            }
        }
    }
}
