// How the renderer reaches the backend of the application it runs in: with
// Tauri's invoke for Rust (and in the designer), or through the Clarity
// service of the Go shell over Wails. The commands and their arguments are
// the same on both; only the way there differs.

import { invoke as tauriInvoke } from "@tauri-apps/api/core";

/** What the Go shell's Invoke gives: the value, or what the screen shows as the error. */
export type Answer = { value?: unknown; error?: unknown };

/** The Go shell's method, as Wails names bound methods: package, type, method. */
export const WAILS_METHOD = "main.Clarity.Invoke";

/** Where Wails serves its JavaScript runtime, matched to its Go version. */
const WAILS_RUNTIME = "/wails/runtime.js";

type WailsRuntime = { Call: { ByName(name: string, ...args: unknown[]): Promise<Answer> } };

export function isTauri(global: object): boolean {
  return "__TAURI_INTERNALS__" in global;
}

/** The value of an answer, or its error thrown as the Tauri shell throws it. */
export function unwrap<T>(answer: Answer): T {
  if (answer.error !== undefined && answer.error !== null) throw answer.error;
  return answer.value as T;
}

let wails: Promise<WailsRuntime> | null = null;

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri(globalThis)) return tauriInvoke<T>(command, args);
  wails ??= import(/* @vite-ignore */ WAILS_RUNTIME) as Promise<WailsRuntime>;
  const { Call } = await wails;
  return unwrap<T>(await Call.ByName(WAILS_METHOD, command, args));
}
