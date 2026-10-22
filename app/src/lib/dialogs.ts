// Clarity's own dialogs, for whatever needs the person before going on:
// confirm (yes or no), message (read and close), prompt (type a text) and
// choose (one of several). They are shown by DialogHost.svelte inside
// Clarity's window, never as a dialog of the system or the browser. A
// dialog asked for while another is open waits its turn.

export type ConfirmDialog = { kind: "confirm"; text: string; title?: string; action: string; danger?: boolean };
export type MessageDialog = { kind: "message"; text: string; title?: string };
export type PromptDialog = { kind: "prompt"; text: string; title?: string; value?: string; placeholder?: string; action: string };
export type ChooseDialog = { kind: "choose"; text: string; title?: string; choices: { value: string; label: string; danger?: boolean }[] };
export type Dialog = ConfirmDialog | MessageDialog | PromptDialog | ChooseDialog;

/** What each kind of dialog gives when it closes. */
type Answer = boolean | string | null | undefined;

type Without<T> = T extends unknown ? Omit<T, "kind"> : never;

export class Dialogs {
  current: Dialog | null = null;
  #shown: (dialog: Dialog | null) => void;
  #waiting: { dialog: Dialog; resolve: (answer: Answer) => void }[] = [];
  #resolve: ((answer: Answer) => void) | null = null;

  /** `shown` is told each time the dialog on screen changes. */
  constructor(shown: (dialog: Dialog | null) => void) {
    this.#shown = shown;
  }

  /** True when the person says yes. */
  confirm(dialog: Without<ConfirmDialog>): Promise<boolean> {
    return this.#open({ kind: "confirm", ...dialog }).then((a) => a === true);
  }

  /** Resolves when the person closes it. */
  message(dialog: Without<MessageDialog>): Promise<void> {
    return this.#open({ kind: "message", ...dialog }).then(() => undefined);
  }

  /** The text typed, or null when cancelled. */
  prompt(dialog: Without<PromptDialog>): Promise<string | null> {
    return this.#open({ kind: "prompt", ...dialog }).then((a) => (typeof a === "string" ? a : null));
  }

  /** The value of the choice made, or null when cancelled. */
  choose(dialog: Without<ChooseDialog>): Promise<string | null> {
    return this.#open({ kind: "choose", ...dialog }).then((a) => (typeof a === "string" ? a : null));
  }

  /**
   * Closes the dialog on screen with the person's answer: true or false
   * for confirm (true for message), the text for prompt, the value for
   * choose, null to cancel.
   */
  answer(answer: Answer) {
    const resolve = this.#resolve;
    this.current = null;
    this.#resolve = null;
    this.#shown(null);
    resolve?.(answer);
    this.#next();
  }

  #open(dialog: Dialog): Promise<Answer> {
    return new Promise((resolve) => {
      this.#waiting.push({ dialog, resolve });
      if (this.current === null) this.#next();
    });
  }

  #next() {
    const next = this.#waiting.shift();
    if (!next) return;
    this.current = next.dialog;
    this.#resolve = next.resolve;
    this.#shown(next.dialog);
  }
}
