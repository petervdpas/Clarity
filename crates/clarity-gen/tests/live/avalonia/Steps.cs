// The steps a person takes in the every-part project on Avalonia, the same as
// tests/live/drive.mjs takes in a browser: each does what a person does (key
// presses, typed text, pointer presses and drags) and then reads the window
// and the database. A step that fails is reported and the next one runs, so
// one run shows everything that is wrong. Only a file chosen in the system's
// file dialog is handed to its part where the dialog would: a headless
// window has no system dialog.

using System.Diagnostics;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.LogicalTree;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Clarity.Avalonia;

namespace Everything.Live;

public sealed class Steps(Window main, string database, string fixture)
{
    readonly List<string> failed = [];

    public int Run()
    {
        // Tall enough that every part of a window is on screen without scrolling.
        main.Width = 1400;
        main.Height = 2000;
        Jobs();
        Albums();
        Gallery();
        Folders();
        Views();
        Friends();
        Console.WriteLine(failed.Count > 0 ? $"\n{failed.Count} failed on csharp avalonia: {string.Join("; ", failed)}" : "\nall steps passed on csharp avalonia");
        return failed.Count > 0 ? 1 : 0;
    }

    /// <summary>Records to work with, as a person would have them, put in before the app starts.</summary>
    public static void Seed(string database) => SqlOn(database, """
            INSERT INTO album (title, position, copies) VALUES ('Alpha', 1, 1), ('Beta', 2, 2), ('Gamma', 3, 3);
            INSERT INTO folder (name, position, size, parent) VALUES ('Music', 1, 10, NULL), ('Docs', 2, 20, NULL);
            INSERT INTO folder (name, position, size, parent) VALUES ('Rock', 1, 5, (SELECT id FROM folder WHERE name = 'Music'));
            INSERT INTO friend (name, city, position) VALUES ('Ann', 'Oslo', 1), ('Bob', 'Rome', 2), ('Cid', 'Lima', 3);
            """);

