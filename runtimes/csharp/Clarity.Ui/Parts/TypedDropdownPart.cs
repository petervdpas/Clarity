// A box to type in with a list of what fits below it, the same as on every
// stack (conformance/dropdown-options.json): the dropdown's `typed` variant
// (any text, the values its field already has offered) and its `search`
// variant (a record a reference points at, picked among many, filtered as
// it is typed). The list opens when the box gets the focus or text is
// typed; Up and Down move through it, Enter or a click picks, Escape closes
// it and puts the value's text back. Typed text that is not picked counts
// only for `typed`; for `search`, empty text is no record. Only a value that
// differs is a change; what code sets is never one.

using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>What the typed and the search dropdown share: the box, the list below it, the keys.</summary>
public abstract class TypeAheadPart : Part
{
    TextEditor editor = null!;
    Overlay? open;
    DropdownList? list;
    List<Choice> rows = [];
    string? shown;

    /// <summary>Where the person types.</summary>
    public TextEditor Editor => editor;

    /// <summary>The list of what fits, while it is open.</summary>
    public Overlay? List => open;


    bool Usable => Enabled && !ReadOnly && Bound is not null;

    /// <summary>What fits the text in the box now.</summary>
    public IReadOnlyList<Choice> Offered => Offer(editor.Text);

    /// <summary>The rows that fit <paramref name="typed"/>.</summary>
    protected abstract List<Choice> Offer(string typed);

    /// <summary>The text its value shows in the box.</summary>
    protected abstract string Shown();

    /// <summary>A row's line in the list.</summary>
    protected virtual string LineOf(Choice row) => row.Label;

    /// <summary>A row's cells, each in a column of the list; none: the row is its line.</summary>
    protected virtual IReadOnlyList<string>? CellsOf(Choice row) => null;

    /// <summary>A row was picked: the value it gives, and whether that differs from the value.</summary>
    protected abstract (object? Value, bool Differs) Picked(Choice row);

    /// <summary>The box was left or Enter pressed with text no row was picked for: the value it gives, and whether that is a change.</summary>
    protected abstract (object? Value, bool Differs) Typed(string typed);

    protected internal override void Made()
    {
        editor = new TextEditor
        {
            Placeholder = Spec.Text("placeholder") is { } placeholder ? Host.Text(placeholder) : null,
            Enabled = () => Enabled,
            ReadOnly = () => ReadOnly,
            Marked = () => State?.HasError == true,
            KeyFirst = Key,
            Entered = () =>
            {
                Take();
                return PartTexts.Submit(this);
            },
        };
        editor.Edited += () =>
        {
            if (editor.Focused)
                Fill(open: true);
        };
        editor.FocusMoved += focused =>
        {
            if (focused)
            {
                if (open is null)
                    Fill(open: true);
                Fire("enter");
            }
            // A press in its own list takes the focus for a moment and gives it back: no leaving.
            else if (list?.Hovered != true)
            {
                Take();
                Fire("leave");
            }
        };
        AddChild(editor);
        PressLooks.Watch(this, Host, State);
        Host.Changed += Sync;
        Sync();
    }

    /// <summary>A value set from elsewhere shows as it is.</summary>
    void Sync()
    {
        var now = Shown();
        if (now == shown)
            return;
        shown = now;
        editor.Text = now;
    }

    /// <summary>Puts the value's text back in the box.</summary>
    void Show()
    {
        shown = null;
        Sync();
    }

    /// <summary>Fills the list with what fits the text; it opens when asked and there is something to offer, and closes when nothing fits.</summary>
    void Fill(bool open)
    {
        if (!Usable || Screen is not { } screen)
            return;
        rows = Offer(editor.Text);
        Close();
        if (!open || rows.Count == 0)
            return;
        var picked = rows;
        var cells = rows.Select(CellsOf).ToList();
        list = new DropdownList([.. rows.Select(LineOf)], -1, at =>
        {
            Close();
            if (at >= 0 && at < picked.Count)
                Pick(picked[at]);
        })
        { Holder = editor, Cells = cells.All(c => c is not null) ? [.. cells.Select(c => c!)] : null };
        this.open = new Overlay(list, DropdownList.Below(screen, Bounds, Bounds.Width, rows.Count))
        {
            Anchor = () => Bounds,
            Closed = () =>
            {
                this.open = null;
                list = null;
            },
        };
        screen.Open(this.open);
    }

    void Close()
    {
        if (open is { } was)
            Screen?.Close(was);
        open = null;
        list = null;
        Invalidate();
    }

