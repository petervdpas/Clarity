// The base of every window's ViewModel. Handlers run through Handle, so what
// goes wrong in one becomes an error message instead of a crash; opening and
// closing windows go through the Shell that shows them.

using System.Collections.ObjectModel;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.Json.Nodes;
using CommunityToolkit.Mvvm.ComponentModel;
using Clarity.Runtime;

namespace Clarity.Runtime;

public abstract class WindowViewModel(Db? db) : ObservableObject
{
    /// <summary>The databases of the application.</summary>
    public Db Db { get; } = db!;

    /// <summary>The number of this open window, its own: what belongs to it, such as an engine's state per window, goes by it.</summary>
    public long Instance { get; } = OpenWindows.Next();

    /// <summary>
    /// The long engine call of the windows this one shows with: the shell that
    /// shows it (the program answering Clarity.UI) gives its own, so a long
    /// call holds back only their handlers. Its own until a shell shows it.
    /// </summary>
    public LongCall LongCall { get; set; } = new();

    /// <summary>The window's title.</summary>
    public virtual string Title => "";

    /// <summary>A dialog opens in a desktop window of its own; other windows open in place.</summary>
    public virtual bool IsDialog => false;

    /// <summary>The size its desktop window opens at (<c>size 320 x 460</c>); none for the standard size.</summary>
    public virtual Extent? WindowSize => null;

    /// <summary>
    /// The window's <c>allow when</c> (step 4), checked before it opens: refuses with
    /// Clarity's <c>notAllowed</c> (<see cref="Signin.Allow"/>) when it does not hold.
    /// </summary>
    public virtual void Allow() { }

    /// <summary>The messages on screen, newest last; the shell's, once it shows the window.</summary>
    public ObservableCollection<Message> Messages { get; internal set; } = [];

    /// <summary>A window this one asks to open: in place, or in a desktop window of its own.</summary>
    public event Action<WindowViewModel, bool>? OpenRequested;

    /// <summary>The window asks to be closed.</summary>
    public event Action? CloseRequested;

    /// <summary>The window asks a question, when the handler that asks has finished (DESIGN 5.14).</summary>
    public event Action<Question>? AskRequested;

    public void Say(Level level, Text text) => Messages.Add(new Message(level, text));

    /// <summary><c>toast saved(...)</c>: the screen shows <paramref name="text"/> as the toast <paramref name="toast"/>.</summary>
    public void ShowToast(ToastLook toast, Text text) => Messages.Add(new Message(Level.Info, text, toast));

    /// <summary>The texts said into text values (a status line), by the value's property: what was said, what it showed and how it is set.</summary>
    readonly Dictionary<string, (Text Said, string Shown, Action<string> Set)> said = [];

    /// <summary>
    /// Sets a text value (<paramref name="property"/>) to <paramref name="text"/>
    /// in the language shown, and keeps the text, so a change of language shows
    /// it again in the new one.
    /// </summary>
    protected void ShowText(string property, Text text, Action<string> set)
    {
        var shown = Locale.Show(text);
        said[property] = (text, shown, set);
        set(shown);
    }

    /// <summary>
    /// The language changed: every text said into a value shows in the new
    /// language, unless the value changed since (the person typed into it).
    /// </summary>
    public void Reshow()
    {
        foreach (var (property, (text, shown, set)) in said.ToList())
        {
            if (!Equals(GetType().GetProperty(property)?.GetValue(this), shown))
            {
                said.Remove(property);
                continue;
            }
            ShowText(property, text, set);
        }
    }

    /// <summary>The edits of this window (DESIGN 5.29), which Ctrl+Z takes back.</summary>
    public EditBus Edits { get; } = new();

    /// <summary>The window's data as the last edit left it; what an edit changed is the difference.</summary>
    JsonObject? baseline;

    /// <summary>The database step the edit that runs made, 0 when none.</summary>
    long editStep;

    /// <summary>Whether an edit's handler runs, whose changes the edit keeps.</summary>
    bool editing;

    /// <summary>Whether an undo puts data back, which no part's change makes an edit.</summary>
    bool applying;

    /// <summary>The scope of the part that had the focus last, for an Undo in a menu.</summary>
    public string? FocusedScope { get; set; }

    /// <summary>Names of a part's own state (an unbound input's text) start so; no name of the window's data does.</summary>
    const string PartState = "#parts.";

