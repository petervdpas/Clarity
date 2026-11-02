// A theme as Clarity.UI draws it (DESIGN 1.5, 7.9): what Rust resolves for
// light or dark mode (clarity_spec::themes::resolve), read as JSON. A
// project's theme lies over Clarity's own, which Clarity.UI carries; what the
// project's leaves out is Clarity's. The look is defined in Clarity; this
// only reads it.

using System.Text.Json;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>How a part looks in a state, or a style: what the theme sets, null for what it leaves.</summary>
public sealed record LookValues(SKColor? Background, SKColor? Text, SKColor? Border, int? Radius, string? Weight, int? Size, string? Font, string? Align)
{
    public static readonly LookValues None = new(null, null, null, null, null, null, null, null);

    public bool Bold => Weight is "bold" or "700" or "800" or "900";

    /// <summary>What this sets, and what it leaves from <paramref name="under"/>.</summary>
    public LookValues Over(LookValues under) => new(
        Background ?? under.Background, Text ?? under.Text, Border ?? under.Border, Radius ?? under.Radius,
        Weight ?? under.Weight, Size ?? under.Size, Font ?? under.Font, Align ?? under.Align);
}

public sealed class ThemeLook
{
    readonly JsonElement own;
    readonly ThemeLook? under;

    ThemeLook(JsonElement own, ThemeLook? under)
    {
        this.own = own;
        this.under = under;
    }

    static readonly ThemeLook light = Carried("clarity.light.json");
    static readonly ThemeLook dark = Carried("clarity.dark.json");

    static ThemeLook Carried(string name)
    {
        using var stream = typeof(ThemeLook).Assembly.GetManifestResourceStream($"Clarity.Ui.Themes.{name}")
            ?? throw new InvalidOperationException($"Clarity.Ui carries no theme {name}");
        return new ThemeLook(JsonDocument.Parse(stream).RootElement.Clone(), null);
    }

    /// <summary>Clarity's own theme, for light or dark mode.</summary>
    public static ThemeLook Clarity(bool dark) => dark ? ThemeLook.dark : light;

    /// <summary>A project's theme, resolved by Clarity, over <paramref name="under"/>.</summary>
    public static ThemeLook Over(string resolved, ThemeLook under) => new(JsonDocument.Parse(resolved).RootElement.Clone(), under);

    public bool Dark => own.TryGetProperty("dark", out var d) ? d.ValueKind == JsonValueKind.True : under?.Dark ?? false;

    JsonElement? Table(string name, string key) =>
        own.TryGetProperty(name, out var table) && table.ValueKind == JsonValueKind.Object && table.TryGetProperty(key, out var value) && value.ValueKind != JsonValueKind.Null ? value : null;

    int? Number(string name) => own.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt32() : null;

    /// <summary>The color of a role; one the themes do not know is the text's, so a part always shows.</summary>
    public SKColor Color(string role) => TryColor(role) ?? (role == "text" ? SKColors.Black : Color("text"));

    public SKColor? TryColor(string role) =>
        Table("colors", role) is { } value ? Read(value.GetString()) : under?.TryColor(role);

    /// <summary>A written color: "#rrggbb", "@role" for Clarity's own value of a role, "none" for none.</summary>
    SKColor? Read(string? written) => written switch
    {
        null => null,
        "none" => SKColors.Transparent,
        _ when written.StartsWith('@') => (under ?? this).TryColor(written[1..]),
        _ => SKColor.TryParse(written, out var color) ? color : null,
    };

    public int Size(string role) => Table("sizes", role)?.GetInt32() ?? under?.Size(role) ?? (role == "text" ? 14 : Size("text"));

    public int Radius(string role) => Table("radii", role)?.GetInt32() ?? Table("radii", "all")?.GetInt32() ?? under?.Radius(role) ?? 0;

    /// <summary>A font role's families, most wanted first, as a list: "Inter" or "system-ui, sans-serif".</summary>
    public string Font(string role) => Table("fonts", role)?.GetString() ?? under?.Font(role) ?? (role == "text" ? "sans-serif" : Font("text"));

    public int Padding => Number("padding") ?? under?.Padding ?? 4;