    /// <summary>Picks a row: a change when it gives another value.</summary>
    public void Pick(Choice row)
    {
        Close();
        if (Usable && Picked(row) is (var value, true))
            Write(value);
        Show();
    }

    /// <summary>Takes what is typed, as when the box is left.</summary>
    public void Take()
    {
        Close();
        var typed = editor.Text;
        if (Usable && typed != Shown() && Typed(typed) is (var value, true))
            Write(value);
        Show();
    }

    void Write(object? value)
    {
        Host.Write(Spec.Name ?? "", Bound!, value);
        Fire("change");
    }

    bool Key(KeyInput key)
    {
        switch (key.Key)
        {
            case Clarity.Ui.Key.Down or Clarity.Ui.Key.Up:
                if (list is null)
                    Fill(open: true);
                list?.Move(key.Key == Clarity.Ui.Key.Down ? 1 : -1);
                return true;
            case Clarity.Ui.Key.Enter when list is { Highlighted: >= 0 } shownList:
                var at = shownList.Highlighted;
                var picked = rows;
                Close();
                if (at < picked.Count)
                    Pick(picked[at]);
                return true;
            case Clarity.Ui.Key.Escape when open is not null:
                Close();
                Show();
                return true;
        }
        return false;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        editor.Arrange(box);
    }

    public override void Draw(Painter painter)
    {
        Sync();
        base.Draw(painter);
    }
}

/// <summary>A dropdown any text may be typed into (<c>typed</c>), offering the values its field already has.</summary>
[PartVariant("dropdown")]
public sealed class TypedDropdownPart : TypeAheadPart
{
    public static bool Fits(PartSpec spec, IPartHost host) => spec.Flag("typed");

    /// <summary>The values its field already has, with repeats and empty ones, as the host offers them.</summary>
    IEnumerable<string> Values => Bound is { } path ? Host.Offered(Spec, path).Select(c => c.Value as string ?? c.Label) : [];

    string Value => Bound is { } path ? Host.Read(path) as string ?? "" : "";

    protected override List<Choice> Offer(string typed) => [.. DropdownOptions.Suggestions(Values, typed).Select(v => new Choice(v, v))];

    protected override string Shown() => Value;

    protected override (object? Value, bool Differs) Picked(Choice row) => Typed((string)row.Value!);

    protected override (object? Value, bool Differs) Typed(string typed) => (typed, typed != Value);
}

/// <summary>A dropdown of a reference whose records filter as it is typed (<c>search</c>), each with the fields it shows.</summary>
[PartVariant("dropdown")]
public sealed class SearchDropdownPart : TypeAheadPart
{
    /// <summary>A dropdown with <c>search</c> bound to a reference.</summary>
    public static bool Fits(PartSpec spec, IPartHost host) =>
        spec.Flag("search") && spec.Text("bind") is { Length: > 0 } path
        && (host.Field(path)?.Refers is not null || host.Offered(spec, path).Any(c => c.Cells is not null));

    /// <summary>The records it can point at, without the empty choice for none.</summary>
    IEnumerable<Choice> Records => Bound is { } path ? Host.Offered(Spec, path).Where(c => c.Value is not null) : [];

    object? Value => Bound is { } path ? Host.Read(path) : null;

    /// <summary>A record's pieces: its text and the fields it shows, each once.</summary>
    static IReadOnlyList<string> Pieces(Choice row) => [.. new[] { row.Label }.Concat(row.Cells ?? []).Where(t => t.Length > 0).Distinct()];

    /// <summary>A record's line: its text and the fields it shows, each once.</summary>
    public static string Line(Choice row) => string.Join(", ", Pieces(row));

    protected override string LineOf(Choice row) => Line(row);

    /// <summary>A record's fields in the list's columns.</summary>
    protected override IReadOnlyList<string>? CellsOf(Choice row) => row.Cells is null ? null : Pieces(row);

    protected override List<Choice> Offer(string typed) => DropdownOptions.Searched(Records, typed);

    protected override string Shown()
    {
        var value = Value;
        return value is null ? "" : Records.FirstOrDefault(c => PressLooks.Same(c.Value, value))?.Label ?? "";
    }

    protected override (object? Value, bool Differs) Picked(Choice row) => (row.Value, !PressLooks.Same(row.Value, Value));

    // Typed text picks nothing; emptied, it is no record.
    protected override (object? Value, bool Differs) Typed(string typed) => (null, typed.Trim() == "" && Value is not null);
}
