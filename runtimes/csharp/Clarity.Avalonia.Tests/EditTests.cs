// Undo per part on Avalonia (DESIGN 5.29): each edit goes on the window's
// bus with what it changed in the window's data and the database step its
// handler made; Ctrl+Z takes back the last edit of the focused scope, so in
// a form it walks back field by field.

using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

sealed record Sketched : IRecord<Sketched>
{
    public static string TableName => "shape";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "x"];
    public long Id { get; set; }
    public string Name { get; set; } = "";
    public long X { get; set; }
    public object?[] ToValues() => [Name, X];
    public static Sketched FromValues(long id, object?[] values) => new() { Id = id, Name = Sql.GetText("name", values[0]), X = Sql.GetInteger("x", values[1]) };
}

sealed class SketchViewModel(Db db) : WindowViewModel(db)
{
    public string Name { get; set; } = "Ada";
    public string City { get; set; } = "Rome";
    public long Moved { get; set; }
    // An unbound checkbox keeps its own tick.
    public PartState VipBox { get; } = new("vipBox");

    // As a generated canvas drop with undo runs its handler.
    public void Dropped(long x) => EditOf("sketch", "sketch", ends: true, () =>
    {
        Moved = x;
        Db.Save(new Sketched { Name = "box", X = x });
    });

    public void Clicked() => Handle(() => City = "Paris");

    // As a handler that loads another record runs: the bound parts follow, which is no edit.
    public void Loaded() => Handle(() =>
    {
        Name = "Bea";
        Edited("nameBox", "customerForm");
    });

    // As a generated `-> undo` menu item runs.
    public void UndoItem() => UndoFocused(true);
}

public class EditTests
{
    static Db Drawing()
    {
        var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE shape (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, x INTEGER NOT NULL);", "CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT, shape INTEGER, text TEXT NOT NULL);"]]);
        return db;
    }

    [Fact]
    public void In_a_form_ctrl_z_walks_back_its_fields_one_by_one()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.Name = "Ann";
        window.Edited("nameBox", "customerForm");
        window.City = "Oslo";
        window.Edited("cityBox", "customerForm");
        window.UndoEdit("customerForm", back: true);
        Assert.Equal(("Ann", "Rome"), (window.Name, window.City));
        window.UndoEdit("customerForm", back: true);
        Assert.Equal("Ada", window.Name);
        window.UndoEdit("customerForm", back: false);
        Assert.Equal("Ann", window.Name);
    }

    /// A change and what its handler changed in the same data are one edit: undo gives back what was there first, redo what came last.
    [Fact]
    public void An_edit_that_changed_the_same_data_twice_is_taken_back_to_the_first()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.Edits.Push(new Edit("nameBox", "customerForm", [new Changed("Name", System.Text.Json.Nodes.JsonValue.Create("Ada"), System.Text.Json.Nodes.JsonValue.Create("Ann")), new Changed("Name", System.Text.Json.Nodes.JsonValue.Create("Ann"), System.Text.Json.Nodes.JsonValue.Create("Bea"))], 0));
        window.Name = "Bea";
        window.UndoEdit("customerForm", back: true);
        Assert.Equal("Ada", window.Name);
        window.UndoEdit("customerForm", back: false);
        Assert.Equal("Bea", window.Name);
    }

    [Fact]
    public void A_canvas_drop_is_taken_back_with_what_its_handler_saved()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.Dropped(300);
        Assert.Single(db.All<Sketched>());
        // A click that is no edit is not taken back.
        window.Clicked();
        window.UndoEdit("sketch", back: true);
        Assert.Equal((0L, "Paris"), (window.Moved, window.City));
        Assert.Empty(db.All<Sketched>());
        window.UndoEdit("sketch", back: false);
        Assert.Equal(300, window.Moved);
        Assert.Single(db.All<Sketched>());
    }

    [Fact]
    public void An_undo_whose_rows_changed_since_is_refused_with_a_message()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.Dropped(300);
        db.Batch("", "UPDATE shape SET x = 99");
        window.UndoEdit("sketch", back: true);
        Assert.Equal(300, window.Moved);
        Assert.Equal("undoChangedSince", Assert.Single(window.Messages).Text.Key);
        Assert.True(window.Edits.CanUndo("sketch"));
    }

    /// A menu's Undo takes back the last edit of the part that had the focus last.
    [Fact]
    public void A_menus_undo_acts_on_the_part_that_had_the_focus_last()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.Dropped(300);
        window.FocusedScope = "sketch";
        window.UndoItem();
        Assert.Equal(0, window.Moved);
        Assert.Empty(db.All<Sketched>());
    }

    /// What a handler changed is no edit of the person's, though a bound part tells of it.
    [Fact]
    public void A_bound_part_following_a_handler_is_no_edit()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.Loaded();
        Assert.False(window.Edits.CanUndo("customerForm"));
    }

    /// An unbound input's own state is an edit too.
    [Fact]
    public void An_unbound_inputs_own_tick_is_taken_back()
    {
        using var db = Drawing();
        var window = new SketchViewModel(db);
        window.Opened();
        window.VipBox.IsChecked = true;
        window.Edited("vipBox", "vipBox");
        window.UndoEdit("vipBox", back: true);
        Assert.False(window.VipBox.IsChecked);
    }
}