    /// <summary>What code sets on a part that an edit compares: the name in an edit, and the field of the part's state.</summary>
    static readonly (string Field, string Key)[] CodeSet = [("caption", "caption"), ("visible", "visible"), ("enabled", "enabled"), ("readonly", "read_only"), ("error", "error")];

    /// <summary>A custom part's values among the names of a part: <c>board.values.count</c>.</summary>
    const string ValuesOf = ".values.";

    /// <summary>Names of the person's settings in an edit start so (<c>#settings.compact</c>), each setting by itself; no name of the window's data has a #.</summary>
    const string SettingsState = "#settings.";

    /// <summary>The person's settings, which parts may change (bound to <c>settings.x</c>); none in a window without them.</summary>
    protected virtual ISettings? AppSettings => null;

    /// <summary>The window's data, and the parts' own text and ticks, as an edit compares them.</summary>
    JsonObject DataNow()
    {
        var of = Snapshot.Of(this);
        var now = of["data"]!.AsObject();
        foreach (var (name, node) in of["parts"]!.AsObject())
        {
            if (node is not JsonObject part)
                continue;
            var own = new JsonObject();
            foreach (var key in new[] { "text", "checked" })
            {
                if (part[key] is { } value)
                    own[key] = value.DeepClone();
            }
            if (own.Count > 0)
                now[PartState + name] = own;
            // What code set on the part, each by its own name (conformance/edit-bus.json, states).
            foreach (var (field, key) in CodeSet)
            {
                if (part[key] is { } value)
                    now[$"{PartState}{name}.{field}"] = value.DeepClone();
            }
        }
        // Each value code set on a custom part, by its own name, with its type so it comes back as it was.
        foreach (var (name, part) in Snapshot.PartsOf(this))
        {
            foreach (var (value, set) in part.Values)
                now[$"{PartState}{name}{ValuesOf}{value}"] = set is null ? null : JsonSerializer.SerializeToNode(set, typeof(object), Snapshot.Options);
        }
        // A part bound to a setting changes the settings, not the window's data.
        // Each setting by itself, so an undo takes back only the settings its edit changed.
        if (AppSettings is { } settings)
        {
            foreach (var (name, value) in settings.ToJson())
                now[SettingsState + name] = value?.DeepClone();
        }
        return now;
    }

    /// <summary>An edit of <paramref name="part"/> ended: what it changed since the last one goes on the bus.</summary>
    public void Edited(string part, string scope)
    {
        // What a handler or an undo changed is no edit of the person's, though the bound parts follow it.
        if (running > 0 || applying)
            return;
        var now = DataNow();
        var changes = EditBus.Between(baseline ?? now, now);
        // The change handler of a value just changed joins its edit: one change, one Ctrl+Z.
        if (joining is { } joined && joined.Part == part && Edits.LastOf(part) == joined)
        {
            joined.Changes.AddRange(changes);
            if (editStep != 0)
                joined.Step = editStep;
        }
        else
        {
            var edit = new Edit(part, scope, changes, editStep);
            Edits.Push(edit);
            if (!editing)
            {
                joining = edit;
                Later.Post(() => joining = null);
            }
        }
        baseline = now;
        editStep = 0;
    }

    /// <summary>The edit a value change just made, which its change handler joins.</summary>
    Edit? joining;

    /// <summary>
    /// Runs the handler of an edit of a part with undo (a canvas's drag, drop
    /// or bend, a change): as one database step; <paramref name="ends"/> ends the edit.
    /// </summary>
    public void EditOf(string part, string scope, bool ends, Action handler)
    {
        baseline ??= DataNow();
        var before = Db?.LastStep ?? 0;
        editing = true;
        try
        {
            Handle(() =>
            {
                if (Db is null)
                    handler();
                else
                    Db.Step(handler);
            }, null, nameof(EditOf));
        }
        finally
        {
            editing = false;
        }
        if (Db is not null && Db.LastStep != before)
            editStep = Db.LastStep;
        if (ends)
            Edited(part, scope);
    }

