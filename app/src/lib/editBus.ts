// Undo (DESIGN 5.29): every edit of a part goes over the window's edit bus
// as a message: the part, the scope it is undone in (the form or panel it
// is in, or the part itself), what changed in the window's data (each path
// with its value before and after), and the id of the backend's step when
// its handler changed the database. Ctrl+Z takes back the last message of
// the scope that has the focus, so in a form it walks back field by field.

import type { JsonValue } from "./generated/serde_json/JsonValue";

/** One path of the window's data, before and after an edit. */
export type Changed = { path: string; before: JsonValue | undefined; after: JsonValue | undefined };

/** One edit of a part. */
export type Edit = { part: string; scope: string; changes: Changed[]; step: number };

export class EditBus {
  private readonly done: Edit[] = [];
  private readonly undone: Edit[] = [];

  /** An edit happened; one that changed nothing (no data, no step) is no message. Ends what could be done again in its scope. */
  push(edit: Edit): void {
    if (edit.changes.length === 0 && edit.step === 0) return;
    this.done.push(edit);
    remove(this.undone, (e) => e.scope === edit.scope);
  }

  canUndo(scope: string): boolean {
    return this.done.some((e) => e.scope === scope);
  }

  canRedo(scope: string): boolean {
    return this.undone.some((e) => e.scope === scope);
  }

  /** The last edit of `scope`, to take back; it is then undone. */
  undo(scope: string): Edit | null {
    const edit = takeLast(this.done, scope);
    if (edit) this.undone.push(edit);
    return edit;
  }

  /** The last undone edit of `scope`, to do again. */
  redo(scope: string): Edit | null {
    const edit = takeLast(this.undone, scope);
    if (edit) this.done.push(edit);
    return edit;
  }

  /** Taking an edit back or doing it again was refused: it stays where it was. */
  refused(edit: Edit, wasUndo: boolean): void {
    const [from, to] = wasUndo ? [this.undone, this.done] : [this.done, this.undone];
    remove(from, (e) => e === edit);
    to.push(edit);
  }
}

function takeLast(list: Edit[], scope: string): Edit | null {
  for (let i = list.length - 1; i >= 0; i--) if (list[i].scope === scope) return list.splice(i, 1)[0];
  return null;
}

function remove(list: Edit[], gone: (e: Edit) => boolean): void {
  for (let i = list.length - 1; i >= 0; i--) if (gone(list[i])) list.splice(i, 1);
}

/** What changed between two copies of the window's data, by its top-level names. */
export function changesOf(before: Record<string, JsonValue>, after: Record<string, JsonValue>): Changed[] {
  const names = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...names].filter((n) => JSON.stringify(before[n]) !== JSON.stringify(after[n])).map((path) => ({ path, before: before[path], after: after[path] }));
}

/** Sets a path (`current.name`) of the window's data to a value; undefined takes it away. */
export function setPath(data: Record<string, JsonValue>, path: string, value: JsonValue | undefined): void {
  const names = path.split(".");
  let at = data as Record<string, JsonValue>;
  // A list's items by their place: `tasks.3.title`.
  for (const name of names.slice(0, -1)) {
    const next = at[name];
    if (next === null || typeof next !== "object") return;
    at = next as Record<string, JsonValue>;
  }
  const last = names.at(-1)!;
  if (value === undefined) delete at[last];
  else at[last] = value;
}
