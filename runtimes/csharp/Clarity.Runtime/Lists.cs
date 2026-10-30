// Lists of values in code (`list<integer>`), as every runtime gives them
// (conformance/lists.json).

namespace Clarity.Runtime;

public static class Lists
{
    /// <summary>
    /// <c>from to to</c> in code: the whole numbers from <paramref name="from"/> to
    /// <paramref name="to"/>, both counted; none when <paramref name="to"/> is below
    /// <paramref name="from"/> (conformance/lists.json, <c>ranges</c>).
    /// </summary>
    public static List<long> Range(long from, long to)
    {
        var list = new List<long>();
        for (var i = from; i <= to; i++)
            list.Add(i);
        return list;
    }

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
