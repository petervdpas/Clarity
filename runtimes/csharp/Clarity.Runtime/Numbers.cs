// Numbers in code: Clarity's round(value, places), sin and cos, as every
// runtime gives them (conformance/rounding.json and drawing.json).

namespace Clarity.Runtime;

public static class Numbers
{
    /// At most `places` decimals, rounded half away from zero; a value with
    /// fewer stays as it is, and places below 0 count as 0.
    public static decimal Round(decimal value, long places) =>
        Math.Round(value, (int)Math.Clamp(places, 0, 28), MidpointRounding.AwayFromZero);

    /// sin(degrees) and cos(degrees), rounded to 10 decimals (conformance/drawing.json).
    public static decimal Sin(decimal degrees) => Angle(degrees, Math.Sin);

    public static decimal Cos(decimal degrees) => Angle(degrees, Math.Cos);

    static decimal Angle(decimal degrees, Func<double, double> of) =>
        Math.Round((decimal)of((double)degrees * Math.PI / 180), 10, MidpointRounding.AwayFromZero);

    /// A value of code as a number a canvas draws with.
    public static double Units(decimal value) => (double)value;
}