    /// <summary>
    /// The person's action on a canvas with a mode (DESIGN 5.31): the runtime
    /// applies it to the window's own records, snapped to the canvas's grid
    /// (CanvasModes decides as every runtime does), chooses what it touched
    /// in <paramref name="part"/>'s selection, and runs <paramref name="changed"/>
    /// (`on change`): with undo one edit, which Ctrl+Z takes back whole. A
    /// refused action, and every action on a readonly canvas, runs nothing:
    /// false. The paths of <paramref name="args"/> name the window's
    /// properties (<c>Note.Picture</c>).
    /// </summary>
    public bool CanvasEvent(PartState part, string? scope, CanvasArgs args, JsonObject action, Action? changed, [CallerMemberName] string command = "")
    {
        if (args.Readonly || part.IsReadOnly || !part.IsEnabled || LongCall.Waiting)
            return false;
        Action? apply;
        try
        {
            apply = CanvasChange(part, args, action);
        }
        catch (ClarityException e)
        {
            Say(Level.Error, e.Said);
            return false;
        }
        if (apply is null)
            return false;
        void Run()
        {
            apply();
            changed?.Invoke();
        }
        if (scope is null)
            Handle(Run, [action], command);
        else
            EditOf(part.Name, scope, ends: true, Run);
        return true;
    }

    /// <summary>What an action on a canvas changes, to do; null when it is refused.</summary>
    Action? CanvasChange(PartState part, CanvasArgs args, JsonObject action)
    {
        switch (args.Mode)
        {
            case "objects":
                {
                    var (id, x, y) = CanvasModes.Move(action);
                    if (ValueOf(args.List) is not System.Collections.IList list)
                        throw new InvalidValueException($"the window has no list {args.List}");
                    foreach (var record in list)
                    {
                        if (record is null || !CanvasModes.SameId(Json(Member(record, "Id")), id))
                            continue;
                        if (CanvasModes.Placed(args, Number(Member(record, args.At[0])), Number(Member(record, args.At[1])), x, y) is not var (px, py))
                            return null;
                        return () =>
                        {
                            SetMember(record, args.At[0], px);
                            SetMember(record, args.At[1], py);
                            part.SelectedItem = record;
                        };
                    }
                    return null;
                }
            case "paint":
                {
                    // A stroke that is no picture is refused: the record never holds it.
                    if (!CanvasModes.IsPicture(action))
                        return null;
                    var picture = Picture.New(CanvasModes.StrokeBytes(action));
                    var (owner, field) = Owner(args.Path);
                    if (picture.Equals(Member(owner, field)))
                        return null;
                    return () =>
                    {
                        SetMember(owner, field, picture);
                        part.SelectedItem = null;
                    };
                }
            case "vector":
                {
                    var (owner, field) = Owner(args.Path);
                    if (CanvasModes.Vector(Member(owner, field) as string ?? "", args, action) is not { } drawn)
                        return null;
                    return () =>
                    {
                        SetMember(owner, field, drawn.Text);
                        // A place from 0, as a table's chosen row.
                        part.SelectedItem = drawn.Selected > 0 ? (long)(drawn.Selected - 1) : null;
                    };
                }
        }
        throw new InvalidValueException($"a canvas's mode is objects, paint or vector, not \"{args.Mode}\"");
    }

    static object? Member(object owner, string name) => owner.GetType().GetProperty(name)?.GetValue(owner);

    static void SetMember(object owner, string name, object? value)
    {
        if (owner.GetType().GetProperty(name) is not { CanWrite: true } p)
            throw new InvalidValueException($"{owner.GetType().Name} has no {name} to set");
        var type = Nullable.GetUnderlyingType(p.PropertyType) ?? p.PropertyType;
        p.SetValue(owner, value is null || type.IsInstanceOfType(value) ? value : Convert.ChangeType(value, type, System.Globalization.CultureInfo.InvariantCulture));
    }

    static double? Number(object? value) => value is IConvertible c and not string and not bool ? c.ToDouble(System.Globalization.CultureInfo.InvariantCulture) : null;

    static JsonNode? Json(object? value) => value is null ? null : System.Text.Json.JsonSerializer.SerializeToNode(value, value.GetType());

    /// <summary>The value at a path of the window's properties: <c>Boxes</c>, <c>Note.Picture</c>.</summary>
    object? ValueOf(string path)
    {
        object? at = this;
        foreach (var name in path.Split('.'))
            at = at is null ? null : Member(at, name);
        return at;
    }

    /// <summary>The record a path ends in, and its field.</summary>
    (object Owner, string Field) Owner(string path)
    {
        var dot = path.LastIndexOf('.');
        var owner = dot < 0 ? this : ValueOf(path[..dot]);
        return (owner ?? throw new InvalidValueException($"the window has no {path}"), path[(dot + 1)..]);
    }

