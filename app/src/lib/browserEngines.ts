// The engines beside the screen of C# with Avalonia in a browser (DESIGN
// 7.6), where Wasmtime cannot run inside .NET: the browser's own
// WebAssembly, on the page's thread, so each call .NET makes is answered at
// once and the canvas's frame loop is the desktop's. An engine that runs
// away holds the page up, since nothing can stop a call on its own thread.

import { loadEngineNow, type Engine, type Input } from "./engineHost.ts";

const engines = new Map<number, Engine>();
let next = 0;
let language = "";

/** A fresh seed for every call, as every host gives one. */
function seed(): bigint {
  const words = crypto.getRandomValues(new Uint32Array(2));
  return (BigInt(words[0]) << 32n) | BigInt(words[1]);
}

/** The language the screen picked, which engines ask for. */
export function setLanguage(tag: string) {
  language = tag;
}

/** Loads engine `name` from its file in base64 and its declaration as JSON; its number. */
export function load(name: string, wasm: string, declared: string): number {
  const bytes = Uint8Array.from(atob(wasm), (c) => c.charCodeAt(0));
  const services = { clock: () => Date.now(), seed, log: (line: string) => console.info(line), language: () => language };
  const engine = loadEngineNow(name, bytes, { ...JSON.parse(declared), screen: true }, services);
  engines.set(++next, engine);
  return next;
}

/** The pointer and keys the engine takes, as JSON. */
export function hears(engine: number): string {
  return JSON.stringify(engines.get(engine)?.hears ?? { pointer: false, keys: false });
}

/** One frame: its shapes and what the engine told, as JSON; it throws what failed. */
export function frame(engine: number, ms: number, input: string): string {
  const running = engines.get(engine);
  if (!running) throw new Error("the engine has stopped");
  return JSON.stringify(running.frame(ms, JSON.parse(input) as Input[]));
}

export function close(engine: number) {
  engines.delete(engine);
}
