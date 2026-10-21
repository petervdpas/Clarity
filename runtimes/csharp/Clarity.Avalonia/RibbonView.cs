// A ribbon (conformance/ribbons.json): the tab control a window's ribbon is.
// Tab is the open tab, counted from 1; a hidden tab gives way to the first
// visible one. A collapsible ribbon folds to its tab strip (IsCollapsed): no
// tab is open then, and opening one unfolds it; a double click folds it
// again, as in an office suite. Tab and IsCollapsed bind both ways to the
// ribbon's part state, so the code reads and sets them.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;

namespace Clarity.Avalonia;

public class RibbonView : TabControl
{
    protected override Type StyleKeyOverride => typeof(TabControl);

    public static readonly StyledProperty<bool> CollapsibleProperty = AvaloniaProperty.Register<RibbonView, bool>(nameof(Collapsible));
    public static readonly StyledProperty<bool> IsCollapsedProperty =
        AvaloniaProperty.Register<RibbonView, bool>(nameof(IsCollapsed), defaultBindingMode: BindingMode.TwoWay);
    public static readonly StyledProperty<long> TabProperty =
        AvaloniaProperty.Register<RibbonView, long>(nameof(Tab), 1, defaultBindingMode: BindingMode.TwoWay);

    public bool Collapsible { get => GetValue(CollapsibleProperty); set => SetValue(CollapsibleProperty, value); }
    public bool IsCollapsed { get => GetValue(IsCollapsedProperty); set => SetValue(IsCollapsedProperty, value); }
    /// <summary>The open tab, counted from 1.</summary>
    public long Tab { get => GetValue(TabProperty); set => SetValue(TabProperty, value); }

    /// <summary>The tab shown (from 1) of tabs that are visible or not, when `asked` is wanted:
    /// it, when it is there and visible, else the first visible one; null when none is.</summary>
    public static long? ShownTab(IReadOnlyList<bool> visible, long asked)
    {
        if (asked >= 1 && asked <= visible.Count && visible[(int)asked - 1])
            return asked;
        for (var i = 0; i < visible.Count; i++)
            if (visible[i])
                return i + 1;
        return null;
    }

    bool showing;

    public RibbonView()
    {
        // Folded, no tab is open: a tab control otherwise always keeps one.
        SelectionMode = global::Avalonia.Controls.SelectionMode.Single;
        SelectionChanged += (_, _) =>
        {
            if (showing || SelectedIndex < 0)
                return;
            // The person opened a tab: it is the open one, and a folded ribbon unfolds.
            Tab = SelectedIndex + 1;
            IsCollapsed = false;
        };
        DoubleTapped += (_, _) =>
        {
            if (Collapsible)
                IsCollapsed = !IsCollapsed;
        };
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == TabProperty || change.Property == IsCollapsedProperty || change.Property == CollapsibleProperty)
            Show();
    }

    protected override void OnAttachedToVisualTree(global::Avalonia.VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        Show();
    }

    /// Opens the tab Tab asks for, or none while folded.
    void Show()
    {
        var visible = Items.Select(i => i is Control c ? c.IsVisible : true).ToList();
        var shown = Collapsible && IsCollapsed ? null : ShownTab(visible, Tab);
        showing = true;
        SelectedIndex = shown is long t ? (int)t - 1 : -1;
        showing = false;
    }
}
