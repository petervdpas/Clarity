// SQLite is turned on once, as a generated program does before it opens
// its databases (Clarity.Runtime.Sqlite).

using System.Runtime.CompilerServices;
using Clarity.Runtime;

namespace Clarity.Runtime.Tests;

static class TurnedOn
{
    [ModuleInitializer]
    internal static void Databases() => Sqlite.Use();
}
