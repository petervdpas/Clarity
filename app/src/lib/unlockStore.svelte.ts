// The password question that shows over any tab when a call finds the
// vault locked. The call waits for it and goes on once the vault is open.

import { invoke } from "@tauri-apps/api/core";
import { Unlocker } from "./unlock";

class UnlockStore {
  asking = $state(false);
  error = $state<string | null>(null);
  busy = $state(false);
  #answer: ((unlocked: boolean) => void) | null = null;
  #unlocked: (() => void)[] = [];

  #unlocker = new Unlocker(
    () =>
      new Promise<boolean>((resolve) => {
        this.error = null;
        this.asking = true;
        this.#answer = resolve;
      }),
  );

  /** Runs a backend command, asking for the password when the vault is locked. */
  invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
    return this.#unlocker.run(() => invoke<T>(command, args));
  }

  /** `then` runs every time the question opens the vault. */
  onUnlocked(then: () => void) {
    this.#unlocked.push(then);
  }

  async submit(password: string) {
    this.busy = true;
    this.error = null;
    try {
      await invoke("vault_unlock", { password });
    } catch (err) {
      this.error = String(err);
      return;
    } finally {
      this.busy = false;
    }
    this.#close(true);
    for (const then of this.#unlocked) then();
  }

  cancel() {
    this.#close(false);
  }

  #close(unlocked: boolean) {
    this.asking = false;
    this.#answer?.(unlocked);
    this.#answer = null;
  }
}

export const unlock = new UnlockStore();
