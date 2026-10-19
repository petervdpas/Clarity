// Turns SQL Server on for a Clarity application: a generated program that
// has a SQL Server data source calls SqlServer.Use() before it opens its databases.

namespace Clarity.Runtime;

public static class SqlServer
{
    /// <summary>Lets the application open SQL Server databases.</summary>
    public static void Use() => Store.Use(Dialect.Mssql, MssqlDriver.Open);
}
