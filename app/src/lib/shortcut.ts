// Matching a button's shortcut, such as `Ctrl+S` or `+`, against a key
// press. The checker in Rust decides which shortcuts are valid; this only
// reads them, the same way (clarity_spec::widgets::shortcut_keys).

/** Key names in a shortcut whose `KeyboardEvent.key` differs. */
const KEYS: Record<string, string> = {
  Space: " ",
  Up: "ArrowUp",
  Down: "ArrowDown",
  Left: "ArrowLeft",
  Right: "ArrowRight",
};

/** The modifiers of a shortcut and its key: `Ctrl++` is Ctrl and `+`. */
export function shortcutKeys(shortcut: string): [string[], string] {
  if (shortcut === "+") return [[], "+"];
  if (shortcut.endsWith("++") && shortcut.length > 2) return [shortcut.slice(0, -2).split("+"), "+"];
  const keys = shortcut.split("+");
  const key = keys.pop() ?? "";
  return [keys, key];
}

/** Whether `shortcut` matches the press `e`. `typing` says the press is in
 *  a text input: a key without Ctrl, Alt or Meta is then the person's
 *  typing, not the button's. */
export function matchesShortcut(e: KeyboardEvent, shortcut: string, typing: boolean): boolean {
  const [modifiers, key] = shortcutKeys(shortcut);
  const held = (name: string) => modifiers.includes(name);
  if (e.ctrlKey !== held("Ctrl") || e.altKey !== held("Alt") || e.metaKey !== held("Meta")) return false;
  // A character key is what is typed, from the keyboard or the number pad.
  const typed = key.length === 1;
  if (typed && !held("Ctrl") && !held("Alt") && !held("Meta") && typing) return false;
  const symbol = typed && !/[a-z0-9]/i.test(key);
  // Shift makes some symbols (+, *), so it only counts for a symbol when written.
  if (!(symbol && !held("Shift")) && e.shiftKey !== held("Shift")) return false;
  const wanted = KEYS[key] ?? key;
  // Letters match either case: Shift is checked on its own.
  const pressed = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  return pressed === (wanted.length === 1 ? wanted.toUpperCase() : wanted);
}
