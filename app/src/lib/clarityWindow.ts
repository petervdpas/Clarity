// The IDE's preview and design canvas draw with Clarity.UI (DESIGN 8.1),
// the library every generated program draws with, compiled for the browser.
// The IDE's backend serves its browser files, the project's windows
// (files.json) and a program answering with sample data
// (clarity_gen::preview); this module finds that server and loads the
// library's page script from it, once (again after its files were built
// again, resetPageScript).

import { invoke } from "@tauri-apps/api/core";

/** What the page script gives for a canvas (runtimes/rust-ui-browser/js/clarity-ui-page.js). */
export interface ClarityUi {
  started: Promise<void>;
  step(step: object): Promise<Record<string, unknown>>;
  shot(): Promise<string | null>;
  /** Starts the program again from new files on the same canvas: null, or why it could not. */
  reload(files: string, chosen?: Chosen): Promise<string | null>;
  layout(): Promise<Layout>;
  stop(): void;
}

/** A part Clarity.UI drew, with where it is written in the project (`item`). */
export type Place = [number, number, number, number];
/** A part Clarity.UI drew, with where it is written in the project (`item`); a frame's button is of kind `command`. */
export type LaidPart = {
  item: string | null;
  name: string | null;
  kind: string;
  at: Place;
  /** A tabs, steps or accordion part's pages, each with the header that shows it, where it has one. */
  pages?: { item: string | null; title: string | null; at: Place | null }[];
  /** A table's columns, each where its header is. */
  columns?: { field: string; at: Place }[];
};
/** A grid Clarity.UI laid out: what holds it, its columns, its place and the height of each row. */
export type LaidGrid = { item: string | null; columns: number; gap: number; at: [number, number, number, number]; rows: number[] };
export type Layout = { parts: LaidPart[]; grids: LaidGrid[] };

/** What the IDE shows: a window, or an app's frame around an empty window. */
export type Shown = { window: string } | { app: string };

/** Options of the page script for one canvas. */
export type StartOptions = { files: string; server: string; resources: string; input: boolean; access: boolean; storage: string; choices: string; chosen: Chosen };

/** The person's choices the drawing starts with, over the kept ones: the IDE's language. */
export type Chosen = { language?: string };

/** What the drawing shows beyond the saved window: the page each container shows, by its key; the draft; the popovers open. */
export type Showing = { pages?: Record<string, number>; draft?: boolean; open?: string[] };

/**
 * The language the IDE shows a window in, as the drawing's choice: the
 * system's (`undefined`, the drawing's own), as written (`null`), or a translation.
 */
export function chosenOf(language: string | null | undefined, written: string): Chosen {
  if (language === undefined) return {};
  return { language: language ?? written };
}

declare global {
  interface Window {
    ClarityUiPage?: { start(canvas: HTMLCanvasElement, options: StartOptions): ClarityUi };
    /** The worker script's face on the page's thread. */
    ClarityUiPageWorker?: unknown;
    /** The library's module maker (clarity-ui.js), on the page's thread. */
    ClarityUi?: unknown;
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

/** Clarity.UI's scripts on the page: the page script, the worker script (run on the page's thread where a worker cannot draw) and the library. */
const LIBRARY_SCRIPTS = ['script[src$="clarity-ui-page.js"]', 'script[src$="clarity-ui-worker.js"]', 'script[src$="clarity-ui.js"]'] as const;

/** Forgets Clarity.UI's scripts, so the next canvas loads them all again from the server: after its files were built again. An old library kept would start the new wasm. */
export function resetPageScript() {
  script = null;
  // A script's global may not be deletable (Emscripten's `var ClarityUi`): it is blanked instead.
  for (const name of ["ClarityUiPage", "ClarityUiPageWorker", "ClarityUi"] as const) {
    try {
      delete window[name];
    } catch {
      window[name] = undefined;
    }
    if (window[name] !== undefined) window[name] = undefined;
  }
  for (const selector of LIBRARY_SCRIPTS) for (const element of document.querySelectorAll(selector)) element.remove();
}

/** The address of files.json for what is shown, new for each change (`revision`), so nothing in between keeps an old one. */
export function filesAddress(base: string, shown: Shown, design: boolean, revision: number, showing: Showing = {}): string {
  const what = "app" in shown ? `app=${encodeURIComponent(shown.app)}` : `window=${encodeURIComponent(shown.window)}`;
  const pages = Object.entries(showing.pages ?? {})
    .map(([key, page]) => `${encodeURIComponent(key)}.${page}`)
    .join(",");
  const more = [pages ? `&pages=${pages}` : "", showing.draft ? "&draft=1" : "", showing.open?.length ? `&open=${showing.open.map(encodeURIComponent).join(",")}` : ""].join("");
  return `${base}files.json?${what}&design=${design ? 1 : 0}${more}&revision=${revision}`;
}

/** The page script's options for a canvas of the preview (`design` false) or of the designer. */
export function startOptions(base: string, shown: Shown, design: boolean, revision: number, showing: Showing = {}, chosen: Chosen = {}): StartOptions {
  return {
    files: filesAddress(base, shown, design, revision, showing),
    chosen,
    server: `${base}invoke`,
    resources: `${base}resources/`,
    // On the design canvas the drawn parts take no input: the designer's layer does.
    input: !design,
    // Screen readers read the preview through the page's accessible mirror; the design canvas has none.
    access: !design,
    // The preview keeps its choices apart from the IDE's own.
    storage: "clarity.preview.settings",
    choices: "clarity.preview.choices",
  };
}
