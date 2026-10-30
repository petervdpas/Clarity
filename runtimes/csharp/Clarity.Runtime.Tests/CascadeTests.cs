// Cascading deletes: a record whose records refer to it with `cascade`
// deletes them first, each with its own cascades and data events, all in
// the transaction of its delete (runtimes/rust/tests/cascade.rs).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class Plan : IRecord<Plan>
{
    public static string TableName => "plan";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name"];
    public static bool HasCascades => true;
    public long Id { get; set; }
    public string Name { get; set; } = "";
    public object?[] ToValues() => [Name];
    public static Plan FromValues(long id, object?[] values) => new() { Id = id, Name = Sql.GetText("name", values[0]) };

    // As generated code writes it: the tasks of the plan, each deleted as any record is.
    public void Cascade(Db db)
    {
        foreach (var task in db.Find<PlanTask>(new Query().Filter(Cond.Compare("plan", Op.Eq, Id))))
            db.Delete(task);
    }
}

public sealed class PlanTask : IRecord<PlanTask>
{
    public static readonly List<string> Deleted = [];
    public static string TableName => "plan_task";
    public static IReadOnlyList<string> ColumnNames { get; } = ["plan", "title"];
    public static bool HasEvents => true;
    public long Id { get; set; }
    public long? Plan { get; set; }
    public string Title { get; set; } = "";
    public object?[] ToValues() => [Plan, Title];
    public static PlanTask FromValues(long id, object?[] values) => new() { Id = id, Plan = Sql.GetReference("plan", values[0]), Title = Sql.GetText("title", values[1]) };

    public void BeforeDelete(Db db)
    {
        if (Title == "locked")
            throw new RefusedException(Text.Plain("This task is locked"));
        Deleted.Add(Title);
    }
}

public class CascadeTests
{
    static (Db, Plan) Planned()
    {
        var db = Sqlite.Memory();
        db.Migrate([[
            "CREATE TABLE plan (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
            "CREATE TABLE plan_task (id INTEGER PRIMARY KEY, plan INTEGER REFERENCES plan(id), title TEXT NOT NULL);",
        ]]);
        var plan = new Plan { Name = "Model" };
        db.Save(plan);
        foreach (var title in new[] { "Draw", "Write" })
            db.Save(new PlanTask { Plan = plan.Id, Title = title });
        return (db, plan);
    }

    [Fact]
    public void Deleting_a_record_deletes_what_cascades_from_it_first()
    {
        var (db, plan) = Planned();
        using var _ = db;
        PlanTask.Deleted.Clear();
        db.Delete(plan);
        Assert.Empty(db.All<Plan>());
        Assert.Empty(db.All<PlanTask>());
        Assert.Equal(["Draw", "Write"], PlanTask.Deleted);
    }

    [Fact]
    public void A_refused_child_keeps_all_of_it()
    {
        var (db, plan) = Planned();
        using var _ = db;
        db.Save(new PlanTask { Plan = plan.Id, Title = "locked" });
        Assert.Throws<RefusedException>(() => db.Delete(plan));
        Assert.Single(db.All<Plan>());
        Assert.Equal(3, db.All<PlanTask>().Count);
    }
}
