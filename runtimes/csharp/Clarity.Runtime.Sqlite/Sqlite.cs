// SQLite, turned on as each server database is (DESIGN 7.6): the desktop
// program and the web server keep their data with it, the program in a
// browser does not carry it.

namespace Clarity.Runtime;

public static class Sqlite
{
    /// <summary>Lets the application open SQLite databases.</summary>
    public static void Use() => Store.Use(Dialect.Sqlite, SqliteDriver.Open);

    /// <summary>One SQLite database, used for every record.</summary>
    public static Db Open(string path) => Db.Of(SqliteDriver.Open(path));

    /// <summary>A SQLite database that lives in memory, for tests and a program whose databases did not open.</summary>
    public static Db Memory() => Db.Of(SqliteDriver.Memory());
}
