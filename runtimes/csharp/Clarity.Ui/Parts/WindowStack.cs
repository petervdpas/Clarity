// The windows of an application in one desktop window or page (DESIGN 7.9):
// `open` shows a window in place, on top of the one before; `close` goes
// back to the one below, which is told it is on top again. Closing the last
// one ends the application, or the desktop window of its own it was in. A
// dialog, and a target marked `in new window`, open in a desktop window of
// their own where the host has them (Apart); elsewhere a dialog shows over
// the window as a modal overlay and a new window opens in place. A question
// a handler asks shows in Clarity's own dialog, and a long engine call its
// bar with Stop. The messages belong to the stack, so one said just before a
// window closes stays on screen in the window below. When the language
// changes every window is made again, so its texts show in it.

using System.Collections.ObjectModel;
using System.Xml;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class WindowStack : Element, IDisposable
{
    /// <summary>The width a dialog that names no size opens at.</summary>
    public const double DialogWidth = 480;

    readonly Func<WindowViewModel, string?> markupOf;
    readonly List<Shown> stack = [];
    readonly List<Dialog> dialogs = [];
    readonly List<Overlay> asking = [];
    Overlay? bar;
    bool disposed;

    sealed record Shown(WindowViewModel Model, ViewModelHost Host, WindowView View);

    /// <summary>A dialog over the window: its view model, host and overlay, and the window that opened it.</summary>
    sealed class Dialog(WindowViewModel model, WindowViewModel opener)
    {
        public WindowViewModel Model { get; } = model;
        public WindowViewModel Opener { get; } = opener;
        public ViewModelHost Host { get; set; } = null!;
        public Overlay Overlay { get; set; } = null!;
        public DialogOverlay Content => (DialogOverlay)Overlay.Content;
    }

    /// <summary>
    /// How the host opens a window in a desktop window of its own: a dialog,
    /// or a target marked <c>in new window</c>, with the window that opened
    /// it. None (a browser page): a dialog shows over the window, and a new
    /// window opens in place.
    /// </summary>
    public Action<WindowViewModel, WindowViewModel>? Apart { get; init; }

    /// <summary>The regions of the app's frame (<c>Apps/Shop.cxaml</c>), which every window but a dialog shows where it has none of its own.</summary>
    public IReadOnlyList<System.Xml.Linq.XElement>? AppFrame { get; init; }

    /// <summary>The window that opened this desktop window of its own; told when it closes. None for the application's.</summary>
    public WindowViewModel? Opener { get; init; }

    /// <summary>How the host opens an address in the system's program (a link's); none, and it is only said.</summary>
    public Action<string>? OpenAddress { get; init; }

    /// <summary>The system's file dialogs the host made, which every window gives its parts; none picks nothing.</summary>
    public IFilePicker? Files { get; init; }

    /// <summary>The windows of an application, each drawn from the markup <paramref name="markupOf"/> gives for its view model.</summary>
    public WindowStack(Func<WindowViewModel, string?> markupOf) => this.markupOf = markupOf;

    /// <summary>On a screen it hears the language and long calls.</summary>
    protected override void OnAttached()
    {
        Locale.Changed += Remake;
        LongCall.Changed += LongCallChanged;
    }

    /// <summary>Off its screen it hears them no more.</summary>
    protected override void OnDetached()
    {
        Locale.Changed -= Remake;
        LongCall.Changed -= LongCallChanged;
    }

    /// <summary>The messages on screen, of every window.</summary>
    public ObservableCollection<Message> Messages { get; } = [];

    /// <summary>The window on top.</summary>
    public WindowViewModel? Current => stack.Count > 0 ? stack[^1].Model : null;

    /// <summary>The view of the window on top.</summary>
    public WindowView? View => stack.Count > 0 ? stack[^1].View : null;

    /// <summary>The open windows, from the bottom to the top, the dialogs over them last.</summary>
    public IReadOnlyList<WindowViewModel> Windows => [.. stack.Select(s => s.Model), .. dialogs.Select(d => d.Model)];

    /// <summary>Another window is on top: the desktop window shows its title.</summary>
    public event Action? TitleChanged;

    /// <summary>The last window closed: the application ends.</summary>
    public event Action? Ended;

    /// <summary>The view model of the name its markup goes by: <c>MainViewModel</c> is the window Main.</summary>
    public static string NameOf(WindowViewModel model)
    {
        var name = model.GetType().Name;
        return name.EndsWith("ViewModel", StringComparison.Ordinal) ? name[..^"ViewModel".Length] : name;
    }

    /// <summary>Opens <paramref name="model"/> on top: it shows, then its on open runs.</summary>
    public void Push(WindowViewModel model)
    {
        Attach(model, () => Close(model));
        Show(Make(model));
        model.Opened();
    }

    /// <summary>Closes <paramref name="model"/>, when it is on top; the window below shows again and is told so.</summary>
    public void Close(WindowViewModel model)
    {
        if (Current != model)
            return;
        var top = stack[^1];
        stack.RemoveAt(stack.Count - 1);
        Forget(top);
        model.Closing();
        if (stack.Count == 0)
        {
            Opener?.Returned();
            Ended?.Invoke();
            return;
        }
        AddChild(stack[^1].View);
        TitleChanged?.Invoke();
        Invalidate();
        stack[^1].Model.Returned();
    }

    /// <summary>The desktop window closes: its dialogs and the window on top hear their on close, and the window that opened it is told.</summary>
    public void Closing()
    {
        foreach (var dialog in dialogs.ToList())
            CloseDialog(dialog);
        Current?.Closing();
        Opener?.Returned();
    }

    void Attach(WindowViewModel model, Action close)
    {
        model.Messages = Messages;
        model.OpenRequested += (other, newWindow) => Open(other, newWindow, model);
        model.CloseRequested += close;
        model.CopyRequested += text => Screen?.Clipboard.SetText(Locale.Show(text));
        model.AskRequested += question => Ask(model, question);
    }

    /// <summary>Opens a window a window asked for: in a desktop window of its own where the host has them, a dialog over the window, else in place.</summary>
    void Open(WindowViewModel other, bool newWindow, WindowViewModel opener)
    {
        if ((other.IsDialog || newWindow) && Apart is { } apart)
            apart(other, opener);
        else if (other.IsDialog)
            ShowDialog(other, opener);
        else
            Push(other);
    }

    // -- Dialogs over the window -------------------------------------------------

    /// <summary>A dialog over the window, sized to it, its keys its own; what it says shows in the window under it.</summary>
    void ShowDialog(WindowViewModel model, WindowViewModel opener)
    {
        var dialog = new Dialog(model, opener);
        Attach(model, () => CloseDialog(dialog));
        var (host, view) = MakeDialogView(model);
        dialog.Host = host;
        dialog.Overlay = new Overlay(new DialogOverlay(model, view) { Cancel = () => CloseDialog(dialog) }, Bounds) { Modal = true, Covers = true };
        dialogs.Add(dialog);
        if (Screen is { } screen)
        {
            screen.Open(dialog.Overlay);
            screen.Focus(view.InOrder().FirstOrDefault(e => e.Focusable && e.ShowsInTree));
        }
        model.Opened();
    }

    (ViewModelHost Host, WindowView View) MakeDialogView(WindowViewModel model)
    {
        var made = Make(model, withMessages: false);
        return (made.Host, made.View);
    }

    /// <summary>The dialog closes: it hears its on close, and the window that opened it, or the one on top, is told.</summary>
    void CloseDialog(Dialog dialog)
    {
        if (!dialogs.Remove(dialog))
            return;
        Screen?.Close(dialog.Overlay);
        dialog.Content.View.Dispose();
        dialog.Host.Dispose();
        dialog.Model.Closing();
        var below = Windows.Contains(dialog.Opener) ? dialog.Opener : Windows.LastOrDefault();
        below?.Returned();
        Invalidate();
    }

    /// <summary>
    /// The size of a dialog's window: as it says, else <see cref="DialogWidth"/>
    /// wide and as high as its rows with the window's padding.
    /// </summary>
    public static Extent DialogSize(WindowView view, WindowViewModel model)
    {
        var width = model.WindowSize?.Width ?? DialogWidth;
        if (model.WindowSize is { } size)
            return new Extent(width, size.Height);
        // Laid out with no spare height, the grid's content is its rows as they are.
        view.Arrange(new Box(0, 0, width, 2 * WindowView.ContentPadding + 1));
        var rows = view.Find<GridPanel>()?.Content ?? 6 * GridLayout.RowHeight;
        var height = Math.Clamp(rows + 2 * WindowView.ContentPadding, GridLayout.RowHeight + 2 * WindowView.ContentPadding, 720);
        return new Extent(width, height);
    }

    // -- Questions -----------------------------------------------------------------

    /// <summary>A question a window asks, in Clarity's own dialog over the window; the answer runs its handler, cancelling runs nothing.</summary>
    void Ask(WindowViewModel model, Question question)
    {
        if (Screen is not { } screen)
        {
            // Fail safe: a stack on no screen cannot ask; it says so, and the window goes on.
            Console.Error.WriteLine($"{NameOf(model)} asks {Locale.Show(question.Text)}, on no screen");
            return;
        }
        var ask = new AskDialog(question);
        var overlay = Dialogs.Open(screen, ask, ask.First, () => asking.RemoveAll(o => o.Content == ask));
        asking.Add(overlay);
        ask.Answered = answer =>
        {
            screen.Close(overlay);
            model.Answer(question, answer);
        };
        ask.Cancelled = () => screen.Close(overlay);
    }

    // -- A long engine call ----------------------------------------------------------

    /// <summary>A long engine call said how far it is: its bar shows over the window, and goes once the handler ended.</summary>
    void LongCallChanged()
    {
        if (disposed || Screen is not { } screen)
            return;
        if (LongCall.Current is not null && bar is null && stack.Count > 0)
        {
            bar = new Overlay(LongCallBar.OfLongCall(), screen.Whole) { Modal = true, Covers = true };
            screen.Open(bar);
        }
        else if (LongCall.Current is null && bar is { } shown)
        {
            bar = null;
            screen.Close(shown);
        }
        Invalidate();
    }

    /// <summary>The stack is no longer shown: it hears the language and long calls no more.</summary>
    public void Dispose()
    {
        if (disposed)
            return;
        disposed = true;
        Locale.Changed -= Remake;
        LongCall.Changed -= LongCallChanged;
    }

    Shown Make(WindowViewModel model, bool withMessages = true)
    {
        var markup = markupOf(model);
        string? problem = null;
        WindowSpec spec;
        var columns = WindowView.StandardColumns;
        try
        {
            if (markup is null)
                throw new FormatException($"the application carries no markup for the window {NameOf(model)}");
            spec = WindowMarkup.Read(markup);
            columns = WindowView.ColumnsOf(markup);
        }
        catch (Exception e) when (e is FormatException or XmlException)
        {
            // Fail safe: the window says why it shows nothing, and the application goes on.
            Console.Error.WriteLine(e.Message);
            spec = new WindowSpec([], []);
            problem = e.Message;
        }
        var host = new ViewModelHost(model, spec) { Opener = OpenAddress, Files = Files ?? new NoFilePicker() };
        // A dialog over a window says what it says in the window under it, so a message shows once.
        var view = new WindowView(spec, host, withMessages ? model : null, columns) { Problem = problem };
        if (AppFrame is { } frame && !model.IsDialog)
            view.AppFrame = frame;
        return new Shown(model, host, view);
    }

    void Show(Shown shown)
    {
        if (stack.Count > 0)
            RemoveChild(stack[^1].View);
        stack.Add(shown);
        AddChild(shown.View);
        Screen?.Focus(null);
        TitleChanged?.Invoke();
        Invalidate();
    }

    void Forget(Shown shown)
    {
        // Disposed while on the screen, so its dialog closes there.
        shown.View.Dispose();
        shown.Host.Dispose();
        RemoveChild(shown.View);
        Screen?.Focus(null);
    }

    /// <summary>Makes every window's view again, as in another language.</summary>
    void Remake()
    {
        for (var i = 0; i < stack.Count; i++)
        {
            var was = stack[i];
            var top = i == stack.Count - 1;
            // What was said into its text values shows in the new language before it is drawn again.
            was.Model.Reshow();
            if (top)
                Forget(was);
            else
            {
                was.View.Dispose();
                was.Host.Dispose();
            }
            stack[i] = Make(was.Model);
            if (top)
                AddChild(stack[i].View);
        }
        foreach (var dialog in dialogs)
        {
            dialog.Model.Reshow();
            dialog.Content.View.Dispose();
            dialog.Host.Dispose();
            var (host, view) = MakeDialogView(dialog.Model);
            dialog.Host = host;
            dialog.Content.Replace(view);
        }
        TitleChanged?.Invoke();
        Invalidate();
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        View?.Arrange(box);
    }

    /// <summary>Time went on: the window on top's info messages go when their time is up; the part with the focus gives its undo scope.</summary>
    public void Tick()
    {
        if (stack.Count == 0)
            return;
        var (model, host, view) = stack[^1];
        view.Tick();
        if (PartOf(Screen?.Focused) is { } part && host.Scope(part.Spec.Name) is { } scope)
            model.FocusedScope = scope;
    }

    /// <summary>
    /// The window's own keys, which the host gives the screen as its
    /// <see cref="Screen.Keys"/>: the window on top's (shortcuts, the default
    /// and cancel buttons), then Ctrl+Z takes back the last edit of the scope
    /// of the part with the focus, Ctrl+Y or Ctrl+Shift+Z does it again
    /// (DESIGN 5.29); a part that takes the key itself (typing not yet an
    /// edit) has it first. True when it took the key.
    /// </summary>
    /// <summary>A key let go, which the host gives the screen as its <see cref="Screen.KeysUp"/>: the window on top's (Alt for its menu bar). True when it took the key.</summary>
    public bool KeyReleased(KeyInput key) => View?.KeyReleased(key) ?? false;

    public bool KeyPressed(KeyInput key)
    {
        if (stack.Count == 0)
            return false;
        if (View is { } view && view.KeyPressed(key))
            return true;
        // A dialog in a desktop window of its own closes on Escape that nothing in it took, as its close does (DESIGN 5.14).
        if (key.Key == Key.Escape && key.Modifiers == Modifiers.None && Current is { IsDialog: true } dialog)
        {
            Close(dialog);
            return true;
        }
        var command = key.Control || key.Modifiers.HasFlag(Modifiers.Meta);
        if (!command || key.Modifiers.HasFlag(Modifiers.Alt))
            return false;
        var back = key.Key == Key.Z && !key.Shift;
        var again = key.Key == Key.Y || (key.Key == Key.Z && key.Shift);
        if (!back && !again)
            return false;
        var focused = Screen?.Focused;
        if (focused is not null && focused.OnKey(key))
            return true;
        var (model, host, _) = stack[^1];
        if (PartOf(focused) is { } part && host.Scope(part.Spec.Name) is { } scope)
            model.UndoEdit(scope, back);
        return true;
    }

    static Part? PartOf(Element? element)
    {
        for (var e = element; e is not null; e = e.Parent)
            if (e is Part { Spec.Name: not null } part)
                return part;
        return null;
    }
}
