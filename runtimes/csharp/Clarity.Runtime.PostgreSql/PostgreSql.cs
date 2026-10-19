// Turns PostgreSQL on for a Clarity application: a generated program that
// has a PostgreSQL data source calls PostgreSql.Use() before it opens its databases.

namespace Clarity.Runtime;

public static class PostgreSql
{
    /// <summary>Lets the application open PostgreSQL databases.</summary>
    public static void Use() => Store.Use(Dialect.Postgres, PostgresDriver.Open);
}
