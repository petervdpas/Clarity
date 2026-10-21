// The theme of Clarity chosen in its app settings, applied to the page and
// remembered for the next start (ideTheme.ts says what each one is).

import { isIdeTheme, schemeOf, type IdeTheme } from "./ideTheme";
import { scheme } from "./scheme.svelte";

const KEY = "clarity-ide-theme";

function remembered(): IdeTheme {
  try {
    const value = localStorage.getItem(KEY);
    return isIdeTheme(value) ? value : "system";
  } catch {
    return "system";
  }
}

export const ideTheme = $state<{ value: IdeTheme }>({ value: "system" });

let media: MediaQueryList | null = null;

function apply() {
  const systemDark = media?.matches ?? false;
  // What follows the system in the designer still follows the system.
  scheme.dark = systemDark;
  const root = document.documentElement;
  root.dataset.scheme = schemeOf(ideTheme.value, systemDark);
  root.dataset.ideTheme = ideTheme.value;
}

/** Applies the remembered theme, and follows the system for the System theme, from now on. */
export function startIdeTheme() {
  media = window.matchMedia("(prefers-color-scheme: dark)");
  ideTheme.value = remembered();
  apply();
  media.addEventListener("change", apply);
  // The manual's window follows a choice made in the main one.
  window.addEventListener("storage", (e) => {
    if (e.key === KEY && isIdeTheme(e.newValue)) {
      ideTheme.value = e.newValue;
      apply();
    }
  });
}

export function setIdeTheme(value: IdeTheme) {
  ideTheme.value = value;
  try {
    localStorage.setItem(KEY, value);
  } catch {
    // Not remembered: the next start shows the system's.
  }
  apply();
}
