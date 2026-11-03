// What lies over a window: a dropdown's list, a menu, a popover, a dialog.
// It is drawn over everything at its place and takes the pointer first; a
// press outside it (and outside what it opened from), or Escape, closes the
// one on top (unless it is modal).

namespace Clarity.Ui;

public sealed class Overlay(Element content, Box place)
{
    public Element Content { get; } = content;

    /// <summary>Where it lies, on the screen.</summary>
    public Box Place { get; set; } = place;

    /// <summary>A dialog: a press outside does not close it, and what is under takes nothing; its content hears keys first when it is an <see cref="IDialogKeys"/>.</summary>
    public bool Modal { get; init; }

    /// <summary>It covers the whole screen, and follows it when it is resized (a dialog with the window dimmed under it).</summary>
    public bool Covers { get; init; }

    /// <summary>
    /// Where what it opened from is (a dropdown's own box, the part a popover
    /// is for): a press there is no press outside, so it goes to that part
    /// and the overlay stays until the part closes it.
    /// </summary>
    public Func<Box>? Anchor { get; init; }

    /// <summary>It was closed: by a press outside, Escape, or <see cref="Screen.Close"/>.</summary>
    public Action? Closed { get; init; }
}

/// <summary>What lies over the window as a dialog and hears keys before the part with the focus: true when it took the key.</summary>
public interface IDialogKeys
{
    bool KeyPressed(KeyInput key);
}

/// <summary>Copying and pasting text: the system's, as the host gives it.</summary>
public interface IClipboard
{
    string? GetText();

    void SetText(string text);
}

/// <summary>A clipboard of the screen's own, until the host gives the system's.</summary>
public sealed class OwnClipboard : IClipboard
{
    string? text;

    public string? GetText() => text;

    public void SetText(string value) => text = value;
}
