// How the renderer reaches the backend of the application it runs in: with
// Tauri's invoke for Rust (and in the designer), through the Clarity
// service of the Go shell over Wails, through the function the C and C++
// shells bind into the page with webview, or over HTTP when the application
// is a web server (the web target, DESIGN 7.3). The commands and their
// arguments are the same on all; only the way there differs.

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
export function transport(global: object): "tauri" | "webview" | "wails" | "server" {
  if (isTauri(global)) return "tauri";
  // A server that serves the page says where it takes commands.
  if ("clarityServer" in global) return "server";
  if ("clarityInvoke" in global) return "webview";
  return "wails";
}

let wails: Promise<WailsRuntime> | null = null;

/** Opens a link of a markdown part in the person's browser, through the application's shell. */
export function openLink(address: string) {
  invoke("open_link", { address }).catch((err) => console.error(`cannot open ${address}: ${err}`));
}

/**
 * A line of an engine beside the screen, into the application's log through
 * the shell, where the backend's engines write; the page's console when the
 * shell cannot take it.
 */
export function logLine(line: string): Promise<void> {
  return invoke<null>("log_line", { line }).then(
    () => undefined,
    () => console.info(line),
  );
}

/** What a page a server serves does in the browser itself: its tab, links, files and the person's settings. */
export interface BrowserPage {
  setTitle(title: string): void;
  /** Opens an address in a new tab. */
  open(address: string): void;
  /** Gives the person a file to save, its contents in base64. */
  download(name: string, data: string): void;
  /** Shows a file in a new tab, as the system would open it. */
  show(name: string, data: string): void;
  read(key: string): string | null;
  write(key: string, value: string): void;
}

/** Where the browser keeps the person's settings. */
const SETTINGS_KEY = "clarity-settings";

/**
 * A command a page a server serves answers in the browser: the window's
 * title is the tab's, a link opens in a new tab, a file is saved as a
 * download, and the person's settings stay in their browser, without the
 * secret ones. Null for a command the server answers.
 */
export function inBrowser(command: string, args: Record<string, unknown>, page: BrowserPage): Answer | null {
  switch (command) {
    case "set_title":
      page.setTitle(String(args.title ?? ""));
      return { value: null };
    case "open_link": {
      const address = String(args.address ?? "");
      if (!["http://", "https://", "mailto:"].some((scheme) => address.startsWith(scheme))) {
        return { error: `a link opens only an http, https or mailto address, not ${address}` };
      }
      page.open(address);
      return { value: null };
    }
    case "save_file":
      page.download(String(args.name ?? ""), String(args.data ?? ""));
      return { value: String(args.name ?? "") };
    case "open_file":
      page.show(String(args.name ?? ""), String(args.data ?? ""));
      return { value: null };
    case "quit_app":
      return { value: null };
    case "read_settings": {
      const kept = page.read(SETTINGS_KEY);
      return { value: kept === null ? null : JSON.parse(kept) };
    }
    case "keep_settings": {
      const secrets = new Set((args.secrets as string[] | undefined) ?? []);
      const settings = Object.fromEntries(Object.entries((args.settings as Record<string, unknown>) ?? {}).filter(([name]) => !secrets.has(name)));
      page.write(SETTINGS_KEY, JSON.stringify(settings));
      return { value: null };
    }
    default:
      return null;
  }
}

/** The browser as a page a server serves uses it. */
function browserPage(): BrowserPage {
  const blob = (name: string, data: string) => {
    const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
    return URL.createObjectURL(new Blob([bytes]));
  };
  return {
    setTitle: (title) => (document.title = title),
    open: (address) => window.open(address, "_blank", "noopener"),
    download: (name, data) => {
      const link = document.createElement("a");
      link.href = blob(name, data);
      link.download = name;
      link.click();
    },
    show: (name, data) => window.open(blob(name, data), "_blank", "noopener"),
    read: (key) => {
      try {
        return localStorage.getItem(key);
      } catch {
        return null;
      }
    },
    write: (key, value) => {
      try {
        localStorage.setItem(key, value);
      } catch {
        // A browser that keeps nothing keeps no settings.
      }
    },
  };
}

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const shell = transport(globalThis);
  if (shell === "tauri") return tauriInvoke<T>(command, args);
  if (shell === "server") {
    const local = inBrowser(command, args, browserPage());
    if (local) return unwrap<T>(local);
    const address = (globalThis as unknown as { clarityServer: string }).clarityServer;
    const reply = await fetch(address, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ command, args }) });
    return unwrap<T>((await reply.json()) as Answer);
  }
  if (shell === "webview") {
    const call = (globalThis as unknown as { clarityInvoke: WebviewInvoke }).clarityInvoke;
    return unwrap<T>(await call(command, args));
  }
  wails ??= import(/* @vite-ignore */ WAILS_RUNTIME) as Promise<WailsRuntime>;
  const { Call } = await wails;
  return unwrap<T>(await Call.ByName(WAILS_METHOD, command, args));
}
