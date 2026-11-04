// The IDE's preview and design canvas draw with Clarity.UI (DESIGN 8.1),
// the library every generated program draws with, compiled for the browser.
// The IDE's backend serves its browser files, the project's windows
// (files.json) and a program answering with sample data
// (clarity_gen::preview); this module finds that server and loads the
// library's page script from it, once.

import { invoke } from "@tauri-apps/api/core";

/** What the page script gives for a canvas (runtimes/rust-ui-browser/js/clarity-ui-page.js). */
export interface ClarityUi {
  started: Promise<void>;
  step(step: object): Promise<Record<string, unknown>>;
  shot(): Promise<string | null>;
  /** Starts the program again from new files on the same canvas: null, or why it could not. */
  reload(files: string): Promise<string | null>;
  layout(): Promise<Layout>;
  stop(): void;
}

/** A part Clarity.UI drew, with where it is written in the project (`item`). */
export type LaidPart = { item: string | null; name: string | null; kind: string; at: [number, number, number, number] };
/** A grid Clarity.UI laid out: what holds it, its columns, its place and the height of each row. */
export type LaidGrid = { item: string | null; columns: number; gap: number; at: [number, number, number, number]; rows: number[] };
export type Layout = { parts: LaidPart[]; grids: LaidGrid[] };

/** What the IDE shows: a window, or an app's frame around an empty window. */
export type Shown = { window: string } | { app: string };

/** Options of the page script for one canvas. */
export type StartOptions = { files: string; server: string; resources: string; input: boolean; storage: string; choices: string };

declare global {
  interface Window {
    ClarityUiPage?: { start(canvas: HTMLCanvasElement, options: StartOptions): ClarityUi };
  }
}

let address: Promise<string> | null = null;
let script: Promise<NonNullable<Window["ClarityUiPage"]>> | null = null;

/** Where the preview server serves, ending in `/`. */
export function previewAddress(): Promise<string> {
  address ??= invoke<string>("preview_server_address");
  address.catch(() => (address = null));
  return address;
}

/** The library's page script, from the preview server. */
export function pageScript(base: string): Promise<NonNullable<Window["ClarityUiPage"]>> {
  script ??= new Promise((resolve, reject) => {
    if (window.ClarityUiPage) return resolve(window.ClarityUiPage);
    const element = document.createElement("script");
    element.src = `${base}clarity-ui-page.js`;
    element.onload = () => (window.ClarityUiPage ? resolve(window.ClarityUiPage) : reject(new Error("Clarity.UI's page script gave nothing")));
    element.onerror = () => reject(new Error(`Clarity.UI's page script did not load from ${base}`));
    document.head.appendChild(element);
  });
  script.catch(() => (script = null));
  return script;
}

/** The address of files.json for what is shown, new for each change (`revision`), so nothing in between keeps an old one. */
export function filesAddress(base: string, shown: Shown, design: boolean, revision: number): string {
  const what = "app" in shown ? `app=${encodeURIComponent(shown.app)}` : `window=${encodeURIComponent(shown.window)}`;
  return `${base}files.json?${what}&design=${design ? 1 : 0}&revision=${revision}`;
}

/** The page script's options for a canvas of the preview (`design` false) or of the designer. */
export function startOptions(base: string, shown: Shown, design: boolean, revision: number): StartOptions {
  return {
    files: filesAddress(base, shown, design, revision),
    server: `${base}invoke`,
    resources: `${base}resources/`,
    // On the design canvas the drawn parts take no input: the designer's layer does.
    input: !design,
    // The preview keeps its choices apart from the IDE's own.
    storage: "clarity.preview.settings",
    choices: "clarity.preview.choices",
  };
}
