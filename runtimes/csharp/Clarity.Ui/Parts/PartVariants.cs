// The parts that draw a variant of a kind another part draws, as the shared
// web screen picks them (runtimes/web/src/kinds/index.ts): a dropdown one
// may type in (`typed`), a dropdown that searches the records a reference
// points at (`search`), and an image bound to a record's picture. A variant
// class carries [PartVariant] with the kind it is a variant of, and a static
// `Fits(PartSpec, IPartHost)` that says whether a part of that kind is its
// variant. PartVariants.Make makes the variant that fits, else the kind's
// own part; PartKinds.Make asks For first, so every window makes them.

using System.Reflection;

namespace Clarity.Ui;

/// <summary>Marks a part class as a variant of <paramref name="kind"/>; the class has <c>public static bool Fits(PartSpec, IPartHost)</c>.</summary>
[AttributeUsage(AttributeTargets.Class)]
public sealed class PartVariantAttribute(string kind) : Attribute
{
    public string Kind { get; } = kind;
}

public static class PartVariants
{
    static readonly Lazy<List<(string Kind, Type Type, Func<PartSpec, IPartHost, bool> Fits)>> variants = new(() =>
        [.. typeof(Part).Assembly.GetTypes()
            .Where(t => !t.IsAbstract && typeof(Part).IsAssignableFrom(t))
            .Select(t => (Type: t, Variant: t.GetCustomAttribute<PartVariantAttribute>()))
            .Where(v => v.Variant is not null)
            .Select(v => (v.Variant!.Kind, v.Type, FitsOf(v.Type)))]);

    static Func<PartSpec, IPartHost, bool> FitsOf(Type type)
    {
        var method = type.GetMethod("Fits", BindingFlags.Public | BindingFlags.Static, [typeof(PartSpec), typeof(IPartHost)])
            ?? throw new InvalidOperationException($"{type.Name} is a part variant without a static Fits(PartSpec, IPartHost)");
        return (spec, host) => (bool)method.Invoke(null, [spec, host])!;
    }

    /// <summary>The variant class of <paramref name="spec"/>; none when its kind's own part draws it.</summary>
    public static Type? For(PartSpec spec, IPartHost host)
    {
        foreach (var (kind, type, fits) in variants.Value)
            if (kind == spec.Kind && fits(spec, host))
                return type;
        return null;
    }

    /// <summary>The part of <paramref name="spec"/>, its variant when one fits: what PartKinds.Make makes.</summary>
    public static Part Make(PartSpec spec, IPartHost host) => PartKinds.Make(spec, host);
}