    /// <summary>Takes back (<paramref name="back"/>) or does again the last edit of <paramref name="scope"/>.</summary>
    public void UndoEdit(string scope, bool back)
    {
        var edit = back ? Edits.Undo(scope) : Edits.Redo(scope);
        if (edit is null)
            return;
        try
        {
            if (edit.Step > 0 && Db is not null)
            {
                if (back)
                    Db.Undo(edit.Step);
                else
                    Db.Redo(edit.Step);
            }
        }
        catch (RefusedException e)
        {
            Edits.Refused(edit, back);
            Say(Level.Error, e.Text);
            return;
        }
        var data = new JsonObject();
        var parts = new JsonObject();
        JsonObject? settings = null;
        // Back from the last change to the first, so the first one's before is what stays; again from the first to the last.
        foreach (var changed in back ? Enumerable.Reverse(edit.Changes) : edit.Changes)
        {
            var value = (back ? changed.Before : changed.After)?.DeepClone();
            if (changed.Name.StartsWith(SettingsState, StringComparison.Ordinal))
            {
                settings ??= AppSettings?.ToJson();
                if (settings is not null)
                    settings[changed.Name[SettingsState.Length..]] = value;
            }
            else if (changed.Name.StartsWith(PartState, StringComparison.Ordinal))
                PutBack(parts, changed.Name[PartState.Length..], value);
            else
                data[changed.Name] = value;
        }
        applying = true;
        try
        {
            PutBackInPlace(data);
            Snapshot.Apply(this, new JsonObject { ["data"] = data });
            ApplyParts(parts);
            if (settings is not null)
                AppSettings?.Apply(settings);
        }
        finally
        {
            applying = false;
        }
        baseline = DataNow();
        Redraw();
    }

    /// <summary>
    /// A record the window shows that is still the same record (its type and
    /// id), alone or in a list of the same records, gets its values back in
    /// place, so a list that holds it keeps it selected and every part that
    /// shows it shows them; those names leave <paramref name="data"/>.
    /// </summary>
    void PutBackInPlace(JsonObject data)
    {
        foreach (var (name, node) in data.ToList())
        {
            var property = GetType().GetProperty(name);
            if (property?.GetValue(this) is not { } now || node is null)
                continue;
            var was = node.Deserialize(property.PropertyType, Snapshot.Options);
            var same = (now, was) switch
            {
                (System.Collections.IList rows, System.Collections.IList back) => rows.Count == back.Count
                    && Enumerable.Range(0, rows.Count).All(i => SameRecord(rows[i], back[i])),
                _ => SameRecord(now, was),
            };
            if (!same)
                continue;
            if (now is System.Collections.IList list && was is System.Collections.IList earlier)
            {
                for (var i = 0; i < list.Count; i++)
                    CopyValues(earlier[i]!, list[i]!);
            }
            else
                CopyValues(was!, now);
            data.Remove(name);
        }
    }

    /// <summary>Whether two values are the same stored record: one type with a long Id, the same id.</summary>
    static bool SameRecord(object? now, object? was) =>
        now is not null && was is not null && now.GetType() == was.GetType() && !now.GetType().IsValueType
        && now.GetType().GetProperty("Id") is { PropertyType: var type } id && type == typeof(long)
        && Equals(id.GetValue(now), id.GetValue(was));

    /// <summary>Gives <paramref name="into"/> every value of <paramref name="from"/> it can be given.</summary>
    static void CopyValues(object from, object into)
    {
        foreach (var field in into.GetType().GetProperties(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public))
        {
            if (field.CanRead && field.GetSetMethod() is { } set && field.GetIndexParameters().Length == 0)
                set.Invoke(into, [field.GetValue(from)]);
        }
    }

    /// <summary>One name of a part's state an undo puts back, into <paramref name="parts"/> by part: its own text and tick, a field code set, or a custom part's value.</summary>
    static void PutBack(JsonObject parts, string name, JsonNode? value)
    {
        var dot = name.IndexOf('.');
        var part = dot < 0 ? name : name[..dot];
        var into = parts[part] as JsonObject ?? new JsonObject();
        parts[part] = into;
        if (dot < 0)
        {
            foreach (var (key, own) in value as JsonObject ?? [])
                into[key] = own?.DeepClone();
        }
        else if (name[dot..].StartsWith(ValuesOf, StringComparison.Ordinal))
        {
            var values = into["values"] as JsonObject ?? new JsonObject();
            into["values"] = values;
            values[name[(dot + ValuesOf.Length)..]] = value;
        }
        else
        {
            // A field that was not set goes back to the design: ApplyParts reads null so.
            into[name[(dot + 1)..]] = value;
        }
    }

