// Lists of values in code (`list<integer>`) and random, as every runtime
// gives them (conformance/lists.json).

namespace Clarity.Runtime;

public static class Lists
{
    /// <summary><c>xs[i]</c> in code: item <paramref name="position"/>, counted from 1.</summary>
    public static T At<T>(List<T> list, long position) =>
        position >= 1 && position <= list.Count ? list[(int)(position - 1)] : throw Outside(position, list.Count);

    /// <summary><c>xs[i] = value</c> in code: sets item <paramref name="position"/>, counted from 1.</summary>
    public static void Set<T>(List<T> list, long position, T value)
    {
        if (position < 1 || position > list.Count)
            throw Outside(position, list.Count);
        list[(int)(position - 1)] = value;
    }

    static ValueException Outside(long position, int count) => new($"there is no item {position} in a list of {count}");
}

/// <summary>The numbers <c>random(max)</c> gives: splitmix64, the sequence every
/// runtime gives alike from the same seed.</summary>
public sealed class Splitmix(ulong seed)
{
    ulong state = seed;

    /// <summary>A whole number from 1 to <paramref name="max"/>.</summary>
    public long Next(long max)
    {
        if (max < 1)
            throw new ValueException($"random takes a highest number of 1 or more, not {max}");
        state = unchecked(state + 0x9E3779B97F4A7C15UL);
        var z = state;
        z = unchecked((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL);
        z = unchecked((z ^ (z >> 27)) * 0x94D049BB133111EBUL);
        z ^= z >> 31;
        return 1 + (long)(z % (ulong)max);
    }

    static readonly Splitmix Shared = new((ulong)System.Random.Shared.NextInt64());

    /// <summary><c>random(max)</c> in an engine written in Clarity, from a sequence
    /// seeded once per run of the application.</summary>
    public static long Random(long max)
    {
        lock (Shared)
            return Shared.Next(max);
    }
}
