// Clarity's dialogs for the whole app: `dialogs.confirm`, `message`,
// `prompt` and `choose` (dialogs.ts), shown by DialogHost.svelte.

import { Dialogs, type Dialog } from "./dialogs";

class DialogStore {
  current = $state<Dialog | null>(null);
  #dialogs = new Dialogs((dialog) => (this.current = dialog));

  confirm = this.#dialogs.confirm.bind(this.#dialogs);
  message = this.#dialogs.message.bind(this.#dialogs);
  prompt = this.#dialogs.prompt.bind(this.#dialogs);
  choose = this.#dialogs.choose.bind(this.#dialogs);

  answer(answer: boolean | string | null) {
    this.#dialogs.answer(answer);
  }
}

export const dialogs = new DialogStore();
