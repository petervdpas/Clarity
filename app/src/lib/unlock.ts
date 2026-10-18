// The vault locks itself after a while without use. A call that finds it
// locked asks for the password, wherever the person is, and then goes on.

/** What the backend says when the vault is locked. */
export const LOCKED = "the vault is locked";

export class Unlocker {
  #ask: () => Promise<boolean>;
  #asking: Promise<boolean> | null = null;

  /** `ask` shows the password question; true when the vault was unlocked. */
  constructor(ask: () => Promise<boolean>) {
    this.#ask = ask;
  }

  /** Runs `call`; when the vault is locked, asks once and runs it again. */
  async run<T>(call: () => Promise<T>): Promise<T> {
    try {
      return await call();
    } catch (err) {
      if (String(err) !== LOCKED) throw err;
      // Calls that find it locked together wait on the same question.
      this.#asking ??= this.#ask().finally(() => {
        this.#asking = null;
      });
      if (!(await this.#asking)) throw err;
      return await call();
    }
  }
}
