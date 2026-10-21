// What a handler can change about a part while the window is open, and what
// the window binds to: shown, enabled, read-only, its caption, an error, the
// text of an unbound input and the selected row of a table or list. Captions,
// errors and help are kept by their keys and shown in the person's language,
// their slots in the person's format (Locale).

using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;

namespace Clarity.Avalonia;

/// <param name="name">The part's name in Clarity.</param>
/// <param name="caption">The caption's key, or a text without letters written as it is.</param>
/// <param name="readOnly">Whether an input starts read-only.</param>
/// <param name="help">The help text's key.</param>
public sealed class PartState(string name, string? caption = null, bool readOnly = false, string help = "") : ObservableObject
{
    /// <summary>The part's name in Clarity, for messages.</summary>
    public string Name { get; } = name;

    public bool IsVisible { get => visible; set => SetProperty(ref visible, value); }
    bool visible = true;

    public bool IsEnabled { get => enabled; set => SetProperty(ref enabled, value); }
    bool enabled = true;

    public bool IsReadOnly { get => readOnly_; set => SetProperty(ref readOnly_, value); }
    bool readOnly_ = readOnly;

    /// <summary>The caption as the design or a handler wrote it.</summary>
    public Text CaptionText
    {
        get => caption_;
        set
        {
            caption_ = value;
            OnPropertyChanged(nameof(Caption));
        }
    }
    Text caption_ = Clarity.Runtime.Text.Of(caption ?? "");

    /// <summary>The caption as shown.</summary>
    public string Caption => Locale.Show(CaptionText);

    /// <summary>A canvas's drawing: its shapes, in order (conformance/drawing.json).</summary>
    public IReadOnlyList<Shape> Drawing { get => drawing; private set => SetProperty(ref drawing, value); }
    IReadOnlyList<Shape> drawing = [];
    List<Shape>? drawn;

    /// <summary>Starts an empty drawing, before the canvas's draw handler runs.</summary>
    public void BeginDrawing() => drawn = [];

    /// <summary>Adds a shape to the drawing being drawn.</summary>
    public void Draw(Shape shape) => (drawn ??= []).Add(shape);

    /// <summary>Shows the drawing: one change, however many shapes it has.</summary>
    public void EndDrawing()
    {
        Drawing = drawn ?? [];
        drawn = null;
    }

    /// <summary>Where a canvas was last clicked; null before any click.</summary>
    public Pointer? Pointer { get; set; }

    /// <summary>Where a canvas was last clicked, across: <c>canvas.x</c>; 0 before any click.</summary>
    public long X => Pointer?.X ?? 0;

    /// <summary>Where a canvas was last clicked, down: <c>canvas.y</c>.</summary>
    public long Y => Pointer?.Y ?? 0;

    /// <summary>Whether a foldable panel or form is folded, as the person or a handler left it
    /// (conformance/folding.json).</summary>
    public bool IsFolded
    {
        get => folded;
        set
        {
            if (SetProperty(ref folded, value))
                OnPropertyChanged(nameof(IsOpen));
        }
    }
    bool folded;

    /// <summary>Whether a foldable part shows its content: not folded. An Expander binds to it.</summary>
    public bool IsOpen { get => !IsFolded; set => IsFolded = !value; }

    /// <summary>Whether a collapsible sidebar is folded to its button, as the person or a handler
    /// left it (conformance/sidebars.json).</summary>
    public bool IsCollapsed { get => collapsed; set => SetProperty(ref collapsed, value); }
    bool collapsed;

    /// <summary>Whether a maximizable sidebar fills the window.</summary>
    public bool IsMaximized { get => maximized; set => SetProperty(ref maximized, value); }
    bool maximized;

    /// <summary>The open tab of a ribbon, counted from 1 (conformance/ribbons.json).</summary>
    public long Tab { get => tab; set => SetProperty(ref tab, value); }
    long tab = 1;

    /// <summary>The text an unbound input holds.</summary>
    public string Text { get => text; set => SetProperty(ref text, value); }
    string text = "";

    /// <summary>Whether an unbound checkbox or switch is on.</summary>
    public bool IsChecked { get => isChecked; set => SetProperty(ref isChecked, value); }
    bool isChecked;

    /// <summary>The row selected in a table or list; null when none is.</summary>
    public object? SelectedItem { get => selected; set => SetProperty(ref selected, value); }
    object? selected;

    /// <summary>What is wrong with the input, as a handler wrote it; empty when nothing is.</summary>
    public Text ErrorText
    {
        get => error;
        set
        {
            error = value;
            OnPropertyChanged(nameof(Error));
            OnPropertyChanged(nameof(HasError));
            OnPropertyChanged(nameof(Shown));
        }
    }
    Text error = Clarity.Runtime.Text.Plain("");

    /// <summary>What is wrong with the input, as shown.</summary>
    public string Error => Locale.Show(ErrorText);

    /// <summary>The help text of the input, shown while there is no error.</summary>
    public string Help => Locale.Translate(help);

    public bool HasError => ErrorText.Key is not null || ErrorText.Template != "";

    /// <summary>What the help text under an input shows: its error, or its help.</summary>
    public string Shown => HasError ? Error : Help;
}
