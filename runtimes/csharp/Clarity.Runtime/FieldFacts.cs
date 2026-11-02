// What a screen needs to know of a field beyond its value (DESIGN 7.9): its
// label, how it is shown and read, whether it is required, its range, and
// what it refers to. The generated application gives a table of them, keyed
// "Entity.field" (the entity's type name, the field as Clarity writes it):
// `Fields.All["Order.total"]`.

namespace Clarity.Runtime;

/// <param name="Label">Its label as written: a text key (<c>@orderTotalLabel</c>) or the text itself.</param>
/// <param name="Format">How its values are shown and read; none for a field without a format.</param>
/// <param name="Required">Whether a record needs a value in it.</param>
/// <param name="Min">Its least value, for a number or a date, as Clarity writes it; none without.</param>
/// <param name="Max">Its greatest value; none without.</param>
/// <param name="Refers">The entity a reference points at (<c>Person</c>); none for any other field.</param>
/// <param name="Parent">For an entity whose records make a tree: the field that names a record's parent; none otherwise.</param>
public sealed record FieldFacts(string Label, FieldFormat? Format = null, bool Required = false, string? Min = null, string? Max = null, string? Refers = null, string? Parent = null);