    public int Border => Number("border") ?? under?.Border ?? 1;

    LookValues OwnLook(string table, string key) => Table(table, key) is { } look ? new LookValues(
        Field(look, "background"), Field(look, "text"), Field(look, "border"), Int(look, "radius"),
        Text(look, "weight"), Int(look, "size"), Text(look, "font"), Text(look, "align")) : LookValues.None;

    SKColor? Field(JsonElement look, string name) => Text(look, name) is { } written ? Read(written) : null;

    static string? Text(JsonElement look, string name) =>
        look.TryGetProperty(name, out var v) ? v.ValueKind switch { JsonValueKind.String => v.GetString(), JsonValueKind.Number => v.GetRawText(), _ => null } : null;

    static int? Int(JsonElement look, string name) => look.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt32() : null;

    LookValues Look(string table, string key) => under is null ? OwnLook(table, key) : OwnLook(table, key).Over(under.Look(table, key));

    /// <summary>How a kind of part looks in a state (hover, pressed, focus, selected), or without one.</summary>
    public LookValues Part(string kind, string? state)
    {
        var plain = Look("parts", kind);
        return state is null ? plain : Look("parts", $"{kind}:{state}").Over(plain);
    }

    /// <summary>Only what a kind of part's state sets itself (its hover, its pressed), without its plain look under it; nothing without a state.</summary>
    public LookValues PartOwn(string kind, string? state) => state is null ? LookValues.None : Look("parts", $"{kind}:{state}");

    /// <summary>A style of the theme: <c>style primary background accent</c>.</summary>
    public LookValues Style(string name) => Look("styles", name);

    /// <summary>
    /// How a part that is pressed (a button, a toggle) of <paramref name="kind"/> looks with its style in a state:
    /// the part's look, the style over it, and the state's own look over both. A style that fills the part keeps
    /// its colors in a state, so its text stays readable: the state shades the style's fill away from its text (a
    /// little on hover, more pressed) and takes only its border from the state's look.
    /// </summary>
    public LookValues Pressable(string kind, string? style, string? state)
    {
        var plain = Part(kind, null);
        var own = style is { Length: > 0 } ? Style(style) : LookValues.None;
        var styled = own.Over(plain);
        if (state is null)
            return styled;
        var stateLook = PartOwn(kind, state);
        if (own.Background is not { } fill)
            return stateLook.Over(styled);
        var text = styled.Text ?? Color("text");
        return (LookValues.None with { Background = Shade(fill, text, state), Text = text, Border = stateLook.Border }).Over(styled);
    }

    /// <summary>A fill shaded for a state, toward whichever of the theme's text and background colors is farther from <paramref name="text"/>, so the text only gains contrast: a little on hover, more for any other state.</summary>
    SKColor Shade(SKColor fill, SKColor text, string state)
    {
        var amount = state == "hover" ? 0.12 : 0.24;
        var (dark, light) = (Color("text"), Color("background"));
        double Far(SKColor c) => Math.Abs(Luminance(c) - Luminance(text));
        var toward = Far(dark) >= Far(light) ? dark : light;
        byte Mix(byte a, byte b) => (byte)Math.Round(a + (b - a) * amount, MidpointRounding.AwayFromZero);
        return new SKColor(Mix(fill.Red, toward.Red), Mix(fill.Green, toward.Green), Mix(fill.Blue, toward.Blue), fill.Alpha);
    }

    /// <summary>A color's relative luminance, as WCAG measures it.</summary>
    public static double Luminance(SKColor color)
    {
        static double Channel(byte c)
        {
            var v = c / 255.0;
            return v <= 0.03928 ? v / 12.92 : Math.Pow((v + 0.055) / 1.055, 2.4);
        }
        return 0.2126 * Channel(color.Red) + 0.7152 * Channel(color.Green) + 0.0722 * Channel(color.Blue);
    }

    /// <summary>The contrast between two colors, from 1 (none) to 21 (black on white), as WCAG measures it.</summary>
    public static double Contrast(SKColor a, SKColor b)
    {
        var (la, lb) = (Luminance(a), Luminance(b));
        return (Math.Max(la, lb) + 0.05) / (Math.Min(la, lb) + 0.05);
    }
}
