// Where a server database is, as the project file writes it for every
// stack (the Rust runtime reads the same): PostgreSQL as a URL
// (postgres://user:password@host:port/database?sslmode=require) or as
// libpq's key=value words, MySQL as a URL (mysql://user:password@host:port/database),
// SQL Server as an ADO.NET connection string. ADO.NET's own form is taken
// as it is for every database.

using System.Text;

namespace Clarity.Runtime;

public static class Locations
{
    /// <summary>libpq's words, and Npgsql's for them.</summary>
    static readonly Dictionary<string, string> PostgresWords = new()
    {
        ["host"] = "Host",
        ["hostaddr"] = "Host",
        ["port"] = "Port",
        ["user"] = "Username",
        ["password"] = "Password",
        ["dbname"] = "Database",
        ["sslmode"] = "SSL Mode",
        ["connect_timeout"] = "Timeout",
        ["application_name"] = "Application Name",
    };

    /// <summary>libpq's SSL modes, as Npgsql writes them.</summary>
    static readonly Dictionary<string, string> SslModes = new()
    {
        ["disable"] = "Disable",
        ["allow"] = "Allow",
        ["prefer"] = "Prefer",
        ["require"] = "Require",
        ["verify-ca"] = "VerifyCA",
        ["verify-full"] = "VerifyFull",
    };

    public static string Postgres(string location)
    {
        if (location.StartsWith("postgres://", StringComparison.Ordinal) || location.StartsWith("postgresql://", StringComparison.Ordinal))
        {
            var parts = FromUrl(location, "Host", "Username", "Database");
            return Join(parts.Concat(Query(location).Select(q => (Word(q.Key), Value(q.Key, q.Value)))));
        }
        if (location.Contains(';') || !location.Contains('='))
            return location;
        return Join(Words(location).Select(w => (Word(w.Key), Value(w.Key, w.Value))));

        static string Word(string key) => PostgresWords.TryGetValue(key, out var word) ? word : key;
        static string Value(string key, string value) => key == "sslmode" && SslModes.TryGetValue(value, out var mode) ? mode : value;
    }

    public static string Mysql(string location) =>
        location.StartsWith("mysql://", StringComparison.Ordinal) ? Join(FromUrl(location, "Server", "User ID", "Database")) : location;

    static string Join(IEnumerable<(string Key, string Value)> parts) => string.Join(";", parts.Select(p => $"{p.Key}={Quoted(p.Value)}"));

    /// <summary>A value as a connection string needs it: quoted when it holds a separator.</summary>
    static string Quoted(string value) => value.IndexOfAny([';', '"', '\'']) >= 0 && !value.Contains('\'') ? $"'{value}'"
        : value.IndexOfAny([';', '"']) >= 0 ? $"\"{value.Replace("\"", "\"\"")}\"" : value;

    /// <summary>The host, port, user, password and database of a URL, under the names the driver uses.</summary>
    static List<(string, string)> FromUrl(string location, string host, string user, string database)
    {
        var rest = location[(location.IndexOf("://", StringComparison.Ordinal) + 3)..];
        var query = rest.IndexOf('?');
        if (query >= 0)
            rest = rest[..query];
        var parts = new List<(string, string)>();
        string? name = null, password = null;
        var at = rest.LastIndexOf('@');
        if (at >= 0)
        {
            var credentials = rest[..at];
            rest = rest[(at + 1)..];
            var colon = credentials.IndexOf(':');
            name = Uri.UnescapeDataString(colon >= 0 ? credentials[..colon] : credentials);
            if (colon >= 0)
                password = Uri.UnescapeDataString(credentials[(colon + 1)..]);
        }
        var slash = rest.IndexOf('/');
        var server = slash >= 0 ? rest[..slash] : rest;
        var db = slash >= 0 ? Uri.UnescapeDataString(rest[(slash + 1)..]) : "";
        var port = server.LastIndexOf(':');
        parts.Add((host, port >= 0 ? server[..port] : server));
        if (port >= 0)
            parts.Add(("Port", server[(port + 1)..]));
        if (name is not null)
            parts.Add((user, name));
        if (password is not null)
            parts.Add(("Password", password));
        if (db.Length > 0)
            parts.Add((database, db));
        return parts;
    }

    static IEnumerable<KeyValuePair<string, string>> Query(string location)
    {
        var at = location.IndexOf('?');
        if (at < 0)
            yield break;
        foreach (var pair in location[(at + 1)..].Split('&', StringSplitOptions.RemoveEmptyEntries))
        {
            var eq = pair.IndexOf('=');
            if (eq > 0)
                yield return new(Uri.UnescapeDataString(pair[..eq]), Uri.UnescapeDataString(pair[(eq + 1)..]));
        }
    }

    /// <summary>libpq's key=value words; a value may be quoted with ' and escape with \.</summary>
    static IEnumerable<KeyValuePair<string, string>> Words(string text)
    {
        var i = 0;
        while (i < text.Length)
        {
            while (i < text.Length && char.IsWhiteSpace(text[i]))
                i++;
            var eq = text.IndexOf('=', i);
            if (eq < 0)
                yield break;
            var key = text[i..eq].Trim();
            i = eq + 1;
            while (i < text.Length && char.IsWhiteSpace(text[i]))
                i++;
            var value = new StringBuilder();
            if (i < text.Length && text[i] == '\'')
            {
                i++;
                while (i < text.Length && text[i] != '\'')
                {
                    if (text[i] == '\\' && i + 1 < text.Length)
                        i++;
                    value.Append(text[i++]);
                }
                i++;
            }
            else
            {
                while (i < text.Length && !char.IsWhiteSpace(text[i]))
                {
                    if (text[i] == '\\' && i + 1 < text.Length)
                        i++;
                    value.Append(text[i++]);
                }
            }
            yield return new(key, value.ToString());
        }
    }
}
