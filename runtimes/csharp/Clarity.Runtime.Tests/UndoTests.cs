// The database side of undo (DESIGN 5.29), as on the other stacks
// (runtimes/rust/tests/undo.rs): everything one handler run changed is one
// step, with an id the window's edit keeps. Undo puts the rows back as they
// were, with their own ids, cascades included; redo does it again; a
// handler that fails leaves nothing behind; an undo whose rows changed
// since is refused.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed record Drawn : IRecord<Drawn>
{
    public static string TableName => "shape";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "x"];
    public static bool HasCascades => true;
    public long Id { get; set; }
    public string Name { get; set; } = "";
    public long X { get; set; }
    public object?[] ToValues() => [Name, X];
    public static Drawn FromValues(long id, object?[] values) => new() { Id = id, Name = Sql.GetText("name", values[0]), X = Sql.GetInteger("x", values[1]) };

    public void Cascade(Db db)
    {
        foreach (var note in db.Find<DrawnNote>(new Query().Filter(Cond.Compare("shape", Op.Eq, Id))))
            db.Delete(note);
    }
}

public sealed record DrawnNote : IRecord<DrawnNote>
{
    public static string TableName => "note";
    public static IReadOnlyList<string> ColumnNames { get; } = ["shape", "text"];
    public long Id { get; set; }
    public long? Shape { get; set; }
    public string Text { get; set; } = "";
    public object?[] ToValues() => [Shape, Text];
    public static DrawnNote FromValues(long id, object?[] values) => new() { Id = id, Shape = Sql.GetReference("shape", values[0]), Text = Sql.GetText("text", values[1]) };
}

public class UndoTests
{
    static Db Drawing()
    {
        var db = Sqlite.Memory();
        db.Migrate([[
            "CREATE TABLE shape (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, x INTEGER NOT NULL);",
            "CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT, shape INTEGER REFERENCES shape(id), text TEXT NOT NULL);",
        ]]);
        return db;
    }

    static Drawn Made(Db db, string name, long x)
    {
        var shape = new Drawn { Name = name, X = x };
        db.Save(shape);
        return shape;
    }

    [Fact]
    public void A_step_of_saves_is_undone_and_done_again_with_the_same_ids()
    {
        using var db = Drawing();
        Drawn a = null!, b = null!;
        db.Step(() => a = Made(db, "A", 10));
        var made = db.LastStep;
        db.Step(() =>
        {
            a.X = 300;
            db.Save(a);
            b = Made(db, "B", 20);
        });
        var moved = db.LastStep;
        Assert.True(db.Undo(moved));
        Assert.Equal([a with { X = 10 }], db.All<Drawn>());
        Assert.False(db.Undo(moved));
        Assert.True(db.Redo(moved));
        Assert.Equal([a, b], db.All<Drawn>());
        db.Undo(moved);
        db.Undo(made);
        Assert.Empty(db.All<Drawn>());
    }

    [Fact]
    public void A_delete_with_its_cascade_comes_back_whole()
    {
        using var db = Drawing();
        var a = Made(db, "A", 10);
        var note = new DrawnNote { Shape = a.Id, Text = "keep" };
        db.Save(note);
        db.Step(() => db.Delete(a));
        Assert.Empty(db.All<DrawnNote>());
        db.Undo(db.LastStep);
        Assert.Equal([a], db.All<Drawn>());
        Assert.Equal([note], db.All<DrawnNote>());
    }

    [Fact]
    public void A_handler_that_fails_leaves_nothing_and_is_no_step()
    {
        using var db = Drawing();
        Assert.Throws<InvalidValueException>(() => db.Step(() =>
        {
            Made(db, "A", 10);
            throw new InvalidValueException("no");
        }));
        Assert.Empty(db.All<Drawn>());
        Assert.Equal(0, db.LastStep);
        db.Step(() => db.All<Drawn>());
        Assert.Equal(0, db.LastStep);
    }

    [Fact]
    public void An_undo_whose_rows_changed_since_is_refused_and_changes_nothing()
    {
        using var db = Drawing();
        var a = Made(db, "A", 10);
        db.Step(() =>
        {
            a.X = 50;
            db.Save(a);
        });
        db.Batch("", "UPDATE shape SET x = 99");
        var refused = Assert.Throws<RefusedException>(() => db.Undo(db.LastStep));
        Assert.Equal("undoChangedSince", refused.Text.Key);
        Assert.Equal(99, db.All<Drawn>()[0].X);
    }
}
