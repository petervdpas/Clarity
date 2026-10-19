// Turns MySQL on for a Clarity application: a generated program that
// has a MySQL data source calls MySql.Use() before it opens its databases.

namespace Clarity.Runtime;

public static class MySql
{
    /// <summary>Lets the application open MySQL databases.</summary>
    public static void Use() => Store.Use(Dialect.Mysql, MysqlDriver.Open);
}
