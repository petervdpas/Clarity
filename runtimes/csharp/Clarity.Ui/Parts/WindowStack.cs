// The windows of an application in one desktop window or page (DESIGN 7.9):
// `open` shows a window in place, on top of the one before; `close` goes
// back to the one below, which is told it is on top again. Closing the last
// one ends the application. The messages belong to the stack, so one said
// just before a window closes stays on screen in the window below. When the
// language changes every window is made again, so its texts show in it.

using System.Collections.ObjectModel;
using System.Xml;
using Clarity.Runtime;

namespace Clarity.Ui;

public sealed class WindowStack : Element
{
    readonly Func<WindowViewModel, string?> markupOf;
    readonly List<Shown> stack = [];

    sealed record Shown(WindowViewModel Model, ViewModelHost Host, WindowView View);

    /// <summary>How the host opens an address in the system's program (a link's); none, and it is only said.</summary>
    public Action<string>? OpenAddress { get; init; }

    /// <summary>The windows of an application, each drawn from the markup <paramref name="markupOf"/> gives for its view model.</summary>
    public WindowStack(Func<WindowViewModel, string?> markupOf)
    {
        this.markupOf = markupOf;
        Locale.Changed += Remake;
    }

    /// <summary>The messages on screen, of every window.</summary>
    public ObservableCollection<Message> Messages { get; } = [];

    /// <summary>The window on top.</summary>
    public WindowViewModel? Current => stack.Count > 0 ? stack[^1].Model : null;

    /// <summary>The view of the window on top.</summary>
    public WindowView? View => stack.Count > 0 ? stack[^1].View : null;

    /// <summary>The open windows, from the bottom to the top.</summary>
    public IReadOnlyList<WindowViewModel> Windows => [.. stack.Select(s => s.Model)];

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
        Attach(model);
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
            Ended?.Invoke();
            return;
        }
        AddChild(stack[^1].View);
        TitleChanged?.Invoke();
        Invalidate();
        stack[^1].Model.Returned();
    }

    /// <summary>The desktop window closes: the window on top hears its on close.</summary>
    public void Closing() => Current?.Closing();

    void Attach(WindowViewModel model)
    {
        model.Messages = Messages;
        // A window in a desktop window of its own, and a dialog in one, come with the frame (G7d); until then they open in place.
        model.OpenRequested += (other, _) => Push(other);
        model.CloseRequested += () => Close(model);
        model.CopyRequested += text => Screen?.Clipboard.SetText(Locale.Show(text));
        model.AskRequested += question => Console.Error.WriteLine($"{NameOf(model)} asks {Locale.Show(question.Text)}: Clarity.UI's questions dialog comes with G7c");
    }

    Shown Make(WindowViewModel model)
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
        var host = new ViewModelHost(model, spec) { Opener = OpenAddress };
        return new Shown(model, host, new WindowView(spec, host, model, columns) { Problem = problem });
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
    public bool KeyPressed(KeyInput key)
    {
        if (stack.Count == 0)
            return false;
        if (View is { } view && view.KeyPressed(key))
            return true;
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
