// The web page's own right-click menu (Back, Reload, Inspect) belongs to a
// browser, not to an application: it is off, but in text fields, where Cut,
// Copy and Paste stay. A part with a menu of its own opens that instead.

/** Whether a right click on `target` keeps the page's menu: in a text field. */
export function keepsPageMenu(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element || typeof element.closest !== "function") return false;
  return element.closest("input, textarea, [contenteditable='true']") !== null;
}

/** Turns the page's own right-click menu off, from now on. */
export function pageMenuOff() {
  window.addEventListener("contextmenu", (e) => {
    if (!keepsPageMenu(e.target)) e.preventDefault();
  });
}
