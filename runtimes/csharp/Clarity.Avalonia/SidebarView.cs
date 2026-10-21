// A sidebar docked to a window's edge (conformance/sidebars.json). Its
// parts sit in Content. The person drags its inner edge between its min and
// max (Resizable), folds it to a narrow strip with a button and back
// (Collapsible, IsCollapsed), and lets it take the whole window and give it
// back (Maximizable, IsMaximized). IsCollapsed and IsMaximized bind both ways
// to its part state, so the code reads and sets them.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Layout;

namespace Clarity.Avalonia;

public class SidebarView : Control
{
    /// <summary>The width of a folded sidebar: room for its button.</summary>
    public const double StripWidth = 28;

    /// <summary>The widths a sidebar may have (clarity_spec::model::SIDEBAR_WIDTHS).</summary>
    public const double Narrowest = 100, Widest = 1600;

    public static readonly StyledProperty<Dock> SideProperty = AvaloniaProperty.Register<SidebarView, Dock>(nameof(Side), Dock.Left);
    public static readonly StyledProperty<double> SidebarWidthProperty = AvaloniaProperty.Register<SidebarView, double>(nameof(SidebarWidth), 240);
    public static readonly StyledProperty<double?> MinSidebarWidthProperty = AvaloniaProperty.Register<SidebarView, double?>(nameof(MinSidebarWidth));
    public static readonly StyledProperty<double?> MaxSidebarWidthProperty = AvaloniaProperty.Register<SidebarView, double?>(nameof(MaxSidebarWidth));
    public static readonly StyledProperty<bool> ResizableProperty = AvaloniaProperty.Register<SidebarView, bool>(nameof(Resizable));
    public static readonly StyledProperty<bool> CollapsibleProperty = AvaloniaProperty.Register<SidebarView, bool>(nameof(Collapsible));
    public static readonly StyledProperty<bool> MaximizableProperty = AvaloniaProperty.Register<SidebarView, bool>(nameof(Maximizable));
    public static readonly StyledProperty<bool> IsCollapsedProperty =
        AvaloniaProperty.Register<SidebarView, bool>(nameof(IsCollapsed), defaultBindingMode: BindingMode.TwoWay);
    public static readonly StyledProperty<bool> IsMaximizedProperty =
        AvaloniaProperty.Register<SidebarView, bool>(nameof(IsMaximized), defaultBindingMode: BindingMode.TwoWay);
    public static readonly StyledProperty<Control?> ContentProperty = AvaloniaProperty.Register<SidebarView, Control?>(nameof(Content));

    /// <summary>The edge it is docked to: Left or Right.</summary>
    public Dock Side { get => GetValue(SideProperty); set => SetValue(SideProperty, value); }
    /// <summary>Its width in pixels while it is neither folded nor filling the window.</summary>
    public double SidebarWidth { get => GetValue(SidebarWidthProperty); set => SetValue(SidebarWidthProperty, value); }
    public double? MinSidebarWidth { get => GetValue(MinSidebarWidthProperty); set => SetValue(MinSidebarWidthProperty, value); }
    public double? MaxSidebarWidth { get => GetValue(MaxSidebarWidthProperty); set => SetValue(MaxSidebarWidthProperty, value); }
    public bool Resizable { get => GetValue(ResizableProperty); set => SetValue(ResizableProperty, value); }
    public bool Collapsible { get => GetValue(CollapsibleProperty); set => SetValue(CollapsibleProperty, value); }
    public bool Maximizable { get => GetValue(MaximizableProperty); set => SetValue(MaximizableProperty, value); }
    public bool IsCollapsed { get => GetValue(IsCollapsedProperty); set => SetValue(IsCollapsedProperty, value); }
    public bool IsMaximized { get => GetValue(IsMaximizedProperty); set => SetValue(IsMaximizedProperty, value); }
    /// <summary>Its parts, on their grid.</summary>
    public Control? Content { get => GetValue(ContentProperty); set => SetValue(ContentProperty, value); }

    /// <summary>The width a drag to `dragged` pixels gives: within the limits, in whole pixels.</summary>
    public static double WidthFor(double dragged, double? min, double? max) =>
        Math.Round(Math.Clamp(dragged, min ?? Narrowest, max ?? Widest), MidpointRounding.AwayFromZero);

    readonly DockPanel root = new();
    readonly StackPanel header = new() { Orientation = Orientation.Horizontal, Spacing = 2 };
    readonly Button fold = new() { Classes = { "sidebar-button" } };
    readonly Button fill = new() { Classes = { "sidebar-button" } };
    readonly Thumb edge = new() { Width = 5, Cursor = new Cursor(StandardCursorType.SizeWestEast), Classes = { "sidebar-edge" } };
    readonly Decorator host = new();

    static SidebarView()
    {
        AffectsMeasure<SidebarView>(SidebarWidthProperty, IsCollapsedProperty, IsMaximizedProperty, SideProperty);
    }

    public SidebarView()
    {
        DockPanel.SetDock(header, Dock.Top);
        header.Children.Add(fill);
        header.Children.Add(fold);
        root.Children.Add(header);
        root.Children.Add(edge);
        root.Children.Add(host);
        fold.Click += (_, _) => IsCollapsed = !IsCollapsed;
        fill.Click += (_, _) => IsMaximized = !IsMaximized;
        edge.DragDelta += (_, e) =>
            SidebarWidth = WidthFor(SidebarWidth + (Side == Dock.Left ? e.Vector.X : -e.Vector.X), MinSidebarWidth, MaxSidebarWidth);
        VisualChildren.Add(root);
        LogicalChildren.Add(root);
        Refresh();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ContentProperty)
            host.Child = Content;
        Refresh();
    }

    /// Shows what the sidebar is now: its buttons, its edge and its parts.
    void Refresh()
    {
        DockPanel.SetDock(edge, Side == Dock.Left ? Dock.Right : Dock.Left);
        header.HorizontalAlignment = Side == Dock.Left ? HorizontalAlignment.Right : HorizontalAlignment.Left;
        header.IsVisible = Collapsible || Maximizable;
        // The fold button points the way the sidebar goes.
        var toEdge = Side == Dock.Left ? "‹" : "›";
        var fromEdge = Side == Dock.Left ? "›" : "‹";
        fold.IsVisible = Collapsible;
        fold.Content = IsCollapsed ? fromEdge : toEdge;
        ToolTip.SetTip(fold, IsCollapsed ? "Show the sidebar" : "Fold the sidebar away");
        fill.IsVisible = Maximizable && !IsCollapsed;
        fill.Content = IsMaximized ? "⤡" : "⤢";
        ToolTip.SetTip(fill, IsMaximized ? "Give the window back" : "Fill the window");
        edge.IsVisible = Resizable && !IsCollapsed && !IsMaximized;
        host.IsVisible = !IsCollapsed;
    }

    /// The width it takes: a strip when folded, all there is when it fills the window, else its own.
    double Wanted(Size available) =>
        IsCollapsed ? StripWidth
        : IsMaximized && double.IsFinite(available.Width) ? available.Width
        : SidebarWidth;

    protected override Size MeasureOverride(Size availableSize)
    {
        var width = Wanted(availableSize);
        root.Measure(new Size(width, availableSize.Height));
        return new Size(width, root.DesiredSize.Height);
    }

    protected override Size ArrangeOverride(Size finalSize)
    {
        root.Arrange(new Rect(finalSize));
        return finalSize;
    }
}
