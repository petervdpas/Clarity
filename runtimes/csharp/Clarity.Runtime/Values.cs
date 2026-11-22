// How Clarity works out values, as every stack does, where C#'s own operators
// do otherwise. An empty date or date and time comes before every value, and two
// empty ones are equal (conformance/compare-empty.json), where C#'s operators on
// nullable values give false whenever one side is empty. A whole number or
// decimal that does not fit is a value error in Clarity's own words
// (conformance/overflow.json), where C# wraps a long and throws its own
// OverflowException for a decimal.

namespace Clarity.Runtime;

public static class Values
{
    /// <summary>Below zero, zero or above zero; no date sorts first, as in the other runtimes.</summary>
    public static int Compare(DateOnly? a, DateOnly? b) => Nullable.Compare(a, b);

    /// <summary>Below zero, zero or above zero; no date and time sorts first, as in the other runtimes.</summary>
    public static int Compare(DateTime? a, DateTime? b) => Nullable.Compare(a, b);

    /// <summary><c>a + b</c> of whole numbers in code.</summary>
    public static long Add(long a, long b) => Fitting(() => checked(a + b));

    /// <summary><c>a - b</c> of whole numbers in code.</summary>
    public static long Subtract(long a, long b) => Fitting(() => checked(a - b));

    /// <summary><c>a * b</c> of whole numbers in code.</summary>
    public static long Multiply(long a, long b) => Fitting(() => checked(a * b));

    /// <summary><c>-a</c> of a whole number in code.</summary>
    public static long Negate(long a) => Fitting(() => checked(-a));

    /// <summary><c>a + b</c> of decimals in code: rounded to the places that fit.</summary>
    public static decimal Add(decimal a, decimal b) => Fitting(() => a + b);

    /// <summary><c>a - b</c> of decimals in code: rounded to the places that fit.</summary>
    public static decimal Subtract(decimal a, decimal b) => Fitting(() => a - b);

    /// <summary><c>a * b</c> of decimals in code: rounded to the places that fit.</summary>
    public static decimal Multiply(decimal a, decimal b) => Fitting(() => a * b);

    /// <summary>Why a number did not fit, in Clarity's own words.</summary>
    internal static ValueException TooLarge() => new("a number too large", ClarityException.Own("numberTooLarge", "The number is too large"));

    static T Fitting<T>(Func<T> work)
    {
        try
        {
            return work();
        }
        catch (OverflowException)
        {
            throw TooLarge();
        }
    }

    /// <summary><c>a / b</c> of decimals, as every stack divides them (conformance/decimal-division.json):
    /// up to 28 places, an exact quotient with only the places it needs; dividing by zero is a value error.</summary>
    public static decimal Divide(decimal a, decimal b)
    {
        if (b == 0)
            throw Lists.DividedByZero();
        var q = Fitting(() => a / b);
        return q * b == a ? q / 1.0000000000000000000000000000m : q;
    }
}