    /// <summary>Gives the parts what an undo put back, only those fields: the rest of each part stays as it is.</summary>
    void ApplyParts(JsonObject parts)
    {
        var all = Snapshot.PartsOf(this).ToDictionary(p => p.Name, p => p.Part);
        foreach (var (name, node) in parts)
        {
            if (!all.TryGetValue(name, out var part) || node is not JsonObject sent)
                continue;
            T Get<T>(string key, T design) => sent[key] is { } value ? value.Deserialize<T>(Snapshot.Options) ?? design : design;
            foreach (var (key, value) in sent)
            {
                switch (key)
                {
                    case "text": part.Text = value?.GetValue<string>() ?? ""; break;
                    case "checked": part.IsChecked = value?.GetValue<bool>() ?? false; break;
                    case "visible": part.IsVisible = Get("visible", true); break;
                    case "enabled": part.IsEnabled = Get("enabled", true); break;
                    case "readonly": part.IsReadOnly = Get("readonly", false); break;
                    case "caption": part.CaptionText = Get("caption", Text.Of("")); break;
                    case "error": part.ErrorText = Get("error", Text.Plain("")); break;
                    case "values":
                        foreach (var (set, was) in value as JsonObject ?? [])
                            part.SetValue(set, was?.Deserialize<object>(Snapshot.Options));
                        break;
                }
            }
        }
    }

    /// <summary><c>-> undo</c> and <c>-> redo</c>: the last edit of the part that had the focus last.</summary>
    protected void UndoFocused(bool back)
    {
        if (FocusedScope is { } scope)
            UndoEdit(scope, back);
    }

    /// <summary>Asks the shell to put a text on the clipboard: <c>copy text</c>.</summary>
    public event Action<Text>? CopyRequested;

    /// <summary><c>copy text</c>: the text, as it shows, goes on the clipboard; the last copy counts.</summary>
    public void Copy(Text text) => CopyRequested?.Invoke(text);

    /// <summary>
    /// Runs the app's <c>on unconnected</c> on this window, which is not shown
    /// (DESIGN 5.27): what it says and the windows it opens go to <paramref name="unconnecting"/>.
    /// </summary>
    public void RunUnconnected(Unconnecting unconnecting, Action<string> handler)
    {
        Messages = unconnecting.Said;
        OpenRequested += (other, _) => unconnecting.Open(other);
        Handle(() => handler(unconnecting.Reason));
    }

    /// <summary>Windows a running handler asked to open, and whether it asked to close this one.</summary>
    readonly List<(WindowViewModel Window, bool NewWindow)> opening = [];
    readonly List<Question> asking = [];

    /// <summary>For a dialog asked for: runs the handler of the window that asked, with what it closes with.</summary>
    Action<object?>? whenClosedWith;
    object? closedWith;
    bool closingWith;
    bool closing;
    int running;

