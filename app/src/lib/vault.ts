// The rules of the Vault tab that need no vault: which secrets show, and
// what is wrong with a connection before it is saved.

import type { ConnectionRow } from "./generated/ConnectionRow";
import type { SecretRow } from "./generated/SecretRow";

/** The secrets of `category` (null: all) holding `filter` in a column, ignoring case. */
export function shownSecrets(rows: SecretRow[], category: string | null, filter: string): SecretRow[] {
  const wanted = filter.trim().toLowerCase();
  return rows.filter(
    (r) =>
      (category === null || r.category.toLowerCase() === category.toLowerCase()) &&
      (wanted === "" || [r.category, r.key, r.description].some((t) => t.toLowerCase().includes(wanted))),
  );
}

/** Why a connection cannot be saved yet; empty when it can. */
export function connectionProblems(row: ConnectionRow): string[] {
  const problems: string[] = [];
  if (row.name.trim() === "") problems.push("The connection needs a name.");
  const seen = new Set<string>();
  for (const [name, value] of row.fields) {
    const trimmed = name.trim();
    if (trimmed === "") {
      if (!problems.includes("Every field needs a name.")) problems.push("Every field needs a name.");
      continue;
    }
    if (seen.has(trimmed.toLowerCase())) problems.push(`Field ${trimmed} is there twice.`);
    seen.add(trimmed.toLowerCase());
    if (value.mode === "vault" && (value.category.trim() === "" || value.key.trim() === "")) {
      problems.push(`Field ${trimmed} needs the category and key of its secret.`);
    }
  }
  return problems;
}

/** A name for a new connection that is not in use yet. */
export function freshName(names: string[]): string {
  const taken = new Set(names.map((n) => n.toLowerCase()));
  if (!taken.has("connection")) return "connection";
  for (let n = 2; ; n++) if (!taken.has(`connection ${n}`)) return `connection ${n}`;
}

/** A rename made in the categories editor: its secrets move from `from` to `to`. */
export type CategoryRename = { from: string; to: string };

const byName = (a: string, b: string) => a.toLowerCase().localeCompare(b.toLowerCase());

/**
 * The categories being edited: added, renamed and removed by the person,
 * saved in one go. Each name remembers the name it had when editing began
 * (null for one added since), so a rename moves the secrets of the first
 * name, however often it was renamed.
 */
export class CategoryEditor {
  items: string[];
  #original = new Map<string, string | null>();
  #usage: Map<string, number>;

  /** `usage` counts the secrets of each category, by its name in lower case. */
  constructor(categories: string[], usage: Map<string, number>) {
    this.items = [...new Set(categories.map((c) => c.trim()).filter((c) => c !== ""))].sort(byName);
    for (const c of this.items) this.#original.set(c.toLowerCase(), c);
    this.#usage = usage;
  }

  /** How many secrets use the category now named `name`. */
  usage(name: string): number {
    const original = this.#original.get(name.toLowerCase());
    return original ? (this.#usage.get(original.toLowerCase()) ?? 0) : 0;
  }

  #taken(name: string, except: string | null = null): boolean {
    return this.items.some((c) => c.toLowerCase() === name.toLowerCase() && c.toLowerCase() !== except?.toLowerCase());
  }

  /** Adds a category; the reason it cannot be added, or null. */
  add(name: string): string | null {
    const trimmed = name.trim();
    if (trimmed === "") return "A category needs a name.";
    if (this.#taken(trimmed)) return `A category named '${trimmed}' already exists.`;
    this.items = [...this.items, trimmed].sort(byName);
    this.#original.set(trimmed.toLowerCase(), null);
    return null;
  }

  /** Renames a category; the reason it cannot be renamed, or null. */
  rename(current: string, name: string): string | null {
    const trimmed = name.trim();
    if (trimmed === "" || trimmed === current) return null;
    if (this.#taken(trimmed, current)) return `A category named '${trimmed}' already exists.`;
    const original = this.#original.get(current.toLowerCase()) ?? null;
    this.#original.delete(current.toLowerCase());
    this.#original.set(trimmed.toLowerCase(), original);
    this.items = this.items.map((c) => (c === current ? trimmed : c)).sort(byName);
    return null;
  }

  /** Removes a category; the reason it cannot go, or null. */
  remove(name: string): string | null {
    const used = this.usage(name);
    if (used > 0) return `Can't remove '${name}': ${used} secret(s) still use it. Edit those secrets to reassign, then remove the category.`;
    this.items = this.items.filter((c) => c !== name);
    this.#original.delete(name.toLowerCase());
    return null;
  }

  /** The list to keep, and the renames that move secrets. */
  result(): { names: string[]; renames: CategoryRename[] } {
    const renames: CategoryRename[] = [];
    for (const name of this.items) {
      const original = this.#original.get(name.toLowerCase());
      if (original && original !== name) renames.push({ from: original, to: name });
    }
    return { names: [...this.items], renames };
  }
}
