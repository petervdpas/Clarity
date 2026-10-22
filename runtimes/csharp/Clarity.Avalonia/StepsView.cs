// A wizard, the steps part (DESIGN 5.15, conformance/steps.json): one step
// at a time, the titles of all steps above with the current one marked, and
// Clarity's own Back, Next and Finish below. Step is the step it is on,
// counted from 1, and binds both ways to the part's state, so code reads and
// sets it. Next runs the handler of `on next` (Next), then moves one step on,
// unless the handler refused (the step stays) or set the step itself (that
// step wins). Back goes one step back and runs nothing. Finish runs the
// handler of `on finish` (Finished).

using Avalonia;
using Avalonia.Collections;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Layout;
using Avalonia.Metadata;

namespace Clarity.Avalonia;

/// <summary>A step of a wizard: its title and its parts.</summary>
public class StepItem : HeaderedContentControl
{
    protected override Type StyleKeyOverride => typeof(ContentControl);
}

public class StepsView : UserControl
{
    public static readonly StyledProperty<long> StepProperty =
        AvaloniaProperty.Register<StepsView, long>(nameof(Step), 1, defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The step it is on, counted from 1.</summary>
    public long Step { get => GetValue(StepProperty); set => SetValue(StepProperty, value); }

    /// <summary>The steps, in order.</summary>
    [Content]
    public AvaloniaList<StepItem> Steps { get; } = [];

    /// <summary>The handler of `on next`: false when it refused. None moves on.</summary>
    public Func<bool>? Next { get; set; }

    /// <summary>Finish was pressed on the last step.</summary>
    public event Action? Finished;

    public Button Back { get; }
    public Button Forward { get; }
    public Button Finish { get; }

    readonly StackPanel titles = new() { Orientation = Orientation.Horizontal, Spacing = 16 };
    readonly ContentControl current = new();

    /// <summary>The step shown when <paramref name="step"/> is asked of <paramref name="count"/>: the nearest there is.</summary>
    public static long Shown(long step, int count) => Math.Clamp(step, 1, Math.Max(count, 1));

    public StepsView()
    {
        Classes.Add("steps");
        Back = new Button { Content = Locale.Translate("back", "Back") };
        Forward = new Button { Content = Locale.Translate("next", "Next"), IsDefault = true, Classes = { "accent" } };
        Finish = new Button { Content = Locale.Translate("finish", "Finish"), IsDefault = true, Classes = { "accent" } };
        Back.Click += (_, _) => GoBack();
        Forward.Click += (_, _) => GoNext();
        Finish.Click += (_, _) => Finished?.Invoke();
        titles.Classes.Add("titles");
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right, Children = { Back, Forward, Finish } };
        DockPanel.SetDock(titles, Dock.Top);
        DockPanel.SetDock(buttons, Dock.Bottom);
        Content = new DockPanel { Children = { titles, buttons, current } };
        Steps.CollectionChanged += (_, _) => Show();
        Show();
    }

    /// <summary>Next: the handler runs on the step shown, then the wizard moves on unless it refused or chose the step.</summary>
    public void GoNext()
    {
        var before = Shown(Step, Steps.Count);
        Step = before;
        if (Next is { } next && !next())
            return;
        if (Step == before && before < Steps.Count)
            Step = before + 1;
    }

    /// <summary>Back: one step back; nothing runs and what was entered stays.</summary>
    public void GoBack()
    {
        var shown = Shown(Step, Steps.Count);
        if (shown > 1)
            Step = shown - 1;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == StepProperty)
            Show();
    }

    /// Shows the step Step asks for, its title marked, and the buttons it has.
    void Show()
    {
        var shown = (int)Shown(Step, Steps.Count);
        titles.Children.Clear();
        for (var i = 0; i < Steps.Count; i++)
        {
            var title = new TextBlock();
            title.Bind(TextBlock.TextProperty, new Binding(nameof(StepItem.Header)) { Source = Steps[i] });
            title.Classes.Add("step");
            if (i + 1 == shown)
                title.Classes.Add("current");
            titles.Children.Add(title);
        }
        current.Content = Steps.Count > 0 ? Steps[shown - 1] : null;
        Back.IsVisible = shown > 1;
        Forward.IsVisible = shown < Steps.Count;
        Finish.IsVisible = shown >= Steps.Count;
    }
}