    /// <summary>
    /// Runs a handler; a refusal or another failure is shown as an error message.
    /// What it asks of windows happens when it is done, on every stack alike:
    /// this window closes first, then the windows it opened open. False when
    /// it refused or failed, so a wizard keeps its step.
    /// </summary>
    public bool Handle(Action handler, object?[]? args = null, [CallerMemberName] string command = "")
    {
        // While the screen waits for a long engine call of these windows, it takes no other handler.
        if (LongCall.Waiting)
            return false;
        // A required field the last save refused is checked again by the next one.
        if (running == 0)
            ClearRequired();
        running++;
        var ran = false;
        try
        {
            // The outermost handler is watched: its engine calls show how far they are.
            // A failure that comes wrapped (a task's) is caught as itself.
            try
            {
                if (running == 1)
                    Watching.Run(LongCall.Start(), handler);
                else
                    handler();
            }
            catch (AggregateException wrapped) when (wrapped.InnerExceptions.Count == 1)
            {
                System.Runtime.ExceptionServices.ExceptionDispatchInfo.Throw(wrapped.InnerExceptions[0]);
            }
            ran = true;
            // The canvases show what the handler left, drawn anew.
            if (running == 1)
            {
                stale = false;
                Redraw();
            }
        }
        catch (RefusedException e)
        {
            if (!ShownAtInput(e))
                Say(Level.Error, e.Text);
        }
        catch (ClarityException e)
        {
            Say(Level.Error, e.Said);
        }
        catch (Exception e)
        {
            // Fail safe: an error nobody foresaw says why, and the window keeps working.
            Console.Error.WriteLine($"{command} failed: {e}");
            Say(Level.Error, Text.Plain(e.Message));
        }
        finally
        {
            running--;
            if (running == 0)
                LongCall.End();
        }
        if (running > 0)
            return ran;
        // What a handler that is no edit changed is not taken back: the edits start from here.
        if (!editing)
            baseline = DataNow();
        var close = closing;
        var open = opening.ToList();
        var asks = asking.ToList();
        closing = false;
        opening.Clear();
        asking.Clear();
        if (close)
            Closed();
        foreach (var (other, newWindow) in open)
            OpenRequested?.Invoke(other, newWindow);
        // The questions it asked, after what changed is on screen.
        foreach (var question in asks)
            AskRequested?.Invoke(question);
        return ran;
    }

    /// <summary>The inputs that show why a save was refused, with the error each was given.</summary>
    readonly List<(PartState Part, Text Error)> requiredShown = [];

    /// <summary>
    /// Shows a refused required field at each input bound to that field of
    /// the refused record, as a form check would. False when no input shows it.
    /// </summary>
    bool ShownAtInput(RefusedException refused)
    {
        if (refused is not { Column: { } column, Record: { } record })
            return false;
        var shown = false;
        foreach (var (_, part) in Snapshot.PartsOf(this))
        {
            if (part.Field is not { } field || field.Column != column)
                continue;
            var data = GetType().GetProperty(char.ToUpperInvariant(field.Data[0]) + field.Data[1..]);
            if (data is null || !ReferenceEquals(data.GetValue(this), record))
                continue;
            part.ErrorText = refused.Text;
            requiredShown.Add((part, refused.Text));
            shown = true;
        }
        return shown;
    }

    /// <summary>Takes away what <see cref="ShownAtInput"/> showed, unless code has set another error since.</summary>
    void ClearRequired()
    {
        foreach (var (part, error) in requiredShown)
        {
            if (ReferenceEquals(part.ErrorText, error))
                part.ErrorText = Text.Plain("");
        }
        requiredShown.Clear();
    }

    /// <summary>
    /// Asks <paramref name="question"/> in Clarity's own dialog when the
    /// handler has finished; <paramref name="answered"/> runs the handler the
    /// answer goes to. Cancelling runs nothing.
    /// </summary>
    protected void Ask(Text question, AskForm form, Action<object?> answered)
    {
        var asked = new Question(question, form, answered);
        if (running > 0)
            asking.Add(asked);
        else
            AskRequested?.Invoke(asked);
    }

    /// <summary>The person answered: the handler the answer goes to runs, as any handler does.</summary>
    public void Answer(Question question, object? answer) => Handle(() => question.Answered(answer));

    /// <summary>A picture or a file of the database, fetched by <paramref name="here"/>.</summary>
    protected Task<byte[]?> Fetched(string fetcher, Func<byte[]?> here, params object?[] args) => Task.FromResult(here());

    /// <summary>The row selected in a table or list; with no row chosen, an
    /// empty record (id 0), as <c>new</c> makes, so code can ask first.</summary>
    protected static R Selected<R>(PartState part) where R : class, new() =>
        part.SelectedItem as R ?? new R();

    /// <summary>The row of <paramref name="rows"/> that code selects with a record
    /// (DESIGN 5.12): the same one, else the first holding the same values; null for none.</summary>
    protected static R? SameRow<R>(IEnumerable<R> rows, R? wanted) where R : class
    {
        if (wanted is null)
            return null;
        var list = rows.ToList();
        if (list.FirstOrDefault(r => ReferenceEquals(r, wanted)) is { } same)
            return same;
        var values = JsonSerializer.SerializeToNode(wanted, JsonValues.Options);
        return list.FirstOrDefault(r => JsonNode.DeepEquals(JsonSerializer.SerializeToNode(r, JsonValues.Options), values));
    }

