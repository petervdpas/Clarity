// Matching a button's shortcut, such as `Ctrl+S`, against a key press. The
// checker in Rust decides which shortcuts are valid; this only reads them.

/** Key names in a shortcut whose `KeyboardEvent.key` differs. */
const KEYS: Record<string, string> = {
  Space: " ",
  Up: "ArrowUp",
  Down: "ArrowDown",
  Left: "ArrowLeft",
  Right: "ArrowRight",
};

export function matchesShortcut(e: KeyboardEvent, shortcut: string): boolean {
  const keys = shortcut.split("+");
  const key = keys.pop() ?? "";
  const wanted = KEYS[key] ?? key;
  // Letters match either case: Shift is checked on its own.
  const pressed = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  return (
    pressed === (wanted.length === 1 ? wanted.toUpperCase() : wanted) &&
    e.ctrlKey === keys.includes("Ctrl") &&
    e.altKey === keys.includes("Alt") &&
    e.shiftKey === keys.includes("Shift") &&
    e.metaKey === keys.includes("Meta")
  );
}
