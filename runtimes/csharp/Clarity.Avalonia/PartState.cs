// What a handler can change about a part while the window is open, and what
// the window binds to: shown, enabled, read-only, its caption, an error, the
// text of an unbound input and the selected row of a table or list. Captions,
// errors and help are kept by their keys and shown in the person's language,
// their slots in the person's format (Locale).

using System.Text.Json;
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

    /// <summary>Shows a drawing as it is, such as the one a server drew (DESIGN 7.6).</summary>
    internal void ShowDrawing(IReadOnlyList<Shape> shapes) => Drawing = shapes;

    /// <summary>Where a canvas was last clicked; null before any click.</summary>
    public Pointer? Pointer { get; set; }

    /// <summary>Where a canvas was last clicked, across: <c>canvas.x</c>; 0 before any click.</summary>
    public long X => Pointer?.X ?? 0;

    /// <summary>Where a canvas was last clicked, down: <c>canvas.y</c>.</summary>
    public long Y => Pointer?.Y ?? 0;

    /// <summary>Where a drag on a canvas started, across: <c>canvas.fromX</c>; 0 without one.</summary>
    public long FromX => Pointer?.FromX ?? 0;

    /// <summary>Where a drag on a canvas started, down: <c>canvas.fromY</c>.</summary>
    public long FromY => Pointer?.FromY ?? 0;

    /// <summary>The name of the link a person bent on a canvas: <c>canvas.bent</c>; empty without one.</summary>
    public string Bent => Pointer?.Bent ?? "";

    /// <summary>That link's bend points now, x, y, x, y, ...: <c>canvas.via</c>.</summary>
    public List<long> Via => [.. Pointer?.Via ?? []];

    /// <summary>A canvas's grid in its units, 0 for none: <c>canvas.grid</c>, as its line says until a handler sets it.</summary>
    public long Grid
    {
        get => grid;
        set => SetProperty(ref grid, value);
    }
    long grid;

    /// <summary>Whether code snaps to a canvas's grid: <c>canvas.snap</c>.</summary>
    public bool Snap
    {
        get => snap;
        set => SetProperty(ref snap, value);
    }
    bool snap;

    /// <summary>What a canvas's clear() fills it with: <c>canvas.background</c>; empty is the theme's panel.</summary>
    public string Background
    {
        get => background;
        set => SetProperty(ref background, value);
    }
    string background = "";

    /// <summary>How many pixels across a canvas's grid dots are: <c>canvas.dotSize</c>.</summary>
    public long DotSize
    {
        get => dotSize;
        set => SetProperty(ref dotSize, value);
    }
    long dotSize = 2;

    /// <summary>The color of a canvas's grid dots: <c>canvas.dotColor</c>; empty is the theme's border.</summary>
    public string DotColor
    {
        get => dotColor;
        set => SetProperty(ref dotColor, value);
    }
    string dotColor = "";

    /// <summary>The tool in hand of a painting or vector canvas (DESIGN 5.31): <c>canvas.tool</c>; empty for its mode's first.</summary>
    public string Tool
    {
        get => tool;
        set => SetProperty(ref tool, value);
    }
    string tool = "";

    /// <summary>How wide a painting or vector canvas paints or draws, in pixels: <c>canvas.brush</c>.</summary>
    public long Brush
    {
        get => brush;
        set => SetProperty(ref brush, value);
    }
    long brush = 2;

    /// <summary>The color a painting or vector canvas paints or draws with: <c>canvas.ink</c>.</summary>
    public string Ink
    {
        get => ink;
        set => SetProperty(ref ink, value);
    }
    string ink = "text";

    /// <summary>The place of the shape a person touched last on a vector canvas, from 1: <c>canvas.selected</c>; 0 for none.
    /// The canvas keeps it from 0 in <see cref="SelectedItem"/>, as a table's chosen row.</summary>
    public long SelectedShape => SelectedItem is long place ? place + 1 : 0;

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

    /// <summary>The step a wizard (a steps part) is on, counted from 1 (conformance/steps.json).</summary>
    public long Step { get => step; set => SetProperty(ref step, value); }
    long step = 1;

    /// <summary>The open section of an accordion, counted from 1, 0 for none (conformance/accordions.json).</summary>
    public long OpenSection { get => openSection; set => SetProperty(ref openSection, value); }
    long openSection = 1;

    /// <summary>The text an unbound input holds.</summary>
    public string Text { get => text; set => SetProperty(ref text, value); }
    string text = "";

    /// <summary>Whether an unbound checkbox or switch is on.</summary>
    public bool IsChecked { get => isChecked; set => SetProperty(ref isChecked, value); }
    bool isChecked;

    /// <summary>The row selected in a table or list; null when none is.</summary>
    public object? SelectedItem
    {
        get => selected;
        set
        {
            if (SetProperty(ref selected, value))
                OnPropertyChanged(nameof(SelectedShape));
        }
    }
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

    // -- A custom part (DESIGN 5.19) ------------------------------------------

    /// <summary>A custom part's properties, as written in the window and set by code.</summary>
    public IReadOnlyDictionary<string, object?> Values => values;
    readonly Dictionary<string, object?> values = [];
    readonly List<(string Hear, object?[] Values)> waiting = [];
    Action<string, object?>? valueSet;
    Action<string, object?[]>? heard;

    /// <summary>A value written on the part's line, as the window starts.</summary>
    public PartState With(string name, object? value)
    {
        values[name] = value;
        return this;
    }

    /// <summary>A property of a custom part as code sets it: <c>arena.speed = 7</c>.</summary>
    public void SetValue(string name, object? value)
    {
        values[name] = value;
        valueSet?.Invoke(name, value);
    }

    /// <summary>A property of a custom part as type <typeparamref name="T"/>: as written or set,
    /// else <paramref name="fallback"/>, its declared default as JSON.</summary>
    public T ValueAs<T>(string name, string fallback) =>
        values.TryGetValue(name, out var value) && value is T typed ? typed : JsonSerializer.Deserialize<T>(fallback, DotNetPart.Options)!;

    /// <summary>Gives a custom part a hear: <c>arena.restart(2)</c>. One given before the part
    /// shows waits for it.</summary>
    public void Hear(string hear, params object?[] given)
    {
        if (heard is null)
            waiting.Add((hear, given));
        else
            heard(hear, given);
    }

    /// <summary>
    /// What code told the part that nothing heard yet, taken: on a server,
    /// where no view listens, what goes to the browser's mirror (DESIGN 7.6).
    /// </summary>
    internal List<(string Hear, object?[] Values)> TakeTold()
    {
        if (heard is not null)
            return [];
        var told = waiting.ToList();
        waiting.Clear();
        return told;
    }

    /// <summary>The part that shows this state hears what is set and told from now on, after
    /// the hears that waited for it.</summary>
    public void Listen(Action<string, object?> onSet, Action<string, object?[]> onHear)
    {
        valueSet = onSet;
        heard = onHear;
        foreach (var (hear, given) in waiting)
            onHear(hear, given);
        waiting.Clear();
    }

    /// <summary>The part no longer shows this state.</summary>
    public void StopListening()
    {
        valueSet = null;
        heard = null;
    }
}
