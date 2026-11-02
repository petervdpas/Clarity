// Undo per part on C# (DESIGN 5.29): each edit goes on the window's
// bus with what it changed in the window's data and the database step its
// handler made; Ctrl+Z takes back the last edit of the focused scope, so in
// a form it walks back field by field.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

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

/// <summary>Settings as a generated app keeps them: read whole and given back whole.</summary>
sealed class KeptSettingsLike : ISettings
{
    public bool Compact { get; set; }
    public string Theme { get; set; } = "Light";
    public System.Text.Json.Nodes.JsonObject ToJson() => new() { ["compact"] = Compact, ["theme"] = Theme };
    public void Apply(System.Text.Json.Nodes.JsonObject sent)
    {
        Compact = sent["compact"]!.GetValue<bool>();
        Theme = sent["theme"]!.GetValue<string>();
    }
}

sealed class SettingsWindowViewModel(Db? db, KeptSettingsLike settings) : WindowViewModel(db)
{
    public string Name { get; set; } = "Ada";
    protected override ISettings? AppSettings => settings;
}

public class SettingsUndoTests
{
    /// Avalonia's InputUndoTests.A_setting_a_part_changed_is_taken_back: a part bound to a setting edits the settings, and Ctrl+Z gives them back.
    [Fact]
    public void A_setting_a_part_changed_is_taken_back()
    {
        var settings = new KeptSettingsLike();
        var window = new SettingsWindowViewModel(null, settings);
        window.Opened();
        settings.Compact = true;
        window.Edited("compactBox", "compactBox");
        settings.Theme = "Dark";
        window.Edited("themeBox", "settingsForm");
        // Each scope takes back its own setting, and only that one.
        window.UndoEdit("compactBox", back: true);
        Assert.Equal((false, "Dark"), (settings.Compact, settings.Theme));
        window.UndoEdit("settingsForm", back: true);
        Assert.Equal((false, "Light"), (settings.Compact, settings.Theme));
        window.UndoEdit("compactBox", back: false);
        Assert.Equal((true, "Light"), (settings.Compact, settings.Theme));
        // The window's own data was never part of these edits.
        Assert.Equal("Ada", window.Name);
    }
}

sealed class ShelfOfShapesViewModel(Db? db) : WindowViewModel(db)
{
    public List<Sketched> Shapes { get; set; } = [new() { Id = 1, Name = "box", X = 3 }, new() { Id = 2, Name = "dot", X = 5 }];
    public Sketched Shape { get; set; } = new();
}

public class RecordUndoTests
{
    /// The record a form shows is the list's row: an undo puts its values back in that record, so the list keeps it selected and shows them too.
    [Fact]
    public void An_undo_puts_back_the_values_of_the_record_the_window_shows()
    {
        var window = new ShelfOfShapesViewModel(null);
        window.Shape = window.Shapes[1];
        window.Opened();
        window.Shape.X = 9;
        window.Edited("xSlider", "shapeForm");
        window.UndoEdit("shapeForm", back: true);
        Assert.Same(window.Shapes[1], window.Shape);
        Assert.Equal(5, window.Shapes[1].X);
        window.UndoEdit("shapeForm", back: false);
        Assert.Same(window.Shapes[1], window.Shape);
        Assert.Equal(9, window.Shape.X);
    }
}
