# Clarity.Runtime.MySql

[![NuGet](https://img.shields.io/nuget/v/Clarity.Runtime.MySql.svg)](https://www.nuget.org/packages/Clarity.Runtime.MySql/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Runtime.MySql** lets the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates keep their data in MySQL. It is the MySQL driver of [Clarity.Runtime](https://www.nuget.org/packages/Clarity.Runtime/), through [MySqlConnector](https://www.nuget.org/packages/MySqlConnector/).

Clarity.Runtime itself only has SQLite. Each server database is a package of its own, so an application takes only the drivers its data sources need.

## How it is used

A Clarity project with a MySQL data source, such as

```
datasource main mysql "env:SHOP_DATABASE"
```

generates a .NET project that references this package, and a program that turns the driver on before it opens its databases:

```csharp
MySql.Use();
var db = Db.Connect(Databases);
```

The location can be written in the project or, with `env:NAME`, read from an environment variable when the application starts, so passwords stay out of the program. For example: `mysql://user:password@host:3306/shop`.

Clarity applies its migration steps to the database when the application starts, and reads and writes the tables of an external database as they are.

## License

MIT, see [LICENSE.txt](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt).