    // ---- The albums window: every input, questions, a wizard, menus.
    void Albums()
    {
        Step("a form's fields are taken back one by one, the last first", () =>
        {
            Edit("albumTitleBox", "Zed");
            Edit("albumCopiesBox", "4");
            Same(Text("albumTitleBox"), "Zed", "title typed");
            Same(Text("albumCopiesBox"), "4", "copies typed");
            Click(Part("albumTitleBox"));
            Undo();
            Same(Text("albumCopiesBox"), "0", "copies taken back first");
            Same(Text("albumTitleBox"), "Zed", "title still there");
            Undo();
            Same(Text("albumTitleBox"), "", "title taken back next");
            Redo();
            Same(Text("albumTitleBox"), "Zed", "title done again");
        });

        Step("a toggle's press and what its handler saved are taken back together", () =>
        {
            var before = int.Parse(Sql("SELECT count(*) FROM album"));
            Click(Part("favouriteToggle"));
            Same(int.Parse(Sql("SELECT count(*) FROM album")), before + 1, "favouriteChanged saved the album");
            Undo();
            Same(int.Parse(Sql("SELECT count(*) FROM album")), before, "the saved album is taken back");
        });

        Step("a spin box is an edit", () =>
        {
            Click(Part("copiesSpin"));
            Key(PhysicalKey.ArrowUp);
            Key(PhysicalKey.Tab);
            var up = Text("copiesSpin");
            Click(Part("copiesSpin"));
            Undo();
            Check(Text("copiesSpin") != up, $"taken back (was {up}, now {Text("copiesSpin")})");
        });

        Step("a switch is an edit", () =>
        {
            var toggle = (ToggleButton)Part("toolbarSwitch");
            var was = toggle.IsChecked;
            Click(toggle);
            Same(toggle.IsChecked, !was, "switched");
            Undo();
            Same(toggle.IsChecked, was, "taken back");
        });

        Step("a dropdown of records is an edit", () =>
        {
            var box = Inner<ComboBox>("favouriteFriendBox");
            // Tabbed to, and Down picks the next choice: the first friend.
            box.Focus();
            Key(PhysicalKey.ArrowDown);
            Check(box.SelectedIndex > 0, $"a friend picked (index {box.SelectedIndex})");
            Undo();
            Check(box.SelectedIndex <= 0, $"taken back (index {box.SelectedIndex})");
        });

        Step("a color typed is an edit", () =>
        {
            Edit("tintBox", "#ff0000");
            Same(Text("tintBox"), "#ff0000", "typed");
            Click(Inner<TextBox>("tintBox"));
            Undo();
            Same(Text("tintBox"), "#3b82f6", "taken back");
        });

        Step("a slider moved with the keys is an edit", () =>
        {
            var slider = Inner<Slider>("ratingSlider");
            slider.Focus();
            Key(PhysicalKey.ArrowRight);
            Check(slider.Value > 0, $"moved ({slider.Value})");
            Undo();
            Same(slider.Value, 0.0, "taken back");
        });

        Step("a tag added is an edit", () =>
        {
            Click(Inner<TextBox>("tagsBox"));
            Type("live");
            Key(PhysicalKey.Enter);
            Check(Texts(Part("tagsBox")).Contains("live"), "the tag shows");
            Undo();
            Check(!Texts(Part("tagsBox")).Contains("live"), "taken back");
        });

        Step("a typed dropdown is an edit", () =>
        {
            Edit("cityBox", "Oslo");
            Same(Text("cityBox"), "Oslo", "typed");
            Click(Inner<TextBox>("cityBox"));
            Undo();
            Same(Text("cityBox"), "", "taken back");
        });

        Step("a file chosen is an edit", () =>
        {
            var file = (FileInput)Part("bookletBox");
            // A press on Choose a file, then where the system's file dialog would hand it over.
            file.GetVisualDescendants().OfType<Button>().First(b => b.IsEffectivelyEnabled).Focus();
            file.Take("notes.pdf", "%PDF-1.4\n%%EOF\n"u8.ToArray());
            Jobs();
            Check(Texts(file).Contains("notes.pdf"), "the file shows");
            file.GetVisualDescendants().OfType<Button>().First().Focus();
            Undo();
            Check(!Texts(file).Contains("notes.pdf"), "taken back");
        });

        Step("an accordion opens another section", () =>
        {
            Click(Labelled(Part("notesAccordion"), "Site"));
            Check(Part("siteText").IsEffectivelyVisible, "the site section shows");
        });

        Step("a question for a number sets what it answers", () =>
        {
            Click(Part("copiesButton"));
            var dialog = Dialog();
            var input = dialog.GetVisualDescendants().OfType<TextBox>().First();
            input.Focus();
            Key(PhysicalKey.A, RawInputModifiers.Control);
            Type("7");
            Key(PhysicalKey.Enter);
            Same(Texts(Part("copiesBadge")).Trim(), "7", "the copies");
            Check(main.OwnedWindows.Count == 0, "the question closed");
        });

        Step("a wizard refuses an empty title, then saves the album", () =>
        {
            var before = int.Parse(Sql("SELECT count(*) FROM album"));
            Click(Part("newButton"));
            var dialog = Dialog();
            var wizard = Part("wizard", dialog);
            Click(Labelled(wizard, "Next"));
            Same(SaidError(), "A title first", "the refusal said");
            Check(Part("titleBox", dialog).IsEffectivelyVisible, "still on the first step");
            Click(Part("titleBox", dialog));
            Type("Delta");
            Key(PhysicalKey.Tab);
            Click(Labelled(wizard, "Next"));
            Click(Labelled(wizard, "Next"));
            Click(Labelled(wizard, "Finish"));
            Same(int.Parse(Sql("SELECT count(*) FROM album")), before + 1, "saved");
            Same(Sql("SELECT count(*) FROM album WHERE title = 'Delta'"), "1", "the title");
        });

        Step("a yes-or-no question deletes on yes", () =>
        {
            Click(Part("deleteButton"));
            var dialog = Dialog();
            Click(Labelled(dialog, "Delete"));
            Same(Sql("SELECT count(*) FROM album WHERE title = 'Delta'"), "0", "deleted");
        });
        // The steps after work on the albums they started with, whatever happened above.
        Sql("DELETE FROM album WHERE title NOT IN ('Alpha', 'Beta', 'Gamma')");

        Step("a split button opens its menu", () =>
        {
            var split = (SplitButton)Part("saveButton");
            Click(split.GetVisualDescendants().OfType<Control>().First(c => c.Name == "PART_SecondaryButton"));
            Check(split.Flyout?.IsOpen == true, "the menu shows");
            split.Flyout?.Hide();
        });

        Step("a menu item's key opens its window", () =>
        {
            Key(PhysicalKey.F, RawInputModifiers.Control);
            Check(Find("folderTree") is not null, "Ctrl+F opened the folders");
            Click(Part("albumsButton"));
        });
    }

