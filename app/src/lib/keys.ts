// Shortcut keys of the IDE (DESIGN 5.28), the pilot of keys in the language:
// every command once, with its name, the layers it is active in and its
// keys. What is active answers a key, the innermost layer first; the part
// with the focus keeps its own keys before that. The person sets the keys,
// kept as what differs from these, and a key taken in the same layer swaps.

/** Where a command is active: everywhere, or one of the views (the ribbon's tabs) and the vault's two lists. */
export type Layer = "global" | "project" | "design" | "preview" | "vault" | "vault.secrets" | "vault.connections";

export type Command = { id: string; name: string; layers: Layer[]; keys: string[] };

/** The layers in the order the keys window shows them, with their names. */
export const LAYERS: { layer: Layer; name: string }[] = [
  { layer: "global", name: "Everywhere" },
  { layer: "project", name: "Project" },
  { layer: "design", name: "Design" },
  { layer: "preview", name: "Preview" },
  { layer: "vault", name: "Vault" },
  { layer: "vault.secrets", name: "Vault: secrets" },
  { layer: "vault.connections", name: "Vault: connections" },
];

/** The IDE's commands with their keys as Clarity gives them. Change password
 *  and Destroy vault have none: what cannot be undone is not one key away. */
export const IDE_COMMANDS: Command[] = [
  { id: "manual", name: "Manual", layers: ["global"], keys: ["F1"] },
  { id: "view.project", name: "Project view", layers: ["global"], keys: ["Ctrl+1"] },
  { id: "view.design", name: "Design view", layers: ["global"], keys: ["Ctrl+2"] },
  { id: "view.preview", name: "Preview view", layers: ["global"], keys: ["Ctrl+3"] },
  { id: "view.vault", name: "Vault view", layers: ["global"], keys: ["Ctrl+4"] },
  { id: "project.open", name: "Open project", layers: ["global"], keys: ["Ctrl+O"] },
  { id: "project.new", name: "New project", layers: ["global"], keys: ["Ctrl+Shift+N"] },
  { id: "settings", name: "Clarity settings", layers: ["global"], keys: ["Ctrl+,"] },
  { id: "agent", name: "Agent panel", layers: ["global"], keys: [] },
  { id: "project.reload", name: "Reload", layers: ["project"], keys: ["F5"] },
  { id: "project.settings", name: "Project settings", layers: ["project"], keys: [] },
  { id: "project.check", name: "Check", layers: ["project"], keys: ["F7"] },
  { id: "window.new", name: "New window", layers: ["project", "design"], keys: ["Ctrl+Shift+W"] },
  { id: "design.undo", name: "Undo", layers: ["design"], keys: ["Ctrl+Z"] },
  { id: "design.redo", name: "Redo", layers: ["design"], keys: ["Ctrl+Y", "Ctrl+Shift+Z"] },
  { id: "design.grid", name: "Grid", layers: ["design"], keys: ["Ctrl+G"] },
  { id: "preview.main", name: "Main window", layers: ["preview"], keys: ["Alt+Home"] },
  { id: "preview.back", name: "Back", layers: ["preview"], keys: ["Alt+Left"] },
  { id: "preview.run", name: "Run", layers: ["preview"], keys: ["F5"] },
  { id: "preview.build", name: "Build", layers: ["preview"], keys: ["Ctrl+Shift+B"] },
  { id: "preview.stop", name: "Stop", layers: ["preview"], keys: ["Shift+F5"] },
  { id: "vault.secrets", name: "Secrets", layers: ["vault"], keys: [] },
  { id: "vault.connections", name: "Connections", layers: ["vault"], keys: [] },
  { id: "vault.lock", name: "Lock", layers: ["vault"], keys: ["Ctrl+L"] },
  { id: "vault.add", name: "Add", layers: ["vault.secrets"], keys: ["Ctrl+N"] },
  { id: "vault.edit", name: "Edit", layers: ["vault.secrets"], keys: ["F2"] },
  { id: "vault.delete", name: "Delete", layers: ["vault.secrets"], keys: ["Delete"] },
  { id: "vault.copy", name: "Copy value", layers: ["vault.secrets"], keys: ["Ctrl+Shift+C"] },
  { id: "vault.categories", name: "Categories", layers: ["vault.secrets"], keys: [] },
  { id: "vault.newConnection", name: "New connection", layers: ["vault.connections"], keys: ["Ctrl+N"] },
  { id: "vault.removeConnection", name: "Delete connection", layers: ["vault.connections"], keys: ["Delete"] },
];

/** The person's keys by command, where they differ from the design's ([] for none). */
export type KeyChoices = Record<string, string[]>;

/** A command's keys: the person's, or the design's. */
export function keysOf(command: Command, choices: KeyChoices): string[] {
  return choices[command.id] ?? command.keys;
}

const NAMES: Record<string, string> = {
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  " ": "Space",
  Esc: "Escape",
  Del: "Delete",
};
const MODIFIERS = new Set(["Control", "Shift", "Alt", "Meta", "OS", "AltGraph"]);

type Press = { key: string; code: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; shiftKey: boolean };

/** A key press as Clarity writes keys (`Ctrl+Shift+N`); on macOS Cmd is Ctrl
 *  and the Control key is `Control`. Null for a modifier on its own. */
