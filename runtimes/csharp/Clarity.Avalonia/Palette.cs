// The color field type (U1c), the same as on the web
// (conformance/colors.json): a color is kept as text "#rrggbb", read from
// what is typed the same way, and picked from a palette every renderer
// shares (clarity_spec::colors::PALETTE).

using System.Globalization;

namespace Clarity.Avalonia;

public static class Palette
{
    /// <summary>The colors the color input offers, in rows: grays, then light to dark shades of ten hues.</summary>
    public static readonly IReadOnlyList<IReadOnlyList<string>> Rows =
    [
        ["#ffffff", "#f8fafc", "#e5e7eb", "#d1d5db", "#9ca3af", "#6b7280", "#4b5563", "#374151", "#1f2937", "#000000"],
        ["#e2e8f0", "#fecaca", "#fed7aa", "#fde68a", "#bbf7d0", "#99f6e4", "#bfdbfe", "#c7d2fe", "#e9d5ff", "#fbcfe8"],
        ["#94a3b8", "#f87171", "#fb923c", "#fbbf24", "#4ade80", "#2dd4bf", "#60a5fa", "#818cf8", "#c084fc", "#f472b6"],
        ["#64748b", "#ef4444", "#f97316", "#f59e0b", "#22c55e", "#14b8a6", "#3b82f6", "#6366f1", "#a855f7", "#ec4899"],
        ["#475569", "#dc2626", "#ea580c", "#d97706", "#16a34a", "#0d9488", "#2563eb", "#4f46e5", "#9333ea", "#db2777"],
        ["#1e293b", "#991b1b", "#9a3412", "#92400e", "#166534", "#115e59", "#1e40af", "#3730a3", "#6b21a8", "#9d174d"],
    ];

    /// <summary>The color <paramref name="typed"/> is, as "#" and six lowercase digits; "" for no color; null when it is no color at all.</summary>
    public static string? Read(string? typed)
    {
        var text = (typed ?? "").Trim();
        if (text.Length == 0)
            return "";
        var digits = text.StartsWith('#') ? text[1..] : text;
        if (!digits.All(Uri.IsHexDigit))
            return null;
        return digits.Length switch
        {
            3 => "#" + string.Concat(digits.Select(c => $"{c}{c}")).ToLowerInvariant(),
            6 => "#" + digits.ToLowerInvariant(),
            _ => null,
        };
    }

    /// <summary>A kept color as a brush; null for no color.</summary>
    public static global::Avalonia.Media.IBrush? Brush(string? color) =>
        Read(color) is { Length: 7 } kept && global::Avalonia.Media.Color.TryParse(kept, out var c) ? new global::Avalonia.Media.SolidColorBrush(c) : null;
}