    // ---- The gallery: a list that reorders, a panel, a no-undo input, a picture.
    void Gallery()
    {
        Step("the gallery opens with the albums in their order", () =>
        {
            Click(Part("galleryButton"));
            Same(Rows("albumList"), ["Alpha | 1", "Beta | 2", "Gamma | 3"], "albums");
        });

        Step("Alt+Down moves a list row, saved; Ctrl+Z takes it back, Ctrl+Y does it again", () =>
        {
            ClickRow("albumList", 0);
            Key(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
            Same(Rows("albumList"), ["Beta | 2", "Alpha | 1", "Gamma | 3"], "moved down");
            Same(SqlRows("SELECT title FROM album ORDER BY position"), ["Beta", "Alpha", "Gamma"], "saved order");
            Undo();
            Same(Rows("albumList"), ["Alpha | 1", "Beta | 2", "Gamma | 3"], "taken back");
            Same(SqlRows("SELECT title FROM album ORDER BY position"), ["Alpha", "Beta", "Gamma"], "saved order taken back");
            Redo();
            Same(SqlRows("SELECT title FROM album ORDER BY position"), ["Beta", "Alpha", "Gamma"], "done again");
            Undo();
        });

        Step("a dragged list row lands where it is dropped, and stays after reopening", () =>
        {
            var rows = RowControls("albumList");
            Drag(rows[2], 0.5, rows[0], 0.2);
            Same(Rows("albumList"), ["Gamma | 3", "Alpha | 1", "Beta | 2"], "dragged to the top");
            Same(SqlRows("SELECT title FROM album ORDER BY position"), ["Gamma", "Alpha", "Beta"], "saved");
            Click(Part("foldersButton"));
            Click(Part("galleryButton"));
            Same(Rows("albumList"), ["Gamma | 3", "Alpha | 1", "Beta | 2"], "order after reopening");
        });

        Step("a panel's fields are taken back one by one", () =>
        {
            DoubleClick(RowControls("albumList")[1]);
            Edit("priceBox", "12");
            Click(Inner<TextBox>("notesMemo"));
            Type("note");
            Click(Part("formatRadio"));
            Same(Text("notesMemo"), "note", "memo typed");
            Click(Inner<TextBox>("notesMemo"));
            Undo();
            Same(Text("notesMemo"), "", "memo taken back first");
            Check(Text("priceBox").StartsWith("12"), $"price still there ({Text("priceBox")})");
            Undo();
            Check(!Text("priceBox").StartsWith("12"), $"price taken back ({Text("priceBox")})");
        });

        Step("a timer keeps ticking in a window under another, without an error", () =>
        {
            var end = DateTime.Now.AddSeconds(2.5);
            while (DateTime.Now < end)
            {
                Jobs();
                Thread.Sleep(50);
            }
        });

        Step("a radio choice is an edit", () =>
        {
            var radios = Part("formatRadio").GetVisualDescendants().OfType<RadioButton>().ToList();
            Click(radios[1]);
            Same(radios[1].IsChecked, true, "CD chosen");
            Undo();
            Same(radios[0].IsChecked, true, "taken back");
        });

        Step("a date typed is an edit", () =>
        {
            var picker = (CalendarDatePicker)Part("releasedOn");
            Click(picker.GetVisualDescendants().OfType<TextBox>().First());
            Type("2/14/2024");
            Key(PhysicalKey.Tab);
            Check(picker.SelectedDate is not null, "typed");
            Click(picker.GetVisualDescendants().OfType<TextBox>().First());
            Undo();
            Check(picker.SelectedDate is null, $"taken back ({picker.SelectedDate})");
        });

        Step("a password and a folder typed are edits", () =>
        {
            Edit("tokenBox", "s3cret");
            Click(Inner<TextBox>("tokenBox"));
            Undo();
            Same(Text("tokenBox"), "", "password taken back");
            Edit("scansBox", "papers");
            Same(Text("scansBox"), "papers", "folder typed");
            Click(Inner<TextBox>("scansBox"));
            Undo();
            Same(Text("scansBox"), "scans", "folder taken back");
        });

        Step("a picture loaded into an image is an edit", () =>
        {
            Click(Labelled(Part("galleryTabs"), "Cover"));
            var picture = Part("coverImage").GetVisualDescendants().OfType<PictureBox>().FirstOrDefault() ?? (PictureBox)Part("coverImage");
            // A press on Load a picture, then where the system's file dialog would hand it over.
            picture.GetVisualDescendants().OfType<Button>().FirstOrDefault(b => b.IsEffectivelyEnabled)?.Focus();
            Check(picture.Load(File.ReadAllBytes(Path.Combine(fixture, "resources/banner.png"))), "the picture loads");
            Jobs();
            Check(picture.Picture is not null, "the picture shows");
            picture.GetVisualDescendants().OfType<Button>().FirstOrDefault()?.Focus();
            Undo();
            Check(picture.Picture is null, "taken back");
            Click(Labelled(Part("galleryTabs"), "Albums"));
        });

        Step("a drag on a canvas and its undo say nothing wrong", () =>
        {
            var dial = Part("dial");
            Drag(dial, 0.3, dial, 0.5, dx: 60);
            dial.Focus();
            Undo();
        });

        // ---- Canvas modes (DESIGN 5.31): each gesture is one edit the runtime makes.
        List<string> Places() => SqlRows("SELECT id || ':' || copies || ',' || position FROM album ORDER BY id");
        Step("an object dragged on a board lands on the grid, saved; Ctrl+Z takes it back", () =>
        {
            var board = (CanvasView)Part("board");
            var before = Places();
            // The object drawn last is on top.
            var top = board.Drawing.OfType<Shape.Object>().LastOrDefault();
            Check(top is not null, "the board shows its objects");
            var at = new Point(top!.X + top.Width / 2, top.Y + top.Height / 2);
            DragUnits(board, at, at + new Vector(150, 75));
            var after = Places();
            var moved = after.Where((row, i) => row != before[i]).ToList();
            Check(moved.Count == 1, $"one object moved: {string.Join(" ", moved)} (mode {board.Mode}, command {board.ActionCommand is not null}, readonly {board.IsReadOnly}, pressed at {at}, top object {top})");
            var xy = moved[0].Split(':')[1].Split(',').Select(int.Parse).ToArray();
            Check(xy[0] % 20 == 0 && xy[1] % 20 == 0, $"snapped to the grid of 20: {xy[0]}, {xy[1]}");
            board.Focus();
            Undo();
            Same(Places(), before, "Ctrl+Z puts it back");
        });

        string Covers() => Sql("SELECT count(*) FROM album WHERE cover IS NOT NULL AND cover != ''");
        Step("a stroke painted on a picture is saved; Ctrl+Z takes it back", () =>
        {
            var sketch = (CanvasView)Part("sketch");
            var before = Covers();
            DragUnits(sketch, new Point(sketch.UnitsWidth * 0.3, sketch.UnitsHeight * 0.5), new Point(sketch.UnitsWidth * 0.6, sketch.UnitsHeight * 0.55));
            Same(Covers(), (int.Parse(before) + 1).ToString(), "the picture saved");
            sketch.Focus();
            Undo();
            Same(Covers(), before, "Ctrl+Z takes the stroke back");
        });

        string Drawn() => Sql("SELECT count(*) FROM album WHERE notes LIKE '%<rect %'");
        Step("a rectangle drawn on a vector drawing is saved as SVG; Delete takes it away, Ctrl+Z brings it back", () =>
        {
            var drawing = (CanvasView)Part("drawing");
            Same(Drawn(), "0", "no rectangle yet");
            Click(drawing.ToolButtons.First(b => Equals(b.Tag, "rect")));
            var from = new Point(drawing.UnitsWidth * 0.3, drawing.UnitsHeight * 0.4);
            var to = new Point(drawing.UnitsWidth * 0.5, drawing.UnitsHeight * 0.6);
            DragUnits(drawing, from, to);
            Same(Drawn(), "1", "the rectangle saved");
            Click(drawing.ToolButtons.First(b => Equals(b.Tag, "select")));
            ClickUnits(drawing, new Point((from.X + to.X) / 2, (from.Y + to.Y) / 2));
            drawing.Focus();
            Key(PhysicalKey.Delete);
            Same(Drawn(), "0", "Delete takes it away");
            drawing.Focus();
            Undo();
            Same(Drawn(), "1", "Ctrl+Z brings it back");
        });

        Step("a sound plays and an engine call answers", () =>
        {
            Click(Part("playButton"));
            Click(Part("planButton"));
            Settle(800);
        });

        Step("a tab shows its markdown", () =>
        {
            Click(Labelled(Part("galleryTabs"), "About"));
            Check(Texts(Part("aboutText")).Trim().Length > 0, "the text shows");
            Click(Labelled(Part("galleryTabs"), "Albums"));
        });

        Step("an input with no undo keeps what was typed", () =>
        {
            Edit("contactBox", "ann@example.com");
            Click(Inner<TextBox>("contactBox"));
            Undo();
            Same(Text("contactBox"), "ann@example.com", "kept");
        });
    }

    // ---- Folders: a tree that reorders, and a tree table that reorders without undo.
    void Folders()
    {
        Step("a tree row moves down, and Ctrl+Z takes it back", () =>
        {
            Click(Part("foldersButton"));
            Same(Rows("folderTree"), ["Music", "Docs"], "the top level");
            ClickRow("folderTree", 0);
            Key(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
            Same(SqlRows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Docs", "Music"], "saved order");
            Undo();
            Same(SqlRows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Music", "Docs"], "taken back");
        });

        Step("Alt+Right puts a tree row under the row before it; Ctrl+Z takes it out again", () =>
        {
            ClickRow("folderTree", Rows("folderTree").IndexOf("Docs"));
            Key(PhysicalKey.ArrowRight, RawInputModifiers.Alt);
            Same(Sql("SELECT p.name FROM folder f JOIN folder p ON p.id = f.parent WHERE f.name = 'Docs'"), "Music", "Docs under Music");
            Check(Rows("folderTree").Contains("Docs"), "Docs shows, its parent open");
            Undo();
            Same(Sql("SELECT count(*) FROM folder WHERE name = 'Docs' AND parent IS NULL"), "1", "Docs back at the top");
        });

        Step("a tree row dragged onto another goes under it", () =>
        {
            var rows = RowControls("folderTree");
            var names = Rows("folderTree");
            Drag(rows[names.IndexOf("Docs")], 0.5, rows[names.IndexOf("Music")], 0.5);
            Same(Sql("SELECT p.name FROM folder f JOIN folder p ON p.id = f.parent WHERE f.name = 'Docs'"), "Music", "dropped into Music");
            Undo();
            Same(Sql("SELECT count(*) FROM folder WHERE name = 'Docs' AND parent IS NULL"), "1", "taken back");
        });

        Step("a tree table that reorders with no undo keeps its move", () =>
        {
            ClickRow("folderTable", 0);
            Key(PhysicalKey.ArrowDown, RawInputModifiers.Alt);
            Same(SqlRows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Docs", "Music"], "moved");
            Undo();
            Same(SqlRows("SELECT name FROM folder WHERE parent IS NULL ORDER BY position"), ["Docs", "Music"], "kept: no undo");
        });
    }

    // ---- What a table and a tree show, a page at a time, sorted, opened.
    void Views()
    {
        Step("a table shows its first order a page at a time, and turns pages", () =>
        {
            Same(Rows("albumGrid"), ["Alpha | 1", "Beta | 2"], "the first page, by title");
            Click(((ClarityTable)Part("albumGrid")).Next);
            Same(Rows("albumGrid"), ["Gamma | 3"], "the second page");
        });

        Step("a header click sorts the other way, from the first page", () =>
        {
            var header = Part("albumGrid").GetVisualDescendants().OfType<DataGridColumnHeader>().First(h => Texts(h).Contains("Title"));
            Click(header);
            Same(Rows("albumGrid"), ["Gamma | 3", "Beta | 2"], "sorted the other way");
        });

        Step("a tree row's toggle opens and closes the rows under it", () =>
        {
            bool Toggle()
            {
                var music = RowControls("folderTree").First(r => Texts(r).Contains("Music"));
                Click(music.GetVisualDescendants().OfType<Button>().First());
                return Rows("folderTree").Contains("Rock");
            }
            var before = Rows("folderTree").Contains("Rock");
            Same(Toggle(), !before, "the first press turns it");
            Same(Toggle(), before, "the second press turns it back");
        });
    }

    // ---- A dialog with a paged table that reorders.
    void Friends()
    {
        Step("a table in a dialog reorders, saved, and Ctrl+Z takes it back", () =>
        {
            Click(Part("albumsButton"));
            Click(Part("pickButton"));
            var dialog = Dialog();
            ClickRow("grid", 2, dialog);
            Key(PhysicalKey.ArrowUp, RawInputModifiers.Alt, dialog);
            Same(SqlRows("SELECT name FROM friend ORDER BY position"), ["Ann", "Cid", "Bob"], "saved order");
            Undo(dialog);
            Same(SqlRows("SELECT name FROM friend ORDER BY position"), ["Ann", "Bob", "Cid"], "taken back");
        });

        Step("a table row's context menu picks it, and the dialog gives it back", () =>
        {
            var dialog = Dialog();
            var row = RowControls("grid", dialog)[1];
            var at = row.TranslatePoint(new Point(row.Bounds.Width / 2, row.Bounds.Height / 2), dialog)!.Value;
            dialog.MouseDown(at, MouseButton.Right);
            dialog.MouseUp(at, MouseButton.Right);
            Jobs();
            // The table's context menu, on the table or a panel around it.
            var menu = row.GetSelfAndVisualAncestors().OfType<Control>().Select(c => c.ContextMenu).FirstOrDefault(m => m is not null)
                ?? throw new Exception("the table has no context menu");
            Check(menu.IsOpen, "the context menu is open");
            var item = menu.GetLogicalDescendants().OfType<MenuItem>().FirstOrDefault(i => i.Name == "pickItem")
                ?? throw new Exception("no Pick in the context menu");
            var top = TopLevel.GetTopLevel(item) ?? throw new Exception("the menu is not shown");
            var p = item.TranslatePoint(new Point(item.Bounds.Width / 2, item.Bounds.Height / 2), top)!.Value;
            top.MouseDown(p, MouseButton.Left);
            top.MouseUp(p, MouseButton.Left);
            Settle(400);
            Check(main.OwnedWindows.Count == 0, "the dialog closed");
        });
    }

    // ---- Doing what a person does.

    void Step(string name, Action run)
    {
        try
        {
            run();
            Jobs();
            if (SaidError() is { } said)
                throw new Exception($"the app said: {said}");
            Console.WriteLine($"ok   {name}");
        }
        catch (Exception e)
        {
            failed.Add(name);
            // What the app said goes with the failure: it is often why.
            var said = SaidError();
            Console.WriteLine($"FAIL {name}\n     {e.Message.Replace("\n", "\n     ")}\n     edits: {EditsNow()}{(said is null ? "" : $"\n     the app said: {said}")}");
            // Whatever the step left open is closed, as a person would.
            foreach (var w in main.OwnedWindows.ToList())
                w.Close();
            Jobs();
        }
    }

    /// <summary>An error the app said in its dialog, closed so the next step starts clean.</summary>
    string? SaidError()
    {
        foreach (var w in main.OwnedWindows.ToList())
        {
            if (w.Content is SaidDialog said && said.Message.Level == Level.Error)
            {
                var text = Locale.Show(said.Message.Text);
                said.Ok.Command?.Execute(null);
                w.Close();
                Jobs();
                return text;
            }
        }
        return null;
    }

    static void Jobs()
    {
        for (var i = 0; i < 5; i++)
            Dispatcher.UIThread.RunJobs();
    }

    static void Settle(int ms)
    {
        var end = DateTime.Now.AddMilliseconds(ms);
        while (DateTime.Now < end)
        {
            Jobs();
            Thread.Sleep(20);
        }
    }

    /// <summary>The window input goes to: the dialog on top, else the main window.</summary>
    TopLevel Top() => (TopLevel?)main.OwnedWindows.LastOrDefault() ?? main;

    void Key(PhysicalKey key, RawInputModifiers modifiers = RawInputModifiers.None, TopLevel? to = null)
    {
        (to ?? Top()).KeyPressQwerty(key, modifiers);
        (to ?? Top()).KeyReleaseQwerty(key, modifiers);
        Settle(150);
    }

    void Type(string text)
    {
        Top().KeyTextInput(text);
        Settle(100);
    }

    void Undo(TopLevel? to = null) => Key(PhysicalKey.Z, RawInputModifiers.Control, to);

    void Redo(TopLevel? to = null) => Key(PhysicalKey.Y, RawInputModifiers.Control, to);

    void Click(Control control, double fx = 0.5, double fy = 0.5, MouseButton button = MouseButton.Left)
    {
        control.BringIntoView();
        Jobs();
        var top = TopLevel.GetTopLevel(control) ?? throw new Exception($"{control.Name ?? control.GetType().Name} is not shown");
        var at = control.TranslatePoint(new Point(control.Bounds.Width * fx, control.Bounds.Height * fy), top) ?? throw new Exception("no place");
        top.MouseMove(at);
        top.MouseDown(at, button);
        top.MouseUp(at, button);
        Settle(200);
    }

    void DoubleClick(Control control)
    {
        var top = TopLevel.GetTopLevel(control)!;
        var at = control.TranslatePoint(new Point(control.Bounds.Width / 2, control.Bounds.Height / 2), top)!.Value;
        top.MouseDown(at, MouseButton.Left);
        top.MouseUp(at, MouseButton.Left);
        // A second press close by in time: Avalonia makes it a double tap itself.
        top.MouseDown(at, MouseButton.Left);
        top.MouseUp(at, MouseButton.Left);
        Settle(300);
    }

    /// <summary>A press on <paramref name="from"/>, moves in steps, a release on <paramref name="to"/>: a person's drag.</summary>
    void Drag(Control from, double fromY, Control to, double toY, double dx = 0)
    {
        var top = TopLevel.GetTopLevel(from)!;
        var a = from.TranslatePoint(new Point(from.Bounds.Width / 2, from.Bounds.Height * fromY), top)!.Value;
        var b = to.TranslatePoint(new Point(to.Bounds.Width / 2 + dx, to.Bounds.Height * toY), top)!.Value;
        top.MouseMove(a);
        top.MouseDown(a, MouseButton.Left);
        for (var i = 1; i <= 8; i++)
            top.MouseMove(new Point(a.X + (b.X - a.X) * i / 8, a.Y + (b.Y - a.Y) * i / 8), RawInputModifiers.LeftMouseButton);
        top.MouseUp(b, MouseButton.Left);
        Settle(400);
    }

    /// <summary>The place on screen of a point of a canvas, in its own units.</summary>
    /// The window scrolls to it first, as a person scrolls to what they press.
    Point OnScreen(CanvasView canvas, Point units, TopLevel top)
    {
        var v = canvas.Viewport;
        var local = new Point((units.X - v.X) * v.Scale, (units.Y - v.Y) * v.Scale);
        canvas.BringIntoView(new Rect(local - new Point(20, 20), new Size(40, 40)));
        Settle(100);
        return canvas.TranslatePoint(local, top)!.Value;
    }

    /// <summary>A person's drag on a canvas, from one point to another in its units.</summary>
    void DragUnits(CanvasView canvas, Point from, Point to)
    {
        var top = TopLevel.GetTopLevel(canvas)!;
        // Both ends in view, the start read again after the end scrolled.
        OnScreen(canvas, from, top);
        var b = OnScreen(canvas, to, top);
        var a = OnScreen(canvas, from, top);
        Lands(canvas, a, top);
        top.MouseMove(a);
        top.MouseDown(a, MouseButton.Left);
        for (var i = 1; i <= 8; i++)
            top.MouseMove(new Point(a.X + (b.X - a.X) * i / 8, a.Y + (b.Y - a.Y) * i / 8), RawInputModifiers.LeftMouseButton);
        top.MouseUp(b, MouseButton.Left);
        Settle(500);
    }

    /// <summary>Checks a press at <paramref name="at"/> reaches the canvas, as it reaches it on a person's screen.</summary>
    static void Lands(CanvasView canvas, Point at, TopLevel top)
    {
        var hit = top.InputHitTest(at) as Visual;
        for (var v = hit; v is not null; v = v.GetVisualParent())
            if (v == canvas)
                return;
        var v0 = canvas.Viewport;
        throw new Exception($"a press at {at} lands on {hit?.GetType().Name ?? "nothing"} ({(hit as Control)?.Name}), not on {canvas.Name}: canvas bounds {canvas.Bounds}, view {v0.X},{v0.Y} x{v0.Scale}, drawing {canvas.Drawing.Count} shapes");
    }

    /// <summary>A click on a canvas at a point in its units.</summary>
    void ClickUnits(CanvasView canvas, Point at)
    {
        var top = TopLevel.GetTopLevel(canvas)!;
        var p = OnScreen(canvas, at, top);
        Lands(canvas, p, top);
        top.MouseMove(p);
        top.MouseDown(p, MouseButton.Left);
        top.MouseUp(p, MouseButton.Left);
        Settle(300);
    }

    /// <summary>Clicks the part's text box, selects what is there, types, and leaves with Tab.</summary>
    void Edit(string part, string text)
    {
        Click(Inner<TextBox>(part));
        Key(PhysicalKey.A, RawInputModifiers.Control);
        Type(text);
        Key(PhysicalKey.Tab);
    }

    // ---- Reading the windows.

    IEnumerable<Visual> Roots() => [main, .. main.OwnedWindows];

    Control? Find(string name, Visual? within = null) =>
        (within is null ? Roots() : [within]).SelectMany(r => r.GetVisualDescendants().Prepend(r)).OfType<Control>().FirstOrDefault(c => c.Name == name && c.IsEffectivelyVisible);

    Control Part(string name, Visual? within = null) => Find(name, within) ?? throw new Exception($"no part {name} on screen");

    T Inner<T>(string part) where T : Control
    {
        var p = Part(part);
        // An input that wraps its text box names it: a color, a typed or search dropdown.
        if (typeof(T) == typeof(TextBox) && p switch { ColorInput c => c.Box, TypeAhead t => t.Box, _ => null } is T box)
            return box;
        return p as T ?? p.GetVisualDescendants().OfType<T>().FirstOrDefault() ?? throw new Exception($"{part} has no {typeof(T).Name}");
    }

    string Text(string part) => Inner<TextBox>(part).Text ?? "";

    static string Texts(Visual v) =>
        string.Join(" ", v.GetVisualDescendants().OfType<TextBlock>().Select(t => t.Text is { Length: > 0 } text ? text : t.Inlines?.Text).Where(t => !string.IsNullOrEmpty(t)));

    /// <summary>The control inside <paramref name="within"/> whose shown text is <paramref name="text"/>: a button, a header, a menu item.</summary>
    static Control Labelled(Visual within, string text)
    {
        var block = within.GetVisualDescendants().OfType<TextBlock>().FirstOrDefault(t => t.Text == text && t.IsEffectivelyVisible)
            ?? throw new Exception($"no {text} in {(within as Control)?.Name ?? within.GetType().Name}");
        return block.FindAncestorOfType<Button>() as Control ?? block.FindAncestorOfType<TabItem>() as Control ?? block.FindAncestorOfType<HeaderedContentControl>() as Control ?? block;
    }

    /// <summary>The dialog on top, once it shows.</summary>
    Window Dialog()
    {
        Settle(300);
        return main.OwnedWindows.LastOrDefault() ?? throw new Exception("no dialog showed");
    }

    /// <summary>The rows a list, table or tree shows, top to bottom.</summary>
    List<Control> RowControls(string part, Visual? within = null)
    {
        var p = Part(part, within);
        return p switch
        {
            ListBox list => [.. list.GetVisualDescendants().OfType<ListBoxItem>().Where(i => i.IsEffectivelyVisible).OrderBy(i => i.TranslatePoint(default, list)?.Y ?? 0)],
            _ => [.. p.GetVisualDescendants().OfType<DataGridRow>().Where(r => r.IsEffectivelyVisible).OrderBy(r => r.TranslatePoint(default, p)?.Y ?? 0)],
        };
    }

    List<string> Rows(string part, Visual? within = null) =>
        [.. RowControls(part, within).Select(r => string.Join(" | ", r.GetVisualDescendants().OfType<TextBlock>().Select(t => t.Text).Where(t => !string.IsNullOrWhiteSpace(t) && t != "|")))];

    void ClickRow(string part, int index, Visual? within = null) => Click(RowControls(part, within)[index], 0.6, 0.5);

    /// <summary>The edits of the window on top, for a step that went wrong.</summary>
    static string EditsNow()
    {
        var model = Shell.Desktops[^1].Current;
        if (model is null)
            return "no window";
        var done = typeof(EditBus).GetField("done", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance)?.GetValue(model.Edits) as System.Collections.IEnumerable;
        return string.Join("; ", (done?.Cast<Clarity.Avalonia.Edit>() ?? []).TakeLast(3).Select(e => $"{e.Part}@{e.Scope} step {e.Step} [{string.Join(",", e.Changes.Select(c => c.Name))}]"));
    }

    // ---- The database.

    string Sql(string query) => SqlOn(database, query);

    static string SqlOn(string database, string query)
    {
        var run = Process.Start(new ProcessStartInfo("sqlite3", ["-separator", "|", database, query]) { RedirectStandardOutput = true, RedirectStandardError = true })!;
        var said = run.StandardOutput.ReadToEnd().Trim();
        var error = run.StandardError.ReadToEnd().Trim();
        run.WaitForExit();
        if (run.ExitCode != 0)
            throw new Exception($"sqlite3: {error}");
        return said;
    }

    List<string> SqlRows(string query) => [.. Sql(query).Split('\n', StringSplitOptions.RemoveEmptyEntries)];

    // ---- Checks.

    static void Check(bool ok, string what)
    {
        if (!ok)
            throw new Exception(what);
    }

    static void Same<T>(T actual, T expected, string what)
    {
        if (!EqualityComparer<T>.Default.Equals(actual, expected))
            throw new Exception($"{what}: expected {expected}, got {actual}");
    }

    static void Same(List<string> actual, List<string> expected, string what)
    {
        if (!actual.SequenceEqual(expected))
            throw new Exception($"{what}: expected [{string.Join(", ", expected)}], got [{string.Join(", ", actual)}]");
    }
}
