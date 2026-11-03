// A dialog shown over the window that opened it (DESIGN 7.9), where the host
// opens no window of its own (a browser page): the window under it dimmed,
// the dialog's title, and its window in a panel sized to the dialog, as its
// own desktop window would be. Its keys are its own: its shortcuts, Enter
// for its default button and Escape for its cancel button (without one,
// Escape closes it as its close does).

using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class DialogOverlay : Element, IDialogKeys
{
    /// <summary>The height of the dialog's title above its window.</summary>
    public const double TitleHeight = 40;

    public DialogOverlay(WindowViewModel model, WindowView view)
    {
        Model = model;
        View = view;
        AddChild(view);
    }

    public WindowViewModel Model { get; }

    /// <summary>The dialog's window; made again in another language.</summary>
    public WindowView View { get; private set; }

    public string Title => Model.Title;

    /// <summary>Where the dialog is: its title and its window, in the middle of the window under it.</summary>
    public Box Panel { get; private set; }

    /// <summary>Shows <paramref name="view"/> as the dialog's window in place of the one before.</summary>
    public void Replace(WindowView view)
    {
        RemoveChild(View);
        View = view;
        AddChild(view);
        Invalidate();
    }

    /// <summary>Closes the dialog as its close does: Escape that nothing in it took.</summary>
    public Action? Cancel { get; init; }

    public bool KeyPressed(KeyInput key)
    {
        if (View.KeyPressed(key))
            return true;
        // Escape that nothing in it took closes it, as its close does (DESIGN 5.14).
        if (key.Key != Key.Escape || key.Modifiers != Modifiers.None || Cancel is not { } cancel)
            return false;
        cancel();
        return true;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        var size = WindowStack.DialogSize(View, Model);
        var width = Math.Min(size.Width, Math.Max(160, box.Width - 32));
        var height = Math.Min(size.Height + TitleHeight, Math.Max(TitleHeight + GridLayout.RowHeight, box.Height - 32));
        Panel = new Box(box.X + (box.Width - width) / 2, box.Y + (box.Height - height) / 2, width, height);
        View.Arrange(new Box(Panel.X, Panel.Y + TitleHeight, Panel.Width, Panel.Height - TitleHeight));
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        Dialogs.Backdrop(painter, Bounds, Panel);
        var title = new Box(Panel.X + WindowView.ContentPadding, Panel.Y, Panel.Width - 2 * WindowView.ContentPadding, TitleHeight);
        painter.Text(Title, title, look.Color("text"), look.Size("heading"), bold: true);
        painter.Fill(new Box(Panel.X, Panel.Y + TitleHeight - 1, Panel.Width, 1), look.Color("border"));
        base.Draw(painter);
    }
}
