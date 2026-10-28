# Clarity.Runtime.Sqlite

[![NuGet](https://img.shields.io/nuget/v/Clarity.Runtime.Sqlite.svg)](https://www.nuget.org/packages/Clarity.Runtime.Sqlite/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt)

**Clarity.Runtime.Sqlite** lets the C# applications that [Clarity](https://github.com/petervdpas/Clarity) generates keep their data in SQLite. It is the SQLite driver of [Clarity.Runtime](https://www.nuget.org/packages/Clarity.Runtime/), through [Microsoft.Data.Sqlite](https://www.nuget.org/packages/Microsoft.Data.Sqlite/).

Each kind of database is a package of its own, so an application takes only the drivers its data sources need, and a program shown in a browser takes none: its server keeps the data.

## How it is used

A Clarity project with a SQLite data source, such as

```clarity
datasource main sqlite "data/shop.db"
```

generates a .NET project that references this package, and a program that turns the driver on before it opens its databases:

```csharp
Sqlite.Use();
var db = Db.Connect(Databases);
```

`Sqlite.Open(path)` opens one database for every record, and `Sqlite.Memory()` one that lives in memory, as tests use.

Clarity applies its migration steps to the database when the application starts.

## License

MIT, see [LICENSE.txt](https://github.com/petervdpas/Clarity/blob/main/runtimes/csharp/assets/LICENSE.txt).
