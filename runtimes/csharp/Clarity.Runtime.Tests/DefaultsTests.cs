// Clarity's defaults are the spec's (conformance/defaults.json); the copies
// this runtime carries say the same.

using System.Text.Json;
using Clarity.Auth;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class DefaultsTests
{
    [Fact]
    public void TheDefaultsAreTheSpecs()
    {
        using var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "defaults.json")));
        var root = file.RootElement;
        long At(params string[] path)
        {
            var at = root;
            foreach (var name in path) at = at.GetProperty(name);
            return at.GetInt64();
        }
        Assert.Equal(SigninSource.DefaultSessionSeconds, At("signin", "sessionMinutes") * 60);
        Assert.Equal(SigninSource.DefaultTries, At("signin", "tries"));
        Assert.Equal(SigninSource.DefaultTriesSeconds, At("signin", "triesMinutes") * 60);
        Assert.Equal(Db.UndoSteps, At("undoSteps"));
        Assert.Equal(Backup.KeptAutomatic, At("keptBackups"));
        Assert.Equal((long)Http.Timeout.TotalSeconds, At("httpSeconds"));
        Assert.Equal(Db.OwnersPerRead, At("ownersPerRead"));
        Assert.Equal(EditBus.Keeps, At("editBusKeeps"));
        Assert.Equal((long)ProgramWire.QuietByDefault.TotalMinutes, At("quietMinutes"));
    }
}
