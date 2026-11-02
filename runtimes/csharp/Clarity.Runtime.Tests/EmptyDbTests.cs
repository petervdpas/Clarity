// An empty database (Db.Empty): what a test or a screen without storage
// works on. It keeps nothing: reads find no records, saves are dropped, and
// it says whether it was let go.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class EmptyDbTests
{
    [Fact]
    public void It_keeps_nothing_and_finds_nothing()
    {
        using var db = Db.Empty();
        Assert.Equal(0, db.SchemaVersion());
        db.Save(new Customer { Name = "Ada" });
        Assert.Empty(db.All<Customer>());
        Assert.Throws<NotFoundException>(() => { db.Get<Customer>(1); });
    }

    [Fact]
    public void It_says_whether_it_was_let_go()
    {
        var db = Db.Empty();
        Assert.False(db.Disposed);
        db.Dispose();
        Assert.True(db.Disposed);
    }
}
