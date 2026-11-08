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

    /// <summary><c>a div b</c> in code: the quotient of two whole numbers, rounded toward zero.</summary>
    public static long WholeDiv(long a, long b) => b == 0 ? throw DividedByZero() : b == -1 ? unchecked(-a) : a / b;

    /// <summary><c>a mod b</c> in code: what is left of <c>a div b</c>, with the sign of <c>a</c>.</summary>
    public static long WholeMod(long a, long b) => b == 0 ? throw DividedByZero() : b == -1 ? 0 : a % b;

    static ValueException DividedByZero() => new("division by zero", ClarityException.Own("divisionByZero", "Division by zero"));

    static ValueException Outside(long position, int count) =>
        new($"there is no item {position} in a list of {count}", ClarityException.Own("itemOutsideList", "There is no item {position} in a list of {count}", ("position", position), ("count", count)));
}