export function eventKey(e: Press, mac: boolean): string | null {
  if (MODIFIERS.has(e.key)) return null;
  const held: string[] = [];
  if (mac ? e.metaKey : e.ctrlKey) held.push("Ctrl");
  if (mac && e.ctrlKey) held.push("Control");
  if (!mac && e.metaKey) held.push("Meta");
  if (e.altKey) held.push("Alt");
  if (e.shiftKey) held.push("Shift");
  // A digit by its place on the keyboard, whatever the layout makes of it.
  const digit = /^Digit(\d)$/.exec(e.code)?.[1];
  const key = digit ?? NAMES[e.key] ?? (e.key.length === 1 ? e.key.toUpperCase() : e.key);
  return [...held, key].join("+");
}

/** A key as the person reads it: Cmd for Ctrl on macOS. */
export function shownKey(key: string, mac: boolean): string {
  return mac ? key.replace(/^Ctrl(?=\+)/, "Cmd") : key;
}

/** The modifiers of a key and the key itself: `Ctrl++` is Ctrl and `+`. */
function parts(key: string): [string[], string] {
  if (key.endsWith("++")) return [key.slice(0, -2).split("+").filter(Boolean), "+"];
  const all = key.split("+");
  const last = all.pop() ?? "";
  return [all, last];
}

/** The command `key` runs among the active layers, innermost first; null when none has it. */
export function commandFor(key: string, active: Layer[], commands: Command[], choices: KeyChoices): string | null {
  for (const layer of active) {
    const found = commands.find((c) => c.layers.includes(layer) && keysOf(c, choices).includes(key));
    if (found) return found.id;
  }
  return null;
}

/** Keys given twice within one layer: the layer, the key and the commands. */
export function clashes(commands: Command[], choices: KeyChoices): [Layer, string, string[]][] {
  const found: [Layer, string, string[]][] = [];
  for (const { layer } of LAYERS) {
    const seen = new Map<string, string[]>();
    for (const c of commands.filter((c) => c.layers.includes(layer))) {
      for (const key of keysOf(c, choices)) seen.set(key, [...(seen.get(key) ?? []), c.id]);
    }
    for (const [key, ids] of seen) if (ids.length > 1) found.push([layer, key, ids]);
  }
  return found;
}

/** Gives `id` the key `key`. A command that has it in a layer they share gets
 *  the keys `id` had instead (`swapped`); what is kept is only what differs from the design. */
export function assign(commands: Command[], choices: KeyChoices, id: string, key: string): { choices: KeyChoices; swapped: string | null } {
  const command = commands.find((c) => c.id === id);
  if (!command) return { choices, swapped: null };
  const keys: Record<string, string[]> = Object.fromEntries(commands.map((c) => [c.id, keysOf(c, choices)]));
  const old = keys[id];
  const other = commands.find((c) => c.id !== id && c.layers.some((l) => command.layers.includes(l)) && keys[c.id].includes(key));
  if (other) keys[other.id] = [...keys[other.id].filter((k) => k !== key), ...old.filter((k) => !keys[other.id].includes(k))];
  keys[id] = [key];
  const next: KeyChoices = {};
  for (const c of commands) {
    const same = keys[c.id].length === c.keys.length && keys[c.id].every((k, i) => k === c.keys[i]);
    if (!same) next[c.id] = keys[c.id];
  }
  return { choices: next, swapped: other?.id ?? null };
}

/** Why a key cannot be a command's: the system and the inputs need it. Null when it can. */
export function refused(key: string): "tab" | "enter" | "escape" | "typing" | null {
  const [held, last] = parts(key);
  if (last === "Tab") return "tab";
  if (last === "Enter") return "enter";
  if (last === "Escape") return "escape";
  const plain = !held.some((m) => m === "Ctrl" || m === "Alt" || m === "Meta" || m === "Control");
  if (plain && (last.length === 1 || last === "Space")) return "typing";
  return null;
}

/** The keys a text field uses itself: plain keys, and its editing keys with Ctrl or Alt. */
const FIELD_KEYS = new Set(["Ctrl+Z", "Ctrl+Y", "Ctrl+Shift+Z", "Ctrl+A", "Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Left", "Ctrl+Right", "Ctrl+Shift+Left", "Ctrl+Shift+Right", "Ctrl+Backspace", "Ctrl+Delete", "Ctrl+Home", "Ctrl+End", "Alt+Left", "Alt+Right", "Alt+Backspace"]);

/** Whether a text field with the focus keeps `key` for itself: then no command takes it. */
export function fieldKeeps(key: string): boolean {
  if (FIELD_KEYS.has(key)) return true;
  const [held, last] = parts(key);
  const plain = !held.some((m) => m === "Ctrl" || m === "Alt" || m === "Meta" || m === "Control");
  return plain && !/^F\d+$/.test(last);
}

/** The commands of other layers that have `key` too: allowed, and the inner one goes first where both are active. */
export function alsoHas(commands: Command[], choices: KeyChoices, id: string, key: string): string[] {
  const command = commands.find((c) => c.id === id);
  if (!command) return [];
  return commands.filter((c) => c.id !== id && !c.layers.some((l) => command.layers.includes(l)) && keysOf(c, choices).includes(key)).map((c) => c.id);
}

/** Puts a command's keys back to the design's: a command that took one of them in a swap gets its own back. */
export function reset(commands: Command[], choices: KeyChoices, id: string): KeyChoices {
  const command = commands.find((c) => c.id === id);
  if (!command) return choices;
  let next = choices;
  for (const key of command.keys) next = assign(commands, next, id, key).choices;
  const { [id]: _mine, ...rest } = next;
  return rest;
}
