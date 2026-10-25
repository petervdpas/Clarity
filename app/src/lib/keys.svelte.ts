// The person's own shortcut keys for the IDE (DESIGN 5.28), kept with
// Clarity's other own choices and remembered for the next start.

import { IDE_COMMANDS, assign, keysOf, reset, type KeyChoices } from "./keys";

const KEY = "clarity-ide-keys";

function remembered(): KeyChoices {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    if (!value || typeof value !== "object") return {};
    // Only commands Clarity still has, each with a list of keys.
    const known = new Set(IDE_COMMANDS.map((c) => c.id));
    return Object.fromEntries(Object.entries(value).filter(([id, keys]) => known.has(id) && Array.isArray(keys) && keys.every((k) => typeof k === "string")));
  } catch {
    return {};
  }
}

export const ideKeys = $state<{ choices: KeyChoices }>({ choices: {} });

/** Whether the keys show as on macOS: Cmd for Ctrl. */
export const mac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export function startIdeKeys() {
  ideKeys.choices = remembered();
  // Another window of Clarity follows a change.
  window.addEventListener("storage", (e) => {
    if (e.key === KEY) ideKeys.choices = remembered();
  });
}

function keep() {
  try {
    localStorage.setItem(KEY, JSON.stringify(ideKeys.choices));
  } catch {
    // Not kept: the keys still hold until Clarity closes.
  }
}

/** Gives a command a key; the command that had it in the same layer gets this one's keys. Gives back that command. */
export function setIdeKey(id: string, key: string): string | null {
  const done = assign(IDE_COMMANDS, ideKeys.choices, id, key);
  ideKeys.choices = done.choices;
  keep();
  return done.swapped;
}

/** Puts one command's keys back to Clarity's, or every one's. */
export function resetIdeKeys(id?: string) {
  ideKeys.choices = id === undefined ? {} : reset(IDE_COMMANDS, ideKeys.choices, id);
  keep();
}

/** A command's keys now. */
export function ideKeysOf(id: string): string[] {
  const command = IDE_COMMANDS.find((c) => c.id === id);
  return command ? keysOf(command, ideKeys.choices) : [];
}
