// The dropdown's options of U1c, as every renderer offers them
// (conformance/dropdown-options.json). A `typed` dropdown offers the values
// its field already has: each once, without empty text, those containing
// the typed text (ignoring case), sorted ignoring case and then as
// written. A `search` dropdown offers the records whose text or shown
// fields hold every typed word, ignoring case, in the order they came.

/** A record a reference can point at, with the fields a search shows of it. */
export type RefOption = { id: number; text: string; fields?: Record<string, unknown> };

function before(a: string, b: string): number {
  const la = a.toLowerCase();
  const lb = b.toLowerCase();
  if (la !== lb) return la < lb ? -1 : 1;
  return a < b ? -1 : a > b ? 1 : 0;
}

/** The suggestions of a typed dropdown for what is typed. */
export function suggestions(values: string[], typed: string): string[] {
  const wanted = typed.trim().toLowerCase();
  const once = [...new Set(values.filter((v) => v !== ""))];
  return once.filter((v) => v.toLowerCase().includes(wanted)).sort(before);
}

/** The records a search dropdown offers for what is typed; `shown` gives a field's text. */
export function searched(options: RefOption[], typed: string, shown: (option: RefOption) => string[] = (o) => Object.values(o.fields ?? {}).map((v) => String(v ?? ""))): RefOption[] {
  const words = typed.toLowerCase().split(/\s+/).filter((w) => w !== "");
  if (words.length === 0) return options;
  return options.filter((o) => {
    const texts = [o.text, ...shown(o)].map((t) => t.toLowerCase());
    return words.every((w) => texts.some((t) => t.includes(w)));
  });
}
