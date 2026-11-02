// CLARITY_DATA moves every SQLite database of a program into a folder of
// its own, under its own file name: a run in the IDE or by hand (a live check
// on a copy) chooses its data without editing the generated program.

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

// The variable is the process's: while it is set no other test opens a database.
[CollectionDefinition(nameof(DataFolderTests), DisableParallelization = true)]
public sealed class DataFolderAlone;

[Collection(nameof(DataFolderTests))]
public class DataFolderTests
{
    static Func<string, string?> Env(string? folder) => name => name == "CLARITY_DATA" ? folder : null;

    [Fact]
    public void Clarity_data_moves_an_sqlite_file_into_its_folder()
    {
        var folder = Path.Combine(Path.GetTempPath(), "copy");
        Assert.Equal(Path.Combine(folder, "noughts.db"), DataFolder.Sqlite("/home/someone/noughts/data/noughts.db", Env(folder)));
        // Unset or empty, the file is where the project put it.
        Assert.Equal("/home/someone/noughts/data/noughts.db", DataFolder.Sqlite("/home/someone/noughts/data/noughts.db", Env(null)));
        Assert.Equal("/home/someone/noughts/data/noughts.db", DataFolder.Sqlite("/home/someone/noughts/data/noughts.db", Env("")));
        // A location from the environment or a secret setting is as it says.
        Assert.Equal("env:SHOP_DB", DataFolder.Sqlite("env:SHOP_DB", Env(folder)));
        Assert.Equal("setting:shop/db", DataFolder.Sqlite("setting:shop/db", Env(folder)));
    }

    [Fact]
    public void A_database_opens_in_the_folder_clarity_data_names()
    {
        var folder = Directory.CreateTempSubdirectory("clarity-data-").FullName;
        var was = Environment.GetEnvironmentVariable(DataFolder.Variable);
        try
        {
            Environment.SetEnvironmentVariable(DataFolder.Variable, folder);
            Sqlite.Use();
            using (Db.Connect(("main", Dialect.Sqlite, Path.Combine(folder, "elsewhere", "shop.db"))))
            {
            }
            Assert.True(File.Exists(Path.Combine(folder, "shop.db")));
        }
        finally
        {
            Environment.SetEnvironmentVariable(DataFolder.Variable, was);
            Microsoft.Data.Sqlite.SqliteConnection.ClearAllPools();
            Directory.Delete(folder, recursive: true);
        }
    }
}
