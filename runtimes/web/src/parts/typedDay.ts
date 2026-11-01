// A day typed into a date input (input type=date), taken as typed text is:
// once, when the input is left or Enter is pressed. While keys are typed the
// browser says each date in between (0002, 0020, 0202 on the way to 2024);
// those are no changes. A day picked from the calendar, with no key typed
// since the day was last taken, is taken at once. Ctrl+Z while typing puts
// back the day it had, as a text input's own undo does (a date input has none).

/** Keys that change a date input's day as they are typed. */
const TYPING = /^([0-9]|Backspace|Delete|ArrowUp|ArrowDown)$/;

export function typedDay(node: HTMLInputElement, take: (day: string) => void) {
  let typing = false;
  let pending: string | null = null;
  let taker = take;
  /** The day the input had when typing started: what Ctrl+Z puts back. */
  let before = node.value;

  const commit = () => {
    if (pending !== null) taker(pending);
    pending = null;
    typing = false;
  };
  const keydown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z" && !e.shiftKey && typing) {
      // The typing is taken back here; the window's undo is for days taken.
      e.preventDefault();
      e.stopPropagation();
      node.value = before;
      pending = null;
      typing = false;
      return;
    }
    if (e.key === "Enter") commit();
    else if (TYPING.test(e.key)) {
      if (!typing) before = node.value;
      typing = true;
    }
  };
  const change = () => {
    if (typing) pending = node.value;
    else taker(node.value);
  };
  node.addEventListener("keydown", keydown);
  node.addEventListener("change", change);
  node.addEventListener("blur", commit);
  return {
    update(next: (day: string) => void) {
      taker = next;
    },
    destroy() {
      node.removeEventListener("keydown", keydown);
      node.removeEventListener("change", change);
      node.removeEventListener("blur", commit);
    },
  };
}
