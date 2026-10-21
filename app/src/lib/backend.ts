// How the renderer reaches the backend of the application it runs in: with
// Tauri's invoke for Rust (and in the designer), through the Clarity
// service of the Go shell over Wails, or through the function the C and C++
// shells bind into the page with webview. The commands and their arguments
// are the same on all; only the way there differs.

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

/** The function the C and C++ shells bind into the page: a command and its arguments, answered like the Go shell. */
type WebviewInvoke = (command: string, args: Record<string, unknown>) => Promise<Answer>;

/** Which shell the page runs in, by what that shell puts in it. */
export function transport(global: object): "tauri" | "webview" | "wails" {
  if (isTauri(global)) return "tauri";
  if ("clarityInvoke" in global) return "webview";
  return "wails";
}

let wails: Promise<WailsRuntime> | null = null;

/** Opens a link of a markdown part in the person's browser, through the application's shell. */
export function openLink(address: string) {
  invoke("open_link", { address }).catch((err) => console.error(`cannot open ${address}: ${err}`));
}

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const shell = transport(globalThis);
  if (shell === "tauri") return tauriInvoke<T>(command, args);
  if (shell === "webview") {
    const call = (globalThis as unknown as { clarityInvoke: WebviewInvoke }).clarityInvoke;
    return unwrap<T>(await call(command, args));
  }
  wails ??= import(/* @vite-ignore */ WAILS_RUNTIME) as Promise<WailsRuntime>;
  const { Call } = await wails;
  return unwrap<T>(await Call.ByName(WAILS_METHOD, command, args));
}
