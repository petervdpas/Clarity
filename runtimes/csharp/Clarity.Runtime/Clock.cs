// The clock where the application runs: Clarity's today() and now(), to the
// second, as the Rust runtime gives them.

namespace Clarity.Runtime;

public static class Clock
{
    public static DateOnly Today() => DateOnly.FromDateTime(DateTime.Now);

    public static DateTime Now()
    {
        var now = DateTime.Now;
        return new DateTime(now.Year, now.Month, now.Day, now.Hour, now.Minute, now.Second, DateTimeKind.Unspecified);
    }
}
