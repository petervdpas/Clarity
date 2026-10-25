// What can go wrong when generated code runs, as exceptions. The Rust
// runtime has the same cases as its Error enum.

namespace Clarity.Runtime;

public abstract class ClarityException(string message) : Exception(message);

/// <summary>A <c>require</c> or <c>ensure</c> of a function or action did not hold.</summary>
public sealed class ContractException(string what) : ClarityException($"condition failed: {what}")
{
    public string What { get; } = what;
}

/// <summary>A record that should exist does not; no id means no record was set.</summary>
public sealed class NotFoundException(string table, long? id)
    : ClarityException(id is { } n ? $"there is no {table} with id {n}" : $"no {table} is set")
{
    public string Table { get; } = table;
    public long? Id { get; } = id;
}

/// <summary>Another record refers to the record being deleted.</summary>
public sealed class InUseException(string table, long id) : ClarityException($"the {table} with id {id} is still referred to")
{
    public string Table { get; } = table;
    public long Id { get; } = id;
}

/// <summary>A stored value does not fit its field.</summary>
public sealed class InvalidValueException(string what) : ClarityException($"invalid value: {what}");

public sealed class StorageException(string what, Exception? inner = null) : ClarityException($"storage: {what}")
{
    public string What { get; } = what;
    public new Exception? InnerException { get; } = inner;
}

/// <summary>A database connects with a secret setting (DESIGN 5.27) the person has not kept yet.</summary>
public sealed class NotKeptException(string setting) : ClarityException($"storage: the secret setting {setting} is not kept yet: the settings name the connection")
{
    public string Setting { get; } = setting;
}

/// <summary>A data event refused a save or delete, or code refused to go on, in its own words.</summary>
public sealed class RefusedException(Text text) : ClarityException(text.Show())
{
    public Text Text { get; } = text;
}

public static class Contracts
{
    /// <summary>Checks a <c>require</c> line of a function or action.</summary>
    public static void Require(bool holds, string what)
    {
        if (!holds)
            throw new ContractException(what);
    }

    /// <summary>Checks an <c>ensure</c> line of a function or action.</summary>
    public static void Ensure(bool holds, string what) => Require(holds, what);
}

/// <summary>A value in code that does not fit where it is used: a position
/// outside its list, a highest number below 1 (conformance/lists.json).</summary>
public sealed class ValueException(string what) : ClarityException(what);
