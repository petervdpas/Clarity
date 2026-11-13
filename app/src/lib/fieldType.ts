// A field's type as the entity editor shows it: a kind from the tools' list
// (clarity_spec::dictionary, through the catalog), what a ref, enum, record
// or list holds, and a text's length or a decimal's precision and scale.
// Written back as the type text of a module (`text(80)`, `decimal(10, 2)`,
// `ref Customer`, `Status`, `list<Order>`), as the tools write it.

export type FieldTypeParts = {
  kind: string;
  /** What a ref, enum, record or list holds; empty for the other kinds. */
  of: string;
  length: number | null;
  precision: number | null;
  scale: number | null;
};

/** The names a type text can hold, by what they are. */
export type TypeNames = { enums: readonly string[]; records: readonly string[] };

/** The parts of a type text; an enum and a record are written by their names alone. */
export function typeParts(typeText: string, names: TypeNames): FieldTypeParts {
  const text = typeText.trim();
  const plain = { of: "", length: null, precision: null, scale: null };
  const sized = /^(text|decimal)\s*\(([^)]*)\)$/.exec(text);
  if (sized) {
    const numbers = sized[2].split(",").map((n) => Number.parseInt(n.trim(), 10));
    return sized[1] === "text"
      ? { ...plain, kind: "text", length: numbers[0] ?? null }
      : { ...plain, kind: "decimal", precision: numbers[0] ?? null, scale: numbers[1] ?? null };
  }
  const ref = /^ref\s+(\w+)$/.exec(text);
  if (ref) return { ...plain, kind: "ref", of: ref[1] };
  const list = /^list\s*<\s*(\w+)\s*>$/.exec(text);
  if (list) return { ...plain, kind: "list", of: list[1] };
  if (names.enums.includes(text)) return { ...plain, kind: "enum", of: text };
  if (names.records.includes(text)) return { ...plain, kind: "record", of: text };
  return { ...plain, kind: text };
}

/** The type text of the parts; null while it misses something (a ref with nothing to refer to, a decimal without its sizes). */
export function typeText(parts: FieldTypeParts): string | null {
  switch (parts.kind) {
    case "text":
      return parts.length ? `text(${parts.length})` : "text";
    case "decimal":
      return parts.precision && parts.scale !== null ? `decimal(${parts.precision}, ${parts.scale})` : null;
    case "ref":
      return parts.of ? `ref ${parts.of}` : null;
    case "list":
      return parts.of ? `list<${parts.of}>` : null;
    case "enum":
    case "record":
      return parts.of || null;
    default:
      return parts.kind || null;
  }
}

/** What a kind's "of" offers: entities for ref, enums, records, and for list the entities, records and simple types. */
export function ofChoices(kind: string, names: TypeNames & { entities: readonly string[]; simple: readonly string[] }): string[] {
  switch (kind) {
    case "ref":
      return [...names.entities];
    case "enum":
      return [...names.enums];
    case "record":
      return [...names.records];
    case "list":
      return [...names.entities, ...names.records, ...names.simple];
    default:
      return [];
  }
}

/** The parts after a new kind is picked: a text starts at 80 characters, a decimal at 10 and 2, a named kind at its first choice. */
export function withKind(kind: string, choices: readonly string[]): FieldTypeParts {
  const parts: FieldTypeParts = { kind, of: choices[0] ?? "", length: null, precision: null, scale: null };
  if (kind === "text") parts.length = 80;
  if (kind === "decimal") {
    parts.precision = 10;
    parts.scale = 2;
  }
  return parts;
}
