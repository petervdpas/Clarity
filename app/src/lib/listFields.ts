// Lists of values as fields (U1c), as every renderer edits and shows them
// (conformance/list-fields.json): a typed item is trimmed; empty text and
// an item the list has add nothing; text that is no value of the list's
// kind is refused and stays as typed. A list in a table cell is its items
// joined by a comma and a space, an enum's values by their labels.

/** What the items of a list field are: `list<text>`, `list<Genre>`. */
export type ItemKind = "text" | "integer" | "decimal" | "date" | "enum";

/** One item as it travels: text, a whole number, a decimal as text, a date as `2026-10-23`, an enum value by name. */
export type Item = string | number;

const WHOLE = /^-?\d+$/;
const DECIMAL = /^-?\d+(\.\d+)?$/;

/** A decimal written alike for the same value: `1.50` and `1.5` are one item. */
function plain(decimal: string): string {
  const negative = decimal.startsWith("-");
  const [whole, fraction = ""] = decimal.replace(/^-/, "").split(".");
  const w = whole.replace(/^0+(?=\d)/, "");
  const f = fraction.replace(/0+$/, "");
  const text = f ? `${w}.${f}` : w;
  return negative && text !== "0" ? `-${text}` : text;
}

/** Whether two items are the same value. */
export function sameItem(kind: ItemKind, a: Item, b: Item): boolean {
  if (kind === "decimal") return plain(String(a)) === plain(String(b));
  return a === b;
}

/**
 * What a typed item does to a list: the list it gives (the same list when
 * nothing is added), or `refused` when the text is no value of its kind.
 */
export function addTyped(kind: ItemKind, items: Item[], typed: string): { items: Item[] } | { refused: true } {
  const text = typed.trim();
  if (text === "") return { items };
  let item: Item;
  if (kind === "integer") {
    if (!WHOLE.test(text)) return { refused: true };
    item = Number(text);
  } else if (kind === "decimal") {
    if (!DECIMAL.test(text)) return { refused: true };
    item = text;
  } else {
    item = text;
  }
  return { items: added(kind, items, item) };
}

/** The list with an item added at its end, unless it has it. */
export function added(kind: ItemKind, items: Item[], item: Item): Item[] {
  return items.some((i) => sameItem(kind, i, item)) ? items : [...items, item];
}

/** A list in a table cell: its items joined, each as `label` shows it. */
export function shownList(items: Item[] | null | undefined, label: (item: Item) => string = String): string {
  return (items ?? []).map(label).join(", ");
}
