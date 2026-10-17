// What a handler can change about a part while the window is open, and what
// the window binds to: shown, enabled, read-only, its caption, an error, the
// text of an unbound input and the selected row of a table or list. Captions,
// errors and help are kept as written and shown translated, their slots in
// the person's format (Locale).

using Clarity.Runtime;
using CommunityToolkit.Mvvm.ComponentModel;

namespace Clarity.Avalonia;

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
    Text caption_ = Clarity.Runtime.Text.Plain(caption ?? "");

    /// <summary>The caption as shown.</summary>
    public string Caption => Locale.Show(CaptionText);

    /// <summary>The text an unbound input holds.</summary>
    public string Text { get => text; set => SetProperty(ref text, value); }
    string text = "";

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

    public bool HasError => ErrorText.Template != "";

    /// <summary>What the help text under an input shows: its error, or its help.</summary>
    public string Shown => HasError ? Error : Help;
}