    /// <summary>
    /// Opens a window: in place, on top of this one, or in a desktop window of
    /// its own when the design marks it (`in new window`).
    /// </summary>
    protected void Open(WindowViewModel other, bool newWindow = false)
    {
        if (running > 0)
            opening.Add((other, newWindow));
        else
            OpenRequested?.Invoke(other, newWindow);
    }

    protected void Close()
    {
        if (running > 0)
            closing = true;
        else
            Closed();
    }

    /// <summary>Closes this dialog with <paramref name="value"/> (<c>close with value</c>), which the handler that asked for it gets.</summary>
    protected void CloseWith(object? value)
    {
        closedWith = value;
        closingWith = true;
        Close();
    }

    /// <summary>The window closes; a dialog that closed with a value then gives it to the window that asked for it.</summary>
    void Closed()
    {
        CloseRequested?.Invoke();
        if (!closingWith)
            return;
        closingWith = false;
        whenClosedWith?.Invoke(closedWith);
    }

    /// <summary>
    /// Opens the dialog <paramref name="dialog"/> (<c>ask PickFriend then lend(album)</c>);
    /// what it closes with runs <paramref name="answered"/> on this window, as any
    /// handler runs. Closed any other way, it runs nothing.
    /// </summary>
    protected void Ask(WindowViewModel dialog, Action<object?> answered)
    {
        dialog.whenClosedWith = value => Handle(() => answered(value));
        Open(dialog);
    }

    /// <summary>Ends the application: the built-in <c>quit</c>.</summary>
    protected static void QuitApplication() => DesktopProgram.Current?.Quit();

    /// <summary>Opens Clarity's keys window over the window the person works in: the built-in <c>keys</c> (DESIGN 5.28).</summary>
    protected static void ShowKeys() => DesktopProgram.Current?.ShowKeys();

    /// <summary>The window has opened with its data.</summary>
    public void Opened() => Handle(OnOpen);

    /// <summary>The window is about to close: its on close runs, then it is reported closed.</summary>
    public void Closing()
    {
        Handle(OnClose);
        OpenWindows.Close(Instance);
    }

    /// <summary>A window this one opened has closed: the place to reload a list.</summary>
    public void Returned() => Handle(OnReturn);

    /// <summary>Whether the window has canvases its draw handlers draw: its Redraw is its own.</summary>
    bool Draws => drawsByType.GetOrAdd(GetType(), static type =>
        type.GetMethod(nameof(Redraw), System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic, Type.EmptyTypes)?.DeclaringType != typeof(WindowViewModel));

    static readonly System.Collections.Concurrent.ConcurrentDictionary<Type, bool> drawsByType = new();

    /// <summary>Whether the person changed what the window shows and its canvases were not drawn anew since.</summary>
    bool stale;

    /// <summary>
    /// The person changed a value the window shows (typed in a box bound to a
    /// setting, ticked a box) and no handler runs: once the work at hand is
    /// done the canvases draw anew, as after a handler, since a draw handler
    /// may read it. A handler that runs first (the part's on change) draws
    /// them itself, and nothing more is drawn.
    /// </summary>
    public void Changed()
    {
        if (running > 0 || applying || stale || !Draws)
            return;
        stale = true;
        Later.Post(() =>
        {
            if (stale)
                DrawAnew();
        });
    }

    /// <summary>
    /// Draws the canvases anew with what the window shows now, as after a
    /// handler; a failing draw handler says why.
    /// </summary>
    public void DrawAnew()
    {
        stale = false;
        if (running > 0 || LongCall.Waiting)
            return;
        running++;
        try
        {
            Redraw();
        }
        catch (RefusedException e)
        {
            Say(Level.Error, e.Text);
        }
        catch (ClarityException e)
        {
            Say(Level.Error, e.Said);
        }
        catch (Exception e)
        {
            // Fail safe: a draw handler that fails says why, and the window keeps working.
            Console.Error.WriteLine($"{nameof(DrawAnew)} failed: {e}");
            Say(Level.Error, Text.Plain(e.Message));
        }
        finally
        {
            running--;
        }
    }

    /// <summary>
    /// Draws the window's canvases anew, each by its draw handler on an
    /// empty drawing (conformance/drawing.json): after every handler, when
    /// the window opens, and after a change the person made (Changed).
    /// </summary>
    protected virtual void Redraw() { }

    protected virtual void OnOpen() { }

    protected virtual void OnClose() { }

    protected virtual void OnReturn() { }
}
